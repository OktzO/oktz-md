//! `src/scraper/txt2img.js` — generator gambar unrestrictedaiimagegenerator.com.
//!
//! Yang disalin dari JS:
//! - Argumen `style` punya bawaan `"anime"`; di Rust jadi `Option<&str>`
//!   (`None` = `undefined`, `Some("")` tetap ditolak seperti JS).
//! - `if (!prompt)` dan `if (!styles.includes(style))` melempar sebelum
//!   request apa pun; teks pesan sama huruf per huruf.
//! - Cookie dari `headers["set-cookie"]?.join("; ")` (utuh, termasuk
//!   atribut) dipasang sebagai header `Cookie` di POST berikutnya.
//! - Body POST adalah `URLSearchParams.toString()`: spasi `+`, dan
//!   `!'()~` ikut di-encode (BEDA dari `encodeURIComponent`).
//! - Fungsi mengembalikan string URL gambar, bukan objek JSON.

use std::sync::LazyLock;

use oktz_plugin_api::{HttpCall, HttpError, HttpGet};

const BASE: &str = "https://unrestrictedaiimagegenerator.com";
const UA: &str = "Mozilla/5.0 (Linux; Android 15) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130 Mobile Safari/537.36";
const STYLES: [&str; 7] = [
    "photorealistic",
    "digital-art",
    "impressionist",
    "anime",
    "fantasy",
    "sci-fi",
    "vintage",
];

/// `unrestrictedai(prompt, style = "anime")` (`txt2img.js:13-62`).
pub fn unrestrictedai(
    http: &dyn HttpGet,
    prompt: &str,
    style: Option<&str>,
) -> Result<String, HttpError> {
    if prompt.is_empty() {
        return Err(HttpError {
            message: "Prompt is required".to_string(),
        });
    }
    let style = style.unwrap_or("anime");
    if !STYLES.contains(&style) {
        return Err(HttpError {
            message: format!("Available styles: {}", STYLES.join(", ")),
        });
    }

    // `client` punya header bawaan origin/referer/user-agent untuk semua
    // request; denganCredentials + jar cookie ditiru dengan meneruskan
    // set-cookie pertama secara manual.
    let res = http.call(
        &HttpCall::get(format!("{BASE}/"))
            .header("origin", BASE)
            .header("referer", format!("{BASE}/"))
            .header("user-agent", UA),
    )?;
    let cookie = res
        .headers
        .iter()
        .filter(|(k, _)| k.eq_ignore_ascii_case("set-cookie"))
        .map(|(_, v)| v.as_str())
        .collect::<Vec<_>>()
        .join("; ");
    let html = res.text();

    let nonce = nonce_dari(&html).ok_or_else(|| HttpError {
        message: "Nonce not found".to_string(),
    })?;

    let body = format!(
        "generate_image=true&image_description={}&image_style={}&_wpnonce={}",
        encode_form(prompt),
        encode_form(style),
        encode_form(&nonce)
    );
    let mut post = HttpCall::post(format!("{BASE}/"))
        .header("origin", BASE)
        .header("referer", format!("{BASE}/"))
        .header("user-agent", UA)
        .header("content-type", "application/x-www-form-urlencoded");
    // `if (cookies) client.defaults.headers.Cookie = cookies` — join array
    // kosong juga falsy, jadi header tidak dipasang.
    if !cookie.is_empty() {
        post = post.header("cookie", &cookie);
    }

    let hasil = http.call(&post.form(body))?;
    let img = img_hasil(&hasil.text()).ok_or_else(|| HttpError {
        message: "Image not found".to_string(),
    })?;
    Ok(img)
}

/// `$('input[name="_wpnonce"]').val()` — input pertama dengan `name` itu.
static RE_INPUT: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"(?i)<input\b[^>]*>").unwrap());

fn nonce_dari(html: &str) -> Option<String> {
    for tag in RE_INPUT.find_iter(html).map(|m| m.as_str()) {
        if atribut(tag, "name").as_deref() == Some("_wpnonce") {
            return atribut(tag, "value");
        }
    }
    None
}

/// `$$("img#resultImage").attr("src")` — img pertama ber-id `resultImage`.
static RE_IMG: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"(?i)<img\b[^>]*>").unwrap());

fn img_hasil(html: &str) -> Option<String> {
    for tag in RE_IMG.find_iter(html).map(|m| m.as_str()) {
        if atribut(tag, "id").as_deref() == Some("resultImage") {
            // `!img` juga benar saat `src` hilang (undefined) atau "".
            return atribut(tag, "src").filter(|s| !s.is_empty());
        }
    }
    None
}

/// Nilai atribut dari teks tag, entitas HTML didekode seperti cheerio.
/// Mendukung `nama="x"`, `nama='x'`, dan `nama=x`; nama atribut tidak peka
/// huruf besar-kecil, nilainya peka.
fn atribut(tag: &str, nama: &str) -> Option<String> {
    let pola = format!(
        r#"(?i)(?:^|[\s"']){}\s*=\s*(?:"([^"]*)"|'([^']*)'|([^\s/>]+))"#,
        regex::escape(nama)
    );
    let re = regex::Regex::new(&pola).ok()?;
    let c = re.captures(tag)?;
    let nilai = c.get(1).or_else(|| c.get(2)).or_else(|| c.get(3))?;
    Some(crate::common::decode_entitas(nilai.as_str()))
}

/// `URLSearchParams` (`application/x-www-form-urlencoded`): alfanumerik +
/// `*-._` dibiarkan, spasi `+`, sisanya persen (huruf besar).
fn encode_form(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        let c = b as char;
        if c.is_ascii_alphanumeric() || matches!(c, '*' | '-' | '.' | '_') {
            out.push(c);
        } else if c == ' ' {
            out.push('+');
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    use std::collections::VecDeque;
    use std::sync::{Arc, Mutex};

    /// Satu balasan stub: (body, header).
    type BalasanStub = (String, Vec<(String, String)>);

    #[derive(Debug, Default)]
    struct StubHttp {
        permintaan: Mutex<Vec<oktz_plugin_api::HttpCall>>,
        balasan: Mutex<VecDeque<BalasanStub>>,
    }

    impl oktz_plugin_api::HttpGet for StubHttp {
        fn get(&self, _url: &str) -> Result<oktz_plugin_api::HttpResponse, HttpError> {
            unreachable!("semua lewat HttpCall::call")
        }
        fn post_json(
            &self,
            _url: &str,
            _body: &Value,
        ) -> Result<oktz_plugin_api::HttpResponse, HttpError> {
            unreachable!("scraper ini pakai form")
        }
        fn post_form(
            &self,
            _url: &str,
            _form: &str,
        ) -> Result<oktz_plugin_api::HttpResponse, HttpError> {
            unreachable!("semua lewat HttpCall::call")
        }
        fn post_multipart(
            &self,
            _url: &str,
            _parts: &[(&str, oktz_plugin_api::HttpBody)],
        ) -> Result<oktz_plugin_api::HttpResponse, HttpError> {
            unreachable!("scraper ini tidak multipart")
        }
        fn call(
            &self,
            req: &oktz_plugin_api::HttpCall,
        ) -> Result<oktz_plugin_api::HttpResponse, HttpError> {
            self.permintaan.lock().unwrap().push(req.clone());
            let mut q = self.balasan.lock().unwrap();
            let (body, headers) = if q.len() > 1 {
                q.pop_front().unwrap()
            } else {
                q.front().cloned().unwrap_or_default()
            };
            Ok(oktz_plugin_api::HttpResponse {
                status: 200,
                body: oktz_plugin_api::HttpBody::Bytes {
                    data: Arc::from(body.as_bytes()),
                    mimetype: None,
                },
                headers,
            })
        }
    }

    fn stub(balasan: Vec<(String, Vec<(String, String)>)>) -> StubHttp {
        StubHttp {
            permintaan: Mutex::new(Vec::new()),
            balasan: Mutex::new(balasan.into()),
        }
    }

    #[test]
    fn sukses_mengirim_nonce_dan_mengembalikan_src() {
        let http = stub(vec![
            (
                r#"<html><body><form><input type="hidden" name="_wpnonce" value="abc123"></form></body></html>"#
                    .to_string(),
                vec![(
                    "set-cookie".to_string(),
                    "sess=abc; Path=/; HttpOnly".to_string(),
                )],
            ),
            (
                r#"<html><body><img class="x" id="resultImage" src="https://img/hasil.png"></body></html>"#
                    .to_string(),
                Vec::new(),
            ),
        ]);
        let hasil = unrestrictedai(&http, "Halo dunia!", None).unwrap();
        assert_eq!(hasil, "https://img/hasil.png");

        let p = http.permintaan.lock().unwrap();
        assert_eq!(p.len(), 2);
        assert_eq!(p[0].url, "https://unrestrictedaiimagegenerator.com/");
        assert_eq!(p[1].url, "https://unrestrictedaiimagegenerator.com/");
        // `URLSearchParams`: spasi jadi `+`, `!` jadi `%21`, urutan sama.
        assert_eq!(
            p[1].body,
            Some(oktz_plugin_api::HttpBody::Form(
                "generate_image=true&image_description=Halo+dunia%21&image_style=anime&_wpnonce=abc123"
                    .to_string()
            ))
        );
        let cari = |nama: &str| {
            p[1].headers
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case(nama))
                .map(|(_, v)| v.clone())
        };
        assert_eq!(cari("cookie").unwrap(), "sess=abc; Path=/; HttpOnly");
        assert_eq!(cari("origin").unwrap(), "https://unrestrictedaiimagegenerator.com");
        assert_eq!(cari("content-type").unwrap(), "application/x-www-form-urlencoded");
        // Header GET juga memakai default client yang sama.
        let cari_get = |nama: &str| {
            p[0].headers
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case(nama))
                .map(|(_, v)| v.clone())
        };
        assert_eq!(cari_get("user-agent").unwrap(), UA);
        assert_eq!(cari_get("origin").unwrap(), "https://unrestrictedaiimagegenerator.com");
    }

    #[test]
    fn urutan_atribut_input_tidak_menghalangi_nonce() {
        let http = stub(vec![
            (
                r#"<input value="v2" name="_wpnonce">"#.to_string(),
                Vec::new(),
            ),
            (r#"<img id="resultImage" src="https://i/2">"#.to_string(), Vec::new()),
        ]);
        let hasil = unrestrictedai(&http, "x", Some("anime")).unwrap();
        assert_eq!(hasil, "https://i/2");
        let p = http.permintaan.lock().unwrap();
        assert_eq!(
            p[1].body,
            Some(oktz_plugin_api::HttpBody::Form(
                "generate_image=true&image_description=x&image_style=anime&_wpnonce=v2".to_string()
            ))
        );
    }

    #[test]
    fn prompt_kosong_melempar_pesan_js() {
        let http = stub(Vec::new());
        let err = unrestrictedai(&http, "", None).unwrap_err();
        assert_eq!(err.message, "Prompt is required");
        assert!(http.permintaan.lock().unwrap().is_empty());
    }

    #[test]
    fn style_tidak_dikenal_melempar_daftar_style() {
        let http = stub(Vec::new());
        let err = unrestrictedai(&http, "x", Some("kartun")).unwrap_err();
        assert_eq!(
            err.message,
            "Available styles: photorealistic, digital-art, impressionist, anime, fantasy, sci-fi, vintage"
        );
        assert!(http.permintaan.lock().unwrap().is_empty());
    }

    #[test]
    fn nonce_hilang_melempar_pesan_js() {
        let http = stub(vec![("<html><body></body></html>".to_string(), Vec::new())]);
        let err = unrestrictedai(&http, "x", None).unwrap_err();
        assert_eq!(err.message, "Nonce not found");
        // POST tidak pernah dikirim.
        assert_eq!(http.permintaan.lock().unwrap().len(), 1);
    }

    #[test]
    fn gambar_hilang_melempar_pesan_js() {
        let http = stub(vec![
            (r#"<input name="_wpnonce" value="n">"#.to_string(), Vec::new()),
            (r#"<html><img id="lain" src="x"></html>"#.to_string(), Vec::new()),
        ]);
        let err = unrestrictedai(&http, "x", None).unwrap_err();
        assert_eq!(err.message, "Image not found");
    }
}
