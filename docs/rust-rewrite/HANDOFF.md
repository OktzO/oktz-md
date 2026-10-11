# Handoff — pindah sesi baru

> Dokumen ini buat sesi berikutnya. **Baca ini dulu**, lalu `docs/rust-rewrite/PROGRESS.md`, lalu lanjut.
> Repo `/home/user/noddjs/ourin-md`, branch `rust-rewrite`. Semua commit sudah aman di git.

---

## 1. Di mana kita

**Fase 0 + Fase 1 DONE (GATE 1 ✅). Fase 2 Task 1–4 DONE — TAPI ada blocker aktif dari reviewer (lihat §3).**

| Komponen | Status | Bukti |
|---|---|---|
| Engine `whatsapp-rust` 0.7 | jalan di stable Rust 1.94 | build + run |
| Impor 514 sesi ratchet | 514 convert, **tapi loader BERMASALAH (lihat §3)** | `loaded=514` |
| Connect + decrypt | authenticated, pesan real-time terbaca | `Successfully authenticated` |
| Tombol WA (S3) | **✅ render + bisa diklik** (verifikasi Oktz) | OKE/GAGAW |
| List native-flow (S4) | **✅ render + bisa diklik** (verifikasi Oktz) | single_select |
| RSS produksi | 13–18 MB (target 60) | smoke 16.8, send 13.1 |
| Data user | 24 file / 874KB byte-identik | `verify-db` |
| Test + clippy | 58 hijau, clippy EXIT=0 | `cargo test --release` |

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
- Bukti: `identities` table isinya `...@lid.0`, `...@s.whatsapp.net.0` — **ada `@server`**.
- Konsekuensi: "514/514 termuat" = konversi sukses, TIDAK berarti berguna. Pesan "hemz" dulu terbaca karena kontak **baru** (prekey), bukan sesi lama. "0 BadMac" karena tidak ada traffic kontak lama saat smoke.

**Yang sudah diperbaiki (commit setelah review):**
- `to_address()` sekarang menambah `@s.whatsapp.net` → `{num}@s.whatsapp.net.0`.
- `sessions_loaded` dihitung SETELAH `put_sessions_batch` sukses (sebelumnya dihitung sebelum — false reporting).
- 3 test address (`address_matches_wacore_protocol_address` dll).

**MASIH PERLU DIVERIFIKASI (sesi berikutnya):**
1. **Server-nya benar?** Untuk kontak LID seharusnya `@lid.0` BUKAN `@s.whatsapp.net.0`. onigis id storing nomor polos tanpa info LID/PN. Perlu resolve via `lid_pn_mapping` (tabel ada di DB) atau `session_keys category=lid-mapping` di Turso. **Belum-comingle test ini.**
2. Rebuild `oktz-bin` (perubahan hanya di test sebelum ini) + jalankan smoke.
3. **Bukti akhir:** session dari kontak yang SUDAH ADA harus decrypt, dan session count di `sessions` table harus ~514 dengan format `@server.0`.

Test decisive: `sqlite` DB `sessions` table sample harus `2404068921294@s.whatsapp.net.0`, bukan `2404068921294.1.0`.

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
- `source /tmp/opencode/build_env.sh` set env + `$NS`.
- Build: `nix-shell -p stdenv.cc pkg-config -I nixpkgs=/tmp/opencode/nixpkgs-25.05 --run "cargo <cmd> --release"`
- `/home` cuma ~695MB free — jangan taruh apa pun besar di sana.
- Untuk iterasi cepat: `cargo check --release -p <crate>`.

## 6.Langkah berikutnya (urut)

1. **Tutup blocker §3** (verifikasi + fix server LID/PN). Bukti: session decrypt dari kontak lama.
2. H3: pasang reconnect/logout handler sesuai `pairing-state.js` (grace 60s×2) — supaya 401 palsu tak mematikan.
3. H4: generikkan `build_card(body, footer, buttons)` + `build_list(sections)` jadi API berparameter.
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