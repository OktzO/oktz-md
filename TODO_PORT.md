# Porting tracker (gel4 1) — SELESAI

Catatan: tidak ada `mod.rs` yang disentuh (semua berisi plugin lain — bukan milik saya). Setiap berkas mendapat catatan `pub mod <nama> perlu ditambah manual ke mod.rs`.

## Dapat di-port (implementasi lengkap + test)
- [x] `group/add.js` → `group/add.rs` (7 test)
- [x] `asupan/asupan.js` → `asupan/asupan.rs` (4 test)
- [x] `tools/invoicemaker.js` → `tools/invoicemaker.rs` (9 test)
- [x] `tools/tovideo.js` → `tools/tovideo.rs` (5 test)
- [x] `asupan/ukhty.js` → `asupan/ukhty.rs` (5 test)

Total: 5 berkas, 30 test.

## Tidak dapat di-port (`//! BLOKIRAN:` header, alasan tertulis, tidak ada `#[plugin]`)
- [x] `sticker/brat.js` → `sticker/brat.rs` (onigis/rich-messages/brat lib, sock.waUploadToServer/relayMessage, Reply::Interactive tanpa tombol)
- [x] `main/carifitur.js` → `main/carifitur.rs` (dynamic import, getCaseCommands, sendButton)
- [x] `owner/colongpp.js` → `owner/colongpp.rs` (JID bot sendiri tidak tersedia untuk set_foto_profil)
- [x] `vps/delvps.js` → `vps/delvps.rs` (config.digitalocean tidak ada di Runtime)
- [x] `canvas/fakedev7.js` → `canvas/fakedev7.rs` (canvas teks/gambar tidak ada di `gambar`)
- [x] `fun/gachawaifu.js` → `fun/gachawaifu.rs` (banyak lib tak ter-port: database, waifu, romance, level, onigis)
- [x] `owner/ganti-namaowner.js` → `owner/ganti-namaowner.rs` (tulis config.js jalur absolut ditolak berkas)
- [x] `owner/hapusdata.js` → `owner/hapusdata.rs` (db.resetToDefaults + interactiveButtons tak tersedia)
- [x] `owner/listsewa.js` → `owner/listsewa.rs` (db.db.data.sewa tidak terjangkau lewat ctx)
- [x] `tools/ocr.js` → `tools/ocr.rs` (OCR native + sendToolsPreview tak tersedia)
- [x] `canvas/profileig.js` → `canvas/profileig.rs` (uploadImage tak tersedia)
- [x] `owner/restart.js` → `owner/restart.rs` (child_process/process.exit tak tersedia)
- [x] `owner/setreply.js` → `owner/setreply.rs` (sendButton/interaktif tak tersedia)
- [x] `owner/srt.js` → `owner/srt.rs` (album message + state global + waUploadToServer)
- [x] `owner/swgc.js` → `owner/swgc.rs` (groupStatusMessage tak tersedia)
- [x] `download/tiktokdl2.js` → `download/tiktokdl2.rs` (cheerio scraping + savett.cc)

Total: 16 berkas BLOKIRAN.

Catatan build: per PORT-BRIEF §3 dilarang menjalankan `cargo` (build/test/check), jadi kompilasi dan tes tidak divalidasi secara langsung di sini — itu tanggung jawab orchestrator. Jangan mengklaim test hijau.
