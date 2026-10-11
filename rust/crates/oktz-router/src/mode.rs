//! Port `checkMode` (`src/lib/middleware.js:141-268`) + gating `botMode` per-grup
//! (`src/handler.js:1709-1808`).
//!
//! Dua mekanisme berbeda dengan nama mirip — pertahankan pemisahan ini:
//!
//!   - `check_mode`: mode AKSES global (`public`/`self`, AFK, only-group).
//!   - `group_feature_mode`: mode FITUR per-grup (`botMode` = all/md/cpanel/…).
//!
//! Mencampur keduanya adalah sumber bug yang sulit dicari.

use oktz_plugin_api::PluginMeta;

use crate::permission::{Caller, GroupState};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BotMode {
    #[default]
    Public,
    SelfOnly,
    /// Nilai tak dikenal — JS `return {allowed:true}` di akhir (fail-open).
    Unknown,
}

impl BotMode {
    pub fn parse(s: &str) -> Self {
        match s {
            "public" => Self::Public,
            "self" => Self::SelfOnly,
            _ => Self::Unknown,
        }
    }
}

/// Setting global yang dibaca `checkMode`.
#[derive(Debug, Clone, Default)]
pub struct ModeSettings {
    pub mode: String,
    pub afk_active: bool,
    pub only_gc: bool,
    pub only_pc: bool,
    /// `Some(jid)` = hanya grup ini. String di JS; object di-normalkan ke jid
    /// di sisi caller (nama/pesan butuh data object, tidak ada di sini).
    pub only_this_group: Option<String>,
    pub self_groups: Vec<String>,
    pub public_groups: Vec<String>,
    pub self_admin: bool,
    pub public_admin: bool,
    pub only_admin: bool,
}

/// Setting grup yang bisaoverride global (`groupFlags.X ?? db.setting("X")`).
#[derive(Debug, Clone, Copy, Default)]
pub struct GroupOverrides {
    pub self_admin: Option<bool>,
    pub public_admin: Option<bool>,
    pub only_admin: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DenyMode {
    Silent,
    Afk,
    OnlyThisGroup,
    SelfGroup,
}

/// Aturan 1-10 `checkMode`, urutan preservasi.
pub fn check_mode(
    caller: &Caller,
    settings: &ModeSettings,
    group_overrides: Option<GroupOverrides>,
) -> Result<(), DenyMode> {
    let mode = BotMode::parse(if settings.mode.is_empty() {
        "public"
    } else {
        &settings.mode
    });

    // `??` semantics: override grup yang eksplisit menang, termasuk `false`.
    let resolve = |global: bool, group: Option<bool>| group.unwrap_or(global);
    let (self_admin, public_admin, only_admin) = match group_overrides {
        Some(g) => (
            resolve(settings.self_admin, g.self_admin),
            resolve(settings.public_admin, g.public_admin),
            resolve(settings.only_admin, g.only_admin),
        ),
        None => (
            settings.self_admin,
            settings.public_admin,
            settings.only_admin,
        ),
    };

    // 1. AFK — hanya owner lolos.
    if settings.afk_active && !caller.is_owner {
        return Err(DenyMode::Afk);
    }

    // 2/3. onlyGc / onlyPc.
    if settings.only_gc && !caller.is_group && !caller.is_owner {
        return Err(DenyMode::Silent);
    }
    if settings.only_pc && caller.is_group && !caller.is_owner {
        return Err(DenyMode::Silent);
    }

    // 4. onlyThisGroup.
    if let Some(only) = &settings.only_this_group
        && caller.is_group
        && !caller.is_owner
        && &caller.chat != only
    {
        return Err(DenyMode::OnlyThisGroup);
    }

    // 5. selfGroups: owner lolos, selain itu tolok diam.
    if caller.is_group && settings.self_groups.contains(&caller.chat) && !caller.is_owner {
        return Err(DenyMode::SelfGroup);
    }

    // 6. publicGroups: short-circuit sebelum cek mode.
    if caller.is_group && settings.public_groups.contains(&caller.chat) {
        return Ok(());
    }

    match mode {
        // 7. mode self: hanya owner.
        BotMode::SelfOnly => {
            if caller.is_owner {
                Ok(())
            } else {
                Err(DenyMode::Silent)
            }
        }
        // 8/9/10. mode public + variant admin.
        BotMode::Public => {
            if only_admin {
                if caller.is_owner || !caller.is_group || caller.is_admin {
                    Ok(())
                } else {
                    Err(DenyMode::Silent)
                }
            } else if self_admin {
                if caller.is_owner || (caller.is_group && caller.is_admin) {
                    Ok(())
                } else {
                    Err(DenyMode::Silent)
                }
            } else if public_admin {
                if caller.is_owner || !caller.is_group || caller.is_admin {
                    Ok(())
                } else {
                    Err(DenyMode::Silent)
                }
            } else {
                Ok(())
            }
        }
        // JS jatuh-through `return {allowed:true}` untuk mode tak dikenal.
        BotMode::Unknown => Ok(()),
    }
}

// ---------- Gate botMode per-grup (handler.js:1709-1808) ----------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupBotMode {
    All,
    Md,
    Cpanel,
    Pushkontak,
    Store,
    Otp,
}

impl GroupBotMode {
    pub fn parse(s: &str) -> Self {
        match s {
            "md" => Self::Md,
            "cpanel" => Self::Cpanel,
            "pushkontak" => Self::Pushkontak,
            "store" => Self::Store,
            "otp" => Self::Otp,
            // `modeConfig[botMode] || modeConfig.all` — permissive.
            _ => Self::All,
        }
    }

    fn display_name(&self) -> &'static str {
        match self {
            Self::All => "All Features",
            Self::Md => "Multi Device",
            Self::Cpanel => "CPanel",
            Self::Pushkontak => "Push Kontak",
            Self::Store => "Store",
            Self::Otp => "OTP",
        }
    }

    /// `modeConfig[mode].excluded` — hanya `md` yang punya deny-list.
    fn excluded(&self) -> Option<&'static [&'static str]> {
        match self {
            Self::Md => Some(&["pushkontak", "store", "panel", "otp"]),
            _ => None,
        }
    }

    /// `modeConfig[mode].allowed` — mode selain `all`/`md` pakai allow-list.
    fn allowed(&self) -> Option<Vec<&'static str>> {
        let base = ["main", "group", "sticker", "owner"];
        match self {
            Self::All | Self::Md => None,
            Self::Cpanel => Some(base.iter().copied().chain(["tools", "panel"]).collect()),
            Self::Pushkontak => Some(base.iter().copied().chain(["pushkontak"]).collect()),
            Self::Store => Some(base.iter().copied().chain(["store"]).collect()),
            Self::Otp => Some(base.iter().copied().chain(["otp"]).collect()),
        }
    }
}

/// `categoryModeMap` (16 entri) — untuk saran mode di pesan denied.
fn suggested_mode(category: &str) -> &'static str {
    match category {
        "download" | "search" | "ai" | "fun" | "game" | "media" | "utility" | "tools"
        | "ephoto" | "religi" | "info" | "jpm" => "md",
        "panel" => "cpanel",
        "pushkontak" => "pushkontak",
        "store" => "store",
        "otp" => "otp",
        _ => "all",
    }
}

/// Penanda command diblokir mode fitur grup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GroupFeatureDenied;

/// Gate fitur per-grup. `Err(GroupFeatureDenied)` = command diblokir mode ini.
pub fn group_feature_mode(
    caller: &Caller,
    meta: &PluginMeta,
    group: Option<GroupState>,
    bot_mode: &str,
    is_jadibot: bool,
) -> Result<(), GroupFeatureDenied> {
    // Jadibot selalu `all` — JS: `if (isJadibot) botMode = "all"`.
    let mode = if is_jadibot {
        GroupBotMode::All
    } else {
        GroupBotMode::parse(bot_mode)
    };

    // Group toggle (game/rpg) checked terpisah di `permission`; di sini hanya
    // category gating. `handler.js:1790` sudah menolak `isAdmin || isOwner`.
    let _ = group;

    let category = meta.category.to_lowercase();

    // Command yang selalu lolos: `botmode`, `menu`, `menucat`.
    if matches!(caller.command.as_str(), "botmode" | "menu" | "menucat") {
        return Ok(());
    }

    let blocked = match mode.allowed() {
        Some(allowed) => !allowed.iter().any(|c| *c == category),
        None => mode
            .excluded()
            .is_some_and(|ex| ex.contains(&category.as_str())),
    };

    if blocked {
        Err(GroupFeatureDenied)
    } else {
        Ok(())
    }
}

/// Pesan deny untuk gate botMode — port dari handler.js:1770-1782.
pub fn group_mode_denied_message(mode: &GroupBotMode, category: &str, prefix: &str, command: &str) -> String {
    let suggested = suggested_mode(category);
    let suggested_name = GroupBotMode::parse(suggested).display_name();
    format!(
        "🔒 *ᴄᴏᴍᴍᴀɴᴅ ᴛɪᴅᴀᴋ ᴛᴇʀsᴇᴅɪᴀ*\n\n\
         > Bot sedang dalam mode *{}*\n\
         > Command `{}{}` tersedia di mode *{}*\n\n\
         💡 Hubungi admin grup untuk mengganti mode:\n\
         `{}botmode {}`",
        mode.display_name(),
        prefix,
        command,
        suggested_name,
        prefix,
        suggested
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn caller() -> Caller {
        Caller {
            sender: "628123@s.whatsapp.net".into(),
            chat: "628123@s.whatsapp.net".into(),
            ..Default::default()
        }
    }

    fn group_caller() -> Caller {
        Caller {
            sender: "628123@s.whatsapp.net".into(),
            chat: "120363@g.us".into(),
            is_group: true,
            ..Default::default()
        }
    }

    #[test]
    fn public_mode_allows_everyone() {
        let s = ModeSettings {
            mode: "public".into(),
            ..Default::default()
        };
        assert_eq!(check_mode(&caller(), &s, None), Ok(()));
    }

    #[test]
    fn afk_blocks_non_owner() {
        let s = ModeSettings {
            mode: "public".into(),
            afk_active: true,
            ..Default::default()
        };
        assert_eq!(check_mode(&caller(), &s, None), Err(DenyMode::Afk));
    }

    #[test]
    fn afk_allows_owner() {
        let s = ModeSettings {
            mode: "public".into(),
            afk_active: true,
            ..Default::default()
        };
        let mut c = caller();
        c.is_owner = true;
        assert_eq!(check_mode(&c, &s, None), Ok(()));
    }

    #[test]
    fn only_gc_blocks_private() {
        let s = ModeSettings {
            mode: "public".into(),
            only_gc: true,
            ..Default::default()
        };
        assert_eq!(check_mode(&caller(), &s, None), Err(DenyMode::Silent));
        assert_eq!(check_mode(&group_caller(), &s, None), Ok(()));
    }

    #[test]
    fn only_pc_blocks_group() {
        let s = ModeSettings {
            mode: "public".into(),
            only_pc: true,
            ..Default::default()
        };
        assert_eq!(check_mode(&group_caller(), &s, None), Err(DenyMode::Silent));
        assert_eq!(check_mode(&caller(), &s, None), Ok(()));
    }

    #[test]
    fn public_group_short_circuits_before_mode() {
        // Grup public harus lolos walau mode=self.
        let s = ModeSettings {
            mode: "self".into(),
            public_groups: vec!["120363@g.us".into()],
            ..Default::default()
        };
        assert_eq!(check_mode(&group_caller(), &s, None), Ok(()));
    }

    #[test]
    fn self_mode_blocks_non_owner() {
        let s = ModeSettings {
            mode: "self".into(),
            ..Default::default()
        };
        assert_eq!(check_mode(&caller(), &s, None), Err(DenyMode::Silent));
    }

    #[test]
    fn only_admin_blocks_group_non_admin() {
        let s = ModeSettings {
            mode: "public".into(),
            only_admin: true,
            ..Default::default()
        };
        assert_eq!(check_mode(&group_caller(), &s, None), Err(DenyMode::Silent));
        // Private tetap lolos walau onlyAdmin.
        assert_eq!(check_mode(&caller(), &s, None), Ok(()));
    }

    #[test]
    fn self_admin_blocks_private() {
        let s = ModeSettings {
            mode: "public".into(),
            self_admin: true,
            ..Default::default()
        };
        // selfAdmin: private DITOLAK (berbeda dari publicAdmin).
        assert_eq!(check_mode(&caller(), &s, None), Err(DenyMode::Silent));
    }

    #[test]
    fn group_override_false_beats_global_true() {
        // `groupFlags.X ?? db.setting("X")`: false eksplisit di grup menang.
        let s = ModeSettings {
            mode: "public".into(),
            only_admin: true,
            ..Default::default()
        };
        let g = GroupOverrides {
            only_admin: Some(false),
            ..Default::default()
        };
        // Override false -> only_admin efektif false -> grup non-admin lolos.
        assert_eq!(check_mode(&group_caller(), &s, Some(g)), Ok(()));
    }

    #[test]
    fn unknown_mode_fails_open() {
        let s = ModeSettings {
            mode: "entah".into(),
            ..Default::default()
        };
        assert_eq!(check_mode(&caller(), &s, None), Ok(()));
    }

    #[test]
    fn self_group_blocks_non_owner_silently() {
        let s = ModeSettings {
            mode: "public".into(),
            self_groups: vec!["120363@g.us".into()],
            ..Default::default()
        };
        assert_eq!(
            check_mode(&group_caller(), &s, None),
            Err(DenyMode::SelfGroup)
        );
    }

    // ---------- group feature mode ----------

    fn meta_cat(cat: &str) -> PluginMeta {
        PluginMeta {
            category: cat.into(),
            ..Default::default()
        }
    }

    #[test]
    fn md_mode_excludes_pushkontak_and_store() {
        let m = meta_cat("store");
        let mut c = group_caller();
        c.command = "store".into();
        assert_eq!(
            group_feature_mode(&c, &m, None, "md", false),
            Err(GroupFeatureDenied),
            "category store di-exclude di mode md"
        );
    }

    #[test]
    fn md_mode_allows_main() {
        let m = meta_cat("main");
        let mut c = group_caller();
        c.command = "ping".into();
        assert_eq!(group_feature_mode(&c, &m, None, "md", false), Ok(()));
    }

    #[test]
    fn cpanel_allows_base_plus_panel() {
        let panel = meta_cat("panel");
        let tools = meta_cat("tools");
        let main = meta_cat("main");
        let mut c = group_caller();
        c.command = "x".into();
        assert_eq!(group_feature_mode(&c, &panel, None, "cpanel", false), Ok(()));
        assert_eq!(group_feature_mode(&c, &tools, None, "cpanel", false), Ok(()));
        assert_eq!(group_feature_mode(&c, &main, None, "cpanel", false), Ok(()));
    }

    #[test]
    fn menu_bypasses_mode_entirely() {
        let m = meta_cat("store");
        let mut c = group_caller();
        c.command = "menu".into();
        //(menu di mode md, store di-exclude, tapi menu exempt)
        assert_eq!(group_feature_mode(&c, &m, None, "md", false), Ok(()));
    }

    #[test]
    fn jadibot_forces_all_mode() {
        let m = meta_cat("store");
        let mut c = group_caller();
        c.command = "store".into();
        // Tanpa jadibot, md mode memblokir store. Dengan jadibot, lolos.
        assert_eq!(group_feature_mode(&c, &m, None, "md", false), Err(GroupFeatureDenied));
        assert_eq!(group_feature_mode(&c, &m, None, "md", true), Ok(()));
    }

    #[test]
    fn unknown_bot_mode_is_permissive() {
        let m = meta_cat("store");
        let mut c = group_caller();
        c.command = "store".into();
        assert_eq!(group_feature_mode(&c, &m, None, "entah", false), Ok(()));
    }

    #[test]
    fn denied_message_reports_current_and_suggested_modes() {
        // Kategori `store` di-block di mode md, dan categoryModeMap["store"]
        // = "store" — sarannya mode store, BUKAN md.
        let msg = group_mode_denied_message(&GroupBotMode::Md, "store", ".", "store");
        assert!(
            msg.contains("Multi Device"),
            "mode saat ini harus disebut: {msg}"
        );
        assert!(
            msg.contains(".botmode store"),
            "saran harus mengikuti categoryModeMap: {msg}"
        );

        // Kategori yang tidak punya entri di map -> saran "all".
        let ai = group_mode_denied_message(&GroupBotMode::Cpanel, "ai", ".", "ai");
        assert!(ai.contains(".botmode md"), "ai -> md: {ai}");

        // Prefix ikut dipakai di kedua tempat.
        let bang = group_mode_denied_message(&GroupBotMode::Md, "store", "!", "store");
        assert!(bang.contains("`!store`"), "prefix custom: {bang}");
        assert!(bang.contains("`!botmode store`"), "prefix custom: {bang}");
    }
}