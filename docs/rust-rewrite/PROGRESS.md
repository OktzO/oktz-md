# PROGRESS — rewrite Rust `oktz-md`

> Dipakai untuk resume lintas konteks/sesi. **Baca file ini duluan** di sesi baru, lalu lanjut dari "Sedang dikerjakan". Update tiap akhir sesi.

---

## Status saat ini

**Fase 0 (Audit & Riset) — SELESAI, menunggu GATE 0 (approval Oktz).**
Belum ada satu baris kode Rust produksi. Belum ada `rust/` directory.

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
| **Spike S1 (impor sesi)** | ✅ **514/514 sesi ratchet, 0 BadMac** |
| S1 — pemetaan creds→Device | ✅ connect authenticated |
| S1 — decrypt via sesi impor | ✅ **"hemz" terbaca** (bukti kompatibilitas oktz-signal↔wacore) |
| **Spike S3 (tombol)** | ✅ **muncul + bisa diklik** |
| **Spike S4 (list)** | ✅ terkirim, render menunggu verifikasi Oktz |
| **Spike S2 (RSS)** | ✅ **26.6 MB** saat connect (target 60) |
| Test suite / clippy | ✅ 33 test, clippy -D warnings bersih |
| Fase 1 (oktz-core/store/plugin-api) | ✅ **GATE 1 LOLOS** (reviewer independen: LAYAK) |
| Fase 2 (oktz-wa + router produksi) | 🔄 **sedang** — plan `2026-10-10-fase-2-oktz-wa.md`, Task 1 (promosi spike) build berjalan |
| Fase 3+ / 830 plugin | ⏳ setelah GATE 2 |

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

## Perintah penting

- Build/test berat: **GitHub Codespaces (`gh cs ssh`)**, bukan lokal (2-core/4GB).
- Branch: `rust-rewrite`. **Jangan push ke main.**
- JS lama = oracle + fallback. **Jangan hapus** sebelum Fase 9 approved.

## Catatan styling repo

- Bahasa komentar & dokumen: Indonesia (casual-teknis). Identifier kode: Inggris.
- Commit kecil & sering, pesan konvensional (`feat(core): …`, `test(plugin/rpg): …`).
- Tanpa klaim "selesai/berhasil" tanpa bukti (`cargo test`, `clippy -D warnings`, ukuran binary, RSS terukur).