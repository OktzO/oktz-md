//! `plugins/fun/putus.rs` — putuskan hubungan dengan pasangan tersimpan di
//! `user.fun.pasangan`.
//!
//! `ponytail:` dua hal tidak bisa dicerminkan persis:
//!
//! 1. `delete senderData.fun.pasangan` tidak ada padanannya di
//!    `Mutation::SetUser` (itu merge `??`), jadi `pasangan` ditulis `null`.
//!    Pembaca JS `userData.fun?.pasangan` memperlakukan `null` sama dengan
//!    tidak ada — tapi byte-nya berbeda.
//! 2. `db.getUser(exPartner)` hanya bisa dijawab kalau pasangannya masih ada di
//!    roster grup (`Participant::data`). Untuk pasangan yang tidak ada di grup,
//!    `isMutual` selalu false dan pencatatan di sisi lama ikut hilang — sama
//!    seperti `db.getUser` untuk user yang belum pernah ada.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};

use crate::common::jid_number;

pub(crate) const NO_PARTNER: &str = "❌ *Kamu gak ada pacar wehh*";

/// `Mutation::SetUser` yang mengosongkan pasangan — lihat catatan `ponytail:`.
fn clear_partner(jid: &str) -> Mutation {
    Mutation::SetUser {
        jid: jid.to_string(),
        data: serde_json::json!({ "fun": { "pasangan": null } }),
    }
}

/// `exData.fun?.pasangan === m.sender` — hanya bisa dijawab untuk member grup.
fn is_mutual(ctx: &Ctx, partner_jid: &str) -> bool {
    ctx.roster
        .participants
        .iter()
        .find(|p| p.jid == partner_jid)
        .and_then(|p| p.data.get("fun"))
        .and_then(|f| f.get("pasangan"))
        .and_then(serde_json::Value::as_str)
        == Some(ctx.sender.as_str())
}

#[plugin(
    name = "putus",
    alias = ["breakup"],
    category = "fun",
    description = "Memutuskan hubungan dengan pasangan",
    usage = ".putus",
    example = ".putus",
    cooldown = 60,
    energi = 0,
    is_group = true,
    is_private = false
)]
pub fn putus(ctx: Ctx) -> Reply {
    let partner = ctx
        .user_field("fun.pasangan")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string);

    let Some(partner) = partner else {
        return Reply::text(format!(
            "{NO_PARTNER}\n\nCari dulu dengan `{}tembak @tag`",
            ctx.prefix
        ))
        .with_reaction("❌");
    };

    let mut mutations = vec![clear_partner(&ctx.sender)];
    if is_mutual(&ctx, &partner) {
        mutations.push(clear_partner(&partner));
    }

    let text = format!(
        "💔 *PUTUS!*\n\n@{} dan @{} resmi putus !!\n\nSemoga mendapat yang lebih baik! 🙏",
        jid_number(&ctx.sender),
        jid_number(&partner)
    );
    Reply::Text {
        text,
        mentions: vec![ctx.sender.clone(), partner],
    }
    .mutating(mutations)
    .with_reaction("💔")
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::registry::{GroupRoster, Participant};
    use serde_json::{Value};

    const ME: &str = "628111@s.whatsapp.net";
    const THEM: &str = "628222@s.whatsapp.net";

    fn run(user_data: Value, roster: GroupRoster) -> Reply {
        (oktz_plugin_api::find("putus").unwrap().handler)(Ctx {
            prefix: ".".into(),
            command: "putus".into(),
            sender: ME.into(),
            chat: "120363@g.us".into(),
            is_group: true,
            user_data,
            roster,
            ..Default::default()
        })
    }

    fn member(jid: &str, data: Value) -> GroupRoster {
        GroupRoster {
            participants: vec![Participant {
                jid: jid.into(),
                is_admin: false,
                is_owner: false,
                data,
            }],
            is_bot_admin: false,
        }
    }

    /// JID yang ditulis `clear_partner`, urut + dedup — bentuk yang dibandingkan
    /// sama di semua test jalur tulis.
    fn cleared_by(reply: &Reply) -> Vec<String> {
        let mut out = Vec::new();
        for m in reply.mutations() {
            let Mutation::SetUser { jid, data } = m else {
                panic!("mutasi harus SetUser, dapat {m:?}");
            };
            assert_eq!(data["fun"]["pasangan"], Value::Null, "harus mengosongkan pasangan");
            out.push(jid.clone());
        }
        out.sort();
        out.dedup();
        out
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("putus").unwrap();
        assert_eq!(p.meta.name, "putus");
        assert_eq!(p.meta.alias, ["breakup"]);
        assert_eq!(p.meta.category, "fun");
        assert_eq!(p.meta.description, "Memutuskan hubungan dengan pasangan");
        assert_eq!(p.meta.usage, ".putus");
        assert_eq!(p.meta.example, ".putus");
        assert_eq!(p.meta.cooldown, 60);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_group && !p.meta.is_private);
        assert!(p.meta.is_enabled);
        assert!(!p.meta.is_owner && !p.meta.is_premium && !p.meta.is_admin);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for key in ["putus", "breakup"] {
            assert_eq!(
                oktz_plugin_api::find(key).map(|p| p.meta.name.as_str()),
                Some("putus"),
                "key {key} salah resolve"
            );
        }
    }

    #[test]
    fn the_normal_branch_announces_the_breakup_and_mentions_both() {
        let r = run(serde_json::json!({ "fun": { "pasangan": THEM } }), GroupRoster::default());
        let (emoji, inner) = match r.without_mutations() {
            Reply::Reaction { emoji, reply } => (emoji.clone(), reply.as_ref()),
            other => panic!("harus Reaction, dapat {other:?}"),
        };
        assert_eq!(emoji, "💔");
        match inner {
            Reply::Text { text, mentions } => {
                assert!(text.starts_with("💔 *PUTUS!*\n\n"), "{text}");
                assert!(
                    text.contains("@628111 dan @628222 resmi putus !!"),
                    "{text}"
                );
                assert!(text.ends_with("Semoga mendapat yang lebih baik! 🙏"), "{text}");
                assert_eq!(mentions, &[ME.to_string(), THEM.to_string()]);
            }
            other => panic!("harus Text, dapat {other:?}"),
        }
    }

    #[test]
    fn a_mutual_partner_in_the_roster_is_cleared_too() {
        let roster = member(THEM, serde_json::json!({ "fun": { "pasangan": ME } }));
        let r = run(serde_json::json!({ "fun": { "pasangan": THEM } }), roster);
        assert_eq!(cleared_by(&r), [ME.to_string(), THEM.to_string()]);
    }

    #[test]
    fn a_one_sided_pair_only_clears_the_sender() {
        // Mantan yang balas-follow tapi tidak ada di grup: `db.getUser` tidak
        // terbaca, jadi `isMutual` false — persis cabang tanpa `if` di JS.
        let roster = member(THEM, serde_json::json!({ "fun": { "pasangan": "999@s.whatsapp.net" } }));
        let r = run(serde_json::json!({ "fun": { "pasangan": THEM } }), roster);
        assert_eq!(cleared_by(&r), [ME.to_string()]);
    }

    #[test]
    fn the_sender_is_written_exactly_once() {
        let roster = member(THEM, serde_json::json!({ "fun": { "pasangan": ME } }));
        let r = run(serde_json::json!({ "fun": { "pasangan": THEM } }), roster);
        assert_eq!(r.mutations().len(), 2, "dua mutasi, tidak ada duplikat");
    }

    #[test]
    fn without_a_partner_it_reacts_with_a_cross_and_writes_nothing() {
        for data in [
            serde_json::json!({}),
            serde_json::json!({ "fun": {} }),
            serde_json::json!({ "fun": { "pasangan": null } }),
            serde_json::json!({ "fun": null }),
        ] {
            let r = run(data.clone(), GroupRoster::default());
            assert!(
                r.mutations().is_empty(),
                "tidak boleh menulis tanpa pasangan: {data}"
            );
            let text = r.text_body().expect("harus ada teks");
            assert!(text.starts_with(NO_PARTNER), "{data} -> {text}");
            assert!(text.contains("Cari dulu dengan `.tembak @tag`"), "{text}");
            assert!(matches!(
                r.without_mutations(),
                Reply::Reaction { emoji, .. } if emoji == "❌"
            ));
        }
    }

    #[test]
    fn the_breakup_hint_uses_the_live_prefix() {
        let r = (oktz_plugin_api::find("putus").unwrap().handler)(Ctx {
            prefix: "!".into(),
            sender: ME.into(),
            ..Default::default()
        });
        assert!(r.text_body().unwrap().contains("`!tembak @tag`"));
    }
}
