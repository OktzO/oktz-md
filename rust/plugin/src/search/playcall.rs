//! `plugins/search/playcall.js` — putar musik YouTube lewat panggilan suara
//! WhatsApp.
//!
//! `ponytail:` `global.voipClient` (`playcall.js:48`), `fs`, `crypto`,
//! `child_process`/`extensionFor`, `yt-search`, dan resolver
//! `resolver.resolve("youtube", ..)` tidak ada di API plugin. Guard
//! `if (!global.voipClient)` jadi selalu benar, jadi itu yang diport — persis
//! `download::sfiledl` dengan kapabilitas yang belum di-port.
//!
//! `pluginConfig.isEnabled` di JS `false`; itu ikut disalin lewat
//! `is_enabled`.

#![allow(dead_code)] // jalur panggilan tidak terjangkau tanpa voipClient

use oktz_plugin_api::{Ctx, Reply, plugin};

/// `playcall.js:36-45` — kartu pakai.
pub(crate) fn help(prefix: &str) -> String {
    format!(
        "⚠️ *PANGGILAN MUSIK (PLAYCALL)*\n\n\
         Fitur ini digunakan untuk memutar musik YouTube langsung melalui panggilan telepon suara WhatsApp ke nomor kamu.\n\n\
         *PENGGUNAAN:*\n\
         - *{prefix}playcall <judul lagu>*\n\n\
         *CONTOH:*\n\
         - *{prefix}playcall surat cinta untuk starla*\n\
         - *{prefix}playcall komang*\n\n\
         _Pastikan nomormu dapat menerima panggilan suara WhatsApp._"
    )
}

/// `playcall.js:49-53` — VoIP belum aktif.
pub(crate) const BELUM_SIAP: &str =
    "⚠️ *LAYANAN BELUM SIAP*\n\nModul panggilan suara VoIP belum aktif atau sedang dalam proses inisialisasi pada server.";

/// `playcall.js:68` — judul tidak ditemukan.
pub(crate) fn tidak_ditemukan(query: &str) -> String {
    format!("❌ Musik dengan judul *{query}* tidak ditemukan.")
}

/// `playcall.js:96-102` — konfirmasi sebelum panggilan redir:.
pub(crate) fn mulai_panggilan(title: &str, timestamp: &str, nomor: &str) -> String {
    format!(
        "📞 *MEMULAI PANGGILAN*\n\n\
         - Judul: *{title}*\n\
         - Durasi: *{timestamp}*\n\
         - Tujuan: *+{nomor}*\n\n\
         _$Panggilan sedang dialihkan ke WhatsApp kamu, silakan angkat telepon untuk mendengarkan lagu._"
    )
}

/// `playcall.js:110` — callback `connected`.
pub(crate) fn terhubung(title: &str) -> String {
    format!(
        "✅ *TERHUBUNG*\n\nPanggilan berhasil tersambung! Lagu *{title}* sedang diputar di telepon."
    )
}

/// `playcall.js:117` — callback `ended`; `reason` kosong jadi `"selesai"`.
pub(crate) fn berakhir(reason: &str) -> String {
    let alasan = if reason.is_empty() { "selesai" } else { reason };
    format!("📵 *PANGGILAN BERAKHIR*\n\nPanggilan telepon telah selesai ({alasan}).")
}

/// `m.sender.split("@")[0].replace(/\D/g, "")` (`playcall.js:93`) — nomor
/// tujuan hanya digit.
pub(crate) fn nomor_dari(jid: &str) -> String {
    jid.split('@')
        .next()
        .unwrap_or("")
        .chars()
        .filter(char::is_ascii_digit)
        .collect()
}

/// `yts(query)` + `resolver.resolve("youtube", ..)` — keduanya tidak ada di
/// crate ini, jadi panggilan selalu gagal dan yang menyala adalah `catch`.
fn awal_panggilan(_query: &str) -> Result<(String, String, String), String> {
    Err(BELUM_SIAP.to_string())
}

#[plugin(
    name = "playcall",
    alias = ["telepon", "call", "playvn"],
    category = "search",
    description = "Memutar musik dari YouTube lewat panggilan suara WhatsApp",
    usage = ".playcall <judul lagu>",
    example = ".playcall komang",
    cooldown = 15,
    energi = 2,
    is_enabled = false,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn playcall(ctx: Ctx) -> Reply {
    let query = ctx.text.trim();

    if query.is_empty() {
        return Reply::text(help(&ctx.prefix));
    }

    // `if (!global.voipClient)` dicek sebelum `m.react("🕕")`, jadi tanpa
    // reaksi.
    match awal_panggilan(query) {
        // `playcall.js:95-124` — file sementara + `voipClient.call` +
        // callback `connected`/`ended`/`error`. Semuanya butuh `fs` dan VoIP,
        // jadi tidak terjangkau.
        Ok((title, timestamp, _audio_url)) => {
            Reply::text(mulai_panggilan(&title, &timestamp, &nomor_dari(&ctx.sender)))
        }
        // `playcall.js:49-53` — `global.voipClient` tidak ada. Guard ini dicek
        // sebelum `m.react("🕕")` di `:55`, jadi tidak ada reaksi.
        Err(message) => Reply::text(message),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(text: &str) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "playcall".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            ..Default::default()
        };
        (oktz_plugin_api::find("playcall").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> String {
        r.without_mutations()
            .text_body()
            .expect("harus ada teks")
            .to_string()
    }

    fn reaction_of(r: &Reply) -> Option<&str> {
        let mut cur = r;
        loop {
            match cur {
                Reply::Reaction { emoji, .. } => return Some(emoji),
                Reply::Mutate { reply, .. } => cur = reply,
                _ => return None,
            }
        }
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("playcall").unwrap();
        assert_eq!(p.meta.name, "playcall");
        assert_eq!(p.meta.alias, ["telepon", "call", "playvn"]);
        assert_eq!(p.meta.category, "search");
        assert_eq!(
            p.meta.description,
            "Memutar musik dari YouTube lewat panggilan suara WhatsApp"
        );
        assert_eq!(p.meta.usage, ".playcall <judul lagu>");
        assert_eq!(p.meta.example, ".playcall komang");
        assert_eq!(p.meta.cooldown, 15);
        assert_eq!(p.meta.energi, 2);
        assert!(!p.meta.is_enabled, "isEnabled false di JS");
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_premium);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["telepon", "call", "playvn"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "playcall");
        }
    }

    #[test]
    fn an_empty_title_shows_the_help_card() {
        for text in ["", "  ", "\t\n"] {
            let t = text_of(&run(text));
            assert!(t.starts_with("⚠️ *PANGGILAN MUSIK (PLAYCALL)*"), "input {text:?}");
            // `*PENGGUNAAN:*` memakai asterisk, bukan backtick, di `playcall.js:40`.
            assert!(t.contains("- *.playcall <judul lagu>*"), "{t}");
            assert!(t.ends_with("_Pastikan nomormu dapat menerima panggilan suara WhatsApp._"), "{t}");
        }
        assert!(help("!").contains("!playcall komang"));
        assert_eq!(reaction_of(&run("")), None);
    }

    #[test]
    fn a_title_hits_the_voip_not_ready_branch() {
        let r = run("komang");
        assert_eq!(text_of(&r), BELUM_SIAP);
        assert_eq!(reaction_of(&r), None, "guard sebelum `m.react('🕕')`");
    }

    #[test]
    fn odd_and_very_long_titles_do_not_panic() {
        for input in ["<b>", "a|b", "☎", "'\"", "a\u{202e}b", &"x".repeat(5000)] {
            assert_eq!(text_of(&run(input)), BELUM_SIAP, "input {input:?}");
        }
    }

    #[test]
    fn the_target_number_keeps_only_digits() {
        assert_eq!(nomor_dari("628111@s.whatsapp.net"), "628111");
        assert_eq!(nomor_dari("+62 811-1@s.whatsapp.net"), "628111");
        assert_eq!(nomor_dari("abc@lid"), "");
        assert_eq!(nomor_dari(""), "");
        assert_eq!(nomor_dari("123"), "123", "tanpa @ tetap diambil");
    }

    #[test]
    fn the_call_messages_keep_their_javascript_wording() {
        let t = mulai_panggilan("Komang", "3:45", "628111");
        assert!(t.starts_with("📞 *MEMULAI PANGGILAN*\n\n- Judul: *Komang*\n"), "{t}");
        assert!(t.contains("- Durasi: *3:45*\n- Tujuan: *+628111*\n"), "{t}");
        assert!(t.ends_with("_$Panggilan sedang dialihkan ke WhatsApp kamu, silakan angkat telepon untuk mendengarkan lagu._"), "{t}");

        assert!(terhubung("Komang").contains("Lagu *Komang* sedang diputar"));
        assert_eq!(
            berakhir(""),
            "📵 *PANGGILAN BERAKHIR*\n\nPanggilan telepon telah selesai (selesai)."
        );
        assert!(berakhir("normal").ends_with("selesai (normal)."));
    }

    #[test]
    fn the_not_found_message_quotes_the_query() {
        assert_eq!(
            tidak_ditemukan("surat cinta untuk starla"),
            "❌ Musik dengan judul *surat cinta untuk starla* tidak ditemukan."
        );
    }
}