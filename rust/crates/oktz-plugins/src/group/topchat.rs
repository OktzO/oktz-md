//! `plugins/group/topchat.js` — `groups[chat].chatStats` (jid → {count,...}).

use oktz_plugin_api::{Ctx, Reply, plugin};
use serde_json::Value;

use crate::common::jid_number;
use crate::tools::kalkulatormbg::thousands_id;

const NO_DATA: &str = "📊 *ᴄʜᴀᴛ sᴛᴀᴛɪsᴛɪᴄs*\n\n> Belum ada data chat di grup ini.\n> Data akan tercatat otomatis setelah member aktif chat.";

/// `(jid, count)` untuk satu entri `chatStats`. `data.count || 0` — `0`,
/// string, dan objek semuanya jadi 0.
pub(crate) fn count_of(data: &Value) -> i64 {
    data.get("count").and_then(Value::as_i64).unwrap_or(0)
}

/// `Object.entries(chatStats).map(...).sort((a,b) => b.count - a.count)`.
///
/// `sort` JS stabil, jadi urutannya `sort_by` + `Reverse` yang sama: seri
/// tetap mengikuti urutan dokumen `chatStats`.
pub(crate) fn sorted_stats(stats: Option<&Value>) -> Vec<(String, i64)> {
    let mut out: Vec<(String, i64)> = stats
        .and_then(Value::as_object)
        .map(|m| {
            m.iter()
                .map(|(jid, data)| (jid.clone(), count_of(data)))
                .collect()
        })
        .unwrap_or_default();
    out.sort_by_key(|(_, n)| std::cmp::Reverse(*n));
    out
}

/// `i === 0 ? '🥇' : i === 1 ? '🥈' : i === 2 ? '🥉' : '▸'`.
pub(crate) const MEDALS: [&str; 4] = ["🥇", "🥈", "🥉", "▸"];

pub(crate) fn render(ctx: &Ctx) -> String {
    let sorted = sorted_stats(ctx.group_field("chatStats"));
    if sorted.is_empty() {
        return NO_DATA.to_string();
    }

    let mut txt = String::from(
        "📊 *TOTAL CHAT*\nBerikut ini adalah jumlah pesan yang dikirim oleh member di grup ini:\n\n",
    );
    for (i, (jid, count)) in sorted.iter().enumerate() {
        txt.push_str(&format!(
            "{} @{} — *{}* pesan\n",
            MEDALS[i.min(3)],
            jid_number(jid),
            thousands_id(*count as f64)
        ));
    }
    let total: i64 = sorted.iter().map(|(_, c)| *c).sum();
    txt.push_str(&format!("\n*Total Pesan: {}*", thousands_id(total as f64)));
    txt
}

#[plugin(
    name = "topchat",
    alias = ["chatstat", "chatstats", "totalchat"],
    category = "group",
    description = "Lihat statistik chat member di grup",
    usage = ".topchat",
    example = ".topchat",
    cooldown = 10,
    energi = 0,
    is_group = true,
    is_private = false
)]
pub fn topchat(ctx: Ctx) -> Reply {
    let sorted = sorted_stats(ctx.group_field("chatStats"));
    if sorted.is_empty() {
        return Reply::text(NO_DATA);
    }
    Reply::Text {
        text: render(&ctx),
        mentions: sorted.into_iter().map(|(jid, _)| jid).collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn run(group: serde_json::Value) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "topchat".into(),
            chat: "120363@g.us".into(),
            is_group: true,
            group_data: group,
            ..Default::default()
        };
        (oktz_plugin_api::find("topchat").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    fn mentions_of(r: &Reply) -> &Vec<String> {
        match r {
            Reply::Text { mentions, .. } => mentions,
            other => panic!("harus Text, dapat {other:?}"),
        }
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("topchat").unwrap();
        assert_eq!(p.meta.name, "topchat");
        assert_eq!(p.meta.alias, ["chatstat", "chatstats", "totalchat"]);
        assert_eq!(p.meta.category, "group");
        assert_eq!(p.meta.description, "Lihat statistik chat member di grup");
        assert_eq!(p.meta.usage, ".topchat");
        assert_eq!(p.meta.example, ".topchat");
        assert_eq!(p.meta.cooldown, 10);
        assert!(p.meta.is_group && !p.meta.is_admin && !p.meta.is_private);
    }

    #[test]
    fn aliases_resolve() {
        for a in ["chatstat", "chatstats", "totalchat"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "topchat");
        }
    }

    #[test]
    fn absent_or_empty_group_data_takes_the_no_data_branch() {
        for g in [json!(null), json!({}), json!({"chatStats": {}}), json!({"chatStats": []})] {
            let r = run(g.clone());
            assert_eq!(text_of(&r), NO_DATA, "input {g}");
            assert!(mentions_of(&r).is_empty());
        }
    }

    #[test]
    fn rows_are_sorted_by_count_descending_with_medals() {
        let r = run(json!({"chatStats": {
            "628111@s.whatsapp.net": {"count": 5},
            "628222@s.whatsapp.net": {"count": 90},
            "628333@s.whatsapp.net": {"count": 50},
            "628444@s.whatsapp.net": {"count": 10}
        }}));
        assert_eq!(
            text_of(&r),
            concat!(
                "📊 *TOTAL CHAT*\nBerikut ini adalah jumlah pesan yang dikirim oleh member di grup ini:\n\n",
                "🥇 @628222 — *90* pesan\n",
                "🥈 @628333 — *50* pesan\n",
                "🥉 @628444 — *10* pesan\n",
                "▸ @628111 — *5* pesan\n",
                "\n*Total Pesan: 155*"
            )
        );
    }

    #[test]
    fn mentions_follow_the_sorted_order_not_the_document_order() {
        let r = run(json!({"chatStats": {
            "628111@s.whatsapp.net": {"count": 1},
            "628222@s.whatsapp.net": {"count": 9}
        }}));
        assert_eq!(
            mentions_of(&r),
            &["628222@s.whatsapp.net", "628111@s.whatsapp.net"]
        );
    }

    #[test]
    fn ties_keep_document_order_because_js_sort_is_stable() {
        let sorted = sorted_stats(Some(&json!({
            "a@s.whatsapp.net": {"count": 3},
            "b@s.whatsapp.net": {"count": 3},
            "c@s.whatsapp.net": {"count": 9}
        })));
        assert_eq!(
            sorted,
            vec![
                ("c@s.whatsapp.net".to_string(), 9),
                ("a@s.whatsapp.net".to_string(), 3),
                ("b@s.whatsapp.net".to_string(), 3)
            ]
        );
    }

    #[test]
    fn a_missing_count_reads_as_zero_and_still_gets_a_row() {
        let r = run(json!({"chatStats": {"628111@s.whatsapp.net": {"lastChat": 5}}}));
        // Satu-satunya baris tetap dapat_medal emas — `i === 0`, bukan `▸`.
        assert!(text_of(&r).contains("🥇 @628111 — *0* pesan"));
        assert!(text_of(&r).ends_with("*Total Pesan: 0*"));
    }

    #[test]
    fn thousands_are_dotted_like_id_id() {
        let r = run(json!({"chatStats": {"628111@s.whatsapp.net": {"count": 1234567}}}));
        assert!(text_of(&r).contains("— *1.234.567* pesan"));
        assert!(text_of(&r).ends_with("*Total Pesan: 1.234.567*"));
    }

    #[test]
    fn the_device_suffix_stays_in_the_displayed_number() {
        let r = run(json!({"chatStats": {"628111:9@s.whatsapp.net": {"count": 1}}}));
        assert!(text_of(&r).contains("@628111:9 —"), "`jid.split('@')[0]` di JS");
        assert_eq!(mentions_of(&r), &["628111:9@s.whatsapp.net"]);
    }
}