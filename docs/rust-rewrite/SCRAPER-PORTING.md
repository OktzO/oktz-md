# Briefing agent porting scraper

Baca ini dulu, lalu buka `rust/lib/oktz-scraper/src/dailymotion.rs` — itu
**contoh acuan** yang sudah lulus test dan clippy. Port-mu harus berbentuk
seperti itu.

## Tugas

Port daftar file `src/scraper/*.js` yang kamu terima ke
`rust/lib/oktz-scraper/src/*.rs`.

**Acuan tunggal adalah file JS-nya.** Kalau JS melakukan hal yang aneh atau
keliru, Rust melakukan hal yang sama. Format angka, urutan kunci, teks pesan
error, dan bentuk objek hasil harus sama persis.

## Nama file dan fungsi

- File JS → file Rust **snake_case**: `geminiVision.js` → `gemini_vision.rs`,
  `logic-bell.js` → `logic_bell.rs`, `img2prompt.js` → `img2prompt.rs`,
  `txt2img2.js` → `txt2img2.rs`, `gpt52.js` → `gpt52.rs`.
- Tiap `export` JS → `pub fn` Rust **snake_case**:
  `export { DailymotionDL }` → `pub fn dailymotion_dl`.
- Argumen pertama **selalu** `http: &dyn oktz_plugin_api::HttpGet`, lalu
  argumen fungsi JS dengan urutan yang sama.
- Balikan: `Result<serde_json::Value, oktz_plugin_api::HttpError>`, bentuknya
  = objek yang di-`return` JS. Kunci yang nilainya `undefined` di JS **tidak
  ikut** (itu yang dilakukan `JSON.stringify`), jadi bangun dengan
  `serde_json::Map` dan sisipkan hanya kunci yang ada — lihat
  `dailymotion.rs`.
- Scraper yang JS-nya mengembalikan string/buffer mentah (bukan JSON)
  mengembalikan `Result<String, _>` / `Result<Vec<u8>, _>`; sebutkan di
  laporan.
- Satu file JS yang punya beberapa export → beberapa `pub fn` di file yang
  sama. Fungsi bantu JS (`const helper = ...` di dalam file) jadi `fn`
  privat di file yang sama.

## File yang TIDAK boleh kamu sentuh

- `rust/lib/oktz-scraper/src/lib.rs` — daftar modul milik orkestrator.
- `rust/lib/oktz-scraper/src/common.rs` — milik orkestrator (lihat di
  bawah, sudah ada helper; kalau kurang, tulis privat di filemu lalu
  laporkan).
- File milik agent lain, file JS asli, apa pun di `oktz-plugins/`.
- Jangan menambah dependency di `Cargo.toml`. Yang tersedia:
  `oktz-plugin-api`, `serde_json`, `rand`, `regex`, `md-5`, `sha1`.

## HTTP: satu pintu lewat `HttpCall`

Jangan bikin klien jaringan sendiri. Untuk axios dengan header kustom,
metode selain GET/POST, atau body mentah, pakai:

```rust
use oktz_plugin_api::{HttpCall, HttpMethod};

let res = http.call(
    &HttpCall::new(HttpMethod::Put, "https://x/api")
        .header("x-csrf-token", "abc")      // nama header apa pun
        .header("cookie", "a=1")
        .header("referer", "...")
        .json(&json!({"a": 1})),            // data: {...} → JSON
)?;
```

- `data` string di axios → `.form(isi_string)` (JS yang meng-encode sendiri),
  atau `.raw(bytes, "mimetype")` untuk body biner.
- `form-data` (`form.append("file", buf, { filename })`) →
  `.multipart(vec![MultipartPart::file("file", "foto.jpg", buf), MultipartPart::field("type", "13")])`.
- `validateStatus: () => true` → tambahkan `.terima_semua_status()`; tanpa itu
  status ≥ 400 jadi `Err` (meniru axios yang melempar, dan itu perilaku
  bawaan yang benar untuk sebagian besar scraper).
- Empat method lama (`http.get`, `http.post_json`, `http.post_form`,
  `http.post_multipart`) tetap ada untuk kasus polos tanpa header — tapi
  `post_multipart` menamai part dengan nama field, bukan `filename`, jadi
  jangan dipakai untuk `form-data` yang mengirim berkas.
- Respons: `res.json()` (Option), `res.text()`, `res.bytes()`.
- Timeout global 20 detik di `UreqHttp`; kalau JS menulis `timeout: 120000`
  itu **tidak** ditiru (batas mesin, bukan tampilan) — sebutkan di laporan
  kalau sebenarnya mempengaruhi alur.

## Helper di `common.rs` (pakai ini, jangan menyalin)

- `encode_uri_component(s)` — `encodeURIComponent` JS: spasi `%20`,
  `!'()*-._~` dibiarkan. **Beda** dari query-string form-urlencoded
  (spasi `+`). Salah pilih = URL beda dari JS.
- `truthy(&Value)` — `!!v` JS: `0`, `""`, `null`, `false` itu falsy.
- `parse_int_js(&Value) -> Option<f64>` — `parseInt` JS, `None` = `NaN`.
- `decode_entitas(s)` — entitas HTML yang umum saja. Bukan port lengkap `he`;
  kalau JS lewat `cheerio`/`he.decode` dan halamannya memakai entitas lain,
  itu **beda yang diketahui** dan wajib muncul di laporan.
- `field/teks/teks_atau` — baca `data.a.b` dengan aman.

## Semantik JS yang wajib disalin

- `if (!data.x || data.x.length === 0)` → cek `None` **dan** array kosong.
- `a || b` → jatuhan seperti `||` JS (perhatikan `0` dan `""` juga falsy).
- `data?.a?.b` → `Value::get` berantai, `None` kalau putus.
- `parseInt(b.quality) - parseInt(a.quality)` di `sort` — komparator `NaN`
  diperlakukan `Array#sort` sebagai 0, dan `sort` V8 stabil. Pakai
  `sort_by` + `Ordering::Equal` untuk kasus `NaN`.
- `JSON.stringify` membuang `undefined` — kunci hilang, bukan `null`.
- `throw` → `Err(HttpError)`; `catch` yang mengembalikan objek error →
  `Ok(json!({...}))` seperti JS.
- Kalau JS melempar string/objek yang membawa pesan tampilan, teksnya sama
  persis (huruf per huruf) — plugin yang memformat nanti memakainya.

## Test

Tiap modul wajib punya `#[cfg(test)] mod tests` di file yang sama. Test
**tidak boleh menyentuh jaringan**. Pakai stub `HttpGet` seperti di
`dailymotion.rs`: catat `HttpCall` yang masuk, balas JSON tetap. Uji:

- kasus normal dengan data yang bentuknya seperti respons asli,
- kasus field hilang / array kosong / angka tidak bisa diparse,
- untuk POST: header dan body yang benar-benar dikirim (pakai
  `assert_eq!(p[0].body, Some(HttpBody::Form("...".into())))`),
- teks error persis seperti JS.

Bentuk JSON di test ditulis rapi; kalau perlu raw string, pakai `r#"..."#`.

## Yang TIDAK boleh kamu lakukan

- Menjalankan `cargo` (build/test/check) — target dir dipakai agent lain.
  Orkestrator yang menjalankan.
- Mengklaim test hijau kalau tidak dijalankan. Laporan jujur: "test ditulis,
  belum dijalankan".
- Mengarang teks tampilan atau URL. Kalau tidak ada di JS, tidak ada di Rust.
- Menulis komentar berbahasa Inggris. Komentar bahasa Indonesia, seperlunya,
  sebut nomor baris/makna JS kalau perlu (contoh di `dailymotion.rs`).
- Menulis huruf CJK (kanji/hanzi/hangul/kana) di kode atau komentar. Glyph
  tampilan milik JS (`「」┈⬡㊗`) boleh karena itu keluaran program.
- Menyerah diam-diam: kalau bagian JS tidak bisa diport tanpa kapabilitas
  yang tidak ada di Rust (ffmpeg, canvas, tulis berkas lokal, websocket
  interaktif), **tulis** itu sebagai terblokir di laporan dengan alasan
  persis, jangan stub yang mengembalikan hasil palsu.

## Laporan akhir

Ringkas, tanpa basa-basi:

1. File Rust yang dibuat (jalur lengkap).
2. Signature tiap `pub fn` yang diekspor (dipakai orkestrator untuk menyambung
   plugin): `nama_fungsi(http, arg1: Tipe, ...) -> Result<Value, HttpError>`.
3. Test yang ditulis dan apa yang diuji; tulis jelas kalau belum dijalankan.
4. Beda yang diketahui dari JS / bagian yang terblokir, lengkap dengan alasan.
5. Helper privat yang kamu tulis dan sebaiknya naik ke `common.rs`.
