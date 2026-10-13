# Handoff — pindah sesi baru

> Dokumen ini buat sesi berikutnya. **Baca ini dulu**, lalu `docs/rust-rewrite/PROGRESS.md`, lalu lanjut.
> Repo `/home/user/noddjs/ourin-md`, branch `rust-rewrite`. Semua commit sudah aman di git.

---

## 1. Di mana kita

**Fase 0 + Fase 1 DONE (GATE 1 ✅). Fase 2 Task 1–4 DONE. Address sesi ratchet DIPERBAIKI + terbukti. `oktz-router` + batch plugin pertama selesai. BadMac masih terbuka.**

| Komponen | Status | Bukti |
|---|---|---|
| Engine `whatsapp-rust` 0.7 | jalan di stable Rust 1.94 | build + run |
| Impor 514 sesi ratchet | 514 convert, **515 address terbaca wacore** (dari 5) | `loaded=514` |
| Connect + decrypt | authenticated; **decrypt kontak lama BELUM terbukti** | `Successfully authenticated` |
| Tombol WA (S3) | **✅ render + bisa diklik** (verifikasi Oktz) | OKE/GAGAW |
| List native-flow (S4) | **✅ render + bisa diklik** (verifikasi Oktz) | single_select |
| RSS produksi | 13–18 MB (target 60) | smoke 16.8, send 13.1 |
| Data user | 24 file / 874KB byte-identik | `verify-db` |
| **`oktz-router`** | ✅ permission + mode + parsing + gate + group + serializer | — |
| **Plugin** | 🔄 **342 plugin** (dari 828) — **BELUM semua** | `{"plugins":342}` |
| Test + clippy | **3651 hijau**, clippy **0 warning** | `cargo test --release` |

## 2. Decisions yang sudah dikunci (jangan diubah tanpa Oktz)

- Engine = `whatsapp-rust` 0.7 (`default-features=false` tanpa `simd` — stable crash kalau `simd`).
- Node & Rust **bergantian** di panel (bukan berdampingan); JS jadi oracle di Codespaces.
- Interaktif **wajib** dan **sudah terbukti** — tapi polanya `viewOnce(FutureProofMessage)` + `messageContextInfo.deviceListMetadata` v2 + extra node `biz>interactive>native_flow`, **bukan** `ListMessage`/`interactiveButtons` proto (WA buang diam-diam).
- `panic="unwind"` + `catch_unwind` per handler.
- `serde_json` `preserve_order` wajib (LowDB byte-compat).
- Public key onigis 32-byte polos → prefix `0x05` (33 byte) di creds DAN di sesi.
- **WA membuang field yang tak dikenal diam-diam.** Selalu rujuk kode JS yang terbukti jalan, bukan `waproto` yang "harusnya" ada.
- **Paling penting:** kalau ada yang tricky, trace balik ke `src/lib/*.js` / `Onigi/WAProto/WAProto.proto` / `oktz-signal/` (Oktz dev Ketiganya) sebelum nebak dari docs upstream.

## 3. ⚠️ BLOCKER AKTIF — baca sebelum lanjut

**Reviewer independen (sesi lalu) menemukan: sesi ratchet yang 514 itu TULIS ke key yang TIDAK PERNAH DIBACA.**

- `load.rs to_address()` lama: onigis id `2404068921294_1.0` → ditulis `2404068921294.1.0`.
- Tapi wacore mencari `<user>[:device]@<server>` + `.0` (`wacore/src/types/jid.rs:79` `write_signal_address_to`).

**✅ SUDAH DIPERBAIKI + DIBUKTIKAN (commit `47402d6`).** Temuan reviewer benar, tapi kesimpulannya belum lengkap — ada **dua** bug, bukan satu:

1. **Server tidak pernah ditulis.** Loader lama menempelkan `@s.whatsapp.net`. Tapi `mapped_server` (`jid.rs:16`) menerjemahkan `s.whatsapp.net` → **`c.us`**. Jadi `@s.whatsapp.net` tidak dibaca wacore dalam bentuk apa pun.
2. **Suffix `_N` disalahartikan sebagai device.** Onigi `libsignal.js:384`: `signalUser = domainType !== WHATSAPP ? "{user}_{domainType}" : user`. Jadi `_1` = **LID**, bukan device. Field setelah titik terakhir yang device.

Bukti sebelum/sesudah di `oktz-smoke.db`:

| | sebelum | sesudah |
|---|---|---|
| sesi tersimpan | 519 | 515 |
| punya `@server` (dibaca wacore) | **5** | **515** |
| key mati | **514** | 0 |

Runtime wacore sekarang **memang load** sesi: `Session loaded for 247957890850846@lid.0`, dan `247957890850846` memang ada di fixture. Device non-zero juga benar: `86222139600981_1.80` → `86222139600981:80@lid.0`.

**Jawaban §3.1 (server LID vs PN): semuanya LID.** Semua 514 id fixture punya suffix `_1`. Tidak perlu `lid_pn_mapping` — suffix itu sudah membawa jawabannya.

**MASIH TERBUKA — BadMac di smoke.** 27 `UndecryptableMessage`. Ini **bukan** regresi impor:
- Base key lokal yang dilaporkan wacore (`1ed71e3c…`) **tidak ada** di 515 record impor (sudah dicek binary-search per blob) → itu sesi baru dari prekey server.
- `262281069899982` tidak punya sesi di fixture sama sekali.
- Semua 514 record fixture punya ratchet state utuh (bukan kosong): 453 `baseKeyType=1`, 85 `=0`, dan 60 record dicek → semua base key hadir di blob hasil konversi.

Jadi: **importasi terbukti benar secara struktur, tapi "pesan kontak lama berhasil decrypt" BELUM terbukti.** Itu masih target §3.3. Kemungkinan besar sisi server (device lama perlu prekey baru / retry receipt), tapi belum dibuktikan — jangan tulis "selesai" sebelum ada output decrypt nyata.

## 3.5 ⚠️ YANG BELUM SELESAI — plugin 293 dari 828

Ini yang paling penting dibaca sesi berikutnya. **Plugin BELUM semua
direalisasi.** `plugins:342` itu 41%, bukan hampir jadi.

| Class | Sisa | Kenapa belum |
|---|---|---|
| A | **104** | butuh `sock.*`, `downloadContentMessage`, `fs`, store terpisah, `setTimeout` multi-langkah |
| B | 260 | HTTP sudah ada (`ctx.http`), tapi butuh audit endpoint + scraping/canvas |
| C | 44 | canvas/ffmpeg/OCR — perlu toolchain media |
| D | 102 | fitur WA non-portabel (interactiveMessage, newsletter, relayMessage…) |
| E | 5 | upstream mati — keputusan sendiri apakah dibuang |
| F | 20 | platform-risky (jadibot dll) |

Ter-port: 293 dari 828 = **35%** (342 file plugin, karena satu file bisa menambah
lebih dari satu entri manifest lewat alias). Per kategori: cek 47, clan 9,
download 3, fun 26, game 8, group 68, info 4, main 6, owner 54, panel 9, random 7,
religi 1, rpg 59, search 2, sticker 1, store 11, tools 12, user 13, utility 2.
Kategori baru di batch terakhir: `download` dan `sticker`.

### Kapabilitas yang SUDAH ditambahkan

Keempatnya yang membuka 156 port terakhir; dipakai juga oleh port berikutnya.

| JS | Rust | Status |
|---|---|---|
| `m.quoted` | `Ctx.quoted() -> Option<&QuotedMessage>` | ✅ |
| `m.mentionedJid` | `Ctx.mentioned` | ✅ (sebelumnya selalu kosong — bug nyata) |
| `db.getUser(peserta)` | `Participant.data` | ✅ |
| `global.*Sessions` | `ctx.session_get/set/take/update` | ✅ |
| `axios`/`fetch` | `ctx.http` (`UreqHttp`; default `NoHttp`) | ✅ |

### Yang MASIH menahan sisanya

1. **`sock.*`** — mutate grup (promote/demote/ubah nama/deskripsi/pic), hapus
   pesan dari plugin, block/unblock, pin, leave. Butuh surface async di `Ctx`
   atau Reply baru. Ini blocker terbesar class A sekarang.
2. **`downloadContentMessage`** — `m.quoted.download`. Reply sudah punya
   `MediaSource`, jalur unduh byte-nya yang belum.
3. **Store terpisah** — `premium.json`, `sewa.json`, `guilds`, `listResponse`.
   `Mutation` cuma punya `SetUser`/`SetGroup`/`SetSetting`.
4. **`getAllUsers`/`getAllGroups`** — butuh iterasi seluruh store.
5. **Canvas/ffmpeg/OCR** — class C.
6. **Reply satu pesan** — JS bisa `react` → tunggu → kirim. Plugin multi-langkah
   jadi mengirim hasil akhir saja (reaksi tetap lewat `with_reaction`).

Pakai yang sudah ada: `Ctx.roster`, `Ctx.user_data`/`group_data`,
`Ctx.setting_*`, `ctx.sessions`, `ctx.http`, `Ctx.quoted`, `Ctx.mentioned`,
`Reply::with_mutation`, `Reply::with_reaction`, `Reply::MediaSource`.

## 4. Temuan lain reviewer (prioritas di bawah blocker)

- **H3** `pairing.rs` nol call-site. whatsapp-rust saat 401 langsung `enable_auto_reconnect=false` + dispatch `LoggedOut` (node_io.rs:1787). Jadi `classify_logout_401` (grace 60s×2) **tidak terjangkau** —false-401 protection TIDAK aktif. Perlu handler reconnect sebelum lanjut (router jangan asumsi koneksi persisten).
- **H4** `cards.rs` payload demo hardcoded ("OKE"/"test_oke"). 830 plugin butuh builder terparameterisasi — generikkan SEBELUM ada banyak call-site.
- **M5** `pairing.rs reset_pairing_creds`: JS selalu `signalIdentities = []`, Rust hanya kalau sudah array. Test tidak cover (fixture tanpa `signalIdentities`).
- **M6** `is_rate_limit_error`: JS baca 4 sumber (message/output.statusCode/data/data.statusCode), Rust cuma message string.
- **M7** `main.rs` hardcode `/tmp/opencode/*.db` di binary produksi.
- **L8** `creds.rs kp()` dead code **tanpa** fix prefix `0x05` — jebakan reintroduksi bug 33-vs-32-byte.

## 5. Operational (WAJIB baca)

- **Selalu `--release`**: `cargo test` (debug) recompile wacore dari nol (~15 menit sia-sia).
- **Toolchain di `/tmp/opencode/rust`**. `/tmp` pernah di-reset total. Recovery:
  ```bash
  export RUSTUP_HOME=/tmp/opencode/rust/rustup CARGO_HOME=/tmp/opencode/rust/cargo
  curl -sSf https://sh.rustup.rs | sh -s -- -y --no-modify-path --profile minimal --default-toolchain 1.94.0
  rustup component add rustfmt clippy
  cd /tmp/opencode && curl -sSL https://github.com/NixOS/nixpkgs/archive/refs/tags/25.05.tar.gz -o n.tar.gz
  mkdir -p nixpkgs-25.05 && tar xzf n.tar.gz -C nixpkgs-25.05 --strip-components=1
  ```
- `source /tmp/opencode/build_env.sh` set env + fungsi `NS`.
- **PAKAI `NS "..."`, JANGAN `$NS "..."`.** Di harness ini bentuk `$NS "..."`
  menyatukan seluruh argumen jadi satu kata, jadi `nix-shell` gagal
  `cargo ...: command not found` tanpa executor yang jelas.
- Build: `NS "cargo test --release"`. `build_env.sh` sudah export
  `NIX_PATH=nixpkgs=/tmp/opencode/nixpkgs-25.05` — `nix-shell` yang terpasang
  TIDAK punya flag `--nixpkgs`, jangan pakai.
- `/home` cuma ~695MB free — jangan taruh apa pun besar di sana.
- Untuk iterasi cepat: `cargo check --release -p <crate>`.
- **Binary hasil build bernama `oktz`**, bukan `oktz-bin` (crate `oktz-bin`, tapi `[[bin]] name = "oktz"`). Ada di `/tmp/opencode/rust/target/release/oktz`.
- Smoke run: hapus `oktz-smoke.db*` dulu kalau mau bukti bersih, atau sekalian `cp` ke `.bak` biar state "sebelum" bisa dibandingkan.
- **Cara cek isi DB tanpa sqlite3/python** (tidak ada di PATH): `node -e "const {DatabaseSync}=require('node:sqlite'); ..."` — Node 22 punya SQLite bawaan.
- `NS "cargo ..."` diam-diam bisa exit 0 tanpa output test kalau `nix-shell` gagal dapat `cc`. Cek `EXIT=$?` **dan** isi log, jangan percaya exit code saja. Jalankan dari dalam `rust/` (bukan repo root).
- **Subagent paralel:** satu kategori = satu agent, supaya tidak ada dua agent
  menulis `mod.rs` yang sama. `src/lib.rs` dan `src/common.rs` selalu milik
  orkestrator; `mod.rs` di-sync ulang dari isi direktori setelah semua agent
  selesai (bukan oleh agent).
- **WAJIB: crate plugin harus dirujuk di `oktz-bin`.** Rust membuang dependency tak terpakai, jadi statik `linkme`-nya ikut hilang dan `all_plugins()` melaporkan **0 tanpa error** — `oktz boot` tetap exit 0 sambil bilang `plugins: 0`. Cek `use_plugin_crates()` di `oktz-bin/src/main.rs` setiap tambah crate plugin. Test `oktz-bin/tests/plugin_link.rs` menjaganya. Verifikasi cepat: `oktz boot | tail -1` harus `{"plugins":N}` dengan N > 0.

## 6. Langkah berikutnya (urut)

1. **Lanjut port plugin.** Batch 110 file terakhir menurunkan sisa class A ke
   **104**. Triage blocker yang longgar sudah diperketat — `tar` di dalam
   "sekitar" dan `canvas:` sebagai peta emoji pernah salah menandai 27 file
   terblokir; 12 di antaranya sebenarnya portable. Konfirmasi satu file dengan
   grep, jangan pakai regex tebakan. Triage ulang pakai `docs/rust-rewrite/plugin-manifest.json`
   (828 entri, sumber kebenaran class A–F) plus daftar nama yang sudah ada di
   `oktz-plugins/src/*/*.rs`. Kapabilitas yang menahan sisanya ada di §3.5.
   **Jangan** port class C/D sebelum toolchain media ada.
2. **`oktz-router` sudah lengkap** — parsing, permission, mode, gate berstate
   (`oktz-wa/src/pipeline_gate.rs`), group protection (`oktz-router/src/group.rs`,
   `default_gate_chain`), serializer (`oktz-router/src/serialize.rs`). Semua
   sudah terpasang di `Pipeline` dan dipanggil dari inbound pesan sungguhan
   (`pipeline.rs` — `Router::route`, `group::default_gate_chain`). Bukti
  lintasan itu ada di `oktz-bin/tests/live_route.rs`: `Pipeline` asli +
   registry plugin asli, `.benefitpartner` dibalas teks `BOT_NAME` yang hanya
   ada di `info/benefitpartner.rs`, dan `antilink` menghentikan pesan
   sebelum plugin jalan. Yang BELUM ada adalah kapabilitas `sock.*` yang
   dibutuhkan plugin, bukan router.
3. **Tutup sisanya: BadMac / decrypt kontak lama.** Address sudah benar (§3), import terbukti byte-faithful. Yang belum: bukti plaintext nyata dari kontak lama. Semua 27 pesan gagal datang dari **satu grup** (`120363@g.us`) — jalur sender-key. Dan `48400707903630` (tidak ada di fixture) menerima pesan dengan counter 4→5→6, padahal kontak baru harus mulai session baru: itu defect berbeda, arahkan ke `process_prekey`/prekey lookup.
4. H3: pasang reconnect/logout handler sesuai `pairing-state.js` (grace 60s×2) — supaya 401 palsu tak mematikan.
5. H4: generikkan `build_card(body, footer, buttons)` + `build_list(sections)` jadi API berparameter.
4. Reviewer lagi untuk GATE 2.
5. **Fase 3**: `oktz-router` (port `handler.js` 2405 baris) + `oktz-scheduler`. Plan sudah: `docs/superpowers/plans/2026-10-10-fase-2-oktz-wa.md` Task list (Task 3+ sudah berubah — baca plan).
6. Baru Fase 6–8: 830 plugin (60–80 batch, subagent paralel max 3–4).

## 7. Dokumen penting

| Dokumen | Isi |
|---|---|
| `docs/rust-rewrite/PROGRESS.md` | status, keputusan, insiden, recovery |
| `docs/rust-rewrite/ENGINE-DECISION.md` | kenapa whatsapp-rust |
| `docs/rust-rewrite/RISKS.md` | R1–R12 (banyak sudah tertutup — update) |
| `docs/rust-rewrite/CRATES.md` | 53 crate terverifikasi |
| `docs/rust-rewrite/ENDPOINTS.md` | audit endpoint eksternal |
| `docs/rust-rewrite/plugin-manifest.json` | 830 plugin + klasifikasi A–F |
| `docs/superpowers/plans/2026-10-10-fase-1-fondasi.md` | plan Fase 1 (selesai) |
| `docs/superpowers/plans/2026-10-10-fase-2-oktz-wa.md` | plan Fase 2 |

## 8. Skill yang dipakai

`brainstorming` (architectural) → `writing-plans` → (reviewer independen via `task`).
MCP: Context7 (verifikasi crate), Firecrawl (riset engine). Subagent: researcher, reviewer.
Rule: **setiap klaim "terbukti" harus punya output nyata**; kalau tidak, tulis "belum terbukti".