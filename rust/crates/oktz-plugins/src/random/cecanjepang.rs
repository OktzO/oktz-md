//! `plugins/random/cecanjepang.js` — kirim satu gambar acak dari API cecan Jepang.
//!
//! `ponytail:` reaksi hilang. JS bereaksi dua kali (bendera lalu `✅`) lewat
//! `m.react`. Di sini `Reply::with_reaction` membungkus reply dalam
//! `Reply::Reaction`, sedangkan `Reply::has_media`/`media_source` hanya
//! menembus `Reply::Mutate` — jadi gambar yang dibungkus reaksi tidak akan
//! pernah di-upload. Gambar lebih penting daripada reaksi, jadi reaksi
//! dibuang demi media yang benar-benar terkirim.

use oktz_plugin_api::{Ctx, MediaSource, Reply, plugin};

pub(crate) const CECANJEPANG_URL: &str = "https://api.nexray.web.id/random/cecan/japan";

pub(crate) fn cecanjepang_reply() -> Reply {
    Reply::image(MediaSource::url(CECANJEPANG_URL), "")
}

#[plugin(
    name = "cecanjepang",
    alias = ["cewekjepang", "cewekjpn"],
    category = "cecan",
    description = "Random gambar cewek cantik Jepang",
    usage = ".cecanjepang",
    example = ".cecanjepang",
    cooldown = 5,
    energi = 1,
    is_group = false,
    is_private = false
)]
pub fn cecanjepang(_ctx: Ctx) -> Reply {
    cecanjepang_reply()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run() -> Reply {
        (oktz_plugin_api::find("cecanjepang").unwrap().handler)(Ctx {
            prefix: ".".into(),
            command: "cecanjepang".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            ..Default::default()
        })
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("cecanjepang").unwrap();
        assert_eq!(p.meta.name, "cecanjepang");
        assert_eq!(p.meta.alias, ["cewekjepang", "cewekjpn"]);
        assert_eq!(p.meta.category, "cecan");
        assert_eq!(p.meta.description, "Random gambar cewek cantik Jepang");
        assert_eq!(p.meta.usage, ".cecanjepang");
        assert_eq!(p.meta.example, ".cecanjepang");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 1);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(p.meta.is_enabled);
        assert!(!p.meta.is_owner && !p.meta.is_premium && !p.meta.is_admin);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for key in ["cecanjepang", "cewekjepang", "cewekjpn"] {
            assert_eq!(
                oktz_plugin_api::find(key).map(|p| p.meta.name.as_str()),
                Some("cecanjepang"),
                "key {key} salah resolve"
            );
        }
    }

    #[test]
    fn the_normal_branch_sends_the_jepang_endpoint_as_an_image() {
        let r = run();
        assert!(r.has_media(), "harus image, bukan teks");
        assert_eq!(r.media_source().map(MediaSource::label).as_deref(), Some("url"));
        match r.media_source().expect("sumber media") {
            MediaSource::Url { url, .. } => assert_eq!(url, CECANJEPANG_URL),
            other => panic!("sumber harus Url, dapat {other:?}"),
        }
    }

    #[test]
    fn the_js_flag_reaction_is_deliberately_dropped() {
        // JS bereaksi "🇯🇵" lebih dulu. Mempertahkannya lewat
        // `with_reaction` akan membuat gambar tidak pernah terkirim.
        let r = run();
        assert!(
            !matches!(r.without_mutations(), Reply::Reaction { .. }),
            "reaksi membungkus media dan membuatnya tidak terkirim"
        );
        assert!(
            r.has_media(),
            "gambar harus tetap terkirim — itu yang dikorbankan demi \
             hilangnya reaksi"
        );
        assert!(r.mutations().is_empty(), "plugin ini hanya membaca");
    }

    #[test]
    fn an_empty_context_still_produces_the_same_image() {
        // JS tidak pernah membaca \`m\`, jadi tidak ada cabang yang bisa gagal.
        let r = cecanjepang(Ctx::default());
        assert!(r.has_media());
    }
}
