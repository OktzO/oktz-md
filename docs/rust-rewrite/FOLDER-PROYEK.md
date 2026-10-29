# Rencana: sistem Rust jadi folder mandiri (seperti `ourin-md`)

Status: **sebagian dieksekusi** — `rust/` kini mandiri dengan `src/`, `lib/`,
`plugin/`, `assets/`, `data/`, `test/`, `native/`, dan `tests/`. Nama folder
target di bawah (`ourin-rs/`, `crates/`) tidak dipakai. Gate §3 tetap berlaku
untuk pemisahan repo final. Referensi: repo JS tetap ada sebagai **acuan
baca-saja** untuk cek paritas — bukan dua sistem yang jalan bersamaan.

## 1. Bentuk target

```
ourin-rs/                       # nama folder bebas
├── Cargo.toml                  # workspace: members = ["crates/*"]
├── crates/                     # pindahkan isi rust/crates/ apa adanya
│   ├── oktz-bin/               # executable (menggantikan index.js)
│   ├── oktz-wa/  oktz-router/  oktz-core/  oktz-store/
│   ├── oktz-plugin-api/  oktz-plugin-macros/
│   ├── oktz-plugins/           # 828 plugin, folder per kategori
│   └── oktz-scraper/           # 59 scraper
├── data/                       # data/ (JS): husbu, waifu, quizbattle.json, font
├── game-data/                  # src/data/*.json (34 file) — dibaca plugin game
├── assets/                     # audio/ fonts/ image/ kertas/ video/ + font.ttf
├── storage/                    # creds, session/, qr/, profiling/
├── config.example.toml         # menggantikan config.js; kunci & env persis
├── docs/rust-rewrite/          # dokumen yang sekarang ini
└── scripts/                    # skrip bantu (build-env, cek-*, sinkron-*)
```

## 2. Yang harus dipindah, dan cara memastikan paritas

| Asal (JS) | Tujuan (Rust) | Aturan paritas |
|---|---|---|
| `index.js` | `crates/oktz-bin` | urutan boot harus sama: muat creds → store → connect → dispatch event |
| `config.js` | `config.example.toml` + env | **nama env harus sama persis** (`APIKEY_NEOXR`, `APIKEY_CUKI`, `APIKEY_FIREFLY`, `APIKEY_LOGICBELL`…). Plugin yang dulu jatuh ke `""` harus tetap jatuh ke `""` sampai §7 selesai |
| `src/data/*.json` (34) | `game-data/` | file disalin apa adanya; plugin game membacanya lewat helper yang sama (bukan `include_str!` per file — itu mengunci isi saat compile) |
| `data/husbu`, `data/waifu`, `quizbattle.json`, `Roboto_Medium.ttf` | `data/` | butuh kapabilitas `fs`/embedding; sampai itu ada, plugin terkait tetap terblokir, bukan diam-diam read path relatif |
| `assets/**` | `assets/**` | `getAssetBuffer()` JS = baca file + base64; di Rust jadi `MediaSource::Path`/bytes |
| `storage/` | `storage/` | format creds/sesi **harus kompatibel** dengan `whatsapp-rust` (bukan format `baileys`) — ini yang jadi gate pembuka |
| `case/foto.js` | ikut `crates/oktz-plugins` (bukan file) | `main/carifitur` memuatnya lewat `import()` dinamis; di Rust daftar perintah harus dari registry, dan itu perubahan hasil yang harus dicatat |
| `tests/`, `scripts/audit-offline.mjs` | `scripts/` | audit offline JS tetap dipakai sebagai pembanding paritas |

## 3. Gate sebelum pemindahan

Tidak ada pemindahan sebelum semua ini terpenuhi — semuanya terukur, bukan
"rasa sudah cukup":

1. **Kapabilitas core §1–§9 BLOCKED.md selesai** (khususnya §2 `sock.*`, §4
   `fs`, §7 config runtime, §9 entry point kedua) — tanpa ini folder baru
   hanya jadi salinan yang masih cacat.
2. **Seluruh plugin ter-port** atau tercatat terblokir dengan alasan final, dan
   tidak ada stub yang mengembalikan hasil palsu.
3. `cargo test --release` workspace hijau, `cargo clippy --release --all-targets`
   0 peringatan.
4. Smoke produksi nyata: sesi ratchet dimuat, handshake berhasil, 50+ perintah
   dipanggil dari grup uji, tidak ada panic worker.
5. Skrip paritas (`scripts/cek-paritas-plugin.mjs`) hijau untuk seluruh plugin
   yang punya pasangan JS.

## 4. Larangan keras

- **Tidak ada dua sumber kebenaran.** Setelah pemindahan, `ourin-md` JS tidak
  boleh dijalankan lagi hanya sebagai "pembanding"; ia(read-only).
- Tidak mengedit file JS "sedikit saja demi kompatibilitas" — setiap
  beda dicatat di dokumen, bukan di kode.
- Tidak memakai `include_str!`/`include_bytes!` untuk aset/data yang di JS
  dibaca saat runtime: begitu, parity data berubah diam-diam tiap build.
- Pemindahan adalah commit besar tunggal yang bisa dibalik (`git revert`), bukan
  rentang commit yang ditinggal setengah jalan.

## 5. Urutan eksekusi (setelah gate 3–5)

1. `git mv rust ourin-rs/rust` + workspace root baru, build tetap hijau.
2. Pindahkan `data/`, `assets/`, `storage/` sebagai file asli.
3. `config.example.toml` dari `config.js` (kunci demi kunci, dengan test yang
   membaca keduanya dan membandingkan daftar env).
4. Smoke produksi ulang di folder baru.
5. Tulis `HANDOFF`/`PROGRESS` final; tandai `ourin-md` read-only.

## 6. Yang TIDAK ikut dipindah

- `node_modules/`, `rust-spike/` (spike sudah dipromosikan jadi crate produksi).
- `src/tiktok/`, `native/`, skrip khusus Node yang tidak dipakai bot saat runtime.