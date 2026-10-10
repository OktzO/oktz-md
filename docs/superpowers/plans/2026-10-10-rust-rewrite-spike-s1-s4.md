# Spike S1–S4 — Bukti Kelayakan Rewrite Rust `oktz-md`

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Buktikan atau refute — tanpa menulis satu baris pun kode produksi — bahwa `whatsapp-rust` 0.7 bisa (S1) membaca sesi WA yang sudah ada tanpa pairing ulang, (S2) berada di bawah target RAM, (S3) mengirim pesan interaktif yang WhatsApp benar-benar render, dan (S4) mengirim list message.

**Architecture:** Empat probe terpisah, masing-masing dijalankan sebagai binary throwaway di `rust-spike/` (di luar `rust/` — hasil spike **tidak** masuk ke rewrite). Tiap probe memakai `whatsapp-rust` 0.7 dengan feature `legacy-session-interop` + `passkey`, dan database SQLite throwaway. Tidak ada kode yang dipakai ulang ke produksi; spike di-archive setelah Putusan.

**Tech Stack:** Rust 1.94+ (edition 2024), `whatsapp-rust` 0.7, `tokio`, `serde_json` (`preserve_order`), `libsql` (hanya baca data Turso), `serde`/`anyhow`.

**Spec:** `docs/superpowers/specs/2026-10-10-oktz-md-rust-rewrite-design.md` · Risiko: `docs/rust-rewrite/RISKS.md` R1/R2/R3 · Engine: `docs/rust-rewrite/ENGINE-DECISION.md`

---

## ⚠ Prasyarat — HARD BLOCKER, belum terpenuhi di mesin ini

Diverifikasi 2026-10-10 di mesin dev lokal:

| Prasyarat | Status | Bukti |
|---|---|---|
| Rust toolchain | ❌ **tidak ada** | `~/.cargo` tidak ada, `~/.rustup` tidak ada, `rustup`/`cargo`/`rustc` tidak di PATH |
| GitHub CLI | ❌ **tidak ada** | `gh: command not found` → Codespaces (`gh cs ssh`) tidak bisa dipakai |
| `.env` | ❌ **tidak ada** | `TURSO_URL`/`TURSO_AUTH_TOKEN` absen → **jalur "full" S1 tidak bisa dijalankan** (sesi ratchet ada di Turso) |
| Akun WhatsApp uji | ❓ belum dikonfirmasi | dibutuhkan untuk S1/S3/S4 |

**Verdict: spike belum bisa dieksekusi sampai prasyarat di atas terpenuhi.** Plan ini sengaja ditulis lengkap supaya langsung bisa dieksekusi begitu toolchain tersedia — bukan untuk dieksekusi sekarang.

Sesuai `docs/rust-migration-audit.md` §3.2: **panel tidak pernah compile**. Build spike wajib di Codespaces atau mesin lain, bukan di laptop 2-core/4GB.

### Yang perlu Oktz siapkan

1. Codespaces dengan Rust toolchain (`gh cs ssh`), atau mesin builder mana pun.
2. Salin `.env` dari mesin production (punya `TURSO_URL` + `TURSO_AUTH_TOKEN`). **Jangan** commit `.env` — secret-guard CI akan menolak.
3. Nomor WhatsApp akun uji yang **tidak** dipakai bot produksi, untuk menguji S3/S4 tanpa disturbing session utama.

---

## Global Constraints

- **Jalur produksi (main / `storage/session/`) JANGAN disentuh.** Spike hanya **membaca** `storage/session/` dan Turso; ia menulis ke direktori throwaway.
- **Nol hardcoded secret.** Semua lewat `.env`.
- **Tidak ada kode spike yang masuk ke `rust/`.** Kalau spike "mengarah ke" sesuatu, tulis ulang bersih di rewrite.
- Bahasa komentar/dokumen: Indonesia. Identifier kode: Inggris.
- Tidak ada klaim berhasil tanpa output nyata (log, angka RSS, screenshot).

---

## Review Focus

Lima kondisi yang paling mungkin menggigit, yang tidak diuji oleh step mana pun di plan ini:

1. **Sesi connect tapi pesan tidak terbaca** — kondisi "setengah berhasil" yang paling mahal. Dicek Task 2 Step 7 (pesan nyata dari akun uji, bukan hanya "connect OK").
2. **Record sesi Turso kosong/`hollow`** — `turso-session.js:209` punya `reportHollowSession()`; kalau Turso mengembalikan 0 key untuk scope utama, jalur "full" otomatis tidak mungkin. Dicek Task 2 Step 3.
3. **`device-list-6285143885645.json` = 6 byte** — DeviceStore multi-device mungkin corrupt/tak terisi. Dicek Task 2 Step 2.
4. **Tombol terkirim tapi tidak bisa diklik** — Telegram-mu "connect OK" tidak menjamin WA merender. Dicek Task 4 Step 6 (klik nyata dari akun kedua).
5. **RSS terukur naik diam-diam setelah decrypt** — spike harus mengukur **setelah** pesan pertama, bukan cuma saat start. Dicek Task 3 Step 5.

---

## Data aktual yang sudah diverifikasi (dipakai spike ini)

`storage/session/creds.json` — format **`onigis`** (bukan Baileys standar):

```
noiseKey {private,public} · signedIdentityKey {private,public} · signedPreKey {keyPair,signature,keyId}
pairingEphemeralKeyPair · advSecretKey · registrationId 225 · registered true
platform "smba" (WhatsApp **Business**) · me {id "6285143885645:54@s.whatsapp.net", name "Oktz", lid "188412430651393:54@lid"}
routingInfo {type,data} · myAppStateKeyId "AAAAAL5Y" · lastPropHash · accountSyncCounter
nextPreKeyId 15556 · firstUnuploadedPreKeyId 15556 · signalIdentities (len 1) · additionalData {} (pairingPending kosong)
```

- **Tidak ada `identityKey`** — field-nya bernama `signedIdentityKey`.
- **`platform: "smba"`** = WhatsApp Business. Ini kemungkinan besar alasan tombol tidak kena penolakan 463 (non-Business) yang dilaporkan di Baileys #2683.
- **Tidak ada `session-*.json` di disk** — 0 file. Sesi ratchet per kontak ada di Turso, tabel `session_keys (scope, category, id, data)`.
- Prekey disk: `pre-key-15546.json` … `pre-key-15555.json` (10 file), format `{"private":{"type":"Buffer","data":"<base64>"},"public":{...}}` — **`data` adalah string base64**, bukan array angka seperti `BufferJSON` Baileys standar. Importer harus baca base64, bukan `data[..]` sebagai byte array.
- `device-list-6285143885645.json` = 6 byte (kemungkinan `[]`).

---

## Task 1: Scaffold spike + ukur baseline

**Files:**
- Create: `rust-spike/Cargo.toml`
- Create: `rust-spike/src/main.rs`
- Create: `rust-spike/README.md` (satu paragraf: "throwaway, tidak masuk produksi")

**Interfaces:**
- Consumes: tidak ada.
- Produces: binary `oktz-spike` dengan subcommand `baseline`.

- [ ] **Step 1: Buat crate dengan dependency yang sudah diverifikasi**

`rust-spike/Cargo.toml`:
```toml
[package]
name = "oktz-spike"
version = "0.0.0"
edition = "2024"

[dependencies]
tokio = { version = "1.53", features = ["rt-multi-thread", "macros", "fs", "time", "signal"] }
serde_json = { version = "1.0.151", features = ["preserve_order"] }
serde = { version = "1.0.229", features = ["derive"] }
anyhow = "1"
whatsapp-rust = { version = "0.7", features = ["legacy-session-interop", "passkey"] }
```

> Bib verifikasi: `whatsapp-rust.jlucaso.com/installation.md` (feature `legacy-session-interop` = "Typed interop with the decoded legacy `SessionRecord` v1 model"). Kalau `cargo build` gagal dengan "feature does not exist", cek feature aktual via `cargo info whatsapp-rust`.

- [ ] **Step 2: Tulis program yang mengukur RSS baseline**

`rust-spike/src/main.rs` — subcommand `baseline` yang: parse argumen, baca `/proc/self/status` untuk `VmRSS`, print JSON `{phase, rss_kb}`. Fungsi pengukuran harus reusable oleh Task 3 (ekspor sebagai `mod rss`).

- [ ] **Step 3: Build dan jalankan baseline**

Run: `cd rust-spike && cargo run --release -- baseline`
Expected: JSON `{"phase":"startup","rss_kb":<angka>}`, exit 0.

- [ ] **Step 4: Catat angka baseline** → `docs/rust-rewrite/spike/RSS.md` (buat folder). Ini pembanding untuk S2.

- [ ] **Step 5: Commit**

```bash
git add rust-spike/
git commit -m "spike: scaffold oktz-spike + baseline RSS harness (throwaway)"
```

---

## Task 2 (S1): Impor sesi tanpa pairing ulang

**Files:**
- Create: `rust-spike/src/import_creds.rs`
- Create: `rust-spike/src/turso_read.rs`
- Create: `rust-spike/src/session_probe.rs`
- Modify: `rust-spike/src/main.rs` — tambah subcommand `s1`

**Interfaces:**
- Consumes: Task 1 (crate + `mod rss`).
- Produces: fungsi `probe(path_a: &Path, path_b: &Path) -> SessionProbe` dan CLI `oktz-spike s1 --full|--creds-only`.

- [ ] **Step 1: Tulis test untuk importer creds (gagal dulu)**

`rust-spike/tests/import_creds.rs` — test dengan creds fixture **asli** (copy `storage/session/creds.json` ke `rust-spike/tests/fixtures/creds.json`):
```rust
#[test]
fn maps_signed_identity_key_to_device_identity() {
    let creds = read_fixture("creds.json");
    let dev = map_to_device_store(&creds).expect("mapping");
    assert_eq!(dev.registration_id, 225);
    assert!(dev.adv_secret_key.is_some());
}
```
Run: `cargo test --test import_creds` → Expected: FAIL (`map_to_device_store` belum ada).

- [ ] **Step 2: Petakan field onigis → `DeviceStore`**

`import_creds.rs` harus menangani pemetaan berikut secara eksplisit (field yang verified ada):

| onigis creds | `DeviceStore` whatsapp-rust | catatan |
|---|---|---|
| `noiseKey` | noise identity keypair | format `{private,public}` Buffer-JSON base64 → decode |
| `signedIdentityKey` | device identity keypair | **bukan `identityKey`** |
| `signedPreKey` | signed prekey | `keyId` = `signedPreKey.keyId` |
| `advSecretKey` | adv secret key | string base64 |
| `registrationId` | registration id | = 225 |
| `platform` | platform | `"smba"` |
| `me.id` / `me.lid` | account + LID | `me.lid` = `188412430651393:54@lid` |
| `routingInfo` | routing info | onigis `{type,data}` — pertahankan verbatim |
| `myAppStateKeyId` | app state key id | base64 |
| `lastPropHash` | last prop hash | |
| `nextPreKeyId` | prekey id counter | 15556 |

Field `login_counter` / `pairing_epoch` / `device_pairing_data` **tidak ada** di creds → synthesize dari `registrationId`. Kalau `whatsapp-rust` menolak, catat error persis ke `docs/rust-rewrite/spike/S1.md`.

- [ ] **Step 3: Baca device-list dan session_keys dari Turso (jalur "full")**

`turso_read.rs`: query `SELECT id, data FROM session_keys WHERE scope = ?` (lihat `turso-session.js:395`). Catat **jumlah key per (scope, category)** ke `S1.md`. Kalau 0 → tulis "jalur full tidak mungkin, pindah ke Task 2 Step 6".

- [ ] **Step 4: Jalankan jalur FULL — impor creds + record sesi**

Run: `cargo run --release -- s1 --full --source ./storage/session`
Expected: laporan `devices_loaded`, `prekeys_loaded`, `sessions_decoded`, `sessions_failed`, plus log koneksi.

- [ ] **Step 5: Ukur kegagalan decode — ini yang menentukan**

Kalau `sessions_failed > 0`, **klasifikasi penyebabnya** dan tulis apa adanya ke `S1.md`:
- `v6-vs-v1`: record ke-decode tapi field tidak lengkap (message keys kosong) → **Lapis 2**.
- `parse-error`: record gagal parse total → **Lapis 2**.
- `derive-mismatch`: record parse, decrypt gagal → **Lapis 3** (lihat Task 2 Step 7).

Jangan skip klasifikasi ini; itu inti dari spike.

- [ ] **Step 6: Jalur CREDS-ONLY — buang record sesi, bangun ulang ratchet**

`SessionProbe` mode `creds-only`: impor creds + prekeys, **jangan** impor `session_keys`. Kataloger: "ratchet tiap kontak akan reset sendiri saat pesan masuk".
Run: `cargo run --release -- s1 --creds-only --source ./storage/session`

- [ ] **Step 7: Uji dengan pesan NYATA (WAJIB — ini yang membedakan sukses dari "connect OK")**

Kirim pesan `.ping` dari akun uji ke nomor bot. Bot harus **membaca** pesan itu (bukan "Menunggu pesan ini…").
Expected: isi pesan terbaca. Kalau tidak → S1 **GAGAL** (walaupun "connect OK"), catat ke `S1.md`, dan **tidak boleh lanjut ke Fase 1**.

- [ ] **Step 8: Tulis Putusan S1** → `docs/rust-rewrite/spike/S1.md`:
```
VERDICT: FULL | CREDS-ONLY | FAIL
Evidence: <path ke creds> · <jumlah prekey> · <jumlah session decoded/failed>
Test nyata: <isi pesan yang terbaca, atau GAGAL>
Konsekuensi: no-re-pair YA/TIDAK
```

- [ ] **Step 9: Commit**

```bash
git add rust-spike/ docs/rust-rewrite/spike/S1.md
git commit -m "spike(S1): impor sesi onigis->whatsapp-rust, jalur full + creds-only"
```

---

## Task 3 (S2): Ukur RSS realistis

**Files:**
- Create: `rust-spike/src/rss_probe.rs`
- Create: `rust-spike/src/main.rs` — tambah subcommand `rss`

**Interfaces:**
- Consumes: Task 1 (`mod rss`), Task 2 (sesi yang **sudah** termuat).
- Produces: CLI `oktz-spike rss --hold <detik>` yang print JSON `{phase, rss_kb, heap_kb}` tiap 5 detik.

- [ ] **Step 1: Bangun** harness yang print RSS tiap 5 detik selama N detik, ke stdout sebagai JSON-lines.
- [ ] **Step 2: Ukur RSS IDLE** — bot connect, tidak ada pesan, 3 menit.
- [ ] **Step 3: Ukur RSS setelah pesan pertama masuk + dibalas** (Review Focus #5).
- [ ] **Step 4: Ukur RSS setelah 20 pesan** (gabungan beberapa tipe pesan: teks, media, tombol).
- [ ] **Step 5: Tulis angka** → `docs/rust-rewrite/spike/RSS.md`: `idle`, `after_first_msg`, `after_20_msg`, `peak`. Bandingkan baseline Node (~84MB) dan target (<60MB idle).

- [ ] **Step 6: Putususkan scope** sesuai aturan R3: kalau idle >60MB → lazy-load media (media/scrapers/OCR tidak load sampai dipakai) → feature flag per kategori → kurangi cache. Naikkan angka target **hanya** sebagai opsi terakhir dan tulis alasannya. Catat keputusan di `RSS.md`.
- [ ] **Step 7: Commit**

```bash
git add rust-spike/ docs/rust-rewrite/spike/RSS.md
git commit -m "spike(S2): ukur RSS whatsapp-rust in-process, tetapkan scope mitigation"
```

---

## Task 4 (S3+S4): Interaktif + list message

**Files:**
- Create: `rust-spike/src/interactive.rs`
- Create: `rust-spike/src/main.rs` — tambah subcommand `s3` dan `s4`

**Interfaces:**
- Consumes: Task 2 (sesi termuat).
- Produces: CLI `oktz-spike s3 --to <jid>` dan `oktz-spike s4 --to <jid>`.

> Konteks wajib: pola yang **terbukti jalan** di bot JS adalah (`src/lib/rich-messages.js:455-509`): `viewOnceMessage` wrapper → `interactiveMessage` → `nativeFlowMessage` + `deviceListMetadata` → `relayMessage` ke node `<biz>`. Format `interactiveMessage` **tanpa wrapper tidak dirender** WA.

- [ ] **Step 1: Rakit envelope `viewOnce` + `interactive` + `nativeFlow` + `deviceListMetadata` + relay ke `<biz>`** persis seperti `socket.js:378-394` dan `builder.js:764-786`.
- [ ] **Step 2: Kirim 1 interactive dengan 2 tombol** dari akun uji.
- [ ] **Step 3: Log payload** yang dikirim (hex/JSON) ke `S3.md` — untuk dibandingkan byte-level dengan output bot JS nanti.
- [ ] **Step 4: Verifikasi dari akun kedua (WAJIB):** tombol **muncul dan bisa diklik** pada percakapan.
- [ ] **Step 5: Jika tidak muncul/ tidak bisa diklik** — WA menolak. Bandingkan payload dengan output bot JS (jalankan bot JS di Codespaces, kirim paket identik, bandingkan). Catat perbedaan field persisnya di `S3.md`.
- [ ] **Step 6: List message (S4)** — rakit `listMessage` dengan 3 section. Verifikasi list muncul dan bisa dipilih dari akun kedua. WA butuh `sections` ≤ 10 dan `rows` ≤ 10 per section; jangan lewat.
- [ ] **Step 7: Tulis Putusan S3/S4** → `docs/rust-rewrite/spike/S3-S4.md`:
```
VERDICT: PASS | FAIL
Tombol: muncul/tidak · bisa diklik/tidak
List: muncul/tidak
Perbedaan payload vs bot JS: <daftar field>
```
- [ ] **Step 8: Commit**

```bash
git add rust-spike/ docs/rust-rewrite/spike/S3-S4.md
git commit -m "spike(S3,S4): interaktif + list message, verifikasi klik nyata"
```

---

## Task 5: Putusan GATE 0.5 + laporan

**Files:**
- Create: `docs/rust-rewrite/spike/REPORT.md`

**Interfaces:**
- Consumes: `S1.md`, `RSS.md`, `S3-S4.md`, `RSS.md` dari Task 3.
- Produces: Putusan go/no-go untuk Fase 1.

- [ ] **Step 1: Tulis `REPORT.md` merangkum S1, S2, S3/S4** dengan angka nyata dan Putusan per probe.
- [ ] **Step 2: Perbarui `PROGRESS.md`** — hapus item yang sudah diputuskan, tambahkan hasil spike + langkah berikutnya.

- [ ] **Step 3: Commit**

```bash
git add docs/rust-rewrite/spike/REPORT.md docs/rust-rewrite/PROGRESS.md
git commit -m "docs(spike): laporan kelayakan S1-S4, keputusan lanjut/tidak ke Fase 1"
```

---

## Done bila

- [ ] S1 punya Putusan + bukti pesan nyata terbaca (atau GAGAL terdokumentasi jujur).
- [ ] S2 punya angka RSS idle / after-first / after-20 dalam KB.
- [ ] S3/S4 punya Putusan + bukti klik nyata dari akun kedua.
- [ ] `REPORT.md` ada, dan go/no-go ke Fase 1 tertulis eksplisit.
- [ ] Tidak ada file spike yang bocor ke `rust/`.
- [ ] Tidak ada file produksi yang berubah.