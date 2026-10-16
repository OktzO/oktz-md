//! Helper yang dipakai lebih dari satu scraper.
//!
//! Isi sengaja kecil: semua yang cuma dipakai satu modul tinggal privat di
//! modul itu. Fungsi di sini hanya yang benar-benar berulang.

use serde_json::Value;

/// Ambil `data.<jalur>` bertitik, `None` kalau bukan objek / tidak ada.
/// Padanan `data?.a?.b` di JS tanpa `undefined` bocor ke luar.
pub fn field<'a>(data: &'a Value, jalur: &str) -> Option<&'a Value> {
    let mut cur = data;
    for bagian in jalur.split('.') {
        cur = cur.get(bagian)?;
    }
    Some(cur)
}

/// `data.<jalur>` sebagai `String`; `None` kalau bukan string.
pub fn teks<'a>(data: &'a Value, jalur: &str) -> Option<&'a str> {
    field(data, jalur).and_then(Value::as_str)
}

/// `data.<jalur> || bawaan` — string atau jatuhan, seperti `||` di JS.
pub fn teks_atau(data: &Value, jalur: &str, bawaan: &str) -> String {
    teks(data, jalur).unwrap_or(bawaan).to_string()
}

/// `!!v` di JS — `undefined`, `null`, `false`, `0`, `""`, `NaN` itu falsy.
///
/// Dipakai untuk `if (m.videoAvailable)` dan `if (!data.status)`; di Rust
/// tidak ada coercition, jadi harus satu pintu yang sama untuk semua scraper.
pub fn truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().map(|f| f != 0.0).unwrap_or(false),
        Value::String(s) => !s.is_empty(),
        Value::Array(_) | Value::Object(_) => true,
    }
}

/// `parseInt(x)` JS: angka dipotong ke bawah, string diparse sampai karakter
/// non-digit pertama, spasi di depan diabaikan. `None` = `NaN`.
///
/// Dipakai sortir `parseInt(b.quality) - parseInt(a.quality)` — di JS hasil
/// `NaN` diperlakukan sebagai 0 oleh `Array#sort`, dan itu ikut dipertahankan
/// pemanggil (lihat `dailymotion.rs`).
pub fn parse_int_js(v: &Value) -> Option<f64> {
    match v {
        Value::Number(n) => n.as_f64(),
        Value::Bool(_) | Value::Null => None,
        Value::String(s) => {
            let s = s.trim_start();
            let mut akhir = 0;
            let bytes = s.as_bytes();
            let mulai = match bytes.first() {
                Some(b'+') | Some(b'-') => {
                    akhir = 1;
                    0
                }
                _ => 0,
            };
            while akhir < s.len() && bytes[akhir].is_ascii_digit() {
                akhir += 1;
            }
            if akhir == mulai {
                return None;
            }
            s[..akhir].parse::<f64>().ok()
        }
        Value::Array(_) | Value::Object(_) => None,
    }
}

/// `encodeURIComponent` JS — beda dari form-urlencoded: spasi jadi `%20`, dan
/// `!'()*-._~` dibiarkan apa adanya (URLSearchParams mengubah spasi jadi `+`,
/// jadi keduanya tidak boleh dicampur).
pub fn encode_uri_component(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        let c = b as char;
        if c.is_ascii_alphanumeric()
            || matches!(c, '-' | '_' | '.' | '!' | '~' | '*' | '\'' | '(' | ')')
        {
            out.push(c);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// Decode entitas HTML yang benar-benar muncul di halaman situs target.
///
/// Bukan port lengkap `he.decode`: yang di luar daftar ini dibiarkan apa
/// adanya, dan itu memang bedanya dengan JS — dicatat di laporan, bukan
/// disembunyikan.
pub fn decode_entitas(teks: &str) -> String {
    let mut out = String::with_capacity(teks.len());
    let mut sisa = teks;
    while let Some(pos) = sisa.find('&') {
        out.push_str(&sisa[..pos]);
        let ekor = &sisa[pos..];
        let Some(akhir) = ekor.find(';') else {
            out.push_str(ekor);
            return out;
        };
        // Entitas yang sah selalu pendek. `&` liar di teks (mis. query string
        // tanpa titik koma) tidak boleh menelan paruh halaman.
        if akhir > 12 {
            out.push('&');
            sisa = &ekor[1..];
            continue;
        }
        let kandidat = &ekor[..=akhir];
        match entitas(kandidat) {
            Some(ganti) => out.push_str(&ganti),
            None => out.push_str(kandidat),
        }
        sisa = &ekor[akhir + 1..];
    }
    out.push_str(sisa);
    out
}

/// Nama entitas jadi teksnya. Angka (`&#39;`, `&#x27;`) dihitung, dan poin
/// di luar Unicode dibiarkan apa adanya — sama seperti `he`.
fn entitas(kode: &str) -> Option<String> {
    let ganti = match kode {
        "&amp;" => "&",
        "&lt;" => "<",
        "&gt;" => ">",
        "&quot;" => "\"",
        "&#39;" | "&apos;" | "&#x27;" => "'",
        "&nbsp;" => " ",
        "&#x2F;" | "&#47;" => "/",
        "&hellip;" => "\u{2026}",
        "&mdash;" => "\u{2014}",
        "&ndash;" => "\u{2013}",
        _ => {
            if let Some(sisa) = kode.strip_prefix("&#x").and_then(|s| s.strip_suffix(';')) {
                return u32::from_str_radix(sisa, 16)
                    .ok()
                    .and_then(char::from_u32)
                    .map(|c| c.to_string());
            }
            if let Some(sisa) = kode.strip_prefix("&#").and_then(|s| s.strip_suffix(';')) {
                return sisa
                    .parse::<u32>()
                    .ok()
                    .and_then(char::from_u32)
                    .map(|c| c.to_string());
            }
            return None;
        }
    };
    Some(ganti.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn field_dan_teks_mengikuti_jalur_titik() {
        let v = json!({"a": {"b": {"c": "x"}}, "angka": 3});
        assert_eq!(teks(&v, "a.b.c"), Some("x"));
        assert_eq!(teks(&v, "a.b"), None, "bukan string");
        assert_eq!(field(&v, "angka").and_then(Value::as_i64), Some(3));
        assert_eq!(teks(&v, "a.tidak.ada"), None);
        assert_eq!(teks_atau(&v, "hilang", "kosong"), "kosong");
    }

    #[test]
    fn decode_entitas_mengurai_bentuk_yang_dipakai_situs() {
        assert_eq!(decode_entitas("a &amp; b"), "a & b");
        assert_eq!(decode_entitas("&lt;b&gt;tebal&lt;/b&gt;"), "<b>tebal</b>");
        assert_eq!(decode_entitas("rahasia &#39;x&#39;"), "rahasia 'x'");
        assert_eq!(decode_entitas("&#x2F;path"), "/path");
        // Titik koma hilang: entitas liar dibiarkan utuh, bukan ditelan.
        assert_eq!(
            decode_entitas("nilai &amp walau aneh"),
            "nilai &amp walau aneh"
        );
        assert_eq!(
            decode_entitas("url?a=1&b=2 panjang sekali"),
            "url?a=1&b=2 panjang sekali"
        );
    }

    #[test]
    fn truthy_meniru_konversi_boolean_js() {
        assert!(truthy(&json!(1)));
        assert!(truthy(&json!("x")));
        assert!(truthy(&json!([])));
        assert!(!truthy(&json!(0)));
        assert!(!truthy(&json!("")));
        assert!(!truthy(&json!(null)));
        assert!(!truthy(&json!(false)));
    }

    #[test]
    fn parse_int_js_mengikuti_aturan_parseint() {
        assert_eq!(parse_int_js(&json!("720p")), Some(720.0));
        assert_eq!(parse_int_js(&json!(" 1080")), Some(1080.0));
        assert_eq!(parse_int_js(&json!(3.9)), Some(3.9), "angka tidak dipotong");
        assert_eq!(parse_int_js(&json!("abc")), None);
        assert_eq!(parse_int_js(&json!(null)), None);
        assert_eq!(parse_int_js(&json!("-12px")), Some(-12.0));
    }

    #[test]
    fn encode_uri_component_spasi_jadi_persen20() {
        assert_eq!(encode_uri_component("a b"), "a%20b");
        assert_eq!(encode_uri_component("https://x/y?a=1&b=2"), "https%3A%2F%2Fx%2Fy%3Fa%3D1%26b%3D2");
        // Tanda yang DIBIARKAN `encodeURIComponent` — salah satu ini lolos
        // pernah bikin URL beda dari JS.
        assert_eq!(encode_uri_component("!'()*-._~"), "!'()*-._~");
    }
}
