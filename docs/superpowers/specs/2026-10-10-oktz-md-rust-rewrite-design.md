# Spec — Rewrite `oktz-md` Node.js → Rust Native

Date: **2026-10-10** · Branch: `rust-rewrite` · Status: **menunggu review Oktz** (GATE 0)
Dokumen terkait: [`docs/rust-rewrite/AUDIT.md`](../../rust-rewrite/AUDIT.md) · [`ENGINE-DECISION.md`](../../rust-rewrite/ENGINE-DECISION.md) · [`ENDPOINTS.md`](../../rust-rewrite/ENDPOINTS.md) · [`CRATES.md`](../../rust-rewrite/CRATES.md) · [`RISKS.md`](../../rust-rewrite/RISKS.md) · [`PLUGIN-CLASSIFICATION.md`](../../rust-rewrite/PLUGIN-CLASSIFICATION.md) · [`PROGRESS.md`](../../rust-rewrite/PROGRESS.md)

---

## 1. Tujuan & batasan

**Tujuan.** Tulis ulang seluruh bot WhatsApp `oktz-md` (Node.js ESM, 830 plugin, 143.186 baris JS) menjadi satu binary Rust `oktz-md`, tanpa Node.js saat runtime — karena target produksi (Pterodactyl x86_64, RAM 1GB total, 1 core, disk 3GB) perlu RSS kecil.

**Batasan keras.**
- Semua keputusan gratis / open-source / free-tier.
- Tidak ada klaim "selesai" tanpa bukti: output `cargo test`, `cargo clippy -- -D warnings`, ukuran binary, RSS terukur.
- **Paritas perilaku**, bukan sekadar kompilasi: command sama → output/efek sama (teks, media, cooldown, permission, energi).
- JS lama **tidak** dihapus sebelum Fase 9 approved. JS = oracle + fallback.
- Rollback = tarik release JS lama (Node & Rust **bergantian** di panel, tidak berdampingan).
- Widget interaktif/buttons/list **wajib** berfungsi (syarat eksplisit Oktz).

**Out of scope.** Menulis ulang `oktz-signal` jadi engine (ditolak, lihat ENGINE-DECISION). Menyingkirkan `oktz-signal` dari repo (tetap dipakai `Onigi`/JS). ffmpeg in-process (ditolak — sudah terbukti baik sebagai subprocess).

## 2. Arsitektur

Cargo workspace di `rust/`:

```
rust/
├─ Cargo.toml                 # profile release: opt-level="z"/3, lto="fat",
│                             #   codegen-units=1, strip=true, panic="unwind"
├─ crates/
│  ├─ oktz-core/              # config, env, error, tracing, time, cache (moka),
│  │                          #   rate-limit (governor), circuit-breaker
│  ├─ oktz-wa/                # wrapper whatsapp-rust: connect, pairing, reconnect,
│  │                          #   send/receive, media, group, LID, builder interaktif
│  ├─ oktz-store/             # JSON byte-compat (lowdb) + Turso/libsql, session,
│  │                          #   backup, pruner
│  ├─ oktz-plugin-api/        # trait Plugin, PluginMeta, Context, Message, permission,
│  │                          #   cooldown, energi
│  ├─ oktz-plugin-macros/     # proc-macro #[plugin(...)] + registry (linkme)
│  ├─ oktz-router/            # handler.js → router + middleware + group-protection
│  ├─ oktz-scheduler/         # cron, sholat, notif, auto-backup, jpm, keepalive
│  ├─ oktz-media/             # ffmpeg (subprocess), image, canvas, OCR (ourin-native)
│  ├─ oktz-scrapers/          # scraper + capabilities, reqwest + cookie store
│  ├─ oktz-plugins/           # 830 plugin, 1 modul per kategori (feature flag)
│  └─ oktz-bin/               # main.rs: init → load → start; graceful shutdown
├─ tests/                     # integrasi + paritas + fixtures/
└─ xtask/                     # audit-plugin-parity, gen-manifest, secret-guard
```

**Prinsip anti-scaffolding (YAGNI):** crate di atas **hanya** yang memang dipakai. Kalau sebuah crate butuh <2 modul, jangan. release profile diukur (opt-level "z" vs 3) — pilih yang menang, jangan asumsi.

**Ren Nonprofit yang sudah ditolak** (jangan tulis ulang tanpa alasan baru):
- `oktz-signal` sebagai engine — tidak punya Noise/protobuf/SenderKey/group-E2EE.
- libwebp/ffmpeg in-process — sudah terbukti out-of-process RSS flat.
- `prost` manual — `waproto` sudah handle protobuf WA.

## 3. Kontrak plugin

Satu trait, dipanggil registry runtime. Meniru `pluginConfig` JS:

```rust
#[plugin(name = "menu", alias = ["m", "help"], category = Main,
         is_group, is_private, cooldown = 5, energi = 0)]
async fn menu(m: &Message, ctx: &Ctx) -> Result<()>;
```

- Field yang **benar-benar dipakai** dataset (dari audit): `name, category, description, usage, example, alias[], cooldown, energi, isEnabled, isOwner, isPremium, isGroup, isPrivate, isAdmin, isBotAdmin, skipRegistration, isBanned, noPrefix[], customTrigger`. Field `limit` ada di default tapi 0 plugin override — jangan diprioritaskan.
- **Hot-toggle `isEnabled`**: di Rust native, hot-reload kode mustahil. Pengganti: flag di registry, dibaca dari `db.setting`, berubah **tanpa restart** (menggantikan `owner/disableplugin.js` yang rewrite file sumber). Ganti kode = restart cepat.
- **Kelas F** (20 plugin) **tidak** di-port sampai Oktz memutuskan.

## 4. Data & kompatibilitas

- **DB JSON byte-compat:** `serde_json` dengan `preserve_order` (wajib — tanpa itu urutan key berubah tiap serialize, `database.js` merge default di atas data). Strategi: struct typed untuk field yang dikenal + `serde_json::Value` passthrough untuk field tak dikenal. **Test:** baca `database/main/users.json` asli (604 user × 35 field) → tulis → `git diff` kosong.
- **Turso:** `libsql` crate (pakai parity dengan `turso.js`). Plan B `rusqlite`.
- **Session:** importer spike S1 (lihat ENGINE-DECISION §3). Format creds + sesi WA harus terbaca binary Rust tanpa pairing ulang; kalau tidak, skrip migrasi + opsi re-pair (keputusan Oktz).
- **Secret:** nol hardcoded; semua lewat `.env`. Pertahankan `scripts/secret-guard.mjs` di CI.

## 5. Interaktif/buttons/list — syarat keras

Pola yang jalan sekarang (`src/lib/rich-messages.js:455-509`): WA hanya me-render `interactiveMessage` (kartu nativeFlow) bila dikirim **di dalam wrapper `viewOnceMessage`** + `deviceListMetadata`, lalu `relayMessage` ke node `<biz>`. Format `interactiveMessage` lama tidak dirender lagi.

- Port **bukan dari nol**: `socket.js` (1.343 LOC) + `builder.js` (2.263 LOC) sudah merakit envelope ini di atas Baileys. Yang berubah cuma generator protobuf (`waproto`/`buffa` vs `protobufjs`).
- `whatsapp-rust` punya tipe protobuf tapi **tanpa builder** → bangun manual di `oktz-wa`.
- **Spike S3/S4 wajib mengirim paket nyata ke akun uji** dan membuktikan tombol bisa diklik. `list_message` butuh wrapper sendiri.
- WA aktif mengubah perilaku buttons (error 463, Baileys #2683) — perilaku bisa berubah **di luar kendali**; spike + golden-output test adalah jaring pengamannya.

## 6. Strategi test paritas

- **Golden output, bukan inspect mata.** Codespaces jalankan bot Node → tangkap output nyata tiap command → `rust/tests/fixtures/parity/*.json`. Rust test replay, assert **byte-equal** (teks, urutan field, hash media).
- **Port langsung** ~68 file assert murni + **51 file/444 kasus** relevan (pairing/router/group-protection/database) — bahan utama test paritas.
- **Trait mock DI** ganti 23 file `mock.module`. **Fixture offline** ganti 13 file network. **Clock trait** ganti 2 file override waktu.
- Scraper: fixture per host dari `ENDPOINTS.md` → 3 shape response (envelope / problem+json / non-JSON-biner) ditangani eksplisit.

## 7. Testimonial & quality bar

- `cargo build --release` sukses, binary tunggal tanpa Node.js runtime.
- `cargo clippy -- -D warnings` & `cargo fmt --check` bersih.
- Tidak ada `unwrap()`/`expect()` liar di jalur runtime; panic hanya di init yang disengaja. `panic="unwind"` + `catch_unwind` per handler (menjaga `handler-error-visibility`, 22 kasus).
- RSS idle & puncak terukur, **lebih kecil** dari baseline JS (tabel angka di `PERF.md`). Soak 24 jam stabil.
- Secret-guard CI hijau.

## 8. Sequencing & gate

| Fase | Isi | Gate |
|---|---|---|
| **0** | Audit, engine, endpoint, crate, klasifikasi, spec | ✅ Oktz setujui engine + scope |
| **0.5** | **Spike S1–S4** (impor sesi, RSS, interaktif, list) | ✅ S1 & S3 wajib; angka RSS dipakai setting scope |
| **1** | Fondasi: `oktz-core`, `oktz-store`, plugin-api/macros | test + clippy + round-trip DB |
| **2** | Core WA: connect, SYSTEM, pairing, serialize, jadibot | login (sesi lama/pairing baru), `.ping`/`.menu` balas, bukti log+screenshot |
| **3** | Router, middleware, group-protection, scheduler | test paritas router + group-protection |
| **4** | Media & canvas: sticker, ffmpeg, image, OCR, TTS, kartu | golden-image parity + RSS vs JS |
| **5** | Scrapers & capabilities | semua test scraper offline lulus |
| **6** | Plugin per gelombang (3 gelombang, subagent paralel + reviewer) | reviewer + `cargo test` hijau |
| **7** | Hardening: fuzz, soak 24 jam, build release glibc/musl, CI | angka RSS final |
| **8** | Cutover: deploy, migration guide, rollback | **approval Oktz → baruarsipkan JS** (branch/tag, bukan hapus) |

**Gelombang plugin** (kecil → besar): G1 main/info/utility/user/religi/primbon/random/store/clan/vps/tts/media · G2 group/tools/cek/sticker/convert/ephoto/canvas/stalker/search/download/anime/asupan · G3 ai/game/rpg/fun/islamic/owner/panel (+ **nsfw/pushkontak/jpm → tunggu keputusan Oktz**).

**Per plugin:** (1) test paritas dulu (TDD), (2) implementasi, (3) clippy+test hijau, (4) tandai ✅ di `plugin-manifest.json`. Kelas E/F dicatat, bukan dilewati diam-diam.

## 9. Aturan besi

1. JS lama tak disentuh sampai Fase 8 approved. Rust paralel di `rust/`.
2. Tidak ada klaim "selesai" tanpa bukti.
3. Paritas perilaku, bukan kompilasi.
4. Nol hardcoded secret.
5. Sesi login tak diregresi; kalau mustahil → skrip migrasi + opsi re-pair, dan **tanya Oktz**.
6. Barang yang sudah terbukti baik tak ditulis ulang tanpa alasan (ffmpeg tetap subprocess, dll).
7. Hemat RAM = fitur; ukur, jangan tebak.
8. Commit kecil & sering di `rust-rewrite`, pesan konvensional, jangan push main.
9. Bug JS yang ditemukan saat membandingkan → catat di `JS-BUGS.md`, **tanya Oktz** apakah dipertahankan atau diperbaiki — jangan diam-diam "memperbaiki" lalu melanggar paritas.
10. Komentar & dokumen bahasa Indonesia; identifier kode Inggris.
11. Skill/MCP/subagent wajib dipakai (lihat §10); subagent **diverifikasi** orchestrator, dipercaya mentah-mentation.

## 10. Skill / MCP / subagent

- **Skill**: `brainstorming` (path architectural), lalu `writing-plans` → `subagent-driven-development`/`executing-plans` → `test-driven-development` → `systematic-debugging` → `verification-before-completion` → `requesting-code-review`/`receiving-code-review`. Daftar lengkap dicatat di `PROGRESS.md`.
- **MCP**:
  - **Context7** — SEBELUM memakai crate apa pun (`resolve-library-id` + `query-docs`). Versi dicatat di `CRATES.md`. Dilarang menulis API dari ingatan.
  - **Firecrawl** — riset engine WA Rust + audit endpoint + cari ekuivalen gratis.
  - **Playwright** — fixture scraper DOM/JS-rendered, verifikasi alur ber-UI web, screenshot-diff kartu canvas. Hormati rate limit, jangan simpan data pribadi.
- **Subagent** (max 3–4 paralel; build berat di Codespaces): researcher (Firecrawl/Context7), scraper-fixture (Playwright), core-implementer, plugin-porter, **reviewer independen** (belum pernah lihat proses tulis), perf-auditor. Setiap subagent dapat brief berisi path JS, kontrak Rust, test yang harus lulus, crate yang boleh dipakai, dan definisi selesai.

## 11. Definition of Done

- [ ] `cargo build --release` sukses, binary `oktz-md`, tanpa Node.js runtime.
- [ ] Login via sesi lama **atau** pairing baru; false-logout & rate-limit pairing sesuai JS.
- [ ] Seluruh 830 plugin berstatus ✅ / ❌-mati-dokumentasi / ⏸-menunggu-keputusan (tak ada yang hilang tanpa catatan).
- [ ] Semua test paritas lulus; `clippy -D warnings` & `fmt --check` bersih; tanpa `unwrap()`/`expect()` liar di jalur runtime.
- [ ] RSS idle & puncak terukur & **lebih kecil** dari baseline JS (`PERF.md`); soak 24 jam stabil.
- [ ] Secret-guard CI hijau; nol secret di repo.
- [ ] Dokumentasi lengkap (README, infra, migration guide, rollback).

## 12. Keputusan terbuka (menunggu Oktz)

- **20 plugin kelas F** — port atau buang? (jangan sentuh sebelum dijawab)
- **R3** — kalau RSS idle `whatsapp-rust` > 60MB: turunkan scope atau naikkan target?
- **R1** — kalau impor sesi + re-encode v6→v1 gagal: re-pair diterima atau proyek ditunda?

## 13. Self-review (done, dipatch inline)

- Placeholder: tidak ada TBD/TODO tersisa; spike S1–S4 terdefinisi dengan kriteria "lolos kalau".
- Konsistensi: engine (A) konsisten di §2/§5/§8; gate S1–S4 ada di §8 sebelum Fase 1 (sesuai syarat "spike sebelum kode produksi").
- Scope: satu plan besar dengan sub-fase; tiap fase punya gate. Kompleksitas nyata (143k LOC, 830 plugin) **tidak** bisa dipecah jadi satu spec kecil — dipakai sebagai master spec + plan per fase terpisah.
- Ambiguitas: "bergantian di panel" (Node/Rust) sudah diklarifikasi; field `limit` yang 0 override sudah dinyatakan agar tidak diprioritaskan.