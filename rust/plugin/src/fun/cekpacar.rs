//! `plugins/fun/cekpacar.js` — status hubungan `user.fun.pasangan`.
//!
//! `ponytail:` `Ctx` hanya membawa store untuk `sender` (`user_data`) dan untuk
//! member grup (`Participant::data`). Target dari quote/mention/angka yang TIDAK
//! ada di roster diperlakukan seperti `db.getUser(...) || {}` — sama dengan
//! user yang belum pernah ada, jadi jawabannya "tidak punya pasangan".
//! Akibatnya `isMutual` hanya bisa `true` untuk pasangan yang ada di roster.

use oktz_plugin_api::{Ctx, Reply, plugin};

use crate::common::jid_number;

const HEADER: &str = "*sᴛᴀᴛᴜs ʜᴜʙᴜɴɢᴀɴ*";

pub(crate) const NO_PAIR_REACT: &str = "💔";

/// Target + flag `isOther`, urutan cabang JS persis:
/// `quoted` → `mentionedJid[0]` → `args[0]` (hanya 6..19 digit).
pub(crate) fn resolve_target(ctx: &Ctx) -> (String, bool) {
    if let Some(q) = ctx.quoted() {
        return (q.sender.clone(), true);
    }
    if let Some(jid) = ctx.mentioned.first() {
        return (jid.clone(), true);
    }
    if let Some(arg) = ctx.args.first() {
        let num: String = arg.chars().filter(char::is_ascii_digit).collect();
        if num.chars().count() > 5 && num.chars().count() < 20 {
            return (format!("{num}@s.whatsapp.net"), true);
        }
    }
    (ctx.sender.clone(), false)
}

/// `isOther ? '@<nomor>' : 'Kamu'`.
fn nama(target: &str, is_other: bool) -> String {
    if is_other {
        format!("@{}", jid_number(target))
    } else {
        "Kamu".to_string()
    }
}

/// `db.getUser(jid).fun?.pasangan` — `None` kalau tidak ada pasangannya.
pub(crate) fn pasangan_of(ctx: &Ctx, jid: &str) -> Option<String> {
    let fun = if jid == ctx.sender {
        ctx.user_field("fun")
    } else {
        ctx.roster
            .participants
            .iter()
            .find(|p| p.jid == jid)
            .and_then(|p| p.data.get("fun"))
    };
    fun?.get("pasangan")?.as_str().map(str::to_string)
}

#[plugin(
    name = "cekpacar",
    alias = ["pacar", "pasangan", "gebetan"],
    category = "fun",
    description = "Cek status hubungan seseorang",
    usage = ".cekpacar atau .cekpacar @tag",
    example = ".cekpacar",
    cooldown = 10,
    energi = 0,
    is_group = true,
    is_private = false
)]
pub fn cekpacar(ctx: Ctx) -> Reply {
    let (target, is_other) = resolve_target(&ctx);
    let nama = nama(&target, is_other);

    let Some(partner) = pasangan_of(&ctx, &target) else {
        return Reply::Text {
            text: format!(
                "💔 {HEADER}\n\n*{nama}* tidak punya pasangan.\n\
                 TIP: Cari pasangan dulu dengan `{}tembak @tag`",
                ctx.prefix
            ),
            mentions: if is_other { vec![target] } else { Vec::new() },
        }
        .with_reaction(NO_PAIR_REACT);
    };

    let partner_num = jid_number(&partner).to_string();
    let mentions = vec![target.clone(), partner.clone()];
    if pasangan_of(&ctx, &partner).as_deref() == Some(target.as_str()) {
        Reply::Text {
            text: format!("💕 {HEADER}\n\n*{nama}* sedang pacaran dengan @{partner_num}! 🥳"),
            mentions,
        }
        .with_reaction("💕")
    } else {
        Reply::Text {
            text: format!(
                "💭 {HEADER}\n\n*{nama}* lagi pdkt sama @{partner_num}\n\
                 Status: *Digantung* 😅\n\nMenunggu jawaban..."
            ),
            mentions,
        }
        .with_reaction("💭")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::QuotedKind;
    use oktz_plugin_api::registry::{GroupRoster, Participant};
    use serde_json::{Value};

    const ME: &str = "628111@s.whatsapp.net";
    const THEM: &str = "628222@s.whatsapp.net";

    fn roster(members: &[(&str, Value)]) -> GroupRoster {
        GroupRoster {
            participants: members
                .iter()
                .map(|(jid, data)| Participant {
                    jid: (*jid).to_string(),
                    is_admin: false,
                    is_owner: false,
                    data: data.clone(),
                })
                .collect(),
            is_bot_admin: false,
        }
    }

    fn run(ctx: Ctx) -> Reply {
        (oktz_plugin_api::find("cekpacar").unwrap().handler)(ctx)
    }

    fn base() -> Ctx {
        Ctx {
            prefix: ".".into(),
            command: "cekpacar".into(),
            sender: ME.into(),
            chat: "120363@g.us".into(),
            is_group: true,
            ..Default::default()
        }
    }

    fn text_and_reaction(r: &Reply) -> (String, String) {
        assert!(
            matches!(r.without_mutations(), Reply::Reaction { .. }),
            "setiap cabang bereaksi: {r:?}"
        );
        match r.without_mutations() {
            Reply::Reaction { emoji, reply } => {
                assert!(reply.text_body().is_some(), "reaksi harus membungkus teks");
                (emoji.clone(), reply.text_body().unwrap().to_string())
            }
            other => panic!("harus Reaction, dapat {other:?}"),
        }
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("cekpacar").unwrap();
        assert_eq!(p.meta.name, "cekpacar");
        assert_eq!(p.meta.alias, ["pacar", "pasangan", "gebetan"]);
        assert_eq!(p.meta.category, "fun");
        assert_eq!(p.meta.description, "Cek status hubungan seseorang");
        assert_eq!(p.meta.usage, ".cekpacar atau .cekpacar @tag");
        assert_eq!(p.meta.example, ".cekpacar");
        assert_eq!(p.meta.cooldown, 10);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_group && !p.meta.is_private);
        assert!(p.meta.is_enabled);
        assert!(!p.meta.is_owner && !p.meta.is_premium && !p.meta.is_admin);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for key in ["cekpacar", "pacar", "pasangan", "gebetan"] {
            assert_eq!(
                oktz_plugin_api::find(key).map(|p| p.meta.name.as_str()),
                Some("cekpacar"),
                "key {key} salah resolve"
            );
        }
    }

    #[test]
    fn target_resolution_follows_the_quoted_mention_arg_order() {
        let mut ctx = base();
        assert_eq!(resolve_target(&ctx), (ME.to_string(), false));

        ctx.quoted = Some(oktz_plugin_api::QuotedMessage {
            id: "x".into(),
            sender: THEM.into(),
            chat: "120363@g.us".into(),
            text: String::new(),
            kind: QuotedKind::Text,
            mimetype: None,
            file_name: None,
            from_me: false,
            timestamp: 0,
        });
        ctx.mentioned = vec!["628999@s.whatsapp.net".into()];
        ctx.args = vec!["628333".into()];
        assert_eq!(resolve_target(&ctx), (THEM.to_string(), true), "quoted menang");

        ctx.quoted = None;
        assert_eq!(
            resolve_target(&ctx),
            ("628999@s.whatsapp.net".to_string(), true),
            "mention menang atas argumen"
        );
    }

    #[test]
    fn a_numeric_argument_is_only_accepted_between_6_and_19_digits() {
        let mut ctx = base();
        // `num.length > 5 && num.length < 20` — batas dikecualikan.
        for (arg, expected_other) in [
            ("12345", false),
            ("123456", true),
            ("62811122233344455566", false),
            ("+62 811-11", true),
            ("abc", false),
        ] {
            ctx.args = vec![arg.to_string()];
            let (target, is_other) = resolve_target(&ctx);
            assert_eq!(is_other, expected_other, "arg {arg}");
            if expected_other {
                assert_eq!(target, format!("{}@s.whatsapp.net", arg.chars().filter(char::is_ascii_digit).collect::<String>()));
            } else {
                assert_eq!(target, ME);
            }
        }
    }

    #[test]
    fn the_normal_branch_reports_a_mutual_partnership() {
        let ctx = Ctx {
            user_data: serde_json::json!({ "fun": { "pasangan": THEM } }),
            roster: roster(&[
                (ME, serde_json::json!({ "fun": { "pasangan": THEM } })),
                (THEM, serde_json::json!({ "fun": { "pasangan": ME } })),
            ]),
            ..base()
        };
        let (emoji, text) = text_and_reaction(&run(ctx.clone()));
        assert_eq!(emoji, "💕");
        assert!(text.starts_with("💕 *sᴛᴀᴛᴜs ʜᴜʙᴜɴɢᴀɴ*\n\n"), "{text}");
        assert!(
            text.contains("*Kamu* sedang pacaran dengan @628222! 🥳"),
            "{text}"
        );
    }

    #[test]
    fn a_one_sided_pair_is_reported_as_hanging() {
        let ctx = Ctx {
            user_data: serde_json::json!({ "fun": { "pasangan": THEM } }),
            roster: roster(&[(THEM, serde_json::json!({ "fun": { "pasangan": "999@s.whatsapp.net" } }))]),
            ..base()
        };
        let (emoji, text) = text_and_reaction(&run(ctx.clone()));
        assert_eq!(emoji, "💭");
        assert!(text.contains("*Kamu* lagi pdkt sama @628222"), "{text}");
        assert!(text.contains("Status: *Digantung* 😅"), "{text}");
        assert!(text.ends_with("Menunggu jawaban..."), "{text}");
    }

    #[test]
    fn without_a_partner_it_says_so_and_mentions_nobody() {
        for data in [serde_json::json!({}), serde_json::json!({ "fun": {} }), serde_json::json!({ "fun": { "pasangan": null } })] {
            let ctx = Ctx {
                user_data: data.clone(),
                ..base()
            };
            let (emoji, text) = text_and_reaction(&run(ctx.clone()));
            assert_eq!(emoji, "💔", "{data}");
            assert!(
                text.starts_with("💔 *sᴛᴀᴛᴜs ʜᴜʙᴜɴɢᴀɴ*\n\n*Kamu* tidak punya pasangan."),
                "{data} -> {text}"
            );
            assert!(text.contains("TIP: Cari pasangan dulu dengan `.tembak @tag`"), "{text}");
            match run(ctx.clone()).without_mutations() {
                Reply::Reaction { reply, .. } => match reply.as_ref() {
                    Reply::Text { mentions, .. } => assert!(mentions.is_empty(), "diri sendiri tidak di-mention"),
                    other => panic!("harus Text, dapat {other:?}"),
                },
                other => panic!("harus Reaction, dapat {other:?}"),
            }
        }
    }

    #[test]
    fn a_mentioned_target_is_named_and_mentioned() {
        let ctx = Ctx {
            mentioned: vec![THEM.into()],
            roster: roster(&[(THEM, serde_json::json!({ "fun": { "pasangan": ME } }))]),
            user_data: serde_json::json!({ "fun": { "pasangan": THEM } }),
            ..base()
        };
        let (_, text) = text_and_reaction(&run(ctx.clone()));
        assert!(text.contains("*@628222* sedang pacaran dengan @628111! 🥳"), "{text}");
        match run(ctx.clone()).without_mutations() {
            Reply::Reaction { reply, .. } => match reply.as_ref() {
                Reply::Text { mentions, .. } => {
                    assert_eq!(mentions, &[THEM.to_string(), ME.to_string()])
                }
                other => panic!("harus Text, dapat {other:?}"),
            },
            other => panic!("harus Reaction, dapat {other:?}"),
        }
    }

    #[test]
    fn the_tip_uses_the_live_prefix() {
        let ctx = Ctx {
            prefix: "!".into(),
            ..base()
        };
        assert!(text_and_reaction(&run(ctx.clone())).1.contains("`!tembak @tag`"));
    }

    #[test]
    fn the_plugin_only_reads_never_writes() {
        let ctx = Ctx {
            user_data: serde_json::json!({ "fun": { "pasangan": THEM } }),
            roster: roster(&[(THEM, serde_json::json!({ "fun": { "pasangan": ME } }))]),
            ..base()
        };
        assert!(run(ctx.clone()).mutations().is_empty());
    }
}
