//! `plugins/asupan/bocil.js` — video bocil, khusus premium/owner.
//!
//! Urutan efek JS dipertahankan: `m.react('🕕')` SELALU jalan lebih dulu,
//! termasuk saat ditolak atau data kosong, jadi balasannya `Sequence`.

use oktz_plugin_api::{Ctx, MediaSource, Reply, plugin};
use serde_json::Value;

use crate::common::random_index;

const PATH: &str = "src/tiktok/bocil.json";

const ERR_KOSONG: &str = "❌ Data tidak tersedia";
const ERR_VIDEO: &str = "❌ *ᴇʀʀᴏʀ*\n\n> Video tidak ditemukan";

/// `m.reply` cabang premium (`bocil.js:35-37`).
pub(crate) fn pesan_premium(prefix: &str) -> String {
    format!(
        "🔞 *Konten Premium*\n\n\
         Video ini hanya bisa diakses pengguna *Premium*.\n\
         > Ketik *{prefix}benefitpremium* untuk info upgrade"
    )
}

/// `loadJsonData('bocil.json')` — item dibiarkan utuh supaya `item.url` yang
/// hilang tetap jatuh ke cabang `catch` seperti di JS.
fn muat_data(berkas: &dyn oktz_plugin_api::berkas::Berkas) -> Vec<Value> {
    let Ok(teks) = berkas.baca_teks(PATH) else {
        return Vec::new();
    };
    match serde_json::from_str::<Value>(&teks) {
        Ok(Value::Array(arr)) => arr,
        _ => Vec::new(),
    }
}

#[plugin(
    name = "bocil",
    alias = ["bocilvid"],
    category = "asupan",
    description = "Video bocil",
    usage = ".bocil",
    example = ".bocil",
    cooldown = 10,
    energi = 1,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn bocil(ctx: Ctx) -> Reply {
    let reaksi = |emoji: &str| Reply::None.with_reaction(emoji);

    if !ctx.is_premium && !ctx.is_owner {
        return Reply::sequence(vec![reaksi("🕕"), Reply::text(pesan_premium(&ctx.prefix))]);
    }

    let data = muat_data(ctx.berkas());
    if data.is_empty() {
        return Reply::sequence(vec![reaksi("🕕"), reaksi("❌"), Reply::text(ERR_KOSONG)]);
    }

    let item = &data[random_index(data.len())];
    let Some(url) = item.get("url").and_then(Value::as_str) else {
        // `sock.sendMedia(m.chat, undefined, ...)` melempar di JS → `catch`.
        return Reply::sequence(vec![reaksi("🕕"), reaksi("❌"), Reply::text(ERR_VIDEO)]);
    };

    Reply::sequence(vec![
        reaksi("🕕"),
        Reply::video(MediaSource::url(url.to_string()), ""),
        reaksi("✅"),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::berkas::BerkasPalsu;
    use std::sync::Arc;

    fn ctx(berkas: BerkasPalsu, premium: bool, owner: bool) -> Ctx {
        Ctx {
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            prefix: ".".into(),
            command: "bocil".into(),
            is_premium: premium,
            is_owner: owner,
            berkas: Arc::new(berkas),
            ..Ctx::default()
        }
    }

    fn bagian(r: &Reply) -> Vec<Reply> {
        match r.without_mutations() {
            Reply::Sequence(v) => v.iter().map(|x| x.without_mutations().clone()).collect(),
            lain => vec![lain.clone()],
        }
    }

    fn emoji_ke(r: &Reply, i: usize) -> Option<String> {
        match bagian(r).get(i) {
            Some(Reply::Reaction { emoji, .. }) => Some(emoji.clone()),
            _ => None,
        }
    }

    fn teks_ke(r: &Reply, i: usize) -> Option<String> {
        bagian(r).get(i)?.text_body().map(str::to_string)
    }

    fn https_berisi() -> BerkasPalsu {
        let b = BerkasPalsu::baru();
        let _ = b.tulis_teks(
            PATH,
            r#"[{"url":"https://cdn/bocil1.mp4"},{"url":"https://cdn/bocil2.mp4"}]"#,
        );
        b
    }

    #[test]
    fn meta_sama_dengan_js() {
        let p = oktz_plugin_api::find("bocil").unwrap();
        assert_eq!(p.meta.name, "bocil");
        assert_eq!(p.meta.alias, ["bocilvid"]);
        assert_eq!(p.meta.category, "asupan");
        assert_eq!(p.meta.description, "Video bocil");
        assert_eq!(p.meta.usage, ".bocil");
        assert_eq!(p.meta.example, ".bocil");
        assert_eq!(p.meta.cooldown, 10);
        assert_eq!(p.meta.energi, 1);
        assert!(!p.meta.is_premium && !p.meta.is_owner && !p.meta.is_group);
    }

    #[test]
    fn pengguna_biasa_ditolak_setelah_reaksi_jam() {
        let r = bocil(ctx(https_berisi(), false, false));
        assert_eq!(emoji_ke(&r, 0).as_deref(), Some("🕕"));
        assert_eq!(
            teks_ke(&r, 1).as_deref(),
            Some(
                "🔞 *Konten Premium*\n\n\
                 Video ini hanya bisa diakses pengguna *Premium*.\n\
                 > Ketik *.benefitpremium* untuk info upgrade"
            )
        );
        assert_eq!(bagian(&r).len(), 2);
        // Tidak ada media yang dikirim.
        assert!(!r.has_media());
    }

    #[test]
    fn premium_dan_owner_tembus_gate() {
        for (premium, owner) in [(true, false), (false, true)] {
            let r = bocil(ctx(https_berisi(), premium, owner));
            assert!(r.has_media(), "premium={premium} owner={owner}");
        }
    }

    #[test]
    fn data_kosong_atau_berkas_hilang_jadi_pesan_kosong() {
        let kosong = BerkasPalsu::baru();
        let _ = kosong.tulis_teks(PATH, "[]");
        for b in [BerkasPalsu::baru(), kosong] {
            let r = bocil(ctx(b, true, false));
            assert_eq!(emoji_ke(&r, 0).as_deref(), Some("🕕"));
            assert_eq!(emoji_ke(&r, 1).as_deref(), Some("❌"));
            assert_eq!(teks_ke(&r, 2).as_deref(), Some(ERR_KOSONG));
        }
    }

    #[test]
    fn json_rusak_diperlakukan_seperti_data_kosong() {
        let b = BerkasPalsu::baru();
        let _ = b.tulis_teks(PATH, "bukan json");
        let r = bocil(ctx(b, true, false));
        assert_eq!(teks_ke(&r, 2).as_deref(), Some(ERR_KOSONG));
    }

    #[test]
    fn item_tanpa_url_masuk_cabang_catch() {
        let b = BerkasPalsu::baru();
        let _ = b.tulis_teks(PATH, r#"[{"judul":"tanpa url"}]"#);
        let r = bocil(ctx(b, true, false));
        assert_eq!(emoji_ke(&r, 1).as_deref(), Some("❌"));
        assert_eq!(teks_ke(&r, 2).as_deref(), Some(ERR_VIDEO));
        assert!(!r.has_media());
    }

    #[test]
    fn url_terpilih_dikirim_sebagai_video_lalu_reaksi_centang() {
        let r = bocil(ctx(https_berisi(), false, true));
        assert_eq!(emoji_ke(&r, 0).as_deref(), Some("🕕"));

        let Some(Reply::Video { media, .. }) = bagian(&r).get(1).cloned() else {
            panic!("pesan kedua harus video");
        };
        match media {
            Some(MediaSource::Url { url, .. }) => {
                assert!(url.ends_with(".mp4"), "url={url}");
            }
            lain => panic!("sumber harus URL, dapat {lain:?}"),
        }

        assert_eq!(emoji_ke(&r, 2).as_deref(), Some("✅"));
    }
}
