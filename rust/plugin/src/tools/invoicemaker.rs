//! `plugins/tools/invoicemaker.js` — membuat invoice/nota penjualan.
//!
//! `pub mod invoicemaker;` perlu ditambah manual ke `src/tools/mod.rs`
//! (mod.rs berisi plugin lain, bukan milik saya — lihat PORT-BRIEF §6).
//!
//! `ponytail:` konteks pengiriman (newsletter `contextInfo`) dari JS tidak
//! ada di `Reply`, jadi dikirim sebagai gambar biasa dengan caption. Kunci
//! API `neoxr` dibaca dari `ctx.config().api_keys.neoxr`.

use oktz_plugin_api::{Ctx, MediaSource, Reply, plugin};
use serde_json::Value;

use crate::tools::phisingataubukan::encode_uri_component;

const API_BASE: &str = "https://api.neoxr.eu/api/invoice-maker";
const QR_IMAGE: &str = "https://i.ibb.co.com/kt5fyrg/qr.jpg";

const ERR_FORMAT: &str = "❌ Format tidak lengkap! Butuh 6 parameter (toko|invoice|tanggal|status|items|total)";
const ERR_STATUS: &str = "❌ Status harus 'paid' atau 'unpaid'!";
const ERR_ITEMS: &str = "❌ Format items salah! Gunakan: Nama:unit:harga (pisah koma untuk multiple)";
const ERR_NEGATIF: &str = "❌ Harga item tidak boleh negatif! Gunakan angka positif.";
const ERR_TOTAL_NEGATIF: &str = "❌ Total tidak boleh negatif! Gunakan angka positif.";

fn help(prefix: &str) -> String {
    format!(
        "🧾 *ɪɴᴠᴏɪᴄᴇ ᴍᴀᴋᴇʀ*\n\n\
        ╭┈┈⬡「 📋 *ꜰᴏʀᴍᴀᴛ* 」\n\
        ┃ `{prefix}invoicemaker <toko>|<invoice>|<tanggal>|<status>|<items>|<total>`\n\
        ╰┈┈⬡\n\n\
        ╭┈┈⬡「 📝 *ᴘᴀʀᴀᴍᴇᴛᴇʀ* 」\n\
        ┃ • toko: Nama toko\n\
        ┃ • invoice: Nomor invoice\n\
        ┃ • tanggal: Format DD/MM/YYYY\n\
        ┃ • status: paid/unpaid\n\
        ┃ • items: Nama:unit:harga (pisah koma)\n\
        ┃ • total: Total harga\n\
        ╰┈┈⬡\n\n\
        > Contoh:\n\
        `{prefix}invoicemaker TokoKu|INV001|15/01/2026|paid|Nasi Goreng:1x:15000,Es Teh:2x:6000|21000`"
    )
}

struct Item {
    nama: String,
    unit: String,
    harga: i64,
}

/// `parseInt` untuk angka item/total — `None` jika tidak diawali angka.
fn to_i64(s: &str) -> Option<i64> {
    crate::common::parse_int(s)
}

/// Format uang dengan pemisah ribuan titik — padanan
/// `(n).toLocaleString("id-ID")`.
fn rupiah(n: i64) -> String {
    let s = n.abs().to_string();
    let mut out = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            out.push('.');
        }
        out.push(c);
    }
    let mut r: String = out.chars().rev().collect();
    if n < 0 {
        r.insert(0, '-');
    }
    r
}

#[plugin(
    name = "invoicemaker",
    alias = ["invoice", "faktur", "nota"],
    category = "tools",
    description = "Membuat invoice/nota penjualan",
    usage = ".invoicemaker <toko>|<invoice>|<tanggal>|<status>|<items>|<total>",
    example = ".invoicemaker TokoKu|INV001|15/01/2026|paid|Nasi Goreng:1x:15000,Es Teh:2x:6000|21000",
    cooldown = 15,
    energi = 2,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn invoicemaker(ctx: Ctx) -> Reply {
    let text = ctx.args.join(" ");
    if text.is_empty() || !text.contains('|') {
        return Reply::text(help(&ctx.prefix));
    }

    let parts: Vec<&str> = text.split('|').map(|p| p.trim()).collect();
    if parts.len() < 6 {
        return Reply::text(ERR_FORMAT);
    }

    let store = parts[0];
    let invoice = parts[1];
    let date = parts[2];
    let status = parts[3];
    let items_raw = parts[4];
    let total_raw = parts[5];

    if !["paid", "unpaid"].contains(&status.to_lowercase().as_str()) {
        return Reply::text(ERR_STATUS);
    }

    let mut items: Vec<Item> = Vec::new();
    for item in items_raw.split(',') {
        let mut kv = item.split(':');
        let nama = kv.next().unwrap_or("").trim();
        let unit = kv.next().unwrap_or("1x").trim();
        let harga = to_i64(kv.next().unwrap_or("0").trim()).unwrap_or(0);
        if harga < 0 {
            return Reply::text(ERR_NEGATIF);
        }
        items.push(Item {
            nama: if nama.is_empty() { "Item".to_string() } else { nama.to_string() },
            unit: unit.to_string(),
            harga,
        });
    }

    if items.is_empty() {
        return Reply::text(ERR_ITEMS);
    }

    let sum: i64 = items.iter().map(|i| i.harga).sum();
    let total = to_i64(total_raw).unwrap_or(sum);
    if total < 0 {
        return Reply::text(ERR_TOTAL_NEGATIF);
    }

    let api_key = ctx.config().api_keys.neoxr.clone();
    let items_value: Vec<Value> = items
        .iter()
        .map(|i| serde_json::json!({ "name": i.nama, "unit": i.unit, "price": i.harga }))
        .collect();
    let items_str = serde_json::to_string(&items_value).unwrap_or_default();
    let url = format!(
        "{API_BASE}?store={}&invoice={}&date={}&status={}&image={}&items={}&total={}&apikey={}",
        encode_uri_component(store),
        encode_uri_component(invoice),
        encode_uri_component(date),
        status.to_lowercase(),
        encode_uri_component(QR_IMAGE),
        encode_uri_component(&items_str),
        total,
        encode_uri_component(&api_key)
    );

    let Ok(res) = ctx.http.get(&url) else {
        return Reply::text("❌ *ɢᴀɢᴀʟ*\n\n> API invoice tidak merespons");
    };
    let data = match res.json() {
        Some(v) => v,
        None => return Reply::text("❌ *ɢᴀɢᴀʟ*\n\n> Balasan API bukan JSON"),
    };

    if !data.get("status").is_some_and(crate::common::js_truthy_value)
        || data.get("data").and_then(|v| v.get("image")).and_then(|v| v.get("url")).is_none()
    {
        return Reply::text("❌ *ɢᴀɢᴀʟ*\n\n> API tidak mengembalikan data yang valid");
    }

    let image_url = data["data"]["image"]["url"].as_str().unwrap_or("");
    let data_obj = data.get("data").cloned().unwrap_or(Value::Null);

    let store_v = data_obj.get("store").and_then(|v| v.as_str()).unwrap_or(store);
    let invoice_v = data_obj.get("invoice").and_then(|v| v.as_str()).unwrap_or(invoice);
    let date_v = data_obj.get("date").and_then(|v| v.as_str()).unwrap_or(date);
    let status_v = data_obj.get("status").and_then(|v| v.as_str()).unwrap_or(status);

    let mut caption = format!(
        "🧾 *ɪɴᴠᴏɪᴄᴇ ɢᴇɴᴇʀᴀᴛᴇᴅ*\n\n\
        ╭┈┈⬡「 📋 *ᴅᴇᴛᴀɪʟ* 」\n\
        ┃ 🏪 Toko: *{store_v}*\n\
        ┃ 🔢 Invoice: *{invoice_v}*\n\
        ┃ 📅 Tanggal: *{date_v}*\n\
        ┃ 📌 Status: *{}*\n\
        ╰┈┈⬡\n\n\
        ╭┈┈⬡「 🛒 *ɪᴛᴇᴍs* 」\n",
        if status_v == "paid" { "✅ LUNAS" } else { "❌ BELUM LUNAS" }
    );
    let items_arr = data_obj
        .get("items")
        .and_then(|v| v.as_array())
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    for (i, item) in items_arr.iter().enumerate() {
        let nama = item.get("name").and_then(|v| v.as_str()).unwrap_or("");
        let unit = item.get("unit").and_then(|v| v.as_str()).unwrap_or("");
        let harga = item.get("price").and_then(|v| v.as_i64()).unwrap_or(0);
        caption.push_str(&format!(
            "┃ {}. {} ({}) - Rp{}\n",
            i + 1,
            nama,
            unit,
            rupiah(harga)
        ));
    }
    let total_v = data_obj.get("total").and_then(|v| v.as_i64()).unwrap_or(total);
    caption.push_str(&format!(
        "╰┈┈⬡\n\n> 💰 Total: *Rp{}*",
        rupiah(total_v)
    ));

    Reply::image(MediaSource::url(image_url.to_string()), caption)
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{HttpBody, HttpError, HttpGet, HttpResponse};
    use std::sync::{Arc, Mutex};

    #[derive(Debug, Default)]
    struct Stub {
        body: Option<String>,
        seen: Mutex<Vec<String>>,
    }

    impl Stub {
        // Mengembalikan `Stub`, bukan `Arc<Stub>`: pemanggil yang membungkus
        // `Arc::new(...)` supaya tidak jadi `Arc<Arc<Stub>>`.
        fn json(v: Value) -> Stub {
            Stub {
                body: Some(v.to_string()),
                seen: Mutex::new(Vec::new()),
            }
        }
    }

    impl HttpGet for Stub {
        fn get(&self, url: &str) -> Result<HttpResponse, HttpError> {
            self.seen.lock().unwrap().push(url.to_string());
            match &self.body {
                Some(b) => Ok(HttpResponse {
                    status: 200,
                    body: HttpBody::Bytes {
                        data: Arc::from(b.as_bytes().to_vec()),
                        mimetype: Some("application/json".into()),
                    },
                    headers: Vec::new(),
                }),
                None => Err(oktz_plugin_api::HttpError {
                    message: "offline".into(),
                }),
            }
        }
        fn post_json(&self, _: &str, _: &serde_json::Value) -> Result<HttpResponse, HttpError> {
            unreachable!()
        }
        fn post_form(&self, _: &str, _: &str) -> Result<HttpResponse, HttpError> {
            unreachable!()
        }
        fn post_multipart(
            &self,
            _: &str,
            _: &[(&str, HttpBody)],
        ) -> Result<HttpResponse, HttpError> {
            unreachable!()
        }
    }

    fn run(http: Arc<Stub>, args: &[&str]) -> Reply {
        let ctx = Ctx {
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            args: args.iter().map(|s| s.to_string()).collect(),
            prefix: ".".into(),
            command: "invoicemaker".into(),
            http,
            config: Arc::new(oktz_plugin_api::config::Runtime {
                api_keys: oktz_plugin_api::config::ApiKeys {
                    neoxr: "KUNCI_NEOXR".into(),
                    ..oktz_plugin_api::config::ApiKeys::default()
                },
                ..oktz_plugin_api::config::Runtime::default()
            }),
            ..Ctx::default()
        };
        invoicemaker(ctx)
    }

    fn body_of(r: &Reply) -> String {
        r.text_body().unwrap_or_default().to_string()
    }

    fn image_of(r: &Reply) -> Option<String> {
        match r.without_mutations() {
            Reply::Image { media, .. } => media.as_ref().and_then(|m| match m {
                MediaSource::Url { url, .. } => Some(url.clone()),
                _ => None,
            }),
            _ => None,
        }
    }

    #[test]
    fn meta_sesuai_js() {
        let p = oktz_plugin_api::find("invoicemaker").unwrap();
        assert_eq!(p.meta.name, "invoicemaker");
        assert_eq!(p.meta.alias, ["invoice", "faktur", "nota"]);
        assert_eq!(p.meta.category, "tools");
        assert_eq!(p.meta.cooldown, 15);
        assert_eq!(p.meta.energi, 2);
    }

    #[test]
    fn tanpa_pipe_menampilkan_bantuan() {
        let http = Arc::new(Stub::json(serde_json::json!({})));
        let r = run(http, &[]);
        assert_eq!(body_of(&r), help("."));
    }

    #[test]
    fn kurang_dari_6_parameter_ditolak() {
        let http = Arc::new(Stub::json(serde_json::json!({})));
        let r = run(http, &["TokoKu|INV001|15/01/2026|paid|items"]);
        assert_eq!(body_of(&r), ERR_FORMAT);
    }

    #[test]
    fn status_bukan_paid_atau_unpaid_ditolak() {
        let http = Arc::new(Stub::json(serde_json::json!({})));
        let r = run(http, &["T|INV|15/01/2026|lunas|items|100"]);
        assert_eq!(body_of(&r), ERR_STATUS);
    }

    #[test]
    fn item_nama_kosong_diganti_item() {
        // JS: `name || "Item"` — nama kosong jadi "Item", bukan ditolak.
        let http = Arc::new(Stub::json(serde_json::json!({
            "status": true,
            "data": {
                "store": "T", "invoice": "I", "date": "D", "status": "paid",
                "items": [{"name": "Item", "unit": "1x", "price": 100}],
                "total": 100,
                "image": {"url": "https://cdn/x.png"}
            }
        })));
        let r = run(http, &["T|I|D|paid|:1x:100|100"]);
        assert!(image_of(&r).is_some(), "nama kosong harus jadi 'Item', tidak ditolak");
    }

    #[test]
    fn harga_negatif_ditolak() {
        let http = Arc::new(Stub::json(serde_json::json!({})));
        let r = run(http, &["T|INV|15/01/2026|paid|Barang:1x:-500|100"]);
        assert_eq!(body_of(&r), ERR_NEGATIF);
    }

    #[test]
    fn total_negatif_ditolak() {
        let http = Arc::new(Stub::json(serde_json::json!({})));
        let r = run(http, &["T|INV|15/01/2026|paid|Barang:1x:100|-100"]);
        assert_eq!(body_of(&r), ERR_TOTAL_NEGATIF);
    }

    #[test]
    fn sukses_mengirim_gambar_dengan_caption() {
        let http = Arc::new(Stub::json(serde_json::json!({
            "status": true,
            "data": {
                "store": "TokoKu",
                "invoice": "INV001",
                "date": "15/01/2026",
                "status": "paid",
                "items": [
                    {"name": "Nasi Goreng", "unit": "1x", "price": 15000},
                    {"name": "Es Teh", "unit": "2x", "price": 6000}
                ],
                "total": 21000,
                "image": {"url": "https://cdn/invoice.png"}
            }
        })));
        let r = run(http.clone(), &["TokoKu|INV001|15/01/2026|paid|Nasi Goreng:1x:15000,Es Teh:2x:6000|21000"]);
        let Some(url) = image_of(&r) else { panic!("harus image"); };
        assert_eq!(url, "https://cdn/invoice.png");
        let cap = body_of(&r);
        assert!(cap.contains("🧾 *ɪɴᴠᴏɪᴄᴇ ɢᴇɴᴇʀᴀᴛᴇᴅ*"));
        assert!(cap.contains("Toko: *TokoKu*"));
        assert!(cap.contains("Invoice: *INV001*"));
        assert!(cap.contains("Nasi Goreng (1x) - Rp15.000"));
        assert!(cap.contains("Es Teh (2x) - Rp6.000"));
        assert!(cap.contains("Total: *Rp21.000*"));
        let seen = http_seen(&http);
        assert!(seen[0].contains("apikey=KUNCI_NEOXR"));
        assert!(seen[0].contains("store=TokoKu"));
    }

    #[test]
    fn total_dihitung_dari_items_saat_kosong() {
        let http = Arc::new(Stub::json(serde_json::json!({
            "status": true,
            "data": {
                "store": "T", "invoice": "I", "date": "D", "status": "paid",
                "items": [{"name": "B", "unit": "1x", "price": 100}],
                "total": 100,
                "image": {"url": "https://cdn/x.png"}
            }
        })));
        let r = run(http, &["T|I|D|paid|B:1x:100|"]);
        assert!(image_of(&r).is_some());
    }

    fn http_seen(http: &Arc<Stub>) -> Vec<String> {
        http.seen.lock().unwrap().clone()
    }
}
