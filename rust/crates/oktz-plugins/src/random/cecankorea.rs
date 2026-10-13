//! `plugins/random/cecankorea.js` — kirim satu gambar acak dari API cecan Korea.
//!
//! `ponytail:` reaksi hilang. JS bereaksi dua kali (bendera lalu `✅`) lewat
//! `m.react`. Di sini `Reply::with_reaction` membungkus reply dalam
//! `Reply::Reaction`, sedangkan `Reply::has_media`/`media_source` hanya
//! menembus `Reply::Mutate` — jadi gambar yang dibungkus reaksi tidak akan
//! pernah di-upload. Gambar lebih penting daripada reaksi, jadi reaksi
//! dibuang demi media yang benar-benar terkirim.

use oktz_plugin_api::{Ctx, MediaSource, Reply, plugin};

pub(crate) const CECANKOREA_URL: &str = "https://api.nexray.web.id/random/cecan/korea";

pub(crate) fn cecankorea_reply() -> Reply {
    Reply::image(MediaSource::url(CECANKOREA_URL), "")
}

#[plugin(
    name = "cecankorea",
    alias = ["cewekkorea", "cewekkor"],
    category = "cecan",
    description = "Random gambar cewek cantik Korea",
    usage = ".cecankorea",
    example = ".cecankorea",
    cooldown = 5,
    energi = 1,
    is_group = false,
    is_private = false
)]
pub fn cecankorea(_ctx: Ctx) -> Reply {
    cecankorea_reply()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run() -> Reply {
        (oktz_plugin_api::find("cecankorea").unwrap().handler)(Ctx {
            prefix: ".".into(),
            command: "cecankorea".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            ..Default::default()
        })
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("cecankorea").unwrap();
        assert_eq!(p.meta.name, "cecankorea");
        assert_eq!(p.meta.alias, ["cewekkorea", "cewekkor"]);
        assert_eq!(p.meta.category, "cecan");
        assert_eq!(p.meta.description, "Random gambar cewek cantik Korea");
        assert_eq!(p.meta.usage, ".cecankorea");
        assert_eq!(p.meta.example, ".cecankorea");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 1);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(p.meta.is_enabled);
        assert!(!p.meta.is_owner && !p.meta.is_premium && !p.meta.is_admin);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for key in ["cecankorea", "cewekkorea", "cewekkor"] {
            assert_eq!(
                oktz_plugin_api::find(key).map(|p| p.meta.name.as_str()),
                Some("cecankorea"),
                "key {key} salah resolve"
            );
        }
    }

    #[test]
    fn the_normal_branch_sends_the_korea_endpoint_as_an_image() {
        let r = run();
        assert!(r.has_media(), "harus image, bukan teks");
        assert_eq!(r.media_source().map(MediaSource::label).as_deref(), Some("url"));
        match r.media_source().expect("sumber media") {
            MediaSource::Url { url, .. } => assert_eq!(url, CECANKOREA_URL),
            other => panic!("sumber harus Url, dapat {other:?}"),
        }
    }

    #[test]
    fn the_js_flag_reaction_is_deliberately_dropped() {
        // JS bereaksi "🇰🇷" lebih dulu. Mempertahkannya lewat
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
        let r = cecankorea(Ctx::default());
        assert!(r.has_media());
    }
}
