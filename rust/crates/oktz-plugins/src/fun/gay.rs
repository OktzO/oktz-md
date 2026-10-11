//! `plugins/fun/gay.js` — nama command di JS adalah `gaycheck`, bukan `gay`
//! (`name: 'gaycheck'`, file-nya `gay.js`). Itu yang terdaftar di registry,
//! jadi nama yang dipakai di sini mengikuti `pluginConfig`, bukan nama file.
//!
//! Perhatikan: JS mengambil DUA anggota secara acak dengan dua pemanggilan
//! `Math.random()` terpisah (`gay.js:26-27`), jadi keduanya boleh orang yang
//! sama. Itu bukan bug yang perlu dibetulkan — mengacak sekali lalu mengulang
//! akan mengubah perilaku yang sudah ada.

use oktz_plugin_api::{Ctx, Reply, plugin};

use crate::common::{jid_number, random_index, roster_jids};

/// `m.reply(text, { mentions: [orang1, orang2] })` — Router yang transmit
/// mention ke `contextInfo`; di sini cukup dikembalikan sebagai JID.
fn build(members: &[String]) -> Option<(String, Vec<String>)> {
    // Dicek SEBELUM `random_index`: helper itu mengasumsikan `len > 0` (JS juga
    // begitu — `participants[NaN]` = undefined, bukan crash). Mengacak dulu
    // akan panic, dan "grup tanpa member" bukan kondisi yang boleh jadi panic.
    if members.is_empty() {
        return None;
    }
    let orang1 = members.get(random_index(members.len()))?;
    let orang2 = members.get(random_index(members.len()))?;
    Some((
        format!(
            "@{} *Nge gay sama* @{}",
            jid_number(orang1),
            jid_number(orang2)
        ),
        vec![orang1.clone(), orang2.clone()],
    ))
}

#[plugin(
    name = "gaycheck",
    alias = ["howgay"],
    category = "fun",
    description = "Menunjuk member paling gay di grup",
    usage = ".gaycheck",
    cooldown = 10,
    energi = 2,
    is_private = false,
    is_group = true,
    is_bot_admin = false,
    is_admin = false
)]
pub fn gaycheck(ctx: Ctx) -> Reply {
    let members = roster_jids(&ctx);
    match build(&members) {
        Some((text, mentions)) => Reply::Text { text, mentions },
        None => Reply::text("❌ Tidak ada member di grup!"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::all_plugins;
    use oktz_plugin_api::registry::{GroupRoster, Participant};

    fn ctx_of(jids: &[&str]) -> Ctx {
        Ctx {
            prefix: ".".into(),
            command: "gaycheck".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "120363@g.us".into(),
            is_group: true,
            roster: GroupRoster {
                participants: jids
                    .iter()
                    .map(|j| Participant {
                        jid: (*j).into(),
                        is_admin: false,
                        is_owner: false,
                    })
                    .collect(),
                is_bot_admin: false,
            },
            ..Default::default()
        }
    }

    fn meta() -> &'static oktz_plugin_api::PluginMeta {
        &oktz_plugin_api::find("gaycheck").unwrap().meta
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        // gay.js:4-18 persis. `example` tidak ada di pluginConfig JS dan
        // juga tidak ada di sini — jangan mengarang.
        let p = meta();
        assert_eq!(p.name, "gaycheck");
        assert_eq!(p.alias, ["howgay"]);
        assert_eq!(p.category, "fun");
        assert_eq!(p.description, "Menunjuk member paling gay di grup");
        assert_eq!(p.usage, ".gaycheck");
        assert_eq!(p.example, "");
        assert_eq!(p.cooldown, 10);
        assert_eq!(p.energi, 2);
        assert!(p.is_group && !p.is_private);
        assert!(p.is_enabled);
        assert!(!p.is_bot_admin && !p.is_admin);
    }

    #[test]
    fn the_alias_resolves_to_this_plugin() {
        assert_eq!(oktz_plugin_api::find("howgay").unwrap().meta.name, "gaycheck");
    }

    #[test]
    fn no_command_key_collides_with_another_plugin() {
        // `gaycheck` dan `howgay` harus eksklusif; `find()` mengembalikan
        // entri pertama yang cocok, jadi tabrakan diam-diam Override.
        for key in ["gaycheck", "howgay"] {
            let owners: Vec<&str> = all_plugins()
                .iter()
                .filter(|e| e.meta.matches(key))
                .map(|e| e.meta.name.as_str())
                .collect();
            assert_eq!(owners, ["gaycheck"], "key {key} diklaim oleh {owners:?}");
        }
    }

    #[test]
    fn the_reply_always_names_two_members_and_strips_the_server() {
        // Dua anggota acak dari dua orang bisa sama (JS juga begitu), jadi
        // yang diuji adalah bentuknya, bukan kombinasi spesifiknya.
        let ctx = ctx_of(&["628111@s.whatsapp.net", "628222@s.whatsapp.net"]);
        for _ in 0..50 {
            let (text, mentions) = build(&roster_jids(&ctx)).unwrap();
            assert!(text.starts_with('@'), "{text}");
            assert!(text.contains(" *Nge gay sama* "), "{text}");
            assert_eq!(mentions.len(), 2);
            assert!(!text.contains("@s.whatsapp.net"), "server harus dibuang");
            for m in &mentions {
                assert!(["628111@s.whatsapp.net", "628222@s.whatsapp.net"].contains(&m.as_str()));
                assert!(text.contains(&format!("@{}", jid_number(m))), "{text} vs {mentions:?}");
            }
        }
    }

    #[test]
    fn a_single_member_may_be_picked_twice_exactly_like_js() {
        // `orang1` dan `orang2` diindependenkan, jadi satu anggota tunggal
        // menghasilkan "X *Nge gay sama* X". Ini perilaku JS, bukan kebetulan.
        let only = vec!["628999@s.whatsapp.net".to_string()];
        let (text, mentions) = build(&only).unwrap();
        assert_eq!(text, "@628999 *Nge gay sama* @628999");
        assert_eq!(mentions, vec!["628999@s.whatsapp.net".to_string(); 2]);
    }

    #[test]
    fn an_empty_roster_reports_instead_of_panicking() {
        // JS tidak mengecek roster kosong; `participants[NaN]` = undefined dan
        // hasilnya `@undefined`. Di sini ditolak eksplisit supaya tidak pernah
        // keluar teks berisi "undefined".
        assert!(build(&[]).is_none());
        let reply = gaycheck(ctx_of(&[]));
        assert_eq!(reply.text_body(), Some("❌ Tidak ada member di grup!"));
    }

    #[test]
    fn the_registered_handler_always_answers_with_text() {
        for n in [0usize, 1, 5] {
            let jids: Vec<String> = (0..n).map(|i| format!("62800{i}@s.whatsapp.net")).collect();
            let refs: Vec<&str> = jids.iter().map(String::as_str).collect();
            let reply = gaycheck(ctx_of(&refs));
            assert!(reply.text_body().is_some(), "roster {n} harus dibalas");
        }
    }
}