# PROGRESS — rewrite Rust `oktz-md`

> Dipakai untuk resume lintas konteks/sesi. **Baca file ini duluan** di sesi baru, lalu lanjut dari "Sedang dikerjakan". Update tiap akhir sesi.

---

## Status saat ini

**Fase 0 (Audit & Riset) — selesai, GATE 0 lolos. Fase 1 (GATE 1) ✅. Fase 2 Task 1–4 ✅.
`oktz-router` lengkap dan tersambung ke inbound pesan WA. Plugin 343 dari 828 (41% entri manifest). Sisa class A = 54, semuanya terblokir kapabilitas.
BadMac / decrypt kontak lama masih terbuka.**

| Item | Status |
|---|---|
| Audit faktual codebase | ✅ `AUDIT.md` |
| Keputusan engine | ✅ disetujui → `ENGINE-DECISION.md` |
| Audit endpoint (live) | ✅ `ENDPOINTS.md` |
| Verifikasi crate + versi | ✅ `CRATES.md` |
| Klasifikasi plugin A–F | ✅ `PLUGIN-CLASSIFICATION.md` + `plugin-manifest.json` |
| Risiko | ✅ `RISKS.md` |
| Spec tertulis | ✅ `docs/superpowers/specs/2026-10-10-oktz-md-rust-rewrite-design.md` |
| Plan spike S1–S4 | ✅ `docs/superpowers/plans/2026-10-10-rust-rewrite-spike-s1-s4.md` |
| Toolchain Rust | ✅ terpasang + build **berhasil** |
| **Probe Turso (S1 pre-check)** | ✅ **selesai — 12.639 session_keys** |
| **GATE 0 — approval Oktz** | ✅ **lolos** (engine, F=port-all, RSS delegated) |
| Spike Task 1 (scaffold+baseline) | ✅ binary 2.0MB, RSS start 3.1MB |
| **Spike S1 (impor sesi)** | ✅ **514/514 sesi ratchet** — ⚠️ klaim "0 BadMac" DIBATALKAN, lihat §Fase 2A |
| S1 — pemetaan creds→Device | ✅ connect authenticated |
| S1 — decrypt via sesi impor | ⚠️ "hemz terbaca" = kontak **baru** (prekey), bukan bukti sesi lama |
| **Spike S3 (tombol)** | ✅ **muncul + bisa diklik** |
| **Spike S4 (list)** | ✅ terkirim, render menunggu verifikasi Oktz |
| **Spike S2 (RSS)** | ✅ **26.6 MB** saat connect (target 60) |
| Test suite / clippy | ✅ **3661 test**, clippy 0 warning (`cargo test --release`) |
| Fase 1 (oktz-core/store/plugin-api) | ✅ **GATE 1 LOLOS** (reviewer independen: LAYAK) |
| Fase 2 (oktz-wa) | 🔄 Task 1+2 **selesai** — smoke nyata: 514 sesi + authenticated, RSS **16.8 MB** |
| Fase 2 Task 3 (kirim text/card/list) | ✅ 3/3 terkirim ke owner, RSS ~13 MB |
| Fase 2 Task 4 (pairing logic) | ✅ **12 test**, port 1:1 pairing-state.js |
| **Fix address sesi ratchet** | ✅ `47402d6` — 5 → **515** address terbaca wacore |
| **oktz-router** | ✅ parsing + permission + mode + gate berstate + **group protection** + **serializer** — semua terpasang di `Pipeline`, dibuktikan `oktz-bin/tests/live_route.rs` (Pipeline asli + registry asli) |
| **Kapabilitas Ctx** | ✅ `quoted`, `Participant.data` (store per peserta), `sessions`, `http`, `mentioned` (fix bug) |
| **Plugin** | 🔄 **352 plugin** terdaftar dari 828 = **42%** manifest ter-port — `{"plugins":352}` |
| **on_purge** | ✅ terpasang (`pairing_run::connect_device_guarded_with_purge`) |
| Fase 2 Task 4 (alur pairing ke WA) | ⏳ butuh device baru + interaksi Oktz |
| **BadMac / decrypt kontak lama** | ❌ **BELUM terbukti** — investigasi selesai, root cause eksternal |
| Reviewer Fase 2 + GATE 2 | ⏳ next |

## Fase 2A — Fix address sesi ratchet (`47402d6`)

Blocker dari reviewer sesi lalu **tertutup dan terbukti**. Diagnosis awal tidak lengkap: ada **dua** bug.

1. Loader menempelkan `@s.whatsapp.net`, tapi `mapped_server` (`jid.rs:16`) memetakan `s.whatsapp.net` → **`c.us`**. Alamat itu tidak dibaca wacore dalam bentuk apa pun.
2. Suffix `_N` disalahartikan sebagai device. `libsignal.js:384`: `signalUser = domainType !== WHATSAPP ? "{user}_{domainType}" : user` — jadi `_1` = **LID**.

| | sebelum | sesudah |
|---|---|---|
| sesi punya `@server` (dibaca wacore) | **5** | **515** |
| key mati | **514** | 0 |

Jawaban pertanyaan lama "LID atau PN?": **semua 514 id punya `_1`** → semuanya LID. `lid_pn_mapping` tidak perlu disentuh.

## Fase 2B — Investigasi BadMac (selesai, tapi decrypt belum terbukti)

- methodical debugging + 3 subagent. Yang **DIREFUTASI** (hipotesis yang sempat masuk):
- ❌ Address salah → sudah diperbaiki, wacore runtime jelas load: `Session loaded for 247957890850846@lid.0`.
- ❌ Import lose data → 60 record dicek, rootKey/lastRemoteEphemeralKey/ephemeral pub+priv semua ada di blob hasil konversi.
- ❌ Identity/session selection → semua kontak punya **tepat satu** `closed == -1`; `retention_order` wacore sudah sama dengan aturan JS (`Current` dulu).
- ❌ `pending_pre_key: None` → hanya dibaca saat **encrypt** dan export, tidak menyentuh decrypt.

**Yang CONFIRMED:**
- Semua 27 pesan gagal datang dari **satu grup** (`120363428677398852@g.us`) — jalannya sender-key, bukan session 1:1.
- `lastRemoteEphemeralKey` tersimpan = `05658aea…`, tapi pesan masuk pakai ratchet key `4dfcaa47…` / `57eb5031…` / `b1735e82…` — **tidak ada yang cocok**. Sender sudah ratchet jauh melewati state yang kita simpan.
- `48400707903630` **tidak ada** di fixture, tapi menerima pesan dengan counter **4→5→6**. Kontak yang benar-benar baru seharusnya mulai session baru dan decrypt. Ini poke defect berbeda, dan `No current session` (3x) mengarah ke `process_prekey`/prekey lookup.
- wacore sudah mengirim **24 retry receipt** (0 kegagalan kirim). Sender tidak pernah mengirim balik dalam jendela smoke — itu perilaku yang diharapkan: receiver ack dan server membuang, pemulihan bergantung device pengirim.

**Kesimpulan:** impor terbukti benar secara struktural; "pesan kontak lama berhasil decrypt" **belum terbukti** dan butuh bukti plaintext nyata. Jangan tandai selesai tanpa itu.

## Fase 2 — Task 1–4 (2026-10-10)

Commit: `410c4da` (crate `oktz-wa`) · `fae3375` (smoke) · `646843b` (send) · `2004791` (pairing logic).

**Task 1** — spike dipromosikan jadi crate produksi `oktz-wa` (bukan tulis ulang):
`build_device`, `prepare_store`, `connect_device`, `build_card`/`build_list`/`biz_node`,
`inspect_creds`, `import_session_record`, `load_into_store`.

**Task 2 — smoke nyata:**
```
sesi ratchet dimuat  loaded=514  failed=0
Successfully authenticated with WhatsApp servers!
{"phase":"CONNECTED","rss_kb":15936}  {"phase":"done","rss_kb":16828}
```

**Task 3 — kirim pesan (3/3 ke `6285608953677`):**
```
text  3EB0B66F3F03EFFF3F6B78   card  3EB053563A8662478C948D   list  3EB089E2B0A9630B87439A
```

**Task 4 — pairing logic** `oktz-wa/src/pairing.rs` port 1:1 `src/lib/pairing-state.js`:
`classifyClose`, `classifyLogout401` (grace 60s ×2 retry), marker `pairingPending`,
cooldown `.pairing-cooldown`, `pruneBrokenSessions` (keep 3), `normalizePairingNumber`.
12 test offline. **Belum:** alur pairing ke server WA (butuh device baru + interaksi).

**RSS produksi 13–17 MB** (smoke 16.8, send ~13). Target 60 → headroom besar.

**Gate:** 54 test hijau · clippy `-D warnings` EXIT=0 · fmt bersih · verify-db 24 file identik.

### Yang perlu Oktz (tidak bisa diverifikasi dari sini)
1. **Klik tombol card/list** yang terkirim di Task 3 — log hanya bukti "terkirim", bukan
   "ter-render + bisa diklik" di WA. Tiga pesan: teks, "Kartu dari produksi Rust", "Pilih Mode".
2. **Kirim pesan dari HP** saat `oktz smoke` jalan → bukti decrypt real-time produksi.

### Operasional
- `cargo test` (debug) recompile wacore dari nol — **selalu `--release`**.
- `/tmp` pernah di-reset (lihat insiden di bawah).

## Fase 2 — berjalan (Task 1–2)

Commit: `410c4da` (crate `oktz-wa`) · `fae3375` (smoke `oktz-bin`).

Spike yang sudah terverifikasi **dipromosikan** jadi crate produksi `oktz-wa` (bukan tulis
ulang): `build_device`, `prepare_store` (import device + sesi ratchet), `connect_device`,
`build_card`/`build_list`/`biz_node`, `inspect_creds`, `import_session_record`,
`load_into_store`.

**Smoke nyata (`oktz smoke`) — bukti:**
```
sesi ratchet dimuat  loaded=514  failed=0
Successfully authenticated with WhatsApp servers!
{"phase":"CONNECTED","rss_kb":15936}
{"phase":"done","rss_kb":16828}
```
**RSS produksi 16.8 MB** (spike 26.6 MB; release `opt-level="z"` + `lto="fat"` membantu).
Target 60 MB → headroom 43 MB.

Gate workspace: **42 test hijau**, clippy `-D warnings` EXIT=0, verify-db 24 file identik.

**Operasional (penting):** di mesin ini `cargo test` (debug) **recompile wacore dari nol**
(~15 menit sia-sia). Selalu `cargo test --release` / `cargo clippy --release`.

## Fase 3 — crate `oktz-scraper` + kapabilitas A1–A3 (2026-10-16)

Tujuan fase ini: port `src/scraper/` (59 file JS, 7483 baris) supaya plugin
`download/*` dan `stalker/*` yang selama ini selalu gagal bisa hidup.

Kapabilitas yang ditambahkan (semua di `oktz-plugin-api` + `oktz-wa`):

| Kode | Isi | Status |
|---|---|---|
| A1 | `HttpCall` + `HttpMethod` (header kustom, metode apa pun, body mentah) | selesai |
| A2 | `Reply::Video`/`Audio`/`Document` (`gif_playback`, `ptt`, `file_name`) | selesai |
| A3 | `Ctx::media` + `Ctx::media_fetch` (`m.download()`, `m.quoted.download()`) | selesai |

crate `oktz-scraper`: kontrak `pub fn <nama>(http: &dyn HttpGet, …) -> Result<Value, HttpError>`,
helper bersama di `common.rs` (`truthy`, `parse_int_js`, `encode_uri_component`,
`decode_entitas`), dan `dailymotion.rs` sebagai port contoh acuan. Divisi 14 grup
masing-masing ~500 baris JS ada di `/tmp/opencode/scraper-batches/S01..S14.tsv`
(regenerasi: `node scripts/bagi-batch-scraper.mjs`).

Gelombang pertama: 16 file scraper masuk (feeb, hd, hokinfo, img2img, imglarger,
mediafire, qwen3, reddit, reelsvideo, seaart, sfiledl, spotify, terabox,
tiktoksearch, topmedia, txt2img, wwchar). Verifikasi: **96 test hijau** di
crate scraper, 4231 di `oktz-plugins`, 238 di `oktz-wa`, seluruh workspace hijau.

Rate limit provider: 6 subagent paralel = semua kena HTTP 429. Rombak: 2–3
agent per gelombang.

## Rencana folder mandiri — lihat `docs/rust-rewrite/FOLDER-PROYEK.md`

Sistem Rust akan jadi folder proyek sendiri yang mencerminkan `ourin-md`
(`crates/`, `data/`, `game-data/`, `assets/`, `storage/`, `config.example.toml`,
`docs/`, `scripts/`). Pemindahan baru sah setelah gate: kapabilitas core §1–§9
selesai, seluruh plugin ter-port atau terblokir dengan alasan final, test+clippy
workspace hijau, smoke produksi nyata, dan skrip paritas hijau. Setelah
dipindah, `ourin-md` jadi **acuan baca-saja** — bukan dua sumber kebenaran.

## ⚠️ INSIDEN LINGKUNGAN — 2026-10-10

`/tmp/opencode/` **direset total** saat build `oktz-wa` (Task 1). Hilang: rustup toolchain,
`CARGO_TARGET_DIR` (seluruh hasil compile), nixpkgs-25.05. Yang selamat: **repo & semua
commit** (aman, sudah di-commit).

**Penyebab:** `/tmp` di-overlay 46G; kemungkinan tmpfiles/periodic-cleanup. `/home` hanya
**695MB free** (takut toolchain ~1GB), `/opt` tidak writable. Jadi `/tmp` satu-satunya
lokasi writable yang cukup besar.

**Prosedur recovery (sudah dikuasai, ~4 menit):**
```bash
export RUSTUP_HOME=/tmp/opencode/rust/rustup CARGO_HOME=/tmp/opencode/rust/cargo
curl -sSf https://sh.rustup.rs | sh -s -- -y --no-modify-path --profile minimal --default-toolchain 1.94.0
cd /tmp/opencode && curl -sSL https://github.com/NixOS/nixpkgs/archive/refs/tags/25.05.tar.gz -o n25.tar.gz
mkdir -p nixpkgs-25.05 && tar xzf n25.tar.gz -C nixpkgs-25.05 --strip-components=1
```
Source `/tmp/opencode/build_env.sh` (RUSTUP_HOME/CARGO_HOME/CARGO_TARGET_DIR/PATH + `NS`
helper nix-shell). Untuk iterasi cepat pakai **`cargo check`** — bukan `build`
(build penuh wacore ~25 menit di 2-core).

### Insiden kedua — 2026-10-15

`/tmp` **direset lagi** (sama seperti 2026-10-10): toolchain, `CARGO_TARGET_DIR`,
dan seluruh skrip bantu di `/tmp/opencode/` hilang. Repo + commit utuh. Recovery
kali ini lebih pendek: nix store sudah punya gcc-wrapper dan binutils-wrapper,
jadi tidak perlu unduh nixpkgs.

**Insiden ketiga dan keempat — 2026-10-16.** Dua reset lagi dalam satu hari,
plus server restart dua kali yang memutus subagent. Karena itu seluruh skrip
bantu dipindah ke repo (commit `35abe5c`) supaya tahan reset:

| Tugas | Skrip repo |
|---|---|
| Env build + auto-pasang-ulang toolchain | `scripts/build-env.sh` |
| Paritas metadata plugin Rust↔JS | `scripts/cek-paritas-plugin.mjs` |
| Daftar belum ter-port + bagi batch | `scripts/daftar-batch-plugin.mjs` |
| Scan kebocoran huruf non-Latin | `scripts/scan-nonlatin.mjs` |
| Sinkron `lib.rs` | `scripts/sinkron-lib-plugin.mjs` |
| Sinkron `mod.rs` per kategori | `scripts/sync-plugin-mods.mjs` (sudah lama) |
| Parser metadata bersama | `scripts/plugin-meta.mjs` |

Recovery sekarang satu langkah:

```bash
cd /home/user/noddjs/ourin-md && source scripts/build-env.sh
node scripts/daftar-batch-plugin.mjs   # batch baru ditulis ke /tmp/opencode/batches
```

Satu-satunya yang tidak bisa diselamatkan dari reset `/tmp` adalah hasil compile
(`CARGO_TARGET_DIR`) — build penuh dari nol butuh ~10 menit.

```bash
mkdir -p /tmp/opencode
curl -sSf https://sh.rustup.rs -o /tmp/rustup-init.sh
RUSTUP_HOME=/tmp/opencode/rustup CARGO_HOME=/tmp/opencode/cargo \
  sh /tmp/rustup-init.sh -y --default-toolchain stable --profile minimal --no-modify-path
```

`build_env.sh` sekarang menunjuk `RUSTUP_HOME=/tmp/opencode/rustup`,
`CARGO_HOME=/tmp/opencode/cargo`, `CARGO_TARGET_DIR=/tmp/opencode/target`, dan
menambah `gcc-wrapper-14.3.0` + `binutils-wrapper-2.43.1` dari nix store ke `PATH`
(`ring` dan `libsqlite3-sys` butuh `cc`/`ld`). Rust 1.99.0 stable terpasang.

**Jebakan yang terulang:** `~/.cargo` + `~/.rustup` kehabisan ruang — partisi `/home`
hanya sisa ~600 MB. Selalu pasang ke `/tmp`.

Skrip bantu yang harus dibuat ulang setelah reset:
- `/tmp/opencode/build_env.sh` — env + helper `NS`
- `/tmp/opencode/sync-lib.mjs` — daftar modul `lib.rs`
- `/tmp/opencode/cek-nama.mjs` — paritas metadata plugin Rust↔JS
- `/tmp/opencode/daftar-belum.mjs` + `bikin-batch.mjs` — daftar & batch plugin
- `/tmp/opencode/scan-nonlatin.mjs` — deteksi kebocoran huruf non-Latin

**Dua bug sinkron yang baru ditemukan (keduanya diam-diam merusak build):**
1. `scripts/sync-plugin-mods.mjs` melempar `#[allow(...)]` ke ekor `mod.rs`, sehingga
   `ephoto/mod.rs` jadi `expected item after attributes` — **doctest gagal**.
   Atribut kini dipetakan ke modul tuannya, bukan jadi "baris lain".
2. Lint `special_module_name` pada `pub mod main;` **tidak bisa ditekan** dengan
   `#[allow]` di level item (dibuktikan dengan crate uji terpisah); harus `#![allow]`
   di puncak `lib.rs`.

## Fase 1 — DONE (GATE 1)

Commit: `c67dfb4` (core) · `d28dc04` (store) · `256aa97` (plugin-api) · `53dc2e6` (bin) · `9a43724` (fix blocker) · `ad032bf` (fix re-review).

| Item | Angka |
|---|---|
| `cargo test --workspace` | **33 hijau** (core 8, store 17, plugin-api 8, bin 0) |
| `cargo clippy -- -D warnings` | **0 warning** |
| `cargo fmt --check` | bersih |
| DB round-trip | **24 file / 874KB, semua byte-identik** |

Crate: `oktz-core` (config/env/error/logging) · `oktz-store` (LowDB byte-compat, JsonStore, stub Turso) ·
`oktz-plugin-api` + `oktz-plugin-macros` (macro `#[plugin]`, linkme registry) · `oktz-bin` (boot stub + verify-db).

**Reviewer independen menemukan 2 blocker yang TIDAK terlihat dari test hijau:**
1. `JsonStore` menghapus seluruh isi file yang top-level-nya array (`owner/partner/premium.json`).
2. `PluginMeta` tidak parity dengan JS (`snake_case` vs `camelCase`; 4 default salah) — dan test
   sendiri **mengunci** bug itu. Reviewer re-review: blocker terbukti tertutup (uji independen,
   file byte-identik). 2 temuan lanjutan (serde default, verify-db false-green) juga sudah fix.

## Keputusan yang sudah diambil

1. **Engine = `whatsapp-rust` v0.7.0** (MIT, aktif). Bukan tulis protokol sendiri (oktz-signal bukan engine), bukan sidecar sebagai hasil akhir. Bukti: `Onigi/` sudah bridge ke `whatsapp-rust-bridge`.
2. **Node & Rust bergantian di panel**, bukan berdampingan (1GB RAM). JS jadi oracle di dev/Codespaces; rollback = tarik release JS lama.
3. **Interaktif/buttons/list wajib**, syarat keras Oktz. Spike S3/S4 wajib buktikan tombol bisa diklik di akun uji, bukan cuma compile.
4. **`panic="unwind"`**, bukan `abort` — `catch_unwind` per handler menjaga `handler-error-visibility` (22 kasus).
5. **`serde_json` `preserve_order` wajib** — byte-compat lowdb.
6. **ffmpeg tetap subprocess**, `ourin-native` (OCR) dipakai ulang, `oktz-signal` bukan engine tapi tidak dihapus.
7. **20 plugin kelas F → PORT SEMUA** (Oktz, GATE 0 2026-10-10). Manifest di-update: semua F `oktzDecision: "port-all"`. Tidak ada yang dibuang.
8. **RSS → orchestrator atur sendiri** (Oktz, GATE 0). Kalau S2 ukur >60MB: lazy-load media → feature flag per kategori → kurangi cache dulu; naikkan angka target **hanya sebagai ops terakhir** + catat di `PERF.md`.
9. **Impor sesi — analisis 3 lapis** (lihat RISKS R1). Fail total jauh lebih sempit dari diasumsikan: hanya device identity hilang. Gate S1 diperbaiki: **harus pesan terbaca, bukan sekadar connect OK**, dan dua jalur (full vs creds-only) diuji terpisah. Jalur creds-only yang menjamin no-re-pair.

## Keputusan yang MENUNGGU Oktz (jangan diasumsikan)

_(tidak ada yang blocking saat ini — R1/R3/R5 sudah diputuskan di atas)_

Items non-blocking yang mungkin nanti muncul:
- Ekuivalen gratis untuk `firefly.maiku.my.id` (text2img) — belum terverifikasi, hanya biteks kalau ada plugin yang butuh.
- Mirror PDDikti (kini 401) — belum terverifikasi.

## Keputusan orchestrator (tanpa tanya Oktz)

- RSS < 60MB: kalau tidak tercapai, urut mitigasi S2 = lazy-load media → feature flag per kategori plugin → kurangi cache global. Angka target hanya dinaikkan sebagai opsi terakhir dengan catatan di `PERF.md`.
- API mati (kelas E): port dengan pesan error **yang sama persis** seperti perilaku JS sekarang, tandai ❌-mati di manifest. Cari ekuivalen gratis hanya kalau jelas bernilai.

## Skill / MCP / subagent yang dipakai

- **Skill**: `brainstorming` (path architectural: explore → clarify → approaches → design → spec → review → writing-plans).
- **MCP**: `Context7` (verifikasi crate whatsapp-rust, reqwest, libsql, serde), `Firecrawl` (riset engine WA Rust + audit 100+ endpoint live).
- **Subagent**: researcher engine (3 kandidat, matriks 13 fitur), researcher endpoint (audit live), explore inventaris (angka faktual), inventaris plugin (manifest + klasifikasi), crate-researcher (53 crate terverifikasi). Semua hasil diverifikasi ulang oleh orchestrator (independen) sebelum dipakai.

## Sejarah yang sudah dikerjakan

- 3 subagent riset paralel (engine / endpoint / inventaris).
- 1 subagent inventaris plugin → `plugin-manifest.json` (830 plugin, 0 parse failure) + `PLUGIN-CLASSIFICATION.md`.
- 1 subagent crate-researcher → `CRATES.md` (53 crate, versi dari crates.io API).
- Verifikasi independen: extractor exit 0, JSON parse ulang 830/830, `git diff` source kosong.

**Task 1 spike — ✅ SELESAI (2026-10-10).** `whatsapp-rust 0.7` + `legacy-session-interop` + `sqlite-storage` + `tokio-transport` + `signal` **berjalan di stable Rust 1.94.0 tanpa `simd`**.
- Binary: **2.0 MB**, RSS saat start (engine belum dimuat): **3.128 KB**.
- Nol error. Build took ~20 min di 2-core.
- ⚠️ Angka RSS **belum** jawaban S2 — 3.1MB cuma floor harness. S2 butuh client ter-initialize + connect ke WA.
- ⚠️ `simd` **wajib dimatikan** (`default-features = false`) atau build gagal di stable. Ini juga constrain produksi.

## Temuan spike (2026-10-10)

**GATE S1–S4 TERTUTUP.** Semua risiko terbuka yang di RISKS R1/R2/R3 sudah selesai
dipert estimations dengan bukti run nyata:

- **S1 impor sesi ratchet: 514/514, 0 BadMac.** Data Turso = JSON double-encoded,
  bukan protobuf v6 — asumsi awal RISKS R1 salah dan lebih benign dari dugaan.
  `LegacySessionV1` map 1:1. Field kunci publik onigis (32 byte polos) wajib
  diprefix `0x05` jadi 33 byte — bug yang sama berulang di creds DAN di sesi.
- **Decrypt real-time via sesi impor: "hemz" terbaca.** Membuktikan ratchet
  `oktz-signal` (JS) ↔ `wacore-libsignal` (Rust) kompatibel → tanpa pairing ulang,
  tanpa kehilangan riwayat decrypt.
- **S3 tombol native flow: muncul + bisa diklik** (verifikasi Oktz).
- **S4 list message: terkirim**, render menunggu verifikasi Oktz.
- **RSS 26.6 MB** dengan 514 sesi (target 60). Semua mitigasi R3 tidak perlu.

**Cara unlock blocker:** 3 blocker schema/session (button params, viewOnce
wrapper, public-key prefix) HANYA ketahuan lewat baca langsung `Onigi/WAProto.proto`,
`src/lib/socket.js`, `jpm.js`, `wacore-libsignal/curve.rs` — bukan dari docs.rs.
Pelajaran: **kalau ada yang tricky, trace balik ke kode JS/Onigis/oktz-signal Oktz
sebelum nebak dari dokumentasi upstream.**

## Sisa pekerjaan spike

- Re-run `cargo test` + clippy (helper berubah).
- Verifikasi render list message dari Oktz.

## Reproduction

**S1 ✅ FULL.** Dump 514 record Turso produksi → **538/538 sesi terimpor**, 0 gagal,
test 4/4 hijau. RSS 9.4MB.

**Asumsi RISKS R1 TERBATALKAN.** Kolom `data` Turso **bukan protobuf biner**, tapi JSON
(double-encoded, 2× parse). Risiko "SessionRecord v6 vs decoder v1" tidak berlaku — semua
field kripto tersedia sebagai base64 dan `LegacySessionV1` map 1:1. Risiko yang tersisa
jauh lebih kecil dari diasumsikan.

`baseKeyType=0` pada **85/538 sesi (16%)** → map ke `Local`, bukan dibuang. Dasar:
`base_key_role` hanya dipakai untuk validasi `Remote + pending_pre_key.is_some()`, dan
importer selalu `pending_pre_key=None` → nol dampak decrypt. Detail + sisa gap:
`docs/rust-rewrite/spike/S1.md`.

**Belum terbukti** (butuh Oktz): pemetaan `creds.json`→`DeviceStore`, pesan benar-benar
terbaca, RSS saat connect aktif.

**Probe Turso (S1 pre-check) — ✅ selesai, tanpa Rust.** `@libsql/client` + `.env` → DB hidup. Isi `session_keys` scope `main`:

| category | n | |
|---|--:|---|
| pre-key | 11.072 | jauh lebih banyak dari 10 di disk |
| **session** | **512** | sesi ratchet per kontak — target utama S1 |
| device-list | 406 | |
| lid-mapping | 366 | |
| **sender-key** | **214** + 10 memory | group E2EE |
| identity-key | 48 | |
| tctoken | 11 | |

Tabel: `stores`(8), `session_creds`(2), `session_keys`(12.639), `backup_snapshots`(0).
**Implikasi:** jalur **full viable** — sesi ratchet 512 + sender-key 224 ada nyata, format `BufferJSON` di Turso. Review Focus #2 (hollow session) **tidak terjadi**. Jalur creds-only tetap jaring pengaman.

**Toolchain.** Rust 1.94.0 via rustup, tapi repo ini **lingkungan Nix** dan `/home` disk-nya kritis (hampir penuh). Dua jebakan yang sudah dilewati:
1. `/home` 100% penuh setelah install Rust → toolchain dipindah ke `/tmp/opencode/rust/` (`RUSTUP_HOME`/`CARGO_HOME`/`CARGO_TARGET_DIR`).
2. Build gagal `GLIBC_2.39 not found` — root cause **bukan** linker salah, tapi **artefak cache** `build-script-build` yang ter-link ke glibc 2.38 dari percobaan nixpkgs lama. `cargo clean` + pin nixpkgs 25.05 (glibc 2.40) → jalan.

Env build (semua di `nix-shell`):
```bash
export NIX_PATH="" NIX_CFLAGS_COMPILE="" NIX_CFLAGS_LINK="" NIX_LDFLAGS=""
export RUSTUP_HOME=/tmp/opencode/rust/rustup CARGO_HOME=/tmp/opencode/rust/cargo
export CARGO_TARGET_DIR=/tmp/opencode/rust/target
nix-shell -p stdenv.cc pkg-config -I nixpkgs=/tmp/opencode/nixpkgs-25.05 --run 'cargo build --release'
```

## Reproduction

```bash
# regenerate manifest
node scripts/extract-plugin-manifest.mjs

# spike build (butuh toolchain di /tmp/opencode/rust — lihat "Toolchain" di atas)
source /tmp/opencode/build_env.sh
cd rust-spike
nix-shell -p stdenv.cc pkg-config -I nixpkgs=/tmp/opencode/nixpkgs-25.05 --run 'cargo build --release'
```

## Migrasi plugin massal (2026-10-14)

Alur sekarang: file JS di `plugins/` dipor ke `rust/plugin/src/<kategori>/<nama>.rs` oleh subagent, satu batch 15 file per agent, dengan brief di `AGENT-BRIEF.md`. Orkestrator yang menjalankan cargo, menyinkronkan `*/mod.rs` dan `lib.rs`, serta memperbaiki error integrasi.

Skrip orkestrator (wajib, agent tidak boleh menyentuh):
- `scripts/sync-plugin-mods.mjs` — daftar modul per kategori; blok `//!` harus tetap di paling atas.
- `/tmp/opencode/sync-lib.mjs` — daftar modul crate, termasuk file tunggal seperti `common.rs`.
- `/tmp/opencode/cek-nama.mjs` — bandingkan `name` di `#[plugin]` dengan `name:` di JS, dicetak sebagai nomor kode (tampilan terminal tidak boleh jadi sumber kebenaran).

Pemicu utama test agent yang gagal: assertion memakai delimiter markdown yang berbeda dari JS (backtick vs asterisk), dan penulisan ulang nama plugin dari ingatan. Keduanya dicek ulang ke file JS sebelum menyatakan lulus.

## Perintah penting

- Build/test berat: **GitHub Codespaces (`gh cs ssh`)**, bukan lokal (2-core/4GB).
- Branch: `rust-rewrite`. **Jangan push ke main.**
- JS lama = oracle + fallback. **Jangan hapus** sebelum Fase 9 approved.

## Catatan styling repo

- Bahasa komentar & dokumen: Indonesia (casual-teknis). Identifier kode: Inggris.
- Commit kecil & sering, pesan konvensional (`feat(core): …`, `test(plugin/rpg): …`).
- Tanpa klaim "selesai/berhasil" tanpa bukti (`cargo test`, `clippy -D warnings`, ukuran binary, RSS terukur).

## Produksi inti: config + sock (2026-10-19)

- `rust/config.json` + `Config::load()`/`apply_file()`/`runtime()` di
  `lib/oktz-core/config.rs`; `main.rs` dan `bot.rs` memakai `Config::load()`.
- Mode baru `oktz pair` (code/QR mengikuti config); `pair-code` tanpa argumen
  memakai `session.pairingNumber`.
- `rust/launcher.js` + `package.json` untuk panel egg Node.js.
- `lib/oktz-wa/sock_client.rs`: `impl Sock` nyata di atas klien WhatsApp,
  dipasang lewat `Pipeline::with_sock`; roster produksi dipasang dari
  `ClientSender`; `KickAndReply` mengeksekusi kick.
- Batas yang belum dikerjakan ada di `docs/rust-rewrite/BLOCKED.md` §29.

## Event peserta grup (2026-10-19)

- Langganan `EventKind::GroupUpdate` di `Pipeline::attach`, antrean sendiri,
  dan `Pipeline::tangani_update_grup` sebagai padanan `handler.groupHandler`.
- Welcome/goodbye tipe teks: `lib/oktz-wa/pesan_grup.rs` (teks, placeholder,
  undian) + `lib/oktz-wa/grup_event.rs` (parse event, LID→JID, default grup,
  gerbang tipe).
- Jalur bot ditambahkan: anticulik (keluar), sewa (tolak grup tak terdaftar),
  salam bot bergabung — semuanya dari `src/connection.js:1199-1328`.
- Batas (kartu tipe 2..8, banner promote/demote, badge newsletter, TZ host)
  dicatat di `docs/rust-rewrite/BLOCKED.md` §30.
## Pairing persisten + restore Turso di launcher (2026-10-21)

- `oktz pair`/`pair-code`/`pair-qr` menulis `storage/session/creds.json` dari
  `Device` sqlite (`creds::device_to_creds`) plus marker `<db>.ident`. Sebelumnya
  pairing sukses tetapi `run` tetap gagal "creds.json tidak ada".
- Launcher tanpa argumen: creds sah -> `run`; belum ada sesi + Turso aktif ->
  pulihkan `session_creds`/`session_keys` scope `main` lewat HTTP
  `/v2/pipeline` (Node `fetch`, tanpa dependensi baru), tulis fixture, `run`
  dengan `OKTZ_SESSION_FIXTURE`; tanpa keduanya -> pairing code/QR.
- Sumber setelan: `config.json` (camelCase) lalu `config.js`; env
  `TURSO_URL`/`TURSO_AUTH_TOKEN` menang. Layout panel (`storage/`) dan dev
  (`../storage/`) sama-sama dikenali.
- `AccountJson` membaca `accountSignatureKey`/`accountSignature`/
  `deviceSignature`; creds produksi camelCase sebelumnya terbuang.
- Bukti: `cargo test --release` 6645 lulus / 0 gagal, clippy 0 warning;
  `cek-raw-string`, `scan-nonlatin`, `bandingkan-nama-plugin` rc=0; launcher
  diuji dengan mock Turso (env + config.json) dan fake binary di dua layout.
- Batas: restore Turso baru dibaca saat boot (bot Rust belum menulis mirror);
  QR pairing masih mencetak payload mentah, belum dirender jadi gambar.

## Menu teks + preflight registry (2026-10-21)

- `plugin/src/main/menu.rs`: `.menu` (alias `help`, `bantuan`, `commands`,
  `m`) mengirim teks `buildMenuText` JS — sapaan WIB, info user/bot, uptime,
  dan tautan `.menucat <kategori>`. Kategori panel/pushkontak/store
  disembunyikan seperti mode `md`; kategori `owner` hanya untuk owner.
- `plugin/src/main/menucat.rs`: `.menucat <kategori>` menggantikan stub;
  daftar nama plugin per kategori dari `all_plugins()`.
- `src/main.rs`: mode `oktz plugins` mencetak JSON
  `{"plugins":N,"list":[...]}` tanpa membuka store/sesi.
- `launcher.js`: preflight menjalankan `oktz plugins`; count 0 atau output
  bukan JSON -> keluar rc=3. Binari lama tetap lolos karena mode tak dikenal
  jatuh ke JSON boot yang juga punya kunci `plugins`.
- `plugin/src/lib.rs`: test registry memastikan count > 500, nama unik, dan
  setiap nama bisa ditemukan lewat `find`; test kedua memastikan `menu` dan
  `menucat` terdaftar. Ini menangkap gejala `.menu` -> `NotACommand` kalau
  `pub mod` lupa ditambahkan.
- Audit `node scripts/pasang-modul.mjs`: 33 kategori, 19 stub terlewat, dan
  7 kandidat. Dari 7 itu 4 sengaja belum didaftarkan (`owner/addprem`,
  `owner/arsip`, `owner/bcgc`, `stalker/stalkdc`) dan 3 sticker butuh
  rasterizer teks (`bratcewek`, `bratvid`, `ttp`). Tidak ada plugin selesai
  yang tertinggal.
- Batas: kartu interactive/gambar/riwayat command tidak dikirim (lihat
  komentar `ponytail:` di `menu.rs`).

## Sistem pesan kaya: kartu native-flow + tabel AI (2026-10-21)

- `lib/oktz-plugin-api/kartu.rs` (baru): model kartu native-flow —
  `Kartu` (body, footer, header, tombol, `messageParamsJson`, mention,
  quote, forwarded, teks cadangan), `Tombol` + `AksiTombol` (9 aksi WA),
  `Baris`/`Bagian`/`Daftar` untuk `single_select`, `KartuHeader` (judul +
  subjudul + media). `Params` membungkus `serde_json::Value` supaya
  `Reply` tetap bisa `Eq`.
- `lib/oktz-plugin-api/tabel.rs` (baru): model tabel AI (`Tabel`).
- `Reply::Interactive { kartu }` menggantikan bentuk lama
  (`body`/`footer`/`buttons` polos) dan `Reply::Tabel { tabel }` baru —
  `Reply::InteractiveButton` dihapus, digantikan `Tombol`.
- `lib/oktz-wa/cards.rs`: kartu → proto `interactiveMessage` dibungkus
  `viewOnceMessage` + `deviceListMetadata` v2; header bisa memakai
  `imageMessage` hasil upload; `biz_node()` sekarang meniru `onigis`
  (`biz` + `engagement` + `interactive/native_flow` + `privacy_mode_ts`).
- `lib/oktz-wa/tabel.rs` (baru): tabel → `botForwardedMessage` →
  `richResponseMessage` dengan submessages (teks/tableMetadata/teks) dan
  `contextInfo` ter-forward asal `META_AI` — padanan `sock.sendTable` JS.
- `lib/oktz-wa/pipeline.rs`: `Rendered` punya `card` + `card_header_media`;
  `MessageSender::send_card` (default + `ClientSender`) meng-upload media
  header lalu mengirim kartu. Kalau kirim gagal dan kartu punya
  `fallback_teks`, teks cadangan dikirim (pola `menu.js:1379`).
- Dokumen: `docs/rust-rewrite/INTERACTIVE.md`.

### Plugin yang sekarang memakai sistem baru

- `main/menu` — kartu native-flow varian 2 (default produksi, dipilih lewat
  setting `menuVariant`; 3 = kartu `single_select`), header gambar `foto`,
  body `Hello Brother` + info bot/user + daftar command per kategori,
  tombol `cta_url` Owner Kami + `quick_reply` Dapatkan Script, dan teks
  cadangan kalau kartu gagal.
- `main/menucat` — kartu per kategori (header `foto2`, `limited_time_offer`,
  tombol kembali ke menu/daftar kategori).
- `main/allmenu` (baru) — kartu semua command per kategori.
- `main/ping2`, `main/stats`, `main/totalfitur` — tabel AI
  (`botForwardedMessage` → `richResponseMessage` → `tableMetadata`).
- `main/sc`, `sticker/brat`, `search/nerdfont`, `tools/styleteks` — kartu
  native-flow memakai model `Kartu`/`Tombol` yang baru.
- `owner/setmenu` — bukan lagi stub: menyimpan setting `menuVariant` (v2/v3)
  lewat `Mutation::SetSetting`, jadi owner bisa ganti tampilan menu tanpa
  sentuh database.
