# Port Plan — gel4-2 (21 plugin JS → Rust) — SELESAI

## File Rust yang dibuat (18 berkas + 1 mod.rs baru)

### Di-port sepenuhnya (dengan test)
| # | File Rust | Kategori | Jumlah test |
|---|-----------|----------|-------------|
| 1 | `owner/hapusgrup.rs` | owner | 2 |
| 2 | `owner/sampah.rs` | owner | 3 |
| 3 | `owner/onlythisgrup.rs` | owner | 2 |
| 4 | `religi/audioquran.rs` | religi | 3 |
| 5 | `fun/confess.rs` | fun | 4 |
| 6 | `search/tiktokfoto.rs` | search | 3 |
| 7 | `tools/tovn.rs` | tools | 2 |

**Total test: 19**

### Blokir (kapabilitas belum ada di Ctx/Sock/Reply)
| # | File Rust | Alasan blokir |
|---|-----------|---------------|
| 8 | `owner/addowner.rs` | `db.data.owner`, `config.pterodactyl`, jadibot-DB, resolusi LID, `getGroupMode` |
| 9 | `owner/disableplugin.rs` | tidak ada `isEnabled` runtime di plugin Rust; `berkas()` tidak bisa ke `plugins/` |
| 10 | `owner/setwelcometype.rs` | `Sock` tidak punya `sendButton`; `getAssetBuffer` tidak ada |
| 11 | `owner/swgcall.rs` | group status/story, `groupFetchAllParticipating`, file-type, state global |
| 12 | `cek/cekfemboy.rs` | scraper `lufemboy` tidak ada di `oktz_scraper` |
| 13 | `canvas/fakeml.rs` | `uploadTo0x0` (unggah ke 0x0.st) tidak tersedia |
| 14 | `canvas/iqcpink.rs` | `@napi-rs/canvas` tidak tersedia |
| 15 | `random/anime.rs` | `relayMessage`, viewOnce, native flow, `prepareWAMessageMedia` |
| 16 | `pushkontak/pushkontak.rs` | state sesi global, resolusi LID, `sock.user.id`, interactive buttons |
| 17 | `game/ulartangga.rs` | `drawBoard` (canvas custom), `sendPreview`, state game global, DB koin/energi |

### Sudah ada di repo / tercover
| # | JS plugin | Rust yang sudah ada | Status |
|---|-----------|--------------------|--------|
| 18 | `owner/ganti-foto2.js` | `owner/ganti-foto2.rs` | Sudah ada (stub, blokir fs/sharp/updateAssetUrl) |
| 19 | `owner/ganti-v8.js` | `owner/ganti-v8.rs` | Sudah ada (stub, blokir fs/sharp/updateAssetUrl) |
| 20 | `sticker/bratextra.js` | `sticker/bratextra.rs` | Sudah ada (stub, blokir canvas) |
| 21 | `stalker/stalkdc.js` | `stalker/discordstalk.rs` (alias `stalkdc`) | Sudah tercover penuh |

### Kategori baru
- `pushkontak/` — folder + `mod.rs` baru (izin pada brief aturan 7)

## Catatan mod.rs (tidak disentuh per brief)
Semua file di kategori yang sudah ada (owner, religi, cek, fun, canvas, random, search, tools, game)
membutuhkan baris `pub mod <nama>;` ditambahkan secara manual ke `mod.rs` masing-masing,
karena `mod.rs` sudah berisi plugin lain dan dilarang menyentuh per brief. Catatan ini
tulis di kepala setiap berkas Rust.

## Aturan yang dipatuhi
- JS produksi adalah acuan mutlak; tidak ada pembetulan pada JS
- File scope eksklusif; tidak menyentuh `Cargo.toml`, `lib.rs`, `common.rs`, `pipeline.rs`,
  `registry.rs`, atau file plugin lain
- Tidak menjalankan `cargo` (build/test/check diurus orchestrator)
- Tidak `git commit`/`git add`/ubah branch
- Komentar dan identifier bahasa Indonesia; tidak ada huruf CJK
- Satu plugin = satu file Rust
- Test di `#[cfg(test)] mod tests` dengan stub `HttpGet` lokal
- Tidak menyebut test hijau (belum dijalankan)
