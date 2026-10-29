# Brief port plugin JS ke Rust

Repo: `/home/user/noddjs/ourin-md` (cwd kerja kamu sudah di repo ini).

Tugasmu: port INDAH plugin dari JS ke Rust. Daftar file ada di bagian bawah
pesan ini. Setiap plugin hanya boleh kamu sentuh file Rust-nya sendiri.

## Aturan keras

1. **JS produksi adalah acuan mutlak.** Kalau Rust dan JS berbeda, JS yang
   benar — termasuk saat JS keliru atau aneh. Jangan membetulkan JS.
   Format angka jangan diseragamkan.
2. **File scope eksklusif.** Hanya boleh membuat/mengubah file Rust untuk
   plugin yang kamu.task. Dilarang menyentuh `Cargo.toml`, `lib.rs`,
   `common.rs`, `pipeline.rs`, `registry.rs`, atau file plugin lain. Kalau
   butuh helper bersama, salin ke file milikmu dan tulis di kepala file bahwa
   itu duplikat.
3. **Dilarang menjalankan `cargo`.** Bukan `build`, bukan `test`, bukan
   `check`. Orkestrator yang menjalankan build dan test. Kamu tidak boleh
   mengklaim test hijau karena tidak pernah menjalankannya.
4. **Dilarang `git commit`, `git add`, atau mengubah branch.**
5. Komentar, doc comment, dan nama identifier bahasa Indonesia. Huruf CJK
   dilarang kecuali glyph yang memang ada di teks tampilan JS.
6. Satu plugin = satu file Rust di `rust/plugin/src/<kategori>/`.
   Daftarkan modulnya di `rust/plugin/src/<kategori>/mod.rs`
   HANYA kalau mod.rs itu milikmu dan belum memuat pluginmu — kalau mod.rs
   sudah punya plugin lain yang bukan milikmu, jangan sentuh; tulis catatan
   di kepala file bahwa baris `pub mod` perlu ditambah manual.
7. Panjang file: jangan mkdir kategori baru kecuali kategori itu belum ada
   sama sekali. Kalau belum ada, buat foldernya dan `mod.rs`-nya.

## Kontrak API

Baca dulu `rust/lib/oktz-plugin-api/registry.rs` dan
`rust/plugin/src/common.rs` sebelum menulis apa pun.

- `#[plugin(name = "...", alias = [...], category = "...", description = "...",
  usage = "...", example = "...", cooldown = N, energi = N, is_group = bool,
  is_private = bool, is_owner = bool, is_premium = bool, is_admin = bool,
  is_bot_admin = bool)]` lalu `pub fn namaplot(ctx: Ctx) -> Reply`.
- `Ctx` memberi: `text` (setelah command), `args`, `sender`, `chat`, `mentioned`,
  `command`, `prefix`, `quoted`, `roster`, `user_data`, `group_data`,
  `settings`, `sessions`, `http`, `media`, `media_fetch`, `is_group`,
  `is_owner`, `uptime_secs`, `config`. Aksesor: `ctx.mention_or_sender()`,
  `ctx.quoted()`, `ctx.setting(k)`, `ctx.session_get(k)` / `session_set`,
  `ctx.user_field(..)` / `ctx.user_list(..)`, `ctx.download_media()` /
  `ctx.download_quoted_media()`, `ctx.config()` untuk `config.js`.
- `Reply`: `Reply::text(..)`, `Reply::text_mention(..)`, `Reply::image(..)`,
  `Reply::video(..)`, `Reply::Reply::audio(..)`, `Reply::sticker(..)`,
  `Reply::document(..)`, `Reply::sequence(vec![..])`, `Reply::None`. Semua
  punya `.with_reaction("..")` dan `.mutating(vec![Mutation])`.
- `HttpGet` untuk jaringan. Scraper yang sudah ada dipanggil dari crate
  `oktz_scraper` (`oktz_scraper::<modul>::<fungsi>(&ctx.http as ..., arg)`) —
  JANGAN menyalin logika scraper ke plugin.
- Helper umum: `crate::common::{error_template, js_truthy, js_truthy_value,
  encode_form, base64_encode, base64_deskep, teks_js, uuid_v4, trim_js,
  basename, mime_dari_nama, interp}` dan
  `crate::tools::phisingataubukan::encode_uri_component`.

## Kapabilitas yang sudah tersedia

Sebelum menyatakan sebuah plugin "terblokir", periksa daftar ini. Yang ada di
sini BUKAN alasan untuk melewatkan plugin.

- `fs` Node -> `ctx.berkas()` (`oktz_plugin_api::berkas::Berkas`):
  `ada`, `baca`, `baca_teks`, `tulis`, `tulis_teks`, `tambah_teks`, `daftar`,
  `hapus`, `buat_folder`, `ukuran`, `salin`. Jalurnya relatif ke akar aset.
- `child_process` + ffmpeg -> `oktz_plugin_api::ffmpeg`:
  `jalankan`, `jalankan_dengan_masukan`, `jalankan_ke_berkas`.
- `sharp` -> `oktz_plugin_api::gambar`:
  `sharp(bytes)?`, lalu `.ubah_ukuran`, `.pas_di_dalam`, `.tanpa_memperbesar`,
  `.putar_otomatis`, `.pastikan_alfa`, `.skala_abu`, lalu `.ke_jpeg(kualitas)`,
  `.ke_png()`, `.ke_webp(kualitas)`, `.metadata()`.
- `sock.onWhatsApp` / `profilePictureUrl` / `groupMetadata` /
  `groupParticipantsUpdate` / `groupLeave` / `groupInviteCode` /
  `groupGetInviteInfo` / `groupCreate` / `groupAcceptInvite` /
  `updateProfilePicture` / `readMessages` / `sendPresenceUpdate` /
  `fetchStatus` / `getName` / `getBusinessProfile` -> `ctx.sock()`
  (`oktz_plugin_api::sock::Sock`).
- `answerHandler` -> `#[answer_handler(...)]` di `oktz_plugin_api`, dan
  `oktz_plugin_api::ANSWER_HANDLERS`.
- `cheerio` -> parser mini per berkas, seperti konvensi repo ini.
- `AIRich` (`src/lib/builder.js`) BELUM ada. Plugin yang benar-benar butuh
  `AIRich` untuk menyusun tampilan: tulis kepala berkas `//! BLOKIRAN:` yang
  menyebut bagian mana yang butuh `AIRich`, jangan mengarang tampilan lain.

## Gaya penulisan

- `ponytail:` untuk penyimpangan yang disengaja, menyebut nama jalan yang
  akan dipakai kalau penyimpangan ditutup.
- Test di `#[cfg(test)] mod tests` pada file yang sama. Stub `HttpGet` lokal
  yang mencatat `HttpCall` dan membalas body tetap — contoh ada di
  `rust/plugin/src/stalker/ytstalk.rs`.
- Test harus mengunci perilaku JS yang mudah salah: operator `||` vs `??`,
  `parseInt` vs `Number`, urutan operasi, batas digit `toFixed`, dan apa yang
  terjadi pada body bukan JSON.
- Jangan pakai `unwrap()` pada `Option` dari `serde_json`; tuliskan nilai
  bawaan yang sama dengan JS.

## Keluaran

Di akhir, balas dengan daftar: file Rust yang dibuat, jumlah test per file,
dan APAKAH ada yang tidak bisa dikerjakan (dengan alasannya). Jangan jangan
menyebut test hijau.