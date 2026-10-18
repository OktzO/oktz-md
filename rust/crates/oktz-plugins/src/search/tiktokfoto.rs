//! `plugins/search/tiktokfoto.js` — cari foto TikTok dan kirim album gambar.
//!
//! `NOTE (mod.rs):` `search/mod.rs` sudah memuat banyak plugin (bukan milik
//! tugas ini), jadi berkas ini sengaja tidak menambahkan `pub mod`.
//! Tambahkan baris berikut secara manual ke `src/search/mod.rs`:
//!   pub mod tiktokfoto;
//
//! Pemanggilan API lewat `ctx.http.get`. Hasil gambar dikirim sebagai
//! `Reply::Sequence` agar terkirim berurutan, mirip `albumMessage` di JS.
use oktz_plugin_api::{Ctx, MediaSource, Reply, plugin};
use serde_json::Value;

fn format_number(n: &str) -> String {
    let value: f64 = n.parse().unwrap_or(0.0);
    if value >= 1_000_000.0 {
        format!("{:.1}M", value / 1_000_000.0)
    } else if value >= 1_000.0 {
        format!("{:.1}K", value / 1_000.0)
    } else {
        format!("{value:.0}")
    }
}

fn trim_text(text: &str, max: usize) -> String {
    let value = text.split_whitespace().collect::<Vec<_>>().join(" ").trim().to_string();
    if value.is_empty() {
        return "-".to_string();
    }
    if value.len() <= max {
        return value;
    }
    format!("{}...", &value[..max])
}

#[plugin(
    name = "tiktokfoto",
    alias = ["ttfoto", "ttphotosearch", "searchtiktokfoto"],
    category = "search",
    description = "Cari foto TikTok dan kirim album gambar",
    usage = ".tiktokfoto <query>",
    example = ".tiktokfoto cosplay",
    cooldown = 10,
    energi = 1,
    is_group = false,
    is_private = false
)]
pub fn tiktokfoto(ctx: Ctx) -> Reply {
    let query = ctx.text.trim();

    if query.is_empty() {
        return Reply::text(format!(
            "📸 *TIKTOK FOTO SEARCH*\n\n> Contoh:\n`{prefix}tiktokfoto cosplay`",
            prefix = ctx.prefix
        ));
    }

    let api_key = ctx.config().api_keys.cuki.clone();
    if api_key.is_empty() {
        return Reply::text(
            "❌ *API Key belum diset!*\n\n> Isi `APIKEY_CUKI` di file `.env`, lalu restart bot",
        );
    }

    let url = format!(
        "https://api.cuki.biz.id/api/search/tiktokfoto?apikey={api_key}&query={}",
        url_encode(&query)
    );

    let Ok(res) = ctx.http.get(&url) else {
        return Reply::text("❌ Gagal terhubung ke API Cuki");
    };
    let body = res.json().unwrap_or(Value::Null);

    let success = body.get("data")
        .and_then(|d| d.get("success").and_then(Value::as_bool))
        .unwrap_or(false);
    if !success {
        let message = body.get("data")
            .and_then(|d| d.get("message").and_then(Value::as_str))
            .unwrap_or("Foto TikTok tidak ditemukan");
        return Reply::text(format!("❌ {message}"));
    }

    let data = body.get("data").unwrap();
    let results = data.get("results").and_then(Value::as_array).unwrap_or(&vec![]);
    let post = results.first().cloned().unwrap_or(Value::Null);

    let images = post.get("images")
        .and_then(Value::as_array)
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect::<Vec<_>>())
        .unwrap_or_default();

    if images.is_empty() {
        return Reply::text(format!("❌ Tidak ditemukan foto TikTok untuk: {query}"));
    }

    let title = trim_text(
        post.get("title").or_else(|| post.get("description"))
            .and_then(Value::as_str).unwrap_or(""),
        180,
    );
    let author = post.get("author")
        .and_then(|a| a.get("nickname").and_then(Value::as_str))
        .unwrap_or("-");
    let region = post.get("region").and_then(Value::as_str).unwrap_or("-");
    let image_count = post.get("image_count").and_then(Value::as_i64).unwrap_or(images.len() as i64);
    let like = format_number(
        post.get("stats").and_then(|s| s.get("like").and_then(Value::as_str)).unwrap_or("0"),
    );
    let comment = format_number(
        post.get("stats").and_then(|s| s.get("comment").and_then(Value::as_str)).unwrap_or("0"),
    );
    let share = format_number(
        post.get("stats").and_then(|s| s.get("share").and_then(Value::as_str)).unwrap_or("0"),
    );
    let post_id = post.get("id").and_then(Value::as_str).unwrap_or("-");
    let description = trim_text(
        post.get("description").or_else(|| post.get("title"))
            .and_then(Value::as_str).unwrap_or(""),
        220,
    );
    let result_query = data.get("query").and_then(Value::as_str).unwrap_or(&query);

    let caption = format!(
        "📸 *TIKTOK FOTO SEARCH*\n\n\
         🔎 *Query:* {result_query}\n\
         📌 *Judul:* {title}\n\
         👤 *Author:* {author}\n\
         🌍 *Region:* {region}\n\
         🖼️ *Foto:* {image_count}\n\
         ❤️ *Like:* {like}\n\
         💬 *Comment:* {comment}\n\
         🔁 *Share:* {share}\n\
         🆔 *ID:* {post_id}\n\n\
         📝 {description}",
    );

    // Ambil maksimal 10 gambar pertama.
    let image_urls: Vec<String> = images.into_iter().take(10).collect();

    let mut replies: Vec<Reply> = vec![Reply::text(caption)];
    for url in &image_urls {
        replies.push(Reply::image(MediaSource::url(url.clone()), String::new()));
    }

    Reply::sequence(replies)
}

fn url_encode(s: &str) -> String {
    s.chars()
        .flat_map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '~') {
                vec![c as u8]
            } else {
                format!("{c}").into_bytes().iter()
                    .flat_map(|b| std::iter::once(b'%').chain(std::iter::once(format!("{b:02X}").chars().next().unwrap()).chain(std::iter::once(format!("{b:02X}").chars().nth(1).unwrap()))))
                    .collect::<Vec<_>>()
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_number_mirip_js() {
        assert_eq!(format_number("0"), "0");
        assert_eq!(format_number("1500"), "1.5K");
        assert_eq!(format_number("1500000"), "1.5M");
        assert_eq!(format_number("999"), "999");
    }

    #[test]
    fn trim_text_mirip_js() {
        assert_eq!(trim_text("  halo  dunia  ", 100), "halo dunia");
        assert_eq!(trim_text("", 100), "-");
        let panjang = "a".repeat(200);
        assert_eq!(trim_text(&panjang, 10).len(), 13); // 10 chars + "..."
    }

    #[test]
    fn url_encode_mirip_encode_uri_component() {
        assert_eq!(url_encode("cosplay"), "cosplay");
        assert_eq!(url_encode("hello world"), "hello%20world");
    }
}
