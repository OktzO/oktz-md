//! `plugins/religi/islami.js` — satu handler untuk 5 fitur: Asmaul Husna,
//! Niat Sholat, Surah, Doa, dan Artikel.
//!
//! Percabangan JS ada di `m.command`, bukan di plugin terpisah — jadi alias
//! `niatsholat`/`surah`/`doa` dan seterusnya semuanya masuk ke sini.
//!
//! Yang tidak ikut ke port: pesan sementara `🕕 Sedang memuat surah...`,
//! karena `Reply` hanya bisa satu pesan.
//!
//! Catatan: `plugins/religi/asmaulhusna.js` juga mendaftarkan command
//! `asmaulhusna` sebagai Namanya. Konflik itu sudah ada di JS dan tidak
//! disentuh di sini — `find("asmaulhusna")` mengembalikan siapa pun yang
//! terdaftar lebih dulu.

use oktz_plugin_api::{Ctx, Reply, plugin};
use serde_json::Value;

use crate::primbon::artinama::{display, truthy};

const ASMA_ENDPOINT: &str = "https://islamic-api-zhirrr.vercel.app/api/asmaulhusna";
const NIAT_ENDPOINT: &str = "https://islamic-api-zhirrr.vercel.app/api/niatshalat";
const SURAH_ENDPOINT: &str = "https://api.siputzx.my.id/api/s/surah";
const DOA_ENDPOINT: &str = "https://doa-doa-api-ahmadramadhan.fly.dev/api";
const ARTIKEL_ENDPOINT: &str = "https://artikel-islam.netlify.app/.netlify/functions/api/ms";

const SYSTEM_ERROR: &str = "❌ Terjadi kesalahan pada sistem.";

/// `arr.map(item => \`- ${item[key]}\`).join('\n')`.
fn bullets(list: &[Value], key: &str) -> String {
    list.iter()
        .map(|item| format!("- {}", display(item.get(key))))
        .collect::<Vec<_>>()
        .join("\n")
}

/// `jir.data.map(...).join('\n')`.
fn asmaul_husna(hasil: &[Value]) -> String {
    let blok: Vec<String> = hasil
        .iter()
        .map(|item| {
            format!(
                "Urutan: {}\nLatin: {}\nArab: {}\nTerjemahan ID: {}\nTerjemahan EN: {}\n",
                display(item.get("index")),
                display(item.get("latin")),
                display(item.get("arabic")),
                display(item.get("translation_id")),
                display(item.get("translation_en")),
            )
        })
        .collect();
    format!("☪️ *ASMAUL HUSNA*\n\n{}", blok.join("\n"))
}

/// `!text` → daftar; selain itu cari via `name.toLowerCase().includes(...)`.
fn niat_sholat(list: &[Value], text: &str, prefix: &str) -> String {
    if text.is_empty() {
        return format!(
            "📋 *DAFTAR NIAT SHOLAT*\n\n{}\n\n\
             📌 Ketik `{prefix}niatsholat [nama sholat]` untuk melihat niat\n\
             Contoh: `{prefix}niatsholat subuh`",
            bullets(list, "name")
        );
    }
    let needle = text.to_lowercase();
    let Some(hit) = list
        .iter()
        .find(|i| display(i.get("name")).to_lowercase().contains(&needle))
    else {
        return "❌ Niat sholat yang kamu cari tidak ditemukan. Cek lagi nama sholatnya!"
            .to_string();
    };
    format!(
        "🕋 *{}*\n\n📄 Arab: {}\n🔤 Latin: {}\n🌍 Terjemahan: {}",
        display(hit.get("name")).to_uppercase(),
        display(hit.get("arabic")),
        display(hit.get("latin")),
        display(hit.get("terjemahan")),
    )
}

/// `doa` → daftar; selain itu cari via `doa.toLowerCase().includes(...)`.
fn doa(list: &[Value], text: &str, prefix: &str) -> String {
    if text.is_empty() {
        return format!(
            "🤲 *DAFTAR DOA*\n\n{}\n\n\
             📌 Ketik `{prefix}doa [nama doa]` untuk melihat doa\n\
             Contoh: `{prefix}doa doa sebelum tidur`",
            bullets(list, "doa")
        );
    }
    let needle = text.to_lowercase();
    let Some(hit) = list
        .iter()
        .find(|i| display(i.get("doa")).to_lowercase().contains(&needle))
    else {
        return "❌ Doa yang kamu cari tidak ditemukan. Cek lagi nama doanya!".to_string();
    };
    format!(
        "🤲 *{}*\n\n📄 Ayat: {}\n🔤 Latin: {}\n🌍 Artinya: {}",
        display(hit.get("doa")).to_uppercase(),
        display(hit.get("ayat")),
        display(hit.get("latin")),
        display(hit.get("artinya")),
    )
}

/// `String.length` di JS = unit kode UTF-16, bukan byte.
fn utf16_len(s: &str) -> usize {
    s.encode_utf16().count()
}

fn surah(data: &[Value]) -> String {
    data.iter()
        .map(|a| {
            format!(
                "۝ Ayat {}:\n{}\n{}\n_{}_",
                display(a.get("no")),
                display(a.get("arab")),
                display(a.get("latin")),
                display(a.get("indo")),
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn artikel(body: &Value, text: &str) -> String {
    if !body.get("success").is_some_and(truthy) {
        return "❌ Gagal mengambil data artikel.".to_string();
    }
    let list = body
        .get("data")
        .and_then(|d| d.get("data"))
        .and_then(Value::as_array)
        .filter(|a| !a.is_empty());
    let Some(list) = list else {
        return "❌ Artikel tidak ditemukan.".to_string();
    };
    let mut message = format!(
        "📚 *HASIL PENCARIAN: {}*\nTotal: {}\n\n",
        text.to_uppercase(),
        list.len()
    );
    for (i, a) in list.iter().enumerate() {
        message.push_str(&format!(
            "{}. *{}*\n🔗 {}\n\n",
            i + 1,
            display(a.get("title")),
            display(a.get("url")),
        ));
    }
    message
}

#[plugin(
    name = "islami",
    alias = [
        "asmaulhusna",
        "niatsholat",
        "niatshalat",
        "surah",
        "doa",
        "berdoa",
        "gislam"
    ],
    category = "religi",
    description = "Kumpulan fitur Islami (Asmaul Husna, Niat Sholat, Surah, Doa, Artikel, Kata Mutiara)",
    usage = ".islami <fitur>",
    example = "",
    cooldown = 5,
    energi = 1,
    is_group = false,
    is_private = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn islami(ctx: Ctx) -> Reply {
    let command = ctx.command.to_lowercase();
    let text = ctx.text.clone();
    let prefix = ctx.prefix.clone();

    // Dua guard JS (`!text`) yang tidak butuh jaringan, jadi dicek lebih dulu.
    match command.as_str() {
        "surah" if text.is_empty() => {
            return Reply::text(format!(
                "⚠️ Ketik nomor surahnya!\nContoh: `{prefix}surah 1` buat ambil ayat-ayat dari Al-Fatihah"
            ));
        }
        "gislam" if text.is_empty() => {
            return Reply::text(format!(
                "❓ Mau cari artikel tentang apa?\nContoh: `{prefix}gislam puasa`"
            ));
        }
        // Tanpa `default` di switch JS: command lain tidak membalas apa pun.
        "asmaulhusna" | "niatsholat" | "niatshalat" | "surah" | "doa" | "berdoa" | "gislam" => {}
        _ => return Reply::None,
    }

    // `fetchJson` + `catch` luar islami.js.
    let fetch = |url: String| ctx.http.get(&url).ok().and_then(|r| r.json());
    let Some(body) = fetch(match command.as_str() {
        "asmaulhusna" => ASMA_ENDPOINT.to_string(),
        "niatsholat" | "niatshalat" => NIAT_ENDPOINT.to_string(),
        "surah" => format!("{SURAH_ENDPOINT}?no={text}"),
        "doa" | "berdoa" => DOA_ENDPOINT.to_string(),
        _ => format!("{ARTIKEL_ENDPOINT}?page=1&s={text}"),
    }) else {
        return Reply::text(SYSTEM_ERROR);
    };

    Reply::text(match command.as_str() {
        "asmaulhusna" => asmaul_husna(body["data"].as_array().map(Vec::as_slice).unwrap_or(&[])),
        "niatsholat" | "niatshalat" => niat_sholat(body.as_array().map(Vec::as_slice).unwrap_or(&[]), &text, &prefix),
        "doa" | "berdoa" => doa(body.as_array().map(Vec::as_slice).unwrap_or(&[]), &text, &prefix),
        "surah" => {
            let data = body["data"].as_array().map(Vec::as_slice).unwrap_or(&[]);
            if data.is_empty() {
                "❌ Gak ketemu, cek lagi nomor surahnya!".to_string()
            } else {
                let rendered = surah(data);
                if utf16_len(&rendered) > 60_000 {
                    "❌ Surah terlalu panjang untuk dikirim via teks. \
                     Silakan cari ayat spesifik atau surah yang lebih pendek."
                        .to_string()
                } else {
                    rendered
                }
            }
        }
        _ => artikel(&body, &text),
    })
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

    fn run_with(http: Arc<dyn HttpGet>, command: &str, text: &str) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: command.into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            http,
            ..Default::default()
        };
        (oktz_plugin_api::find("islami").unwrap().handler)(ctx)
    }

    fn reply_text(r: &Reply) -> String {
        r.text_body().unwrap_or_default().to_string()
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("islami").unwrap();
        assert_eq!(p.meta.name, "islami");
        assert_eq!(
            p.meta.alias,
            [
                "asmaulhusna",
                "niatsholat",
                "niatshalat",
                "surah",
                "doa",
                "berdoa",
                "gislam"
            ]
        );
        assert_eq!(p.meta.category, "religi");
        assert_eq!(p.meta.usage, ".islami <fitur>");
        // `example` tidak ada di pluginConfig JS.
        assert_eq!(p.meta.example, "");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 1);
        assert!(!p.meta.is_group && !p.meta.is_private);
    }

    /// `asmaulhusna` tidak ikut di sini: `plugins/religi/asmaulhusna.js`
    /// mendaftarkannya sebagai Nama, jadi siapa yang menang tergantung
    /// urutan registrasi.
    #[test]
    fn every_other_alias_resolves_to_this_plugin() {
        for a in ["niatsholat", "niatshalat", "surah", "doa", "berdoa", "gislam"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "islami", "{a}");
        }
    }

    /// Switch JS tidak punya `default`, jadi command di luar 5 fiturnya diam.
    #[test]
    fn an_unmatched_command_replies_nothing() {
        for cmd in ["islami", "lain"] {
            let r = run_with(Arc::new(NoHttp), cmd, "");
            assert!(r.is_none(), "{cmd}: {r:?}");
        }
    }

    #[test]
    fn asmaul_husna_lists_every_entry_in_order() {
        let list = json!([
            { "index": 1, "latin": "Ar-Rahman", "arabic": "ar", "translation_id": "Yang Maha Pengasih", "translation_en": "The Most Gracious" },
            { "index": 2, "latin": "Ar-Rahim", "arabic": "ar", "translation_id": "Maha Penyayang", "translation_en": "The Most Merciful" }
        ]);
        let t = asmaul_husna(list.as_array().unwrap());
        assert!(t.starts_with("☪️ *ASMAUL HUSNA*\n\n"), "{t}");
        assert!(t.contains("Urutan: 1\nLatin: Ar-Rahman"), "{t}");
        assert!(t.contains("Terjemahan EN: The Most Gracious"), "{t}");
        assert!(t.contains("Urutan: 2"), "{t}");
    }

    #[test]
    fn niat_sholat_lists_when_empty_and_searches_case_insensitively() {
        let list = json!([
            { "name": "Niat Sholat Subuh", "arabic": "n", "latin": "l", "terjemahan": "Subuh" },
            { "name": "Niat Sholat Maghrib", "arabic": "g", "latin": "l", "terjemahan": "Magrib" }
        ]);
        let arr = list.as_array().unwrap();

        let daftar = niat_sholat(arr, "", ".");
        assert!(daftar.starts_with("📋 *DAFTAR NIAT SHOLAT*"), "{daftar}");
        assert!(
            daftar.contains("- Niat Sholat Subuh\n- Niat Sholat Maghrib"),
            "{daftar}"
        );
        assert!(
            daftar.ends_with(
                "\n\n📌 Ketik `.niatsholat [nama sholat]` untuk melihat niat\nContoh: `.niatsholat subuh`"
            ),
            "{daftar}"
        );

        let satu = niat_sholat(arr, "MAGHRIB", ".");
        assert!(satu.starts_with("🕋 *NIAT SHOLAT MAGHRIB*"), "{satu}");
        assert!(satu.ends_with("🌍 Terjemahan: Magrib"), "{satu}");

        assert_eq!(
            niat_sholat(arr, "zuhur", "."),
            "❌ Niat sholat yang kamu cari tidak ditemukan. Cek lagi nama sholatnya!"
        );
    }

    #[test]
    fn doa_lists_when_empty_and_searches_by_substring() {
        let list = json!([
            { "doa": "Doa Sebelum Tidur", "ayat": "a", "latin": "l", "artinya": "b" },
            { "doa": "Doa Setelah Sholat", "ayat": "c", "latin": "l", "artinya": "e" }
        ]);
        let arr = list.as_array().unwrap();

        let daftar = doa(arr, "", ".");
        assert!(daftar.starts_with("🤲 *DAFTAR DOA*"), "{daftar}");
        assert!(daftar.contains("- Doa Sebelum Tidur\n- Doa Setelah Sholat"), "{daftar}");
        assert!(
            daftar.ends_with(
                "\n\n📌 Ketik `.doa [nama doa]` untuk melihat doa\nContoh: `.doa doa sebelum tidur`"
            ),
            "{daftar}"
        );

        let satu = doa(arr, "setelah", ".");
        assert!(satu.starts_with("🤲 *DOA SETELAH SHOLAT*"), "{satu}");
        assert!(satu.ends_with("🌍 Artinya: e"), "{satu}");

        assert_eq!(
            doa(arr, "nonsense", "."),
            "❌ Doa yang kamu cari tidak ditemukan. Cek lagi nama doanya!"
        );
    }

    #[test]
    fn surah_without_a_number_asks_for_one_and_skips_http() {
        let r = run_with(Arc::new(NoHttp), "surah", "");
        let t = reply_text(&r);
        assert!(t.starts_with("⚠️ Ketik nomor surahnya!"), "{t}");
        assert!(t.contains("`.surah 1`"), "{t}");
    }

    #[test]
    fn surah_joins_verses_with_a_blank_line_and_counts_utf16_units() {
        let ayat = json!([
            { "no": 1, "arab": "bismi", "latin": "bismi", "indo": "Dengan nama" },
            { "no": 2, "arab": "allah", "latin": "allah", "indo": "Allah" }
        ]);
        assert_eq!(
            surah(ayat.as_array().unwrap()),
            "۝ Ayat 1:\nbismi\nbismi\n_Dengan nama_\n\n۝ Ayat 2:\nallah\nallah\n_Allah_"
        );

        // `surahText.length > 60000` — batasnya inklusif: tepat 60000 lolos.
        assert_eq!(utf16_len(&"a".repeat(60_000)), 60_000);
        assert!(utf16_len(&"a".repeat(60_001)) > 60_000);
        // Emoji di luar BMP = 2 unit di JS, bukan 4 byte seperti `str::len()`.
        assert_eq!(utf16_len("😀"), 2);
        assert_eq!("😀".len(), 4);
    }

    #[test]
    fn gislam_without_a_query_asks_what_to_search_for() {
        let r = run_with(Arc::new(NoHttp), "gislam", "");
        let t = reply_text(&r);
        assert!(t.starts_with("❓ Mau cari artikel tentang apa?"), "{t}");
        assert!(t.contains("`.gislam puasa`"), "{t}");
    }

    #[test]
    fn artikel_separates_success_empty_results_and_failure() {
        let ok = json!({
            "success": true,
            "data": { "data": [
                { "title": "Puasa", "url": "https://a/1" },
                { "title": "Zakat", "url": "https://a/2" }
            ] }
        });
        let t = artikel(&ok, "puasa");
        assert!(t.starts_with("📚 *HASIL PENCARIAN: PUASA*\nTotal: 2\n\n"), "{t}");
        assert!(t.contains("1. *Puasa*\n🔗 https://a/1\n\n"), "{t}");
        assert!(t.contains("2. *Zakat*\n🔗 https://a/2\n\n"), "{t}");

        assert_eq!(
            artikel(&json!({ "success": true, "data": { "data": [] } }), "x"),
            "❌ Artikel tidak ditemukan."
        );
        assert_eq!(
            artikel(&json!({ "success": false }), "x"),
            "❌ Gagal mengambil data artikel."
        );
    }

    #[test]
    fn a_transport_error_hits_the_outer_catch_for_every_remote_branch() {
        for (cmd, text) in [
            ("asmaulhusna", ""),
            ("niatsholat", ""),
            ("doa", ""),
            ("surah", "1"),
            ("gislam", "puasa"),
        ] {
            let r = run_with(Arc::new(NoHttp), cmd, text);
            assert_eq!(reply_text(&r), SYSTEM_ERROR, "{cmd}");
        }
    }

    /// Command ditulis kapital di chat — switch JS sudah `toLowerCase()`.
    #[test]
    fn the_command_is_lowercased_before_the_switch() {
        let r = run_with(Arc::new(NoHttp), "SURAH", "");
        assert!(
            reply_text(&r).starts_with("⚠️ Ketik nomor surahnya!"),
            "{}",
            reply_text(&r)
        );
    }
}
