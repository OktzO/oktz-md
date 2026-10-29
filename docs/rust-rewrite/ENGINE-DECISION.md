# ENGINE DECISION — mesin WhatsApp untuk rewrite Rust

Generated: **2026-10-10** · Status: **menunggu approval Oktz (GATE 0)** · Installer: kenapa (A), kenapa (B)/(C) gugur, dan apa yang harus dibuktikan spike.

---

## 1. Kebutuhan nyata bot ini

Diambil dari kode, bukan asumsi. Fitur WA yang benar-benar dipakai:

| Fitur | Pemakaian | Plugin/Module |
|---|---|---|
| `m.reply` | 801 plugin | hampir semua |
| `m.react` | 502 plugin | hampir semua |
| `sendMessage` | 178 plugin | — |
| `m.download` | 78 plugin | sticker/download/media |
| `buttonParamsJson` | 48 plugin | interaktif |
| `groupMetadata` | 42 plugin | group |
| newsletter (JID + `forwardedNewsletterMessageInfo`) | 35 plugin | info/stalker |
| `relayMessage` | 31 plugin | builder/socket |
| `waUploadToServer` | 24 plugin | media upload |
| `interactiveButtons` / `interactiveMessage` / `nativeFlowMessage` | 18 / 17 / 16 | **interaktif — syarat keras** |
| `prepareWAMessageMedia` / `generateWAMessageFromContent` | 14 / 28 | builder/socket/auto-download |
| `viewOnce` | 16 plugin | serialize |
| `groupParticipantsUpdate` | 10 plugin | group-protection |
| jadibot (multi-session) | 9 plugin | main/owner |
| poll / presence / call offer / reaction wire | **0** | tidak dipakai |

Tambahan dari `src/lib/pairing-state.js`: pairing **kode** (`config.session.pairingNumber`), fallback **QR**, cooldown rate-limit, `pairingPending`, `classifyClose`, false-logout grace 60s ×2 retry. Ini port 1:1, tidak ada ruang desain.

## 2. Engine kandidat

### (A) `whatsapp-rust` v0.7 — **DIPILIH**

- Repo https://github.com/oxidezap/whatsapp-rust · MIT · v0.7.0 (7 Agu 2026)
- Aktif: **1.874 commit, commit terakhir hari ini** (2026-10-10), 36 kontributor, 7 open issue, bukan archived
- Satu baris dependency; `wacore` (libsignal+codec) / `waproto` (protobuf WA) / storage / transport semuanya pluggable
- Context7: `/oxidezap/whatsapp-rust`
- Docs: https://whatsapp-rust.jlucaso.com/ · docs.rs: https://docs.rs/whatsapp-rust

Matriks fitur (semua ✅ kecuali dua baris):

| Fitur | Status | Bukti |
|---|---|---|
| Pairing kode 8 digit | ✅ | `Client::pair_with_code(PairCodeOptions)`; `PairCodeRejection::is_throttled()` untuk 400 |
| Pairing QR | ✅ | `Event::PairingQrCode` + `PairingQrCodesExhausted` |
| Multi-device (Noise XX/IK, ADV) | ✅ | docs `waproto.md` |
| LID | ✅ | `Jid::lid()`, `StoredDeviceSummary {pn, lid, linked}` |
| Grup + metadata + admin | ✅ | `create_group`, `GroupQueryIq` → `Full/NotModified/ServerError{backoff}` |
| Media up/down + thumbnail | ✅ | `client.upload/download`, `Downloadable`, media-reupload |
| Interactive/buttons/list | ⚠️ | **protobuf ada, builder helper tidak ada** — harus bangun sendiri (lihat §4) |
| Newsletter | ✅ | `client.newsletter()` |
| Status/view-once | ✅ | `Jid::StatusBroadcast`, `view_once_message_v2` |
| Reaction/edit/revoke/poll | ✅ | `send_reaction`, `EditRequest`, `revoke_message`, `Polls` |
| Multi-session | ✅ | `create_device`/`share_for_device` |
| Reconnect + 401 | ✅ / ❓ | auto-reconnect ada; kurva backoff & kode 440 belum dicek → wajib baca `/api/errors.md` |

**Alasan utama (paling kuat):** `Onigi/` yang dipakai bot sekarang **sudah bridge ke `whatsapp-rust-bridge` karya author yang sama**. Jalur ke engine ini sudah teruji di produksi repo ini.

### (B) Tulis protokol sendiri di atas `oktz-signal` — **DITOLAK**

`oktz-signal` = 4.343 LOC Rust, MIT, tapi README-nya sendiri: *"Not implemented, by design: SenderKey / group E2EE"*, tanpa Noise handshake, tanpa binary node codec, tanpa protobuf WA, tanpa app-state/history-sync/USync/LID/newsletter/upload. Biaya = implementasi penuh protokol WA multi-device + raw WhatsApp butuh pairing ulang (device key baru). Fragilitas tertinggi. **Gugur.**

### (C) Sidecar hibrida (Node `onigis` + Rust) — **DITOLAK sebagai hasil akhir**

 Nol fitur hilang dan sesi pasti terbaca, tapi nol penurunan RAM di sisi WA — pustaka WA tetap Node/JS — dan dua runtime untuk satu bot. Cocok **hanya sebagai jembatan** sementara kalau ada fitur yang proved tidak bisa di-port, dan dipakai untuk menghasilkan expected-output test paritas di Codespaces.

---

## 3. Risiko open #1 — impor sesi lama (S1)

Syarat mutlak Oktz: user **tidak scan ulang**. Risiko nyata:

- `storage/session/creds.json` (22 key, format Baileys/onigis) berisi `noiseKey`, `identityKey`, `signedPreKey`, `advSecretKey`, `account`, `me`, `registered` → **bisa** diimpor. `whatsapp-rust` punya feature flag **`legacy-session-interop`** (default off) untuk ini.
- **Jebakan:** E2EE bot ini bukan libsignal asli, tapi `oktz-signal` v0.3.0-rc.3 yang klaim wire-compatible dengan libsignal **v6**, padahal decoder `legacy-session-interop` adalah **v1**. Kompatibilitas **belum terbukti**.
- Field `DeviceStore` lengkap wajib diisi, termasuk `login_counter` (ada adopter yang ditolak 0.6.0 karena field ini kurang).

**Rencana jika S1 gagal:** re-encode `SessionRecord` v6 → v1 lewat skrip migrasi (bukan pairing ulang). Kalau itu juga gagal → **keputusan Oktz**: re-pair diterima atau proyek ditunda. Ini satu-satunya hal yang tidak reversible, jadi spike jalan sebelum kode produksi apa pun.

## 4. Risiko open #2 — parity interaktif (syarat keras Oktz)

Oktz: interactive/buttons/list **harus** di-rewrite ke Rust dan **harus bisa**.

Temuan menentukan di `src/lib/rich-messages.js:455-509`: WhatsApp **hanya** me-render `interactiveMessage` (kartu nativeFlow) bila dikirim **di dalam wrapper `viewOnceMessage`** + `deviceListMetadata`, lalu `relayMessage` ke node `<biz>`. Format `interactiveMessage` lama sudah tidak dirender. Pola ini ditemukan Oktz sendiri dan **sedang berjalan normal di produksi** (dikonfirmasi).

Implikasi:
- `whatsapp-rust` punya tipe protobuf-nya (`interactive_message`, `buttons_message`, `list_message`, `template_message`) tapi **tidak ada builder/helper API** → bangun manual di `oktz-wa`.
- Port bukan dari nol: `src/lib/socket.js` (1.343 LOC, ~30 method monkey-patch) + `src/lib/builder.js` (2.263 LOC) **sudah** merakit envelope ini secara manual di atas primitif Baileys. Yang berubah cuma schema generator (`waproto`/`buffa` vs `protobufjs`). List message butuh wrapper sendiri — belum pernah dipakai luas di kode lama, tapi tetap harus ada.
- Spike S3/S4 wajib mengirim paket nyata ke akun uji dan **membuktikan tombol bisa diklik**, bukan cuma compile.

## 5. Risiko open #3 — RSS belum terukur

Target: RSS idle < 60MB, puncak < +40MB (dari brief). Baseline Node terukur: idle ~84MB, puncak OCR 216MB, limit runtime 550MB. `whatsapp-rust` membundel `libsignal`+`wacore`+SQLite in-process — **belum ada angkanya**. Kalau `wacore` sendiri makan >60MB, target 60MB tidak tercapai tanpa menyederhanakan scope (mis. lazy-load subsistem media). Spike S2 mengukur ini; kalau meleset, keputusan scope diambil bersama Oktz, bukan diam-diam.

---

## 6. Yang perluEDE tidak berubah (guardrail)

- `oktz-signal` **tidak jadi engine** (alasan di §2B), tapi kode Rust-nya **tidak dihapus** dari repo — masih dipakai `Onigi`/JS.
- `ourin-native` (OCR tesseract-rs) **dipakai ulang** di Rust, dilepas dari napi jadi lib biasa.
- ffmpeg tetap **subprocess** (`tokio::process::Command`). Tidak `ffmpeg-sys`. Audit lama sudah mengukurnya: out-of-process RSS Node flat (84→88→82), rewrite in-process menaikkan baseline tanpa win.
- Tidak ada `prost` manual — `waproto` sudah menangani protobuf WA.

## 7. Feature flag wajib

`legacy-session-interop` (impor sesi) dan `passkey` (pairing) **wajib aktif**. Tanpa `passkey`, `pass_key_prologue_request` jatuh jadi notifikasi biasa dan pairing bisa gagal kalau WA enforce passkey gate (bug sama dilaporkan di OnWA #560).

## Verdict

**Pilih (A) `whatsapp-rust` v0.7, dengan spike S1–S4 sebagai syarat sebelum kode produksi apa pun, dan sidecar (C) hanya sebagai alat oracle-test di Codespaces.**

Menunggu approval Oktz.