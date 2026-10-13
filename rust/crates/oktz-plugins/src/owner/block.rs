//! `plugins/owner/block.js` — blokir nomor WhatsApp.
//!
//! `ponytail:` dua langkah JS TIDAK di-port karena keduanya butuh `sock`:
//! `sock.user?.id` (cek "jangan blokir bot sendiri") dan
//! `sock.updateBlockStatus(jid, 'block')`. `Ctx` memang tidak memberi `sock`,
//! jadi plugin tidak akan pernah benar-benar memblokir siapa pun. Karena itu
//! balasan aksi TIDAK memakai teks sukses JS ("🚫 *ɴᴏᴍᴏʀ ᴅɪʙʟᴏᴋɪʀ*") —
//! teks itu menyatakan blokir terjadi, dan tidak ada yang terjadi.
//! Yang dipindah: seluruh resolusi target, kedua guard, dan teks usage.
//!
//! `unblock.rs` memakai `resolve_target` dari file ini; lihat laporan batch.

use oktz_plugin_api::{Ctx, Reply, plugin};

pub(crate) const INVALID_NUMBER: &str = "❌ Nomor tidak valid.";

pub(crate) const USAGE: &str = "⚠️ *ᴄᴀʀᴀ ᴘᴀᴋᴀɪ*\n\n\
     > `.block 628xxx` — Blokir via nomor\n\
     > `.block` (reply pesan) — Blokir pengirim\n\
     > `.block @mention` — Blokir yang di-mention\n\
     > `.block` (di private chat) — Blokir user ini";

/// Hasil resolusi target, memetakan tiga cabang `else if` di JS.
pub(crate) enum Target {
    /// Punya JID — akan diblokir.
    Jid(String),
    /// `m.args[0]` ada tapi setelah filter digit jadi string kosong.
    InvalidNumber,
    /// Tidak ada cabang yang kena → tampilkan [`USAGE`].
    None,
}

/// `m.mentionedJid[0]` → `m.quoted.sender` → angka dari `m.args[0]` →
/// `m.chat` kalau private. Urutannya tidak boleh diacak: mention menang atas
/// quote, quote menang atas angka.
pub(crate) fn resolve_target(ctx: &Ctx) -> Target {
    if let Some(first) = ctx.mentioned.first() {
        return Target::Jid(first.clone());
    }
    if let Some(quoted) = ctx.quoted() {
        // JS: `m.quoted.sender || m.quoted.participant`. `QuotedMessage` hanya
        // punya `sender`; string kosong = falsy di JS, jadi hasil "")
        // sama dengan tidak kena dan jatuh ke guard `!targetJid`.
        if quoted.sender.is_empty() {
            return Target::None;
        }
        return Target::Jid(quoted.sender.clone());
    }
    if let Some(raw) = ctx.args.first() {
        let num: String = raw.chars().filter(char::is_ascii_digit).collect();
        if num.is_empty() {
            return Target::InvalidNumber;
        }
        return Target::Jid(format!("{num}@s.whatsapp.net"));
    }
    if !ctx.is_group {
        return Target::Jid(ctx.chat.clone());
    }
    Target::None
}

/// Pengganti langkah `sock.updateBlockStatus(..)` yang tidak bisa di-port.
/// Menyebut target agar owner tetap tahu nomor mana yang akan di-blokir, dan
/// tidak mengirim `mentions` — JS memakai mentions supaya WA menandai target,
/// dan itu tidak boleh terjadi untuk aksi yang tidak jadi dijalankan.
pub(crate) fn sock_unavailable(action: &str, target: &str) -> String {
    format!(
        "⚠️ *{action} tidak tersedia di port Rust*\n\n\
         > Ctx tidak punya `sock`, jadi `updateBlockStatus` tidak bisa dipanggil.\n\
         > Target: @{target}"
    )
}

#[plugin(
    name = "block",
    alias = ["blokir"],
    category = "owner",
    description = "Blokir nomor WhatsApp",
    usage = ".block <nomor/reply/mention>",
    example = ".block 628xxx",
    cooldown = 5,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = true,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn block(ctx: Ctx) -> Reply {
    let target = match resolve_target(&ctx) {
        Target::Jid(jid) => jid,
        Target::InvalidNumber => return Reply::text(INVALID_NUMBER),
        Target::None => return Reply::text(USAGE),
    };
    let number = target.split('@').next().unwrap_or(&target);
    Reply::text(sock_unavailable("Blokir", number))
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{PluginEntry, QuotedKind, QuotedMessage, find};

    fn quote_of(sender: &str) -> QuotedMessage {
        QuotedMessage {
            id: "Q1".into(),
            sender: sender.into(),
            chat: "628111@s.whatsapp.net".into(),
            text: String::new(),
            kind: QuotedKind::Text,
            mimetype: None,
            file_name: None,
            from_me: false,
        }
    }

    struct Spec {
        prefix: &'static str,
        is_group: bool,
    }

    fn ctx(spec: Spec, args: &[&str], mentioned: &[&str], quoted: Option<QuotedMessage>) -> Ctx {
        Ctx {
            prefix: spec.prefix.into(),
            command: "block".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            is_group: spec.is_group,
            args: args.iter().map(|s| (*s).to_string()).collect(),
            mentioned: mentioned.iter().map(|s| (*s).to_string()).collect(),
            quoted,
            ..Default::default()
        }
    }

    /// `find("block")` tidak bisa dipakai di test: `ban.js` juga mendaftarkan
    /// `block` sebagai alias (`plugins/owner/ban.js:6`), jadi lookup itu
    /// mengembalikan siapa saja yang menang di distributed slice. `blokir`
    /// milik plugin ini saja.
    fn entry() -> &'static PluginEntry {
        find("blokir").expect("block harus terdaftar lewat alias blokir")
    }

    fn run(c: Ctx) -> String {
        (entry().handler)(c)
            .text_body()
            .expect("harus ada teks")
            .to_string()
    }

    const PC: Spec = Spec { prefix: ".", is_group: false };
    const GC: Spec = Spec { prefix: ".", is_group: true };

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = entry();
        assert_eq!(p.meta.name, "block");
        // JS `name: ['block','blokir']` — entri kedua jadi alias, alias JS kosong.
        assert_eq!(p.meta.alias, ["blokir"]);
        assert_eq!(p.meta.category, "owner");
        assert_eq!(p.meta.description, "Blokir nomor WhatsApp");
        assert_eq!(p.meta.usage, ".block <nomor/reply/mention>");
        assert_eq!(p.meta.example, ".block 628xxx");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_owner);
        assert!(!p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn the_second_name_in_the_js_array_resolves_as_an_alias() {
        assert_eq!(find("blokir").unwrap().meta.name, "block");
    }

    /// Tabrakan warisan JS: `ban.js` mendaftarkan `block` sebagai alias dan
    /// `block.js` mendaftarkan `block` sebagai nama. `find("block")` karena
    /// itu mengembalikan salah satu dari keduanya, bukan selalu plugin ini.
    /// Tidak dirapikan diam-diam; lihat laporan batch.
    #[test]
    fn the_block_command_is_shared_with_ban_just_like_in_javascript() {
        let owner = find("block").unwrap().meta.name.clone();
        assert!(
            owner == "block" || owner == "ban",
            "pemilik `block` harus salah satu dari dua plugin JS: {owner}"
        );
    }

    #[test]
    fn a_mention_wins_over_both_the_quote_and_the_number() {
        let t = resolve_target(&ctx(
            GC,
            &["628777"],
            &["628999@s.whatsapp.net"],
            Some(quote_of("628888@s.whatsapp.net")),
        ));
        assert!(matches!(t, Target::Jid(j) if j == "628999@s.whatsapp.net"));
    }

    #[test]
    fn a_quote_wins_over_the_number() {
        let t = resolve_target(&ctx(
            GC,
            &["628777"],
            &[],
            Some(quote_of("628888@s.whatsapp.net")),
        ));
        assert!(matches!(t, Target::Jid(j) if j == "628888@s.whatsapp.net"));
    }

    #[test]
    fn a_number_is_stripped_to_digits_and_gets_the_wa_server() {
        // `+62 (8) 811-1` -> hanya digit: 6,2,8,8,1,1,1
        let t = resolve_target(&ctx(GC, &["+62 (8) 811-1"], &[], None));
        assert!(matches!(t, Target::Jid(j) if j == "6288111@s.whatsapp.net"));
    }

    #[test]
    fn a_number_with_no_digits_at_all_is_rejected() {
        let t = resolve_target(&ctx(GC, &["abc"], &[], None));
        assert!(matches!(t, Target::InvalidNumber));
        assert_eq!(run(ctx(GC, &["abc"], &[], None)), INVALID_NUMBER);
    }

    #[test]
    fn a_bare_command_in_private_falls_back_to_the_chat_itself() {
        let t = resolve_target(&ctx(PC, &[], &[], None));
        assert!(matches!(t, Target::Jid(j) if j == "628111@s.whatsapp.net"));
    }

    #[test]
    fn a_bare_command_in_a_group_has_no_target_and_shows_the_usage() {
        assert!(matches!(resolve_target(&ctx(GC, &[], &[], None)), Target::None));
        assert_eq!(run(ctx(GC, &[], &[], None)), USAGE);
    }

    #[test]
    fn a_quote_without_a_sender_also_lands_on_the_usage_branch() {
        // JS: `m.quoted.sender || m.quoted.participant` → undefined → `!targetJid`.
        assert!(matches!(
            resolve_target(&ctx(GC, &[], &[], Some(quote_of("")))),
            Target::None
        ));
    }

    #[test]
    fn the_reply_names_the_target_without_tagging_it() {
        let r = (entry().handler)(ctx(GC, &["628777"], &[], None));
        assert_eq!(
            r.text_body(),
            Some(sock_unavailable("Blokir", "628777").as_str())
        );
        assert!(
            r.mutations().is_empty(),
            "tidak ada store yang boleh ditulis: blokir tidak dijalankan"
        );
        if let oktz_plugin_api::Reply::Text { mentions, .. } = r.without_mutations() {
            assert!(mentions.is_empty(), "target tidak boleh di-mention");
        }
    }
}
