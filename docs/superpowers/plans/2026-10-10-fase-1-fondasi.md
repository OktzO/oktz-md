# Fase 1 Implementation Plan — Fondasi `oktz-md` Rust

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Membangun tiga crate fondasi — `oktz-core`, `oktz-store`, `oktz-plugin-api` — yang langsung dipakai Fase 2+ (WA, router, plugin). Target: `cargo test` hijau, `clippy -D warnings` bersih, DB round-trip **byte-identik**.

**Architecture:** Cargo workspace di `rust/`. Spike yang sudah terbukti (impor 514 sesi, connect, decrypt, tombol) dipromosikan jadi `oktz-wa` di Fase 2; Fase 1 hanya pondasi yang tidak butuh WA. `oktz-store` menjaga kompatibilitas byte-level dengan LowDB JS sehingga user tidak kehilangan data. `oktz-plugin-api` meniru kontrak `pluginConfig` JS.

**Tech Stack:** Rust 1.94 (edition 2024), `tokio`, `serde`/`serde_json` (**`preserve_order` wajib**), `anyhow`/`thiserror`, `tracing`, `moka`, `governor`, `linkme` (registry plugin), `proc-macro2`/`quote`/`syn` (macro `#[plugin]`).

**Spec:** `docs/superpowers/specs/2026-10-10-oktz-md-rust-rewrite-design.md` · Audit: `docs/rust-rewrite/AUDIT.md` · Crates: `docs/rust-rewrite/CRATES.md` · Progres: `docs/rust-rewrite/PROGRESS.md`

---

## Global Constraints

- **Build di Codespaces atau mesin berdaya**, bukan laptop 2-core lokal — build penuh ~20 menit lokal. Kalau Codespaces tidak tersedia, `nix-shell -p stdenv.cc pkg-config -I nixpkgs=/tmp/opencode/nixpkgs-25.05` sudah terbukti jalan (lihat `PROGRESS.md`).
- **`serde_json` WAJIB feature `preserve_order`** — tanpa itu urutan key JSON berubah tiap serialize dan round-trip LowDB pecah.
- **Identifier kode: Inggris. Komentar & dokumen: Indonesia**, casual-teknis seperti repo ini.
- **Tanpa hardcoded secret** — semua lewat `.env`/config. `scripts/secret-guard.mjs` CI tetap hijau.
- **Nol `unwrap()`/`expect()` di jalur runtime** (panic hanya di init yang disengaja). `panic="unwind"`.
- **Setiap task berakhir dengan `cargo test` + `cargo clippy -- -D warnings` hijau + satu commit.** Kalau gagal, perbaiki — jangan lanjut task berikutnya dengan merah.
- **Test dulu (TDD)**: tulis test yang gagal → implementasi → test hijau.
- **Komentar hanya yang penting** (alasan non-obvious, gotcha, constraint). Jangan komentar basa-basi.
- Reviewer independen per batch (subagent `reviewer` yang belum pernah lihat proses tulis).

---

## Review Focus

Lima kondisi yang paling mungkin menggigit dan **tidak** diuji step biasa:

1. **Field DB yang tak dikenal hilang saat tulis** — user kehilangan history karena field baru di-cast. → test: file dengan field asing harus bulat-balik utuh.
2. **Urutan key berubah** — lowdb merge default di atas data (`database.js:127-153`); urutan hilang = diff churn + regresi. → test: byte-identik round-trip.
3. **Angka vs string** — JS JSON.stringify `1` untuk float `1.0`; Rust bisa tulis `1.0`. → test: nilai numerik dari file asli.
4. **Macro `#[plugin]` tidak wokenya alias/handler** — plugin termuat tapi tidak respond. → test: registry berisi entry yang benar.
5. **Cooldown/energi race** — dua pesan bersamaan melewati batas cooldown. → test: dua pesan berurutan melewati batas cooldown.

---

## Task 1: `oktz-core` — config, env, error, logging

**Files:**
- Create: `rust/Cargo.toml` (workspace)
- Create: `rust/crates/oktz-core/Cargo.toml`, `src/lib.rs`, `src/error.rs`, `src/env.rs`, `src/config.rs`, `src/logging.rs`
- Test: inline `#[cfg(test)]` per modul

**Interfaces:**
- Consumes: tidak ada.
- Produces:
  - `oktz_core::error::Error` (enum, `thiserror`) + `oktz_core::error::Result<T>`
  - `oktz_core::env::Env` — `load()` baca `.env`, `get(key)`, `require(key) -> Result<&str>`, `get_optional`
  - `oktz_core::config::Config` — `from_env()`, field mirror `config.js` yang relevan
  - `oktz_core::logging::init(level)`

- [ ] **Step 1: Scaffold workspace + crate, stub yang compile**

`rust/Cargo.toml` (workspace dengan member list; `panic="unwind"`, profile release diatur nanti di crate utama). `oktz-core/Cargo.toml` berisi `thiserror`, `serde`, `serde_json` (preserve_order), `tracing`, `tracing-subscriber`, `dotenvy`.
`src/lib.rs`: `pub mod error; pub mod env; pub mod config; pub mod logging;`

- [ ] **Step 2: Tulis test `env` yang gagal dulu**

`env.rs` test: set variabel via test-only constructor, `get`/`require`/`get_optional` benar; `require` key hilang → error.

- [ ] **Step 3: Implementasi `error.rs` + `env.rs` sampai test hijau**

`Error`: `Io`, `Config(String)`, `NotFound(String)`, `Other(#[from] anyhow::Error)`.

- [ ] **Step 4: Tulis test `config` yang gagal dulu**

`Config::from_env()`: field wajib (`session.pairingNumber` optional, `owner.number`) — test bahwa default+Oktzryang dipakai bot benar-benar terbaca.
Baca `config.js` dulu: `config.owner.number: string[]`, `config.session.pairingNumber`, `config.database.path` = `./database/main`.

- [ ] **Step 5: Implementasi `config.rs` sampai hijau**

Gunakan `serde_json` dengan `preserve_order`. Field yang tidak ada di `.env` → default dari `config.js` (port default, BUKAN hardcode acak — salin nilai config.js).

- [ ] **Step 6: `logging.rs` + test init tidak panic**

`init(level)` idempotent (panggil dua kali aman).

- [ ] **Step 7: `cargo test -p oktz-core` + `clippy` hijau, `cargo fmt`**

Run: `cargo test -p oktz-core && cargo clippy -p oktz-core -- -D warnings`
Expected: hijau, 0 warning.

- [ ] **Step 8: Commit**

```bash
git add rust/
git commit -m "feat(core): config, env, error, logging oktz-core"
```

---

## Task 2: `oktz-store` — LowDB byte-compat + round-trip

**Files:**
- Create: `rust/crates/oktz-store/Cargo.toml`, `src/lib.rs`, `src/json_store.rs`, `src/lowdb.rs`, `src/turso.rs`
- Test: `tests/roundtrip.rs` (pakai file DB asli — read-only copy)

**Interfaces:**
- Consumes: `oktz_core::error::Result`.
- Produces:
  - `oktz_store::JsonStore` — `open(path)`, `get(key) -> Result<serde_json::Value>`, `set(key, value)`, `write()` (atomic: temp+rename), `all() -> Value`
  - `oktz_store::json_store::roundtrip_bytes(raw: &str) -> String` — helper parse→serialize identik (dipakai test)

- [ ] **Step 1: Salin file DB asli jadi fixture (read-only)**

`rust/crates/oktz-store/tests/fixtures/users.json` ← copy `database/main/users.json` (604 user×35 field). `settings.json` juga. JANGAN commit data asli ke repo production — pakai di test via path relatif `../../../../database/main/users.json` atau fixture yang sudah di-sanitasi (hapus PII kalau perlu).

- [ ] **Step 2: Tulis test round-trip yang gagal dulu**

```rust
#[test]
fn reads_real_users_db_and_roundtrips_byte_identical() {
    let raw = std::fs::read_to_string(FIXTURE).unwrap();
    let v: serde_json::Value = serde_json::from_str(&raw).unwrap();
    let out = serde_json::to_string(&v).unwrap();
    assert_eq!(out, raw, "lowdb kompat: serialize harus identik dengan file asli");
}
```
Expected: FAIL jika `preserve_order` belum aktif atau format beda.

- [ ] **Step 3: Implementasi `lowdb.rs` (round-trip helper) sampai hijau**

Kuncinya: `serde_json` dengan `preserve_order`, `to_string` (tanpa indent, cocok `JSON.stringify`).

- [ ] **Step 4: Tulis test `JsonStore` yang gagal dulu**

- `open` file yang tidak ada → buat baru dengan default `{}`.
- `set` + `write()` → file di disk identik dengan `serde_json::to_string`.
- **Field asing harus bertahan**: JSON `{a:1, fieldAsing:{x:1}}` → set key lain → write → `fieldAsing` masih ada (Review Focus #1).
- Numeric round-trip: `1.0` → cek output tidak jadi `1.0` string.

- [ ] **Step 5: Implementasi `JsonStore` sampai hijau**

`write()` atomic: tulis ke `<file>.tmp` lalu `fs::rename` (samakan `database.js:49`).

- [ ] **Step 6: `turso.rs` — stub koneksi Turso (Fase 5 penuh, stub dulu)**

`TursoStore::from_env()` baca `TURSO_URL`/`TURSO_AUTH_TOKEN`; stub yang compile, test bahwa `from_env()` tanpa kredensial → error jelas. JANGAN konek Turso sungguhan di test.

- [ ] **Step 7: `cargo test -p oktz-store` + clippy hijau**

- [ ] **Step 8: Commit**

```bash
git commit -m "feat(store): LowDB byte-compat round-trip + JsonStore atomic + stub Turso"
```

---

## Task 3: `oktz-plugin-api` + macro `#[plugin]`

**Files:**
- Create: `rust/crates/oktz-plugin-api/Cargo.toml`, `src/lib.rs`, `src/meta.rs`, `src/context.rs`, `src/registry.rs`
- Create: `rust/crates/oktz-plugin-macros/Cargo.toml`, `src/lib.rs`
- Test: inline + satu integration test registry

**Interfaces:**
- Consumes: `oktz_core::error::Result`.
- Produces:
  - `oktz_plugin_api::meta::PluginMeta` — `name`, `alias: Vec<String>`, `category`, `description`, `usage`, `example`, `cooldown: u32`, `energi: u32`, `is_owner`, `is_premium`, `is_group`, `is_private`, `is_admin`, `is_bot_admin`, `is_enabled`, `skip_registration`
  - `oktz_plugin_api::context::Ctx` — `db`, `config`, `text`, `args`, `sender`, `is_owner`, `uptime`
  - `oktz_plugin_api::registry::{register, all_plugins() -> &'static [PluginEntry]}`
  - macro `#[plugin(...)]` dari `oktz_plugin_macros`

- [ ] **Step 1: Tulis test meta yang gagal dulu**

`PluginMeta::default()` isi default yang benar-benar dipakai bot (dari `plugin-manifest.json`: `cooldown:0`, `energi:0`, `is_enabled:true`, `alias:[]`).

- [ ] **Step 2: Implementasi `meta.rs` sampai hijau**

Field persis seperti `pluginConfig` yang **benar-benar muncul** di 797 file (lihat `AUDIT.md` §1) — jangan tambah field yang 0 plugin pakai (mis. `limit`).

- [ ] **Step 3: Tulis test registry yang gagal dulu**

Satu plugin dummy didaftarkan via `#[plugin(...)]`; `all_plugins()` memuatnya dengan meta benar + handler non-null.

- [ ] **Step 4: `#[plugin]` macro (proc-macro)**

Macro generate: `fn plugin_entry() -> PluginEntry` yang mendaftarkan `PluginMeta` ke registry statis (`linkme`). **PENTING:** hot-reload `isEnabled` = flag runtime di `PluginEntry`, bukan rewrite source (gantikan `disableplugin.js`).

- [ ] **Step 5: `context.rs` (Ctx struct) sampai test hijau**

- [ ] **Step 6: `cargo test -p oktz-plugin-api -p oktz-plugin-macros` + clippy hijau**

- [ ] **Step 7: Commit**

```bash
git commit -m "feat(plugin-api): trait Plugin, PluginMeta, registry + macro #[plugin]"
```

---

## Task 4: Integrasi — workspace build + gate GATE 1

**Files:**
- Modify: `rust/Cargo.toml` (workspace members final)
- Create: `rust/crates/oktz-bin/Cargo.toml`, `src/main.rs` (stub)

**Interfaces:**
- Consumes: oktz-core, oktz-store, oktz-plugin-api.
- Produces: `oktz-bin` yang boot (init config+log, buka store, muat registry, print "ok").

- [ ] **Step 1: `oktz-bin` stub yang boot**

`main.rs`: `oktz_core::logging::init("info")`, `Config::from_env()`, buka `JsonStore` di `config.database.path`, muat plugin registry, print ringkasan. Tidak konek WA (Fase 2).

- [ ] **Step 2: Full workspace test + clippy + fmt**

Run: `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --check`
Expected: semua hijau, 0 warning.

- [ ] **Step 3: Round-trip DB end-to-end di `oktz-bin`**

Jalankan `oktz-bin` (mode `--verify-db`) yang: baca users.json asli, tulis ulang ke file copy, bandingkan byte. Harus identik.

- [ ] **Step 4: Commit**

```bash
git commit -m "feat(bin): oktz-bin stub boot + verifikasi round-trip DB"
```

- [ ] **Step 5: Reviewer independen**

Subagent `reviewer` review diff Task 1–4: korektness, paritas, panic/unwrap, blocking async, kebocoran memori, secret.

- [ ] **Step 6: Update `PROGRESS.md` + GATE 1**

Catat angka (test count, clippy, DB round-trip byte-identik). **GATE 1** = test hijau + clippy bersih + round-trip lulus → lanjut Fase 2.

---

## Definition of Done (Fase 1)

- [ ] `cargo test --workspace` hijau.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` bersih.
- [ ] `cargo fmt --check` bersih.
- [ ] DB round-trip **byte-identik** pada file(lowdb asli.
- [ ] `oktz-bin` boot stub jalan.
- [ ] Nol hardcoded secret; secret-guard hijau.
- [ ] Reviewer independen reviewRQ.
- [ ] Update `PROGRESS.md`.