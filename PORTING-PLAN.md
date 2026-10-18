# Rencana port 17 plugin (gel3-6)

Aturan yang mengikat: `docs/rust-rewrite/PORT-BRIEF.md` + `docs/rust-rewrite/AUDIT.md` §1.
Dilarang: `cargo`, `git commit/add`, sentuh `mod.rs` (yang sudah punya plugin lain), `common.rs`, `Cargo.toml`.
Satu plugin = satu file `rust/crates/oktz-plugins/src/<kategori>/<nama>.rs`.
Catatan di kepala file jika `pub mod` perlu ditambahkan manual ke `mod.rs` (karena mod.rs bukan milik saya).

## Portabel (8) — akan dibuat berkas Rust + tests

| # | JS | Rust | Kategori | Alasan portabel |
|---|----|------|----------|-----------------|
| 1 | plugins/tools/am-data.js | tools/am_data.rs | tools | HTTP POST ke obscura, reply/image saja |
| 2 | plugins/tools/cekxl.js | tools/cekxl.rs | tools | HTTP GET ke xl-ku.my.id, reply teks |
| 3 | plugins/main/donasi.js | main/donasi.rs | main | config.donasi via ctx.config(), fetch QRIS; tombol/button tidak ada di Reply API → teks/gambar saja |
| 4 | plugins/stalker/ffstalk.js | stalker/ffstalk.rs | stalker | HTTP GET ke neoxr freefire stalker, image/teks |
| 5 | plugins/religi/jadwalsholat.js | religi/jadwalsholat.rs | religi | HTTP GET ke myquran.com (pola autosholat.rs), audio adzan + teks |
| 6 | plugins/search/ptvsearch.js | search/ptvsearch.rs | search | scraper `oktz_scraper::tiktoksearch`, video reply |
| 7 | plugins/ai/txt2img.js | ai/txt2img.rs | ai (name: text2img3) | HTTP GET ke neoxr stablediff, image reply |
| 8 | plugins/ai/tofigurine.js | ai/tofigurine.rs | ai (name: tofigure3) | scraper `oktz_scraper::seaart::live3d`, image reply |

## Tidak portabel (9) — hanya dicatat dengan alasan

| # | JS | Alasan |
|---|----|--------|
| A | plugins/owner/bcpc.js | Butuh `sock.store` (chats/messages/contacts), `sock.user.id`, DB `getDatabase`, global mutable `statusBcpc`/`stopBcpc` — tidak ada di `Ctx` |
| B | plugins/owner/ganti-mp3.js | Butuh `updateAssetUrl` (upload ke host aset) dari `src/lib/uploader.js` — tidak tersedia sebagai scraper/API plugin |
| C | plugins/canvas/gura.js | Butuh `uploadImage` dari `src/lib/uploader.js` sebelum memanggil API canvas — tidak tersedia |
| D | plugins/owner/listsewa.js | Butuh `getDatabase()` + `db.db.data.sewa` — tidak ada akses store di `Ctx` |
| E | plugins/tools/ocr.js | Butuh `ocr(buffer)` dari `src/lib/native-loader.js` + `sendToolsPreview` dari `src/lib/context.js` — tidak tersedia |
| F | plugins/owner/sessionstats.js | Butuh `sessionKeyStats()` dari `src/lib/turso-session.js` — tidak tersedia |
| G | plugins/owner/sistemdaftar.js | Butuh `getDatabase()` + `db.setting("registrationRequired")` + `global.registrationSessions` — tidak tersedia |
| H | plugins/owner/startschedule.js | Butuh `startSchedulerByName`, `initSholatScheduler`, `getDatabase` — tidak tersedia |
| I | plugins/tools/tourl.js | Butuh `runUploadFanout` + `UPLOAD_PROVIDERS` (upload fanout) + `sendButton` — tidak tersedia |

## Keluaran akhir
Daftar: berkas Rust yang dibuat, jumlah test per berkas, berkas yang tidak jadi di-port + alasan.
JANGAN menyebut test hijau (tidak pernah menjalankan `cargo`).
