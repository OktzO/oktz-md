# Fase 2 Implementation Plan — `oktz-wa` (produksi)
> **STATUS UPDATE (2026-10-11).** Checkbox di file ini TIDAK lagi akurat — plan ini
> ditulis sebelum Task 1–4 dikerjakan dan tidak pernah di-update. Sumber kebenaran
> sekarang `docs/rust-rewrite/PROGRESS.md`. Ringkasan:
>
> - Task 1 DONE — `410c4da`
> - Task 2 DONE — `fae3375`
> - Task 3 DONE — `646843b`
> - Task 4 PARTIAL — logika pairing sudah `2004791` (12 test), tapi subcommand
>   `pair-code`/`pair-qr` BELUM ada di `oktz-bin`, dan `pairing.rs` masih nol
>   call-site (item H3)
> - Task 5 belum mulai
> - GATE 2 belum
> - Di luar plan ini: fix address sesi ratchet `47402d6`, `oktz-router` + 10 plugin
>   `cek*` `c72dd03`


> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax.

**Goal:** Promosikan spike yang sudah terverifikasi menjadi crate produksi `oktz-wa` — device/creds import, session ratchet import, connect, send/receive, interactive (card + list), pairing.

**Architecture:** Kode yang sudah terbukti dari `rust-spike/` dipindah ke `rust/crates/oktz-wa/` sebagai library dengan API bersih. Tidak ada penulisan ulang protokol — spike sudah membuktikan 514/514 sesi import, decrypt pesan nyata, tombol WA render. Fase 2 = assembling + merapikan + test, bukan riset.

**Tech Stack:** `whatsapp-rust` 0.7 (`default-features=false`, tanpa `simd`), `wacore-libsignal` 0.7 (`legacy-session-interop`), `tokio`, `serde_json` (`preserve_order`).

**Spec:** `docs/superpowers/specs/2026-10-10-oktz-md-rust-rewrite-design.md` · Progres: `docs/rust-rewrite/PROGRESS.md`

---

## Global Constraints

- **Build berat di Codespaces/mesin berdaya.** Di 2-core lokal build penuh ~25 menit. Untuk iterasi cepat pakai `cargo check` (bukan `build`).
- **`CARGO_TARGET_DIR` di luar `/tmp/opencode`?** TIDAK — home disk 695MB, tidak cukup. Tetap `/tmp/opencode/rust/target`; **tahu `/tmp` pernah di-reset** (pada 2026-10-10 toolchain+target hilang total). Kalau hilang: reinstall rustup + nixpkgs (lihat `PROGRESS.md`).
- **Nol hardcoded secret.** `.env` sudah ada & gitignored.
- **Komentar hanya yang penting** (alasan non-obvious, gotcha).
- Test dulu, implementasi, `cargo test` + `clippy -D warnings` hijau + commit per task.
- **Jangankip verifikasi nyata:** setiap klaim "terbaca/kirim" harus punya output log nyata.

---

## Review Focus

1. **Session import benar-benar dipakai saat decrypt** — bukan session baru yang kebetulan berhasil. (Regresi dari spike: sebelum session dimuat, 258 pesan "berhasil" via session baru.)
2. **Public key prefix `0x05`** — onigis 32 byte polos → wacore 33 byte. Kalau lupa, `Bad Mac` massal.
3. **Pairing tidak merusak sesi lama** — pairing = device baru; jangan overwrite device_id existing.
4. **RSS tidak naik** — importer 514 sesi = 26.6MB. Kalau ada leak, spiked regression.
5. **Kartu interaktif = envelope yang benar** — `viewOnce` (`FutureProofMessage`) + `messageContextInfo.deviceListMetadata` v2 + extra node `biz>interactive>native_flow`. Tanpa salah satu, WA diam-diam buang tombol.

---

## Task 1: Promosikan spike → `oktz-wa` (lib)

**Files:**
- Create: `rust/crates/oktz-wa/{Cargo.toml, src/{lib,bot,cards,creds,session,load,rss}.rs}`
- Source: `rust-spike/src/{connect,buttons,creds_map,onigis_session,load_sessions,rss}.rs`

**Interfaces:**
- Produces:
  - `oktz_wa::build_device(&Creds) -> Device`
  - `oktz_wa::prepare_store(creds_path, db_path, session_fixture) -> SqliteStore`
  - `oktz_wa::connect_device(store, on_ready) -> BotHandle`
  - `oktz_wa::{build_card, build_list, biz_node}`
  - `oktz_wa::{inspect_creds, import_session_record, load_into_store}`

- [ ] **Step 1: `cargo check -p oktz-wa`** (bukan build — lebih cepat) sampai bersih.
- [ ] **Step 2: `cargo test -p oktz-wa`** — test yang dibawa dari spike harus tetap hijau.
- [ ] **Step 3: `clippy -D warnings`** bersih.
- [ ] **Step 4: Commit**

---

## Task 2: Smoke nyata — connect + decrypt via `oktz-wa`

- [ ] **Step 1: Subcommand smoke di `oktz-bin`** — `oktz smoke <creds> <fixture>`: prepare_store dengan session fixture, connect, tunggu `on_message`, print. Timeout 90s.
- [ ] **Step 2: Jalankan terhadap sesi nyata.** Expected: `Successfully authenticated`, `sesi ratchet dimuat` loaded=514, dan (kalau ada traffic) `MESSAGE` terbaca. **Bukti wajib: capture output.**
- [ ] **Step 3: Catat angka RSS** (target <60MB).
- [ ] **Step 4: Commit.**

---

## Task 3: API kirim pesan (text + interactive)

- [ ] **Step 1: `oktz send-text <jid> <teks>`** dan `oktz send-card <jid>` / `oktz send-list <jid>` via `oktz-bin`.
- [ ] **Step 2: Uji nyata ke nomor owner** — verify text sampai; card **harus diverifikasi Oktz** (tombol render = butuh mata manusia; log saja tak cukup).
- [ ] **Step 3: Commit.**

---

## Task 4: Pairing (kode + QR fallback)

- [ ] **Step 1: `oktz pair-code <nomor>`** — pair via kode 8 digit; print `PairCodeOptions` flow.
- [ ] **Step 2: `oktz pair-qr`** — fallback QR; print QR string.
- [ ] **Step 3: Test offline** — pairing butuh device/interaksi; test unit untuk rate-limit cooldown (`PAIRING_RATE_LIMIT_COOLDOWN_MS`), `classifyClose` 401 handling, `pairingPending` marker. Ini port 1:1 dari `pairing-state.js`.
- [ ] **Step 4: Commit.**

---

## Task 5: Media + LID (terbatas)

- [ ] **Step 1: kirim/terima media** (image) — upload via `prepareWAMessageMedia` equivalent (`client.upload`).
- [ ] **Step 2: LID resolve** — `me.lid` mapping; port helper `lid.js` yang relevan saja (anti-ban helper dari onigis **tidak** dipindah — wacore punya sendiri).
- [ ] **Step 3: Commit.**

---

## GATE 2

- [ ] `cargo test --workspace` hijau.
- [ ] `clippy -D warnings` bersih.
- [ ] Smoke nyata: connect + 514 sesi + decrypt pesan nyata (bukti log).
- [ ] Card interactive **terverifikasi render oleh Oktz** (tombol bisa diklik).
- [ ] RSS terukur <60MB saat connect.
- [ ] Pairing code/QR alur ada (verifikasi penuh bisa ditunda ke GATE 2.5 kalau butuh interaksi).
- [ ] Reviewer independen review.
- [ ] Update `PROGRESS.md`.

---

## Definition of Done (Fase 2)

- [ ] Bot Rust login via **sesi lama** (bukan pairing baru), terima & balas pesan.
- [ ] Pairing code + QR tersedia sebagai fallback.
- [ ] Interactive card + list benar-benar render (verifikasi manusia).
- [ ] RSS <60MB.
- [ ] Semua test hijau, clippy bersih, reviewer LAYAK.