//! `plugins/primbon/tafsirmimpi.js` — cari tafsir mimpi lewat API primbon.

use oktz_plugin_api::{Ctx, Reply, plugin};
use serde_json::Value;

use crate::primbon::artinama::{display, error_reply, truthy};
use crate::tools::phisingataubukan::encode_uri_component;

const ENDPOINT: &str = "https://api.siputzx.my.id/api/primbon/tafsirmimpi";

/// `r.total` dipakai sebagai angka (`r.total - 10`), jadi nilai non-angka
/// di-JS jadi `NaN` dan seluruh perbandingan `>` berikutnya salah.
fn total_of(r: &Value) -> Option<i64> {
    r.get("total").and_then(Value::as_i64)
}

fn render(r: &Value) -> String {
    let mut out = format!(
        "🌙 *ᴛᴀꜰsɪʀ ᴍɪᴍᴘɪ*\n\n> Kata kunci: *{}*\n> Ditemukan: *{} hasil*\n\n",
        display(r.get("keyword")),
        display(r.get("total")),
    );

    for (i, h) in r["hasil"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or(&[])
        .iter()
        .take(10)
        .enumerate()
    {
        out.push_str(&format!(
            "*{}. {}*\n> {}\n\n",
            i + 1,
            display(h.get("mimpi")),
            display(h.get("tafsir")),
        ));
    }

    if let Some(sisa) = total_of(r).filter(|t| *t > 10) {
        out.push_str(&format!("_...dan {} hasil lainnya_", sisa - 10));
    }
    out
}

#[plugin(
    name = "tafsirmimpi",
    alias = ["artimimpi", "mimpi"],
    category = "primbon",
    description = "Cari tafsir mimpi",
    usage = ".tafsirmimpi <kata kunci>",
    example = ".tafsirmimpi bertemu",
    cooldown = 5,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false
)]
pub fn tafsirmimpi(ctx: Ctx) -> Reply {
    let keyword = ctx.args.join(" ");
    if keyword.is_empty() {
        return Reply::text(format!(
            "🌙 *ᴛᴀꜰsɪʀ ᴍɪᴍᴘɪ*\n\n> Masukkan kata kunci mimpi\n\n\
             `Contoh: {}tafsirmimpi bertemu`",
            ctx.prefix
        ));
    }

    let url = format!("{ENDPOINT}?mimpi={}", encode_uri_component(&keyword));
    let gagal = format!("❌ *ɢᴀɢᴀʟ*\n\n> Tidak ditemukan tafsir untuk: {keyword}");

    let Ok(resp) = ctx.http.get(&url) else {
        return Reply::text(error_reply(&ctx.prefix, &ctx.command)).with_reaction("☢");
    };
    let Some(body) = resp.json() else {
        return Reply::text(gagal).with_reaction("❌");
    };

    // `!data?.status || !data?.data?.hasil?.length`
    let Some(data) = body
        .get("data")
        .filter(|v| truthy(v))
        .filter(|d| d.get("hasil").and_then(Value::as_array).is_some_and(|a| !a.is_empty()))
    else {
        return Reply::text(gagal).with_reaction("❌");
    };
    if !body.get("status").is_some_and(truthy) {
        return Reply::text(gagal).with_reaction("❌");
    }

    Reply::text(render(data)).with_reaction("✅")
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{HttpBody, HttpError, HttpGet, HttpResponse, NoHttp};
    use serde_json::json;
    use std::sync::Arc;

    #[derive(Debug)]
    struct Stub(Value);

    impl HttpGet for Stub {
        fn get(&self, _: &str) -> Result<HttpResponse, HttpError> {
            Ok(HttpResponse {
                status: 200,
                body: HttpBody::Bytes {
                    data: Arc::from(serde_json::to_vec(&self.0).unwrap()),
                    mimetype: None,
                },
                headers: vec![],
            })
        }
        fn post_json(&self, _: &str, _: &Value) -> Result<HttpResponse, HttpError> {
            unreachable!("tidak dipakai")
        }
        fn post_form(&self, _: &str, _: &str) -> Result<HttpResponse, HttpError> {
            unreachable!("tidak dipakai")
        }
        fn post_multipart(
            &self,
            _: &str,
            _: &[(&str, HttpBody)],
        ) -> Result<HttpResponse, HttpError> {
            unreachable!("tidak dipakai")
        }
    }

    fn run_with(http: Arc<dyn HttpGet>, args: &[&str]) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "tafsirmimpi".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: args.join(" "),
            args: args.iter().map(|a| a.to_string()).collect(),
            http,
            ..Default::default()
        };
        (oktz_plugin_api::find("tafsirmimpi").unwrap().handler)(ctx)
    }

    fn reply_text(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    fn hasil(n: usize) -> Value {
        Value::Array(
            (1..=n)
                .map(|i| json!({ "mimpi": format!("mimpi {i}"), "tafsir": format!("tafsir {i}") }))
                .collect(),
        )
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("tafsirmimpi").unwrap();
        assert_eq!(p.meta.name, "tafsirmimpi");
        assert_eq!(p.meta.alias, ["artimimpi", "mimpi"]);
        assert_eq!(p.meta.category, "primbon");
        assert_eq!(p.meta.description, "Cari tafsir mimpi");
        assert_eq!(p.meta.usage, ".tafsirmimpi <kata kunci>");
        assert_eq!(p.meta.example, ".tafsirmimpi bertemu");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
        assert!(!p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn an_empty_keyword_prints_the_help_and_skips_http() {
        let r = run_with(Arc::new(NoHttp), &[]);
        let t = reply_text(&r);
        assert!(t.starts_with("🌙 *ᴛᴀꜰsɪʀ ᴍɪᴍᴘɪ*"), "{t}");
        assert!(t.contains("Masukkan kata kunci mimpi"), "{t}");
        assert!(t.contains(".tafsirmimpi bertemu"), "{t}");
    }

    /// Kata kunci ber-spasi digabung dengan spasi, lalu di-encode `%20`.
    #[test]
    fn a_multi_word_keyword_is_joined_then_url_encoded() {
        assert_eq!(encode_uri_component("nama panjang"), "nama%20panjang");
        assert_eq!(encode_uri_component("a+b"), "a%2Bb");
    }

    #[test]
    fn a_small_result_list_is_numbered_from_one() {
        let r = run_with(
            Arc::new(Stub(json!({
                "status": true,
                "data": { "keyword": "bertemu", "total": 3, "hasil": hasil(3) }
            }))),
            &["bertemu"],
        );
        let t = reply_text(&r);
        assert!(t.starts_with("🌙 *ᴛᴀꜰsɪʀ ᴍɪᴍᴘɪ*"), "{t}");
        assert!(t.contains("> Kata kunci: *bertemu*"), "{t}");
        assert!(t.contains("> Ditemukan: *3 hasil*"), "{t}");
        assert!(t.contains("*1. mimpi 1*\n> tafsir 1\n\n"), "{t}");
        assert!(t.contains("*3. mimpi 3*\n> tafsir 3\n\n"), "{t}");
        assert!(!t.contains("hasil lainnya"), "total <= 10 tidak ada ekor: {t}");
    }

    /// `r.hasil.slice(0, 10)` — item kesepuluh ikut, sisanya tidak.
    #[test]
    fn more_than_ten_results_are_capped_and_counted() {
        let r = run_with(
            Arc::new(Stub(json!({
                "status": true,
                "data": { "keyword": "bangun", "total": 25, "hasil": hasil(25) }
            }))),
            &["bangun"],
        );
        let t = reply_text(&r);
        assert!(t.contains("10. mimpi 10"), "{t}");
        assert!(!t.contains("mimpi 11"), "{t}");
        assert!(t.ends_with("_...dan 15 hasil lainnya_"), "{t}");
    }

    /// `hasil` kosong / bukan array → `?.length` falsy → cabang gagal.
    #[test]
    fn an_empty_or_missing_result_array_gives_the_failure_message() {
        for body in [
            json!({ "status": true, "data": { "hasil": [] } }),
            json!({ "status": true, "data": {} }),
            json!({ "status": true, "data": { "hasil": null } }),
            json!({ "status": false, "data": { "hasil": [1] } }),
            json!({}),
        ] {
            let r = run_with(Arc::new(Stub(body.clone())), &["bertemu"]);
            assert_eq!(
                reply_text(&r),
                "❌ *ɢᴀɢᴀʟ*\n\n> Tidak ditemukan tafsir untuk: bertemu",
                "{body}"
            );
        }
    }

    #[test]
    fn a_transport_error_uses_the_error_template() {
        let r = run_with(Arc::new(NoHttp), &["bertemu"]);
        let t = reply_text(&r);
        assert!(t.starts_with("☢ Kayaknya command "), "{t}");
        assert!(t.contains(".tafsirmimpi"), "{t}");
    }
}
