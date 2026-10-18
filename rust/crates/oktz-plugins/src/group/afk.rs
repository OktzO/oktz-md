//! `plugins/group/afk.js` — set status AFK dengan alasan.
//!
//! `ponytail:` hanya `handler` yang dipindah. `checkAfk` (`afk.js:76-126`)
//! adalah hook middleware yang dipanggil sebelum plugin lain — ia perlu
//! `db.getUser` untuk JID peserta yang di-mention (yang tidak ada di `Ctx`,
//! hanya ada untuk `sender`) dan `notifyThrottle` lintas-pesan, jadi tidak
//! ikut. Lihat laporan batch.
//!
//! `sweepAfkStorage` juga tidak ikut: penyapuan based-on-ukuran hanya
//! memangkas cache in-memory, dan `Ctx` tidak punya cache AFK sendiri.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};

pub(crate) const NO_REASON: &str = "Tidak ada alasan";

/// `canPersist(jid)` — JID yang aman ditulis ke store. Key `lid` (>15 digit)
/// dan yang berawalan `120` tidak bisa dipakai sebagai key store, jadi AFK-nya
/// hanya hidup selama ada di memori.
pub(crate) fn can_persist(jid: &str) -> bool {
    let clean = jid.split('@').next().unwrap_or(jid);
    clean.len() <= 15 && !clean.starts_with("120")
}

#[plugin(
    name = "afk",
    alias = ["away", "brb"],
    category = "group",
    description = "Set status AFK dengan alasan",
    usage = ".afk <alasan>",
    example = ".afk lagi makan",
    cooldown = 5,
    energi = 0,
    is_group = false,
    is_private = false
)]
pub fn afk(ctx: Ctx) -> Reply {
    let reason = if ctx.text.is_empty() {
        NO_REASON.to_string()
    } else {
        ctx.text.clone()
    };
    let record = serde_json::json!({ "reason": reason, "since": crate::common::now_ms() });

    let reply = Reply::Text {
        text: format!(
            "💤 *ᴀꜰᴋ ᴀᴋᴛɪꜰ*\n\n\
             ```@{} sekarang AFK```\n\
             🍀 `Alasan:` *{}*\n\n\
             _Ketik apapun untuk menonaktifkan AFK._",
            crate::common::jid_number(&ctx.sender),
            reason
        ),
        mentions: vec![ctx.sender.clone()],
    };

    if can_persist(&ctx.sender) {
        reply.with_mutation(Mutation::SetUser {
            jid: ctx.sender.clone(),
            data: serde_json::json!({ "afk": record }),
        })
    } else {
        reply
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    const SENDER: &str = "6281234567890@s.whatsapp.net";

    fn run(sender: &str, text: &str) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "afk".into(),
            sender: sender.into(),
            chat: "628999@s.whatsapp.net".into(),
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            ..Default::default()
        };
        (oktz_plugin_api::find("afk").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    fn written_afk(r: &Reply) -> &Value {
        assert_eq!(r.mutations().len(), 1, "harus tepat satu mutasi");
        match &r.mutations()[0] {
            Mutation::SetUser { jid, data } => {
                assert_eq!(jid, SENDER);
                &data["afk"]
            }
            other => panic!("harus SetUser, dapat {other:?}"),
        }
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("afk").unwrap();
        assert_eq!(p.meta.name, "afk");
        assert_eq!(p.meta.alias, ["away", "brb"]);
        assert_eq!(p.meta.category, "group");
        assert_eq!(p.meta.description, "Set status AFK dengan alasan");
        assert_eq!(p.meta.usage, ".afk <alasan>");
        assert_eq!(p.meta.example, ".afk lagi makan");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
        assert!(!p.meta.is_group, "JS: isGroup: false");
        assert!(!p.meta.is_private, "JS: isPrivate: false");
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["away", "brb"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "afk");
        }
    }

    #[test]
    fn the_sender_is_mentioned_in_the_reply() {
        match run(SENDER, "lagi makan").without_mutations() {
            Reply::Text { mentions, text } => {
                assert_eq!(mentions, &vec![SENDER.to_string()]);
                assert!(text.contains("@6281234567890 sekarang AFK"), "{text}");
                assert!(text.contains("🍀 `Alasan:` *lagi makan*"), "{text}");
            }
            other => panic!("harus Text, dapat {other:?}"),
        }
    }

    #[test]
    fn an_empty_reason_falls_back_to_the_default_wording() {
        let r = run(SENDER, "");
        assert!(
            text_of(&r).contains("🍀 `Alasan:` *Tidak ada alasan*"),
            "{}",
            text_of(&r)
        );
        assert_eq!(written_afk(&r)["reason"], serde_json::json!("Tidak ada alasan"));
    }

    #[test]
    fn the_reason_keeps_its_inner_spacing_and_case() {
        let r = run(SENDER, "  Makan  Nasi  ");
        assert_eq!(written_afk(&r)["reason"], serde_json::json!("  Makan  Nasi  "));
    }

    #[test]
    fn a_persistable_jid_is_written_to_the_store_with_a_timestamp() {
        let before = crate::common::now_ms();
        let r = run(SENDER, "jalan");
        let afk = written_afk(&r);
        assert_eq!(afk["reason"], serde_json::json!("jalan"));
        let since = afk["since"].as_i64().expect("since harus angka");
        assert!(
            (crate::common::now_ms() - since) < 5_000 && since >= before,
            "since harus waktu sekarang: {since}"
        );
    }

    #[test]
    fn a_lid_jid_is_not_written_but_still_gets_a_reply() {
        // Key `lid` >15 digit tidak bisa jadi key store.
        let lid = "12345678901234567@lid";
        let r = run(lid, "sibuk");
        assert!(r.mutations().is_empty(), "tidak boleh menulis ke store");
        assert!(text_of(&r).contains("sibuk"), "{}", text_of(&r));
    }

    #[test]
    fn a_group_jid_prefix_is_not_persistable() {
        assert!(can_persist(SENDER));
        assert!(can_persist("628123@lid"));
        assert!(!can_persist("120363123456789@g.us"), "prefix 120");
        assert!(!can_persist("1234567890123456@s.whatsapp.net"), ">15 digit");
        // `"".replace(/@.+/g, '')` tetap `""` dan panjangnya 0, jadi JS
        // justru Menganggapnya boleh dipersist. Meng faithfully; sender
        // kosong tidak mungkin terjadi dari router.
        assert!(can_persist(""));
    }

    #[test]
    fn a_120_prefixed_lid_is_also_skipped() {
        let r = run("120363123456789012@lid", "x");
        assert!(r.mutations().is_empty());
        assert!(text_of(&r).contains("```@120363123456789012 sekarang AFK```"));
    }
}