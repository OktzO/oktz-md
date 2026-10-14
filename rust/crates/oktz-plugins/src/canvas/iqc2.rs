//! `plugins/canvas/iqc2.js` — fake quote iOS dengan provider, jam, dan
//! baterai acak.
//!
//! Tidak ada HTTP di plugin ini: JS juga tidak memanggil axios, hanya
//! menyerahkan URL ke Baileys (`image: { url }`).
//!
//! `ponytail:` `m.pushName` tidak ada di `Ctx`. `serialize.js:760` memakai
//! `pushName || "Unknown"`, jadi teks help memakai `"Unknown"` sebagai
//! fallback — bukan `'User'` seperti `error.js`.
//!
//! `ponytail:` cabang `catch` di JS (`iqc2.js:45-49`) menangkap kegagalan
//! `sock.sendMessage`. Pengiriman sekarang dikerjakan pipeline, jadi cabang
//! itu tidak lagi punya pemicu di dalam handler.
//!
//! `pluginConfig` tidak punya key `example`, jadi `example` kosong — bukan
//! diisi tebakan.

use oktz_plugin_api::{Ctx, MediaSource, Reply, plugin};

use crate::common::{now_ms, random_index, roll, time_hm};
use crate::tools::phisingataubukan::encode_uri_component;

/// `providers` (`iqc2.js:31`).
const PROVIDERS: [&str; 6] = ["INDOSAT", "TELKOMSEL", "XL", "TRI", "SMARTFREN", "WIFI"];

/// Batas atas `baterai`: `Math.floor(Math.random() * (100 - 10 + 1)) + 10`.
const BATERAI_SPAN: u32 = 100 - 10 + 1;

/// `providers[Math.floor(Math.random() * providers.length)]` (`iqc2.js:32`).
pub(crate) fn random_provider() -> &'static str {
    PROVIDERS[random_index(PROVIDERS.len())]
}

/// `Math.floor(Math.random() * (100 - 10 + 1)) + 10` (`iqc2.js:37`) — inklusif
/// 10..=100, jadi rentang Rust `0..91` digeser 10.
pub(crate) fn random_baterai() -> u32 {
    roll(BATERAI_SPAN) + 10
}

/// `toLocaleTimeString("id-ID", { hour: "2-digit", minute: "2-digit" })`
/// lalu `.replace('.', ':')` (`iqc2.js:35`).
///
/// id-ID memisahkan jam dan menit dengan titik (`"19.05"`), dan `replace`
/// tanpa flag `g` hanya mengganti kemunculan pertama — hasilnya persis
/// `HH:mm` Asia/Jakarta. `time_hm` sudah mengunci offset +7 (`common.rs`).
pub(crate) fn jam() -> String {
    time_hm(now_ms())
}

/// `text || (m.quoted && m.quoted.text ? m.quoted.text : "")` (`iqc2.js:18`).
pub(crate) fn target_text(text: &str, quoted: Option<&str>) -> String {
    if !text.is_empty() {
        return text.to_string();
    }
    quoted.filter(|q| !q.is_empty()).unwrap_or("").to_string()
}

/// `https://api.nexray.eu.cc/maker/v1/iqc?...` (`iqc2.js:39`).
pub(crate) fn api_url(target: &str, provider: &str, jam: &str, baterai: u32) -> String {
    format!(
        "https://api.nexray.eu.cc/maker/v1/iqc?text={}&provider={}&jam={}&baterai={}",
        encode_uri_component(target),
        encode_uri_component(provider),
        encode_uri_component(jam),
        baterai
    )
}

/// Cabang `if (!targetText)` (`iqc2.js:21-26`).
pub(crate) fn help(push_name: &str) -> String {
    format!(
        "Halo *{push_name}*, sepertinya kamu belum memasukkan teksnya.\n\n\
         Silakan gunakan perintah dengan format:\n\
         - .iqc2 <teks kamu>\n\
         - Atau balas (reply) pesan orang lain dengan .iqc2"
    )
}

#[plugin(
    name = "iqc2",
    alias = ["qc3"],
    category = "canvas",
    description = "Membuat Fake Quote iOS style dengan informasi baterai dan provider.",
    usage = ".iqc2 [text/reply]",
    example = "",
    cooldown = 5,
    energi = 2,
    is_owner = false,
    is_premium = false,
    is_group = false,
    is_private = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn iqc2(ctx: Ctx) -> Reply {
    let quoted = ctx.quoted().map(|q| q.text.as_str());
    let target = target_text(&ctx.text, quoted);
    if target.is_empty() {
        return Reply::text(help("Unknown"));
    }

    // `await m.react('🕕')` lalu `await m.react('✅')` — `Reply` hanya menyimpan
    // satu reaksi, jadi yang dipegang adalah reaksi terakhir.
    let url = api_url(&target, random_provider(), &jam(), random_baterai());
    Reply::image(MediaSource::url(url), "").with_reaction("✅")
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{QuotedKind, QuotedMessage};

    fn run(text: &str, quoted: Option<&str>) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "iqc2".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            quoted: quoted.map(|t| QuotedMessage {
                id: "MSG1".into(),
                sender: "628999@s.whatsapp.net".into(),
                chat: "628111@s.whatsapp.net".into(),
                text: t.into(),
                kind: QuotedKind::Text,
                mimetype: None,
                file_name: None,
                from_me: false,
            }),
            ..Default::default()
        };
        (oktz_plugin_api::find("iqc2").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> String {
        r.without_mutations()
            .text_body()
            .unwrap_or_default()
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

    fn url_of(r: &Reply) -> String {
        match r.media_source() {
            Some(MediaSource::Url { url, .. }) => url.clone(),
            other => panic!("harus Url, dapat {other:?}"),
        }
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let meta = oktz_plugin_api::find("iqc2").unwrap().meta.clone();
        assert_eq!(meta.name, "iqc2");
        assert_eq!(meta.alias, ["qc3"]);
        assert_eq!(meta.category, "canvas");
        assert_eq!(
            meta.description,
            "Membuat Fake Quote iOS style dengan informasi baterai dan provider."
        );
        assert_eq!(meta.usage, ".iqc2 [text/reply]");
        assert_eq!(meta.cooldown, 5);
        assert_eq!(meta.energi, 2);
        assert!(!meta.is_group);
        assert!(!meta.is_private);
        assert!(!meta.is_owner);
        assert!(!meta.is_premium);
        // `pluginConfig` JS tidak punya key `example`.
        assert_eq!(meta.example, "");
    }

    #[test]
    fn the_alias_resolves_to_iqc2() {
        assert_eq!(oktz_plugin_api::find("qc3").unwrap().meta.name, "iqc2");
    }

    #[test]
    fn the_url_carries_text_provider_jam_and_baterai() {
        assert_eq!(
            api_url("halo", "INDOSAT", "19:05", 87),
            "https://api.nexray.eu.cc/maker/v1/iqc?text=halo&provider=INDOSAT&jam=19%3A05&baterai=87"
        );
    }

    #[test]
    fn typed_text_becomes_the_image_url() {
        let r = run("halo dunia", None);
        assert!(r.has_media());
        assert_eq!(reaction_of(&r), Some("✅"));
        let u = url_of(&r);
        assert!(
            u.starts_with("https://api.nexray.eu.cc/maker/v1/iqc?text=halo%20dunia&"),
            "{u}"
        );
    }

    #[test]
    fn a_quoted_text_is_used_when_no_text_is_typed() {
        let r = run("", Some("teks yang dibalas"));
        let u = url_of(&r);
        assert!(u.contains("text=teks%20yang%20dibalas"), "{u}");
    }

    #[test]
    fn typed_text_wins_over_the_quoted_text() {
        assert_eq!(
            target_text("yang diketik", Some("yang di-quote")),
            "yang diketik"
        );
        assert_eq!(target_text("", Some("")), "");
        assert_eq!(target_text("", None), "");
    }

    #[test]
    fn no_text_and_no_quote_shows_the_help_without_reacting() {
        for (text, quoted) in [("", None), ("", Some("")), ("   ", Some(""))] {
            let r = run(text.clone(), quoted);
            assert_eq!(
                text_of(&r),
                help("Unknown"),
                "text {text:?} quoted {quoted:?}"
            );
            assert!(!r.has_media());
            assert_eq!(reaction_of(&r), None, "guard ini sebelum `m.react`");
        }
    }

    #[test]
    fn every_provider_is_reachable_and_baterai_spans_ten_to_one_hundred() {
        let mut seen = [false; 6];
        let (mut lo, mut hi) = (u32::MAX, 0);
        for _ in 0..3_000 {
            let p = random_provider();
            let idx = PROVIDERS
                .iter()
                .position(|x| *x == p)
                .expect("provider asing");
            seen[idx] = true;
            let b = random_baterai();
            assert!((10..=100).contains(&b), "baterai di luar 10..=100: {b}");
            lo = lo.min(b);
            hi = hi.max(b);
        }
        assert!(
            seen.iter().all(|x| *x),
            "ada provider yang tidak pernah terpilih"
        );
        assert_eq!((lo, hi), (10, 100), "kedua ujung harus terjangkau");
    }

    #[test]
    fn jam_uses_a_colon_and_two_digits_per_field() {
        let j = jam();
        let (h, m) = j
            .split_once(':')
            .unwrap_or_else(|| panic!("tanpa titik dua: {j}"));
        assert_eq!(h.len(), 2, "jam harus 2 digit: {j}");
        assert_eq!(m.len(), 2, "menit harus 2 digit: {j}");
        assert!(h.parse::<u32>().unwrap() < 24, "jam di luar 0..24: {j}");
        assert!(m.parse::<u32>().unwrap() < 60, "menit di luar 0..60: {j}");
    }

    #[test]
    fn the_generated_url_is_internally_consistent() {
        let r = run("cek", None);
        let u = url_of(&r);
        assert!(
            PROVIDERS
                .iter()
                .any(|p| u.contains(&format!("provider={p}&"))),
            "provider tidak dikenal di URL: {u}"
        );
        let (_, qs) = u.split_once('?').unwrap();
        assert_eq!(qs.split('&').count(), 4, "harus ada 4 parameter: {u}");
        let baterai: u32 = qs
            .rsplit("baterai=")
            .next()
            .unwrap()
            .parse()
            .expect("baterai harus angka");
        assert!(
            (10..=100).contains(&baterai),
            "baterai di luar 10..=100: {baterai}"
        );
    }

    #[test]
    fn special_characters_and_very_long_text_do_not_panic() {
        assert!(api_url("a&b c?d=e", "XL", "00:00", 10).contains("text=a%26b%20c%3Fd%3De"));
        let panjang = "z".repeat(10_000);
        let r = run(&panjang, None);
        assert!(r.has_media());
        assert!(url_of(&r).len() > 10_000);
    }
}
