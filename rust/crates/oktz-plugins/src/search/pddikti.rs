//! `plugins/search/pddikti.js` — cari data mahasiswa/dosen/PT/prodi dari PDDIKTI.
//!
//! `pddiktiGet` memakai `validateStatus: () => true`, jadi status 4xx/5xx
//! TIDAK jadi error: plugin membaca bodinya sendiri lalu melempar
//! `PDDIKTI HTTP <status>: <potongan json>`. Itu diterjemahkan ke
//! `HttpCall::get(..).terima_semua_status()`.
//!
//! `pluginConfig` di JS tidak punya `isOwner`/`isPremium`/`isGroup`/
//! `isPrivate` — semuanya `false`.

use oktz_plugin_api::{Ctx, HttpCall, Reply, plugin};
use serde_json::Value;

use crate::common::error_template;
use crate::tools::phisingataubukan::encode_uri_component;

/// `pddikti.js:16-18`.
pub(crate) const CORS_PROXY: &str = "https://cors.rifkyshre.biz.id/";
pub(crate) const API_BASE: &str = "https://api-pddikti.kemdiktisaintek.go.id";
pub(crate) const FRONTEND_ORIGIN: &str = "https://pddikti.kemdiktisaintek.go.id";

/// `pddikti.js:43-44` — mode detail tanpa `mahasiswaId`. Tidak terjangkau dari
/// handler: guard `args.length < 2` di `:162` lebih dulu membalas.
#[allow(dead_code)]
pub(crate) const BUTUH_MHS_ID: &str =
    "Field 'mahasiswaId' wajib untuk mode 'detail'. Ambil dari hasil search mhs (field 'id').";

/// `pddikti.js:74` — kata kunci kosong. Tidak terjangkau dari handler: guard
/// `if (!query)` di `:189` lebih dulu membalas dengan `BUTUH_KATA_KUNCI`.
#[allow(dead_code)]
pub(crate) const KATA_KUNCI_WAJIB: &str = "Kata kunci pencarian wajib (nama, NIM, NIDN, dll).";

/// `pddikti.js:77` — query pendek.
pub(crate) const QUERY_TERLALU_PENDEK: &str = "Query terlalu pendek (min 3 karakter).";

/// `pddikti.js:141-153` — kartu pakai.
pub(crate) fn help(prefix: &str) -> String {
    format!(
        "🎓 *PDDIKTI SEARCH*\n\n\
         > Mode pencarian:\n\
         - `{prefix}pddikti all <query>`\n\
         - `{prefix}pddikti mhs <nama/NIM>`\n\
         - `{prefix}pddikti dosen <nama/NIDN>`\n\
         - `{prefix}pddikti pt <nama_kampus>`\n\
         - `{prefix}pddikti prodi <nama_prodi>`\n\
         - `{prefix}pddikti detail <id_mahasiswa>`\n\n\
         *Contoh:* `{prefix}pddikti mhs Gibran Rakabuming`"
    )
}

/// `pddikti.js:162` / `:189` — argumen mode tidak lengkap.
pub(crate) const BUTUH_ID: &str = "❌ Masukkan ID Mahasiswa!";
pub(crate) const BUTUH_KATA_KUNCI: &str = "❌ Masukkan kata kunci pencarian!";

/// `pddikti.js:20-33` — URL + dua header.
pub(crate) fn request(path: &str) -> HttpCall {
    HttpCall::get(format!("{CORS_PROXY}{API_BASE}{path}"))
        .header("Accept", "application/json")
        .header("X-Cors-Spoof-Origin", FRONTEND_ORIGIN)
        .terima_semua_status()
}

/// `pddikti.js:81-93` — mode → path. `None` = mode tidak dikenal.
pub(crate) fn path_for(mode: &str, query: &str) -> Option<String> {
    let q = encode_uri_component(query);
    Some(match mode {
        "all" => format!("/pencarian/all/{q}"),
        "mhs" | "mahasiswa" => format!("/pencarian/mhs/{q}"),
        "dosen" => format!("/pencarian/dosen/{q}"),
        "pt" => format!("/pencarian/pt/{q}"),
        "prodi" => format!("/pencarian/prodi/{q}"),
        _ => return None,
    })
}

/// `pddikti.js:92` — pesan mode tak dikenal.
pub(crate) fn unknown_mode(mode: &str) -> String {
    format!("Unknown mode '{mode}'. Pakai: all | mhs | dosen | pt | prodi | detail")
}

/// `pddikti.js:46` — path detail.
pub(crate) fn detail_path(mhs_id: &str) -> String {
    format!("/detail/mhs/{}", encode_uri_component(mhs_id))
}

/// `pddikti.js:47-49` — baris ringkasan detail. Handler JS tidak memakainya
/// (hanya `Result.message`); dipakai test untuk membuktikan bentuknya.
#[allow(dead_code)]
pub(crate) fn detail_message(data: &Value) -> String {
    let nama = data.get("nama").and_then(Value::as_str);
    match nama {
        Some(nama) => format!(
            "🎓 {nama} — {} | {} @ {}",
            interp(data.get("nim")),
            interp(data.get("prodi")),
            interp(data.get("nama_pt"))
        ),
        None => "🎓 Detail mahasiswa".to_string(),
    }
}

/// `${value}` di JS.
fn interp(value: Option<&Value>) -> String {
    match value {
        None => "undefined".to_string(),
        Some(Value::Null) => "null".to_string(),
        Some(Value::String(s)) => s.clone(),
        Some(other) => other.to_string(),
    }
}

/// `pddikti.js:56` — `L`/`P` ke label, selain itu `null` yang jadi `-` lewat `??`.
pub(crate) fn jenis_kelamin(value: Option<&Value>) -> Option<&'static str> {
    match value.and_then(Value::as_str) {
        Some("L") => Some("Laki-laki"),
        Some("P") => Some("Perempuan"),
        _ => None,
    }
}

/// `pddikti.js:97-115` — ringkasan mode `all`.
pub(crate) fn all_message(query: &str, mhs: usize, dosen: usize, pt: usize, prodi: usize) -> String {
    format!(
        "🔍 {} hasil untuk \"{query}\" ({mhs} mhs, {dosen} dosen, {pt} pt, {prodi} prodi)",
        mhs + dosen + pt + prodi
    )
}

/// `pddikti.js:121-123` — ringkasan mode selain `all`.
pub(crate) fn list_message(mode: &str, query: &str, count: usize) -> String {
    if count > 0 {
        format!("🔍 {count} hasil {mode} untuk \"{query}\"")
    } else {
        format!("🔍 Gak ada hasil {mode} untuk \"{query}\"")
    }
}

/// `pddikti.js:172-184` — blok `DETAIL MAHASISWA`. Spasi kolom ditulis
/// eksplisit di JS, jadi ikut dikunci di sini.
pub(crate) fn detail_block(data: &Value) -> String {
    let jk = jenis_kelamin(data.get("jenis_kelamin")).unwrap_or("-");
    let mut txt = String::from("🎓 *DETAIL MAHASISWA*\n\n");
    txt.push_str(&format!("- 📝 Nama          : *{}*\n", interp(data.get("nama"))));
    txt.push_str(&format!("- 🆔 NIM           : *{}*\n", interp(data.get("nim"))));
    txt.push_str(&format!("- 👤 Jenis Kelamin : *{jk}*\n"));
    txt.push_str(&format!("- 🎓 Jenjang       : *{}*\n", interp(data.get("jenjang"))));
    txt.push_str(&format!("- 📚 Prodi         : *{}*\n", interp(data.get("prodi"))));
    txt.push_str(&format!("- 🏛️ Perguruan T.  : *{}*\n", interp(data.get("nama_pt"))));
    txt.push_str(&format!("- 📅 Tgl Masuk     : *{}*\n", interp(data.get("tanggal_masuk"))));
    txt.push_str(&format!("- 📊 Status        : *{}*\n", interp(data.get("status_saat_ini"))));
    txt.push_str(&format!("- 💼 Jenis Daftar  : *{}*\n", interp(data.get("jenis_daftar"))));
    txt
}

/// `pddikti.js:198-245` — blok hasil mode `all`; tiap kategori maksimal 3.
pub(crate) fn all_block(data: &Value, query: &str) -> String {
    let ambil = |k: &str| -> Vec<Value> {
        data.get(k).and_then(Value::as_array).cloned().unwrap_or_default()
    };
    let (mhs, dosen, pt, prodi) = (ambil("mahasiswa"), ambil("dosen"), ambil("pt"), ambil("prodi"));
    let mut txt = format!("*{}*\n\n", all_message(query, mhs.len(), dosen.len(), pt.len(), prodi.len()));

    let top3 = |v: &Vec<Value>| -> Vec<Value> { v.iter().take(3).cloned().collect() };
    let mh = top3(&mhs);
    if !mh.is_empty() {
        txt.push_str("👨🎓 *MAHASISWA (top 3):*\n");
        for m in &mh {
            txt.push_str(&format!(
                "- {} [{}] — {}, {}\n",
                interp(m.get("nama")),
                interp(m.get("nim")),
                interp(m.get("nama_prodi")),
                interp(m.get("nama_pt"))
            ));
        }
        txt.push('\n');
    }
    let ds = top3(&dosen);
    if !ds.is_empty() {
        txt.push_str("👨🏫 *DOSEN (top 3):*\n");
        for d in &ds {
            txt.push_str(&format!(
                "- {} [NIDN {}] — {}, {}\n",
                interp(d.get("nama")),
                interp(d.get("nidn")),
                interp(d.get("nama_prodi")),
                interp(d.get("nama_pt"))
            ));
        }
        txt.push('\n');
    }
    let pt3 = top3(&pt);
    if !pt3.is_empty() {
        txt.push_str("🏛️ *PERGURUAN TINGGI (top 3):*\n");
        for p in &pt3 {
            // `p.nama_singkat ?? p.sinkatan_pt ?? "-"` — `??` bukan `||`.
            let singkat = match p.get("nama_singkat") {
                None | Some(Value::Null) => interp(p.get("sinkatan_pt")),
                Some(other) => interp(Some(other)),
            };
            let singkat = if singkat == "undefined" || singkat == "null" {
                "-".to_string()
            } else {
                singkat
            };
            txt.push_str(&format!("- {} ({singkat})\n", interp(p.get("nama"))));
        }
        txt.push('\n');
    }
    let pr3 = top3(&prodi);
    if !pr3.is_empty() {
        txt.push_str("📚 *PRODI (top 3):*\n");
        for p in &pr3 {
            txt.push_str(&format!("- {} @ {}\n", interp(p.get("nama")), interp(p.get("nama_pt"))));
        }
        txt.push('\n');
    }
    txt
}

/// `pddikti.js:226-245` — mode selain `all`, maksimal 10 baris.
pub(crate) fn list_block(items: &[Value], mode: &str, query: &str, count: usize) -> String {
    let mut txt = format!("*{}*\n\n", list_message(mode, query, count));
    for (i, item) in items.iter().take(10).enumerate() {
        match mode {
            "mhs" | "mahasiswa" => {
                txt.push_str(&format!(
                    "*#{} {} [NIM {}]*\n- 📚 {} @ {} ({})\n- 🆔 ID: `{}`\n\n",
                    i + 1,
                    interp(item.get("nama")),
                    interp(item.get("nim")),
                    interp(item.get("nama_prodi")),
                    interp(item.get("nama_pt")),
                    or_dash(item.get("sinkatan_pt")),
                    interp(item.get("id"))
                ));
            }
            "dosen" => {
                txt.push_str(&format!(
                    "*#{} {} [NIDN {}]*\n- 📚 {} @ {}\n\n",
                    i + 1,
                    interp(item.get("nama")),
                    interp(item.get("nidn")),
                    interp(item.get("nama_prodi")),
                    interp(item.get("nama_pt"))
                ));
            }
            "pt" => {
                // `p.nama_singkat ?? "-"` — `??` bukan `||`, jadi 0 tetap tampil.
                let singkat = match item.get("nama_singkat") {
                    None | Some(Value::Null) => "-".to_string(),
                    Some(v) => interp(Some(v)),
                };
                txt.push_str(&format!(
                    "*#{} {} ({singkat})*\n- Kode: {}\n\n",
                    i + 1,
                    interp(item.get("nama")),
                    or_dash(item.get("kode"))
                ));
            }
            _ => {
                txt.push_str(&format!(
                    "*#{} {}*\n- @ {}\n\n",
                    i + 1,
                    interp(item.get("nama")),
                    match item.get("nama_pt") {
                        None | Some(Value::Null) => "-".to_string(),
                        Some(v) => interp(Some(v)),
                    }
                ));
            }
        }
    }
    if count > 10 {
        txt.push_str(&format!("> ... +{} hasil lainnya.\n", count - 10));
    }
    txt
}

/// `x || "-"` — `0` dan `""` falsy di JS.
fn or_dash(value: Option<&Value>) -> String {
    match value {
        Some(Value::String(s)) if !s.is_empty() => s.clone(),
        Some(Value::Number(n)) if n.as_f64().is_some_and(|f| f != 0.0) => n.to_string(),
        Some(Value::Bool(true)) => "true".to_string(),
        _ => "-".to_string(),
    }
}

/// `pddikti.js:30` — pesan error HTTP.
pub(crate) fn http_error(status: u16, body: &str) -> String {
    let json = if body.trim().is_empty() {
        String::from("undefined")
    } else {
        serde_json::from_str::<Value>(body)
            .map(|v| v.to_string())
            .unwrap_or_else(|_| String::from("undefined"))
    };
    let chars: Vec<char> = json.chars().collect();
    let potong: String = chars
        .iter()
        .take(150)
        .collect::<String>();
    format!("PDDIKTI HTTP {status}: {potong}")
}

#[plugin(
    name = "pddikti",
    alias = ["dikti", "carimahasiswa"],
    category = "search",
    description = "Cari data Mahasiswa, Dosen, PT, dan Prodi dari PDDIKTI",
    usage = ".pddikti <mode> <query>",
    example = ".pddikti all Gibran Khalil\n.pddikti detail <id_mahasiswa>",
    cooldown = 15,
    energi = 1,
    is_enabled = true,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn pddikti(ctx: Ctx) -> Reply {
    if ctx.args.is_empty() {
        return Reply::text(help(&ctx.prefix));
    }

    // `m.react("🕕")` di `:156` sudah jalan; reaksi terakhir per cabang yang
    // diambil di bawah.
    let mode = ctx.args[0].to_lowercase();

    if mode == "detail" {
        // `if (args.length < 2)` (`:162`) dan `if (!query)` (`:189`) keduanya
        // `return m.reply(..)` tanpa bereaksi lagi — jadi reaksi terakhir yang
        // sudah jalan, `🕕`, tetap berlaku.
        if ctx.args.len() < 2 {
            return Reply::text(BUTUH_ID).with_reaction("🕕");
        }
        let call = request(&detail_path(&ctx.args[1]));
        let Ok(res) = ctx.http.call(&call) else {
            return Reply::text(error_template(&ctx.prefix, &ctx.command)).with_reaction("☢");
        };
        if res.status != 200 {
            return Reply::text(format!(
                "❌ {}",
                http_error(res.status, &res.text())
            ))
            .with_reaction("❌");
        }
        let data = res.json().unwrap_or(Value::Null);
        return Reply::text(detail_block(&data)).with_reaction("✅");
    }

    let query = ctx.args[1..].join(" ");
    if query.is_empty() {
        return Reply::text(BUTUH_KATA_KUNCI).with_reaction("🕕");
    }

    // `pddikti()` menolak query kosong dan query < 3 karakter SEBELUM memanggil
    // API, jadi keduanya tidak melewati jaringan.
    if query.chars().count() < 3 {
        return Reply::text(format!("❌ {QUERY_TERLALU_PENDEK}")).with_reaction("❌");
    }
    let Some(path) = path_for(&mode, &query) else {
        return Reply::text(format!("❌ {}", unknown_mode(&mode))).with_reaction("❌");
    };

    let Ok(res) = ctx.http.call(&request(&path)) else {
        return Reply::text(error_template(&ctx.prefix, &ctx.command)).with_reaction("☢");
    };
    if res.status != 200 {
        return Reply::text(format!("❌ {}", http_error(res.status, &res.text())))
            .with_reaction("❌");
    }
    let data = res.json().unwrap_or(Value::Null);

    // `mode === "all" && data && typeof data === "object" && !Array.isArray(data)`
    // (`:97`) — objek apa pun yang bukan array dianggap hasil `all`.
    let all_mode = mode == "all" && data.is_object() && !data.is_array();
    let txt = if all_mode {
        all_block(&data, &query)
    } else {
        let items = data.as_array().cloned().unwrap_or_default();
        list_block(&items, &mode, &query, items.len())
    };
    Reply::text(txt.trim().to_string()).with_reaction("✅")
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{HttpBody, HttpError, HttpGet, HttpResponse};
    use serde_json::json;
    use std::sync::{Arc, Mutex};

    const ME: &str = "628111@s.whatsapp.net";

    #[derive(Debug)]
    struct Stub {
        /// Prefix URL -> (status, body).
        pages: Vec<(String, u16, String)>,
        seen: Mutex<Vec<String>>,
    }

    impl Stub {
        fn new(pages: &[(&str, u16, &str)]) -> Arc<Stub> {
            Arc::new(Stub {
                pages: pages
                    .iter()
                    .map(|(u, s, b)| ((*u).to_string(), *s, (*b).to_string()))
                    .collect(),
                seen: Mutex::new(Vec::new()),
            })
        }
        fn offline() -> Arc<Stub> {
            Arc::new(Stub {
                pages: vec![],
                seen: Mutex::new(Vec::new()),
            })
        }
    }

    impl HttpGet for Stub {
        fn get(&self, url: &str) -> Result<HttpResponse, HttpError> {
            self.seen.lock().unwrap().push(url.to_string());
            Err(HttpError {
                message: "cuma dipakai lewat call()".into(),
            })
        }
        fn post_json(&self, _: &str, _: &Value) -> Result<HttpResponse, HttpError> {
            unimplemented!("cuma GET")
        }
        fn post_form(&self, _: &str, _: &str) -> Result<HttpResponse, HttpError> {
            unimplemented!("cuma GET")
        }
        fn post_multipart(
            &self,
            _: &str,
            _: &[(&str, HttpBody)],
        ) -> Result<HttpResponse, HttpError> {
            unimplemented!("cuma GET")
        }

        // `HttpCall` bypassing `get()`: pddikti memakai header + status apa pun.
        fn call(&self, req: &oktz_plugin_api::HttpCall) -> Result<HttpResponse, HttpError> {
            self.seen.lock().unwrap().push(req.url.clone());
            let hit = self
                .pages
                .iter()
                .find(|(u, _, _)| req.url.starts_with(u.as_str()));
            let Some((_, status, body)) = hit else {
                return Err(HttpError {
                    message: "tidak ada stub untuk URL ini".into(),
                });
            };
            Ok(HttpResponse {
                status: *status,
                body: HttpBody::Bytes {
                    data: Arc::from(body.clone().into_bytes()),
                    mimetype: Some("application/json".into()),
                },
                headers: vec![],
            })
        }
    }

    fn run(http: Arc<Stub>, text: &str) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "pddikti".into(),
            sender: ME.into(),
            chat: ME.into(),
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            http,
            ..Default::default()
        };
        (oktz_plugin_api::find("pddikti").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> String {
        r.without_mutations()
            .text_body()
            .expect("harus ada teks")
            .to_string()
    }

    fn reaction_of(r: &Reply) -> Option<&str> {
        let mut cur = r;
        loop {
            match cur {
                Reply::Reaction { emoji, .. } => return Some(emoji),
                Reply::Mutate { reply, .. } => cur = reply,
                _ => return None,
            }
        }
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("pddikti").unwrap();
        assert_eq!(p.meta.name, "pddikti");
        assert_eq!(p.meta.alias, ["dikti", "carimahasiswa"]);
        assert_eq!(p.meta.category, "search");
        assert_eq!(
            p.meta.description,
            "Cari data Mahasiswa, Dosen, PT, dan Prodi dari PDDIKTI"
        );
        assert_eq!(p.meta.usage, ".pddikti <mode> <query>");
        assert_eq!(
            p.meta.example,
            ".pddikti all Gibran Khalil\n.pddikti detail <id_mahasiswa>"
        );
        assert_eq!(p.meta.cooldown, 15);
        assert_eq!(p.meta.energi, 1);
        // Tidak ada di pluginConfig JS.
        assert!(!p.meta.is_owner && !p.meta.is_premium);
        assert!(!p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["dikti", "carimahasiswa"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "pddikti");
        }
    }

    #[test]
    fn no_arguments_shows_the_mode_card() {
        let http = Stub::offline();
        let r = run(http.clone(), "");
        assert_eq!(text_of(&r), help("."));
        assert!(help(".").contains("- `.pddikti detail <id_mahasiswa>`"));
        assert!(help("#").contains("`#pddikti all <query>`"));
        assert!(http.seen.lock().unwrap().is_empty());
        assert_eq!(reaction_of(&r), None);
    }

    #[test]
    fn each_mode_maps_to_its_own_path() {
        assert_eq!(path_for("all", "a b").unwrap(), "/pencarian/all/a%20b");
        assert_eq!(path_for("mhs", "Gibran").unwrap(), "/pencarian/mhs/Gibran");
        assert_eq!(
            path_for("mahasiswa", "Gibran").unwrap(),
            "/pencarian/mhs/Gibran",
            "alias mahasiswa memakai endpoint yang sama"
        );
        assert_eq!(path_for("dosen", "X").unwrap(), "/pencarian/dosen/X");
        assert_eq!(path_for("pt", "X").unwrap(), "/pencarian/pt/X");
        assert_eq!(path_for("prodi", "X").unwrap(), "/pencarian/prodi/X");
        assert!(path_for("detail", "X").is_none(), "detail punya jalurnya sendiri");
        assert!(path_for("ngawur", "X").is_none());
        assert_eq!(
            unknown_mode("ngawur"),
            "Unknown mode 'ngawur'. Pakai: all | mhs | dosen | pt | prodi | detail"
        );
    }

    #[test]
    fn short_queries_and_unknown_modes_never_reach_the_network() {
        let http = Stub::offline();
        for (text, msg, reaksi) in [
            (
                "mhs ab",
                "❌ Query terlalu pendek (min 3 karakter).",
                "❌",
            ),
            // Guard `if (!query)` di `:189` tidak bereaksi lagi: `🕕` dari
            // `:156` masih yang terakhir.
            ("all  ", "❌ Masukkan kata kunci pencarian!", "🕕"),
            (
                "ngawur abc",
                "❌ Unknown mode 'ngawur'. Pakai: all | mhs | dosen | pt | prodi | detail",
                "❌",
            ),
            ("detail", "❌ Masukkan ID Mahasiswa!", "🕕"),
        ] {
            let r = run(http.clone(), text);
            assert_eq!(text_of(&r), msg, "input {text:?}");
            assert_eq!(reaction_of(&r), Some(reaksi), "input {text:?}");
        }
        assert!(http.seen.lock().unwrap().is_empty());
    }

    #[test]
    fn a_three_character_query_is_accepted() {
        assert_eq!(QUERY_TERLALU_PENDEK, "Query terlalu pendek (min 3 karakter).");
        assert_eq!(
            BUTUH_KATA_KUNCI,
            "❌ Masukkan kata kunci pencarian!"
        );
        assert!(BUTUH_MHS_ID.starts_with("Field 'mahasiswaId' wajib"));
        // Tiga karakter tepat lolos: handler memanggil jaringan.
        let http = Stub::offline();
        let _ = run(http.clone(), "mhs abc");
        assert_eq!(http.seen.lock().unwrap().len(), 1);
    }

    #[test]
    fn the_request_goes_through_the_cors_proxy_with_both_headers() {
        let call = request(&path_for("mhs", "Gibran Khalil").unwrap());
        assert_eq!(call.method, oktz_plugin_api::HttpMethod::Get);
        assert_eq!(
            call.url,
            "https://cors.rifkyshre.biz.id/https://api-pddikti.kemdiktisaintek.go.id/pencarian/mhs/Gibran%20Khalil"
        );
        assert!(call.headers.iter().any(|(k, v)| k == "Accept" && v == "application/json"));
        assert!(call
            .headers
            .iter()
            .any(|(k, v)| k == "X-Cors-Spoof-Origin" && v == FRONTEND_ORIGIN));
        assert!(call.terima_semua_status, "validateStatus: () => true");
    }

    #[test]
    fn mode_all_lists_three_of_each_category() {
        let body = json!({
            "mahasiswa": (0..5).map(|i| json!({ "nama": format!("Mhs {i}"), "nim": format!("22{i}"), "nama_prodi": "TI", "nama_pt": "PT" })).collect::<Vec<_>>(),
            "dosen": (0..4).map(|i| json!({ "nama": format!("Dosen {i}"), "nidn": format!("00{i}"), "nama_prodi": "TI", "nama_pt": "PT" })).collect::<Vec<_>>(),
            "pt": [ { "nama": "ITB", "nama_singkat": "ITB" } ],
            "prodi": [ { "nama": "S1 Informatika", "nama_pt": "ITB" } ],
        });
        let t = all_block(&body, "x");
        assert!(t.starts_with("*🔍 11 hasil untuk \"x\" (5 mhs, 4 dosen, 1 pt, 1 prodi)*\n\n"), "{t}");
        assert!(t.contains("👨🎓 *MAHASISWA (top 3):*\n- Mhs 0 [220] — TI, PT\n"), "{t}");
        assert!(t.contains("👨🏫 *DOSEN (top 3):*\n- Dosen 0 [NIDN 000] — TI, PT\n"), "{t}");
        assert!(t.contains("🏛️ *PERGURUAN TINGGI (top 3):*\n- ITB (ITB)\n"), "{t}");
        assert!(t.contains("📚 *PRODI (top 3):*\n- S1 Informatika @ ITB\n"), "{t}");
        assert!(!t.contains("Mhs 3"), "top 3: {t}");
        assert!(!t.contains("Dosen 3"), "top 3: {t}");
    }

    #[test]
    fn a_pt_without_a_short_name_falls_back_to_the_acronym_field() {
        let body = json!({ "pt": [ { "nama": "Institut Teknologi", "sinkatan_pt": "IT" } ] });
        assert!(all_block(&body, "q").contains("- Institut Teknologi (IT)\n"));
        let body = json!({ "pt": [ { "nama": "Tanpa apa-apa" } ] });
        assert!(all_block(&body, "q").contains("- Tanpa apa-apa (-)\n"));
    }

    #[test]
    fn an_empty_all_object_produces_only_the_headline() {
        let t = all_block(&json!({}), " gibran ");
        assert_eq!(t, "*🔍 0 hasil untuk \" gibran \" (0 mhs, 0 dosen, 0 pt, 0 prodi)*\n\n");
    }

    #[test]
    fn student_rows_carry_id_and_cap_at_ten() {
        let items: Vec<Value> = (0..13)
            .map(|i| json!({ "nama": format!("S{i}"), "nim": "1", "nama_prodi": "P", "nama_pt": "U", "id": format!("id-{i}") }))
            .collect();
        let t = list_block(&items, "mhs", "s", 13);
        assert!(t.starts_with("*🔍 13 hasil mhs untuk \"s\"*\n\n"), "{t}");
        assert!(t.contains("*#1 S0 [NIM 1]*\n- 📚 P @ U (-)\n- 🆔 ID: `id-0`\n"), "{t}");
        assert!(t.contains("*#10 S9 [NIM 1]*"), "{t}");
        assert!(!t.contains("*#11"), "baris ke-11 terpotong: {t}");
        assert!(t.contains("> ... +3 hasil lainnya.\n"), "{t}");
    }

    #[test]
    fn an_empty_list_says_so_and_adds_no_rows() {
        let t = list_block(&[], "dosen", "zzz", 0);
        assert_eq!(t, "*🔍 Gak ada hasil dosen untuk \"zzz\"*\n\n");
        assert!(!t.contains("+0 hasil lainnya"));
    }

    #[test]
    fn the_four_non_all_modes_render_their_own_row_shape() {
        // Tanpa `nama_singkat`, JS memakai `?? "-"` — bukan `nama` itu sendiri.
        let pt = vec![json!({ "nama": "ITB", "kode": "1" })];
        assert!(list_block(&pt, "pt", "q", 1).contains("*#1 ITB (-)*\n- Kode: 1\n\n"));
        let pt = vec![json!({ "nama": "ITB", "nama_singkat": "ITB", "kode": "1" })];
        assert!(list_block(&pt, "pt", "q", 1).contains("*#1 ITB (ITB)*\n- Kode: 1\n\n"));
        let pt = vec![json!({ "nama": "ITB", "nama_singkat": "", "kode": 0 })];
        assert!(
            list_block(&pt, "pt", "q", 1).contains("*#1 ITB ()*\n- Kode: -\n\n"),
            "`??` bukan `||`: string kosong tetap tampil"
        );
        let dosen = vec![json!({ "nama": "D", "nidn": "1" })];
        assert!(list_block(&dosen, "dosen", "q", 1).contains("*#1 D [NIDN 1]*\n- 📚 undefined @ undefined\n\n"));
        let prodi = vec![json!({ "nama": "P" })];
        assert!(list_block(&prodi, "prodi", "q", 1).contains("*#1 P*\n- @ -\n\n"));
    }

    #[test]
    fn detail_renders_all_nine_labelled_lines() {
        let data = json!({
            "nama": "Gibran", "nim": "22", "jenis_kelamin": "L", "jenjang": "S1",
            "prodi": "Informatika", "nama_pt": "ITB", "tanggal_masuk": "2020-09-01",
            "status_saat_ini": "Aktif", "jenis_daftar": "Reguler",
        });
        let t = detail_block(&data);
        assert!(t.starts_with("🎓 *DETAIL MAHASISWA*\n\n- 📝 Nama          : *Gibran*\n"), "{t}");
        assert!(t.contains("- 👤 Jenis Kelamin : *Laki-laki*\n"), "{t}");
        assert!(t.contains("- 💼 Jenis Daftar  : *Reguler*\n"), "{t}");
        assert!(detail_message(&data).starts_with("🎓 Gibran — 22 | Informatika @ ITB"));
        assert_eq!(detail_message(&json!({})), "🎓 Detail mahasiswa");
    }

    #[test]
    fn an_unknown_gender_prints_the_dash_fallback() {
        assert_eq!(jenis_kelamin(Some(&json!("L"))), Some("Laki-laki"));
        assert_eq!(jenis_kelamin(Some(&json!("P"))), Some("Perempuan"));
        assert_eq!(jenis_kelamin(Some(&json!("X"))), None);
        assert_eq!(jenis_kelamin(None), None);
        assert!(detail_block(&json!({})).contains("Jenis Kelamin : *-*\n"));
        assert!(detail_block(&json!({})).contains("Nama          : *undefined*\n"));
    }

    #[test]
    fn a_full_round_trip_renders_the_stubbed_search_and_detail() {
        let http = Stub::new(&[
            (
                "https://cors.rifkyshre.biz.id/https://api-pddikti.kemdiktisaintek.go.id/pencarian/mhs/",
                200,
                r#"[{"nama":"Gibran","nim":"221","nama_prodi":"Informatika","nama_pt":"ITB","id":"abc"}]"#,
            ),
            (
                "https://cors.rifkyshre.biz.id/https://api-pddikti.kemdiktisaintek.go.id/detail/mhs/",
                200,
                r#"{"nama":"Gibran","nim":"221","jenis_kelamin":"P","prodi":"Informatika","nama_pt":"ITB"}"#,
            ),
        ]);
        let r = run(http.clone(), "mhs Gibran");
        assert!(text_of(&r).contains("*#1 Gibran [NIM 221]*"), "{}", text_of(&r));
        assert_eq!(reaction_of(&r), Some("✅"));
        assert_eq!(
            http.seen.lock().unwrap()[0],
            "https://cors.rifkyshre.biz.id/https://api-pddikti.kemdiktisaintek.go.id/pencarian/mhs/Gibran"
        );

        let r = run(http.clone(), "detail abc");
        let t = text_of(&r);
        assert!(t.contains("Jenis Kelamin : *Perempuan*\n"), "{t}");
        assert!(text_of(&r).contains("Nama          : *Gibran*\n"), "{t}");
        assert_eq!(
            http.seen.lock().unwrap()[1],
            "https://cors.rifkyshre.biz.id/https://api-pddikti.kemdiktisaintek.go.id/detail/mhs/abc"
        );
    }

    #[test]
    fn a_non_200_status_becomes_the_http_error_line() {
        let http = Stub::new(&[(
            "https://cors.rifkyshre.biz.id/",
            503,
            r#"{"message":"lagi tutup"}"#,
        )]);
        let r = run(http, "all gibran");
        assert_eq!(
            text_of(&r),
            "❌ PDDIKTI HTTP 503: {\"message\":\"lagi tutup\"}"
        );
        assert_eq!(reaction_of(&r), Some("❌"));
    }

    #[test]
    fn a_non_json_body_still_produces_a_message() {
        let http = Stub::new(&[("https://cors.rifkyshre.biz.id/", 500, "<html>oops</html>")]);
        let r = run(http, "all gibran");
        assert_eq!(text_of(&r), "❌ PDDIKTI HTTP 500: undefined");
    }

    #[test]
    fn a_transport_failure_falls_back_to_the_error_template() {
        let r = run(Stub::offline(), "all gibran");
        assert_eq!(text_of(&r), error_template(".", "pddikti"));
        assert_eq!(reaction_of(&r), Some("☢"));
    }

    #[test]
    fn odd_and_very_long_queries_do_not_panic() {
        for input in [
            "mhs <b>", "mhs a|b", "mhs 🎨🎨🎨", "mhs '\"", "mhs a\u{202e}b", "mhs ??",
        ] {
            let r = run(Stub::offline(), input);
            assert!(!text_of(&r).is_empty(), "input {input:?}");
        }
        let panjang = "x".repeat(6000);
        let http = Stub::offline();
        let _ = run(http.clone(), &format!("mhs {panjang}"));
        let seen = http.seen.lock().unwrap().clone();
        assert_eq!(seen.len(), 1);
        assert!(
            seen[0].contains(&"x".repeat(2000)),
            "query ter-encode utuh: {}",
            &seen[0][..seen[0].len().min(80)]
        );
    }
}