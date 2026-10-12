//! `plugins/store/list.js` — `db.setting('storeLists')` (daftar info toko).

use oktz_plugin_api::registry::MediaSource;
use oktz_plugin_api::{Ctx, Reply, plugin};
use serde_json::Value;

const EMPTY: &str = "📋 *Belum Ada Informasi Toko*\n\nSaat ini belum ada informasi yang ditambahkan oleh admin 😔\n\nSilakan cek kembali nanti atau hubungi admin untuk informasi lebih lanjut.\n\n_Terima kasih atas ketertarikan Anda_ 🙏";
const FOOTER: &str = "💡 _Ketik `";
const FOOTER_TAIL: &str = "` untuk membaca detail informasi_";

/// `l.image ? "🖼️" : l.video ? "🎬" : "📝"` — `list.js:59`.
fn media_icon(item: &Value) -> &'static str {
    if truthy_field(item, "image") {
        "🖼️"
    } else if truthy_field(item, "video") {
        "🎬"
    } else {
        "📝"
    }
}

/// `l.image` / `l.video` dibaca sebagai JS: key/value yang bukan `null`/
/// `undefined`/`false`/`0`/`""` dianggap ada.
fn truthy_field(item: &Value, key: &str) -> bool {
    match item.get(key) {
        None | Some(Value::Null) => false,
        Some(Value::Bool(b)) => *b,
        Some(Value::Number(n)) => n.as_f64().is_some_and(|f| f != 0.0),
        Some(Value::String(s)) => !s.is_empty(),
        Some(_) => true,
    }
}

#[plugin(
    name = "list",
    alias = ["liststore", "daftar", "info"],
    category = "store",
    description = "📋 Lihat daftar informasi toko",
    usage = ".list atau .list <nomor>",
    example = ".list 1",
    cooldown = 5,
    energi = 0,
    is_group = false,
    is_private = false
)]
pub fn list(ctx: Ctx) -> Reply {
    let lists = ctx.setting_list("storeLists");

    if lists.is_empty() {
        return Reply::text(EMPTY);
    }

    // `parseInt(input) - 1` (`list.js:27`). `parseInt` membaca AWALAN
    // angka lalu membuang sisa non-digit, jadi "1abc" tetap 1 — beda dari
    // `parseInt` versi ketat yang akan menolak.
    if let Some(idx) = js_parse_int(ctx.text.trim())
        .and_then(|n| n.checked_sub(1))
        && (idx as usize) < lists.len()
    {
        let item = &lists[idx as usize];
        let txt = item
            .get("content")
            .and_then(Value::as_str)
            .unwrap_or("undefined")
            .to_string();
        // `list.js:37-49`: image → `Reply::Image` dengan URL; video TIDAK
        // punya padanan Reply (tidak ada variant video) dan `ctx` tidak
        // membawa media, jadi video jatuh ke teks — sama seperti `m.reply`
        // yang jadi tempat teks dikirim.
        return match (item.get("image"), item.get("video")) {
            (Some(Value::String(url)), _) if !url.is_empty() => {
                Reply::image(MediaSource::url(url), txt)
            }
            _ => Reply::text(txt),
        };
    }

    let mut txt = String::from("📋 *DAFTAR INFORMASI TOKO*\n\n");
    txt.push_str("Berikut informasi yang tersedia saat ini 📝\n");
    txt.push_str(&format!(
        "Ketik `{}list <nomor>` untuk melihat detail.\n\n",
        ctx.prefix
    ));
    for (i, item) in lists.iter().enumerate() {
        txt.push_str(&format!(
            "*{}.* {} *{}*\n",
            i + 1,
            media_icon(item),
            item.get("name").and_then(Value::as_str).unwrap_or("undefined")
        ));
    }
    txt.push('\n');
    txt.push_str(&format!(
        "{FOOTER}{}list <nomor>{FOOTER_TAIL}",
        ctx.prefix
    ));
    Reply::text(txt)
}

/// `parseInt(s)` (`list.js:27`) untuk basis-10 tanpa tanda ekspon: baca
/// digit awal, buang sisa. `None` kalau tidak ada digit sama sekali —
/// persis `isNaN(parseInt(...))`.
///
/// Penting: yang diparse HANYA digit prefiksnya. `parseInt("2.9")` = 2,
/// bukan 2.9 — kalau seluruh string diparse, `"1abc"` jadi `NaN` dan
/// branch "pilih nomor" tidak pernah tercapai untuk input berantakan.
pub(crate) fn js_parse_int(s: &str) -> Option<i64> {
    let t = s.trim_start();
    let (sign, rest) = match t.strip_prefix('-') {
        Some(r) => (-1i64, r),
        None => (1i64, t.strip_prefix('+').unwrap_or(t)),
    };
    let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
    if digits.is_empty() {
        return None;
    }
    // Digit bisa jauh melebihi i64 hanya untuk input sintetis; saturate
    // seperti yang dilakukan mesin JS, jangan panic.
    Some(sign * digits.parse::<i64>().unwrap_or(i64::MAX))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn run(settings: serde_json::Value, text: &str) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "list".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: text.into(),
            settings,
            ..Default::default()
        };
        (oktz_plugin_api::find("list").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("list").unwrap();
        assert_eq!(p.meta.name, "list");
        assert_eq!(p.meta.alias, ["liststore", "daftar", "info"]);
        assert_eq!(p.meta.category, "store");
        assert_eq!(p.meta.description, "📋 Lihat daftar informasi toko");
        assert_eq!(p.meta.usage, ".list atau .list <nomor>");
        assert_eq!(p.meta.example, ".list 1");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_premium);
    }

    #[test]
    fn aliases_resolve() {
        for a in ["liststore", "daftar", "info"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "list");
        }
    }

    #[test]
    fn an_absent_setting_shows_the_empty_branch() {
        // `db.setting('storeLists') || []` — key tidak ada = array kosong.
        assert_eq!(text_of(&run(json!({}), "")), EMPTY);
    }

    #[test]
    fn a_present_but_empty_list_shows_the_same_empty_branch() {
        assert_eq!(text_of(&run(json!({"storeLists": []}), "")), EMPTY);
    }

    #[test]
    fn the_listing_numbers_every_item_with_its_media_icon() {
        let r = run(
            json!({"storeLists": [
                {"name": "S&K", "content": "isi"},
                {"name": "Order", "content": "isi", "image": "https://x/a.png"},
                {"name": "Promo", "content": "isi", "video": "https://x/b.mp4"}
            ]}),
            "",
        );
        assert_eq!(
            text_of(&r),
            concat!(
                "📋 *DAFTAR INFORMASI TOKO*\n\n",
                "Berikut informasi yang tersedia saat ini 📝\n",
                "Ketik `.list <nomor>` untuk melihat detail.\n\n",
                "*1.* 📝 *S&K*\n",
                "*2.* 🖼️ *Order*\n",
                "*3.* 🎬 *Promo*\n",
                "\n",
                "💡 _Ketik `.list <nomor>` untuk membaca detail informasi_"
            )
        );
    }

    #[test]
    fn picking_a_number_sends_that_item_content_as_plain_text() {
        let r = run(
            json!({"storeLists": [{"name": "A", "content": "baris 1\nbaris 2"}, {"name": "B", "content": "b"}]}),
            "1",
        );
        assert_eq!(text_of(&r), "baris 1\nbaris 2");
        assert!(!r.has_media());
    }

    #[test]
    fn picking_an_item_with_an_image_replies_with_an_image_reply() {
        let r = run(
            json!({"storeLists": [{"name": "A", "content": "caption", "image": "https://x/a.png"}]}),
            "1",
        );
        assert!(r.has_media(), "harus jadi Reply::Image");
        assert_eq!(
            r.media_source().map(|m| m.label()).as_deref(),
            Some("url"),
            "sumber harus URL, bukan Path — `list.js:39` meneruskan l.image apa adanya"
        );
        assert_eq!(text_of(&r), "caption");
    }

    // Cabang `idx` di luar jangkauan jatuh ke daftar, bukan error.
    #[test]
    fn out_of_range_and_non_numeric_fall_back_to_the_listing() {
        let settings = json!({"storeLists": [{"name": "A", "content": "x"}]});
        for bad in ["0", "2", "abc", "", "-1"] {
            let r = run(settings.clone(), bad);
            assert!(
                text_of(&r).starts_with("📋 *DAFTAR INFORMASI TOKO*"),
                "input {bad:?} harus jatuh ke daftar"
            );
        }
    }

    // `parseInt` membaca awalan digit lalu membuang sisa — "2abc" = 2.
    #[test]
    fn parse_int_matches_javascript_including_trailing_garbage() {
        assert_eq!(js_parse_int("1"), Some(1));
        assert_eq!(js_parse_int(" 2 "), Some(2));
        assert_eq!(js_parse_int("2abc"), Some(2));
        assert_eq!(js_parse_int("2.9"), Some(2));
        assert_eq!(js_parse_int("-3"), Some(-3));
        assert_eq!(js_parse_int("abc"), None);
        assert_eq!(js_parse_int(""), None);
        assert_eq!(js_parse_int("   "), None);
        assert_eq!(js_parse_int("1e3"), Some(1));
    }
}