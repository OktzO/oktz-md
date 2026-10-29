# Port Progress — gel4 5 (21 plugins) — SELESAI

## Berkas Rust yang dibuat (3 plugin diporkan)

| # | JS | Rust | Kategori | Test |
|---|----|------|----------|------|
| 1 | plugins/tools/emojitoimage.js | src/tools/emojitoimage.rs | tools | 7 |
| 2 | plugins/tools/hdvid.js | src/tools/hdvid.rs | tools | 3 |
| 3 | plugins/fun/cekkhodam.js | src/fun/cekkhodam.rs | fun | 4 |

## Berkas yang tidak jadi di-port (17) + alasan

| # | JS | Alasan blokir |
|---|----|---------------|
| 4 | plugins/store/addproduk.js | `uploadImage()` (uploader CDN) tidak ada di API plugin |
| 5 | plugins/owner/backupsc.js | `archiver` zip + fs seluruh cwd + `contextInfo` newsletter tidak ada |
| 6 | plugins/sticker/bratvid.js | `brat-canvas/video` tidak ada padanan Rust |
| 7 | plugins/owner/ganti-mp3.js | `updateAssetUrl()` uploader tidak ada di API plugin |
| 8 | plugins/owner/gantiscraper.js | `import()` JS dinamis + hot reload modul tidak ada di Rust |
| 9 | plugins/panel/createserver.js | `config.pterodactyl` tidak ada di `Runtime`; interactive message terbatas |
| 10 | plugins/main/menucat.js | `getCommandsByCategory`/cases + interactive tombol tidak ada di `Reply` |
| 11 | plugins/search/playch.js | `yt-search` + `src/lib/resolve.js` tidak ada di API plugin |
| 12 | plugins/nsfw/nsfw.js | `src/data/nsfw` JSON di luar akar aset; interactive tombol; `m.isPremium` |
| 13 | plugins/main/sc.js | interactive `cta_url` tombol + `sock.relayMessage` tidak ada di `Reply` |
| 14 | plugins/sticker/smeme-animated.js | `uploadImage()` uploader tidak ada di API plugin |
| 15 | plugins/sticker/stickerly.js | `sendStickerPack` (banyak stiker) + `db.getUser` store tidak ada |
| 16 | plugins/main/system.js | `os.*` / `child_process.exec` / `messageTimestamp` tidak ada di `Ctx` |
| 17 | plugins/ai/tofigurev2.js | `uploadImage()` + `applyImageEffect()` effect-api tidak ada |
| 18 | plugins/sticker/ttp.js | `@napi-rs/canvas` rendering teks tidak ada di `gambar` |
| 19 | plugins/game/werewolf.js | `global.werewolfGames` state + `sock.sendPreview` + PM ke player tidak ada |

## Kategori baru
- `src/jpm/mod.rs` + `src/jpm/jpm.rs` (kategori `jpm` belum ada; dibuat baru, rule 7)

## Aturan yang ditaati
- Tidak ada `Cargo.toml`/`lib.rs`/`common.rs`/`registry.rs`/mod.rs yang disentuh (kecuali `jpm/mod.rs` baru).
- Tidak dijalankan `cargo` (build/test/orchestrator).
- Tidak ada `git commit`/`add`/branch.
- Komentar + identifier bahasa Indonesia; tidak ada huruf CJK.
- Satu plugin = satu file Rust di `src/<kategori>/`.
- `pub mod` ditambahkan manual ke `mod.rs` masing-masing (catatan di kepala file).
- `#[plugin(...)]` + `pub fn nama(ctx: Ctx) -> Reply`; test di `#[cfg(test)] mod tests` dengan stub `HttpGet`.

## Catatan
- Tidak ada klaim test hijau (belum pernah dijalankan `cargo`).
