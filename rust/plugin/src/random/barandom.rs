//! `plugins/random/barandom.js` — kirim satu gambar acak Blue Archive.
//!
//! `ponytail:` cabang `catch` JS (reaksi `☢` + `te(...)`) tidak bisa terjadi
//! di sini: plugin ini tidak melakukan HTTP apa pun, pipeline yang mengunduh
//! URL dan melaporkan kegagalan sebagai `PipelineError`, bukan sebagai reply.
//! Reaksi `🕕` di JS juga hilang — hanya reaksi terakhir (`✅`) yang dibawa.

use oktz_plugin_api::{Ctx, MediaSource, Reply, plugin};

pub(crate) const BARANDOM_URL: &str = "https://api.nexray.web.id/random/ba";

#[plugin(
    name = "barandom",
    alias = ["bluearchive", "ba"],
    category = "random",
    description = "Random gambar Blue Archive",
    usage = ".barandom",
    example = ".barandom",
    cooldown = 5,
    energi = 1,
    is_group = false,
    is_private = false
)]
pub fn barandom(_ctx: Ctx) -> Reply {
    Reply::image(MediaSource::url(BARANDOM_URL), "").with_reaction("✅")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run() -> Reply {
        (oktz_plugin_api::find("barandom").unwrap().handler)(Ctx {
            prefix: ".".into(),
            command: "barandom".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            ..Default::default()
        })
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("barandom").unwrap();
        assert_eq!(p.meta.name, "barandom");
        assert_eq!(p.meta.alias, ["bluearchive", "ba"]);
        assert_eq!(p.meta.category, "random");
        assert_eq!(p.meta.description, "Random gambar Blue Archive");
        assert_eq!(p.meta.usage, ".barandom");
        assert_eq!(p.meta.example, ".barandom");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 1);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(p.meta.is_enabled);
        assert!(!p.meta.is_owner && !p.meta.is_premium && !p.meta.is_admin);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for key in ["barandom", "bluearchive", "ba"] {
            assert_eq!(
                oktz_plugin_api::find(key).map(|p| p.meta.name.as_str()),
                Some("barandom"),
                "key {key} salah resolve"
            );
        }
    }

    #[test]
    fn the_normal_branch_sends_the_blue_archive_endpoint_as_an_image() {
        let r = run();
        assert!(r.has_media(), "harus image, bukan teks");
        match r.media_source().expect("sumber media") {
            MediaSource::Url { url, .. } => assert_eq!(url, BARANDOM_URL),
            other => panic!("sumber harus Url, dapat {other:?}"),
        }
    }

    #[test]
    fn the_image_has_no_caption_because_js_passes_none() {
        let r = run();
        assert_eq!(r.media_source().map(MediaSource::label).as_deref(), Some("url"));
        assert_eq!(r.text_body(), Some(""), "sock.sendMedia(.., caption=null)");
    }

    #[test]
    fn the_last_reaction_is_kept_and_does_not_hide_the_image() {
        let r = run();
        let mut cur = r.without_mutations();
        while let Reply::Reaction { emoji, reply } = cur {
            assert_eq!(emoji, "✅", "reaksi JS terakhir sebelum kirim media");
            cur = reply;
        }
        assert!(cur.has_media(), "reaksi tidak boleh menutupi media");
        assert!(r.mutations().is_empty(), "plugin ini hanya membaca");
    }

    #[test]
    fn an_empty_context_still_produces_the_same_image() {
        // JS tidak pernah membaca `m`, jadi tidak ada cabang yang bisa gagal.
        let r = barandom(Ctx::default());
        assert!(r.has_media());
    }
}