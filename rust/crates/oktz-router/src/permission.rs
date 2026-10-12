//! Port `checkPermission` dari `src/lib/middleware.js:34-139`.
//!
//! Urutan aturan itu load-bearing: JS pakai rantai `return`, jadi aturan
//! pertama yang gagal adalah alasan yang dibalas user. Menukar urutan mengubah
//! pesan balasan — bug yang sulit ketahuan karena masih "jalan".
//!
//! Divergensi yang disengaja (catat di test):
//! - `hasAccess` di JS menulis ke DB (prune grant expired). Di sini murni
//!   terhadap `&UserState`; prune jadi urusan caller.

use oktz_plugin_api::{PluginMeta, QuotedMessage};

/// Identitas caller — cerminan flag `m.*` yang dipakai checkPermission.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Caller {
    pub sender: String,
    pub chat: String,
    /// `m.command` — sudah lowercase. Dipakai gate botMode untuk exempt check.
    pub command: String,
    pub is_group: bool,
    pub is_owner: bool,
    pub is_partner: bool,
    pub is_premium: bool,
    pub is_admin: bool,
    pub is_bot_admin: bool,
    /// `m.mentionedJid` — router meneruskannya apa adanya ke `Ctx`.
    pub mentioned: Vec<String>,
    /// `m.quoted` — pesan yang di-reply, diteruskan ke `Ctx`.
    pub quoted: Option<QuotedMessage>,
}

/// Grant per-command dari `user.access[]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccessGrant {
    pub cmd: String,
    /// `None` = tidak pernah kedaluwarsa (JS: `expired === null`).
    pub expired: Option<i64>,
}

/// State user yang dibaca checkPermission.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UserState {
    pub access: Vec<AccessGrant>,
    pub is_registered: bool,
}

/// Setting grup yang dipakai aturan 8/9.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct GroupState {
    /// `None` = grup tidak ada di DB. JS: `if (group) { ... }` — jadi
    /// `None` berarti aturan 8/9 DILEWATI, bukan di-tolak.
    pub game: Option<bool>,
    pub rpg: Option<bool>,
}

// Override `isPremium` per-plugin dari `db.setting("capprem")`.
#[derive(Debug, Clone)]
pub struct PermissionInput<'a> {
    pub meta: &'a PluginMeta,
    /// `now_ms` untuk cek `expired`. JS pakai `Date.now()`.
    pub now_ms: i64,
    /// Override per-plugin; `Some` menang atas `meta.is_premium`.
    pub capprem: Option<bool>,
    pub group: Option<GroupState>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DenyReason {
    OwnerOnly,
    PartnerOnly,
    PremiumOnly,
    GroupOnly,
    PrivateOnly,
    AdminOnly,
    BotAdminOnly,
    GameDisabled,
    RpgDisabled,
}

impl DenyReason {
    /// Balasan yang router kirim. String persis seperti JS supaya output
    /// tidak berubah bagi user. Tidak ada placeholder — pesan permission JS
    /// juga statis.
    pub fn message(&self) -> &'static str {
        match self {
            Self::OwnerOnly => "🚫 Owner only!",
            Self::PartnerOnly => "🤝 Partner only!",
            Self::PremiumOnly => "💎 Premium only!",
            Self::GroupOnly => "👥 Group only!",
            Self::PrivateOnly => "📱 Private chat only!",
            Self::AdminOnly => "👮 Admin grup only!",
            Self::BotAdminOnly => "🤖 Bot harus menjadi admin grup!",
            Self::GameDisabled => "🎮 Fitur Game sedang dinonaktifkan di grup ini oleh Admin!",
            Self::RpgDisabled => "⚔️ Fitur RPG sedang dinonaktifkan di grup ini oleh Admin!",
        }
    }
}

/// `true` kalau caller punya grant akses aktif untuk command ini.
fn has_access(user: &UserState, command: &str, now_ms: i64) -> bool {
    user.access
        .iter()
        .find(|a| a.cmd.eq_ignore_ascii_case(command))
        .is_some_and(|a| a.expired.is_none_or(|exp| exp > now_ms))
}

/// Port persis `checkPermission` (middleware.js:34-139).
pub fn check(
    caller: &Caller,
    user: &UserState,
    input: &PermissionInput<'_>,
) -> Result<(), DenyReason> {
    let m = input.meta;
    let access = has_access(user, &m.name, input.now_ms);

    if m.is_owner && !caller.is_owner && !access {
        return Err(DenyReason::OwnerOnly);
    }

    if m.is_partner && !caller.is_partner && !caller.is_owner && !access {
        return Err(DenyReason::PartnerOnly);
    }

    // `capprem[name]` menang atas `isPremium` — hanya kalau key-nya ADA.
    // `Option<bool>` dipakai supaya `false` (override implisit) tidak dianggap
    // "tidak ada override" seperti yang akan terjadi kalau pakai `map_or`.
    let is_premium_feature = input.capprem.unwrap_or(m.is_premium);
    if is_premium_feature && !caller.is_premium && !caller.is_owner && !caller.is_partner && !access
    {
        return Err(DenyReason::PremiumOnly);
    }

    // Aturan 4/5 tidak punya bypass sama sekali — owner pun ditolak.
    if m.is_group && !caller.is_group {
        return Err(DenyReason::GroupOnly);
    }
    if m.is_private && caller.is_group {
        return Err(DenyReason::PrivateOnly);
    }

    if m.is_admin && caller.is_group && !caller.is_admin && !caller.is_owner && !access {
        return Err(DenyReason::AdminOnly);
    }

    // JS: tidak ada bypass `isOwner`/`hasAccess` di aturan botAdmin.
    if m.is_bot_admin && caller.is_group && !caller.is_bot_admin {
        return Err(DenyReason::BotAdminOnly);
    }

    // Aturan 8/9 hanya jalan kalau `db.getGroup(chat)` mengembalikan objek.
    if caller.is_group
        && let Some(group) = input.group
    {
        let elevated = caller.is_admin || caller.is_owner || access;
        if !elevated {
            if m.category == "game" && group.game == Some(false) {
                return Err(DenyReason::GameDisabled);
            }
            if m.category == "rpg" && group.rpg == Some(false) {
                return Err(DenyReason::RpgDisabled);
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn meta() -> PluginMeta {
        PluginMeta::default()
    }

    fn input<'a>(m: &'a PluginMeta, group: Option<GroupState>) -> PermissionInput<'a> {
        PermissionInput {
            meta: m,
            now_ms: 1_000_000,
            capprem: None,
            group,
        }
    }

    fn caller() -> Caller {
        Caller {
            sender: "628123@s.whatsapp.net".into(),
            chat: "628123@s.whatsapp.net".into(),
            ..Default::default()
        }
    }

    #[test]
    fn plain_plugin_is_allowed_for_anyone() {
        let m = meta();
        assert_eq!(check(&caller(), &UserState::default(), &input(&m, None)), Ok(()));
    }

    #[test]
    fn owner_only_denies_normal_user() {
        let mut m = meta();
        m.is_owner = true;
        assert_eq!(
            check(&caller(), &UserState::default(), &input(&m, None)),
            Err(DenyReason::OwnerOnly)
        );
    }

    #[test]
    fn owner_only_allows_owner() {
        let mut m = meta();
        m.is_owner = true;
        let mut c = caller();
        c.is_owner = true;
        assert_eq!(check(&c, &UserState::default(), &input(&m, None)), Ok(()));
    }

    #[test]
    fn unexpired_grant_bypasses_premium() {
        let mut m = meta();
        m.is_premium = true;
        let user = UserState {
            access: vec![AccessGrant {
                cmd: "m".into(),
                expired: Some(1_000_001),
            }],
            ..Default::default()
        };
        m.name = "m".into();
        assert_eq!(check(&caller(), &user, &input(&m, None)), Ok(()));
    }

    #[test]
    fn expired_grant_does_not_bypass() {
        let mut m = meta();
        m.is_premium = true;
        m.name = "m".into();
        let user = UserState {
            access: vec![AccessGrant {
                cmd: "m".into(),
                expired: Some(999_999),
            }],
            ..Default::default()
        };
        assert_eq!(
            check(&caller(), &user, &input(&m, None)),
            Err(DenyReason::PremiumOnly)
        );
    }

    #[test]
    fn null_expiry_grant_never_expires() {
        let mut m = meta();
        m.is_owner = true;
        m.name = "m".into();
        let user = UserState {
            access: vec![AccessGrant {
                cmd: "m".into(),
                expired: None,
            }],
            ..Default::default()
        };
        assert_eq!(check(&caller(), &user, &input(&m, None)), Ok(()));
    }

    #[test]
    fn capprem_false_overrides_is_premium_true() {
        let mut m = meta();
        m.is_premium = true;
        let mut i = input(&m, None);
        i.capprem = Some(false);
        // Caller non-premium: tanpa override ditolak, dengan override lolos.
        assert_eq!(
            check(&caller(), &UserState::default(), &i),
            Ok(()),
            "capprem=false harus mematikan gate premium"
        );
    }

    #[test]
    fn group_only_has_no_bypass_even_for_owner() {
        let mut m = meta();
        m.is_group = true;
        m.is_private = false;
        let mut c = caller();
        c.is_owner = true;
        c.is_group = false;
        assert_eq!(
            check(&c, &UserState::default(), &input(&m, None)),
            Err(DenyReason::GroupOnly),
            "JS: aturan isGroup tidak punya bypass owner"
        );
    }

    #[test]
    fn private_only_has_no_bypass_even_for_owner() {
        let mut m = meta();
        m.is_private = true;
        let mut c = caller();
        c.is_owner = true;
        c.is_group = true;
        assert_eq!(
            check(&c, &UserState::default(), &input(&m, None)),
            Err(DenyReason::PrivateOnly)
        );
    }

    #[test]
    fn bot_admin_rule_ignores_owner() {
        let mut m = meta();
        m.is_bot_admin = true;
        let mut c = caller();
        c.is_group = true;
        c.is_owner = true;
        c.is_bot_admin = false;
        assert_eq!(
            check(&c, &UserState::default(), &input(&m, None)),
            Err(DenyReason::BotAdminOnly),
            "JS tidak memberi bypass owner di aturan botAdmin"
        );
    }

    #[test]
    fn game_rule_needs_group_record_to_apply() {
        let mut m = meta();
        m.category = "game".into();
        m.is_private = false;
        let mut c = caller();
        c.is_group = true;
        // Grup tidak ada di DB -> aturan dilewati.
        assert_eq!(check(&c, &UserState::default(), &input(&m, None)), Ok(()));
        // Grup ada dengan game=false -> ditolak.
        let g = Some(GroupState {
            game: Some(false),
            rpg: None,
        });
        assert_eq!(
            check(&c, &UserState::default(), &input(&m, g)),
            Err(DenyReason::GameDisabled)
        );
    }

    #[test]
    fn game_rule_lets_admin_through() {
        let mut m = meta();
        m.category = "game".into();
        m.is_private = false;
        let mut c = caller();
        c.is_group = true;
        c.is_admin = true;
        let g = Some(GroupState {
            game: Some(false),
            rpg: None,
        });
        assert_eq!(check(&c, &UserState::default(), &input(&m, g)), Ok(()));
    }

    #[test]
    fn only_explicit_false_disables_the_toggle() {
        // `game: Some(true)` dan `None` (field tak ada) keduanya BOLEH.
        let mut m = meta();
        m.category = "game".into();
        m.is_private = false;
        let mut c = caller();
        c.is_group = true;
        for g in [None, Some(true)] {
            let g = GroupState {
                game: g,
                rpg: None,
            };
            assert_eq!(check(&c, &UserState::default(), &input(&m, Some(g))), Ok(()));
        }
    }

    #[test]
    fn rule_order_owner_beats_group_only() {
        // Plugin owner-only + group-only, dipanggil dari private oleh user biasa:
        // aturan owner (1) harus menang, bukan group-only (4).
        let mut m = meta();
        m.is_owner = true;
        m.is_group = true;
        assert_eq!(
            check(&caller(), &UserState::default(), &input(&m, None)),
            Err(DenyReason::OwnerOnly)
        );
    }

    #[test]
    fn deny_messages_are_non_empty_and_emoji_preserved() {
        for r in [
            DenyReason::OwnerOnly,
            DenyReason::PremiumOnly,
            DenyReason::GameDisabled,
        ] {
            assert!(!r.message().is_empty());
        }
        assert!(DenyReason::GameDisabled.message().contains("🎮"));
    }
}