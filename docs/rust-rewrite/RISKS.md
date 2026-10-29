# RISKS — rewrite Rust `oktz-md`

Generated: **2026-10-10** · Skala: **Blocker** (hentikan proyek) / **Tinggi** / **Sedang** / **Rendah**. Diurutkan besaran, bukan nomor.

---

## R1 — Impor sesi lama mungkin mustahil — **Blocker** *(sempit, lihat analisis)*

**Analisis 3 lapis (hasilThinking deeper, 2026-10-10):** `storage/session/` hanya berisi `creds.json` + 10 `pre-key-*.json` + `device-list-*.json` — **nol `session-*.json`**. Sesi ratchet per kontak **tidak di file, tapi di Turso** (`turso-session.js`, format `BufferJSON` Baileys). Tiga lapis yang bisa gagal:

1. **Struktural** (`creds.json` → `DeviceStore`): butuh `login_counter`, `device_pairing_data`, `pairing_epoch`, `server_cert_chain` yang mungkin tidak ada di 22 key Baileys. Murah — synthesize. Precedent: adopter ditolak 0.6.0 karena kurang `login_counter`.
2. **Format record** (Turso `SessionRecord` v6 → decoder v1): **gagal diam-diam**. Protobuf mengabaikan unknown field, jadi record bisa ke-parse tanpa error tapi isinya salah (message keys kosong) → decrypt gagal terus-menerus.
3. **Algoritma** (`oktz-signal` vs libsignal): paling mungkin buntu. README `oktz-signal`: tidak pernah diaudit, "wire compatibility tidak dijamin". Kalau X3DH/Double Ratchet deviate sedikit, libsignal di `whatsapp-rust` derive key berbeda → session tak akan pernah sync. Tidak tertangkap validasi struktural.

**Kenapa ini TIDAK fail total.** Sesi ratchet per kontak bisa dibangun ulang dari nol selama `identityKey` + `signedPreKey`/prekeys + device registration selamat — ketiganya ada di disk. Kalau record v6 tidak bisa di-decode: **buang record, jangan gagalkan import**; bot tetap device yang sama, ratchet direset, WA kirim prekey baru di pesan masuk berikutnya, session rebuild sendiri. Yang hilang hanya history sync belum-terbaca + mungkin window pesan terakhir.

**Fail total** hanya kalau device identity hilang — `creds.json` korup/tidak terbaca, atau WA menolak device dan memaksa unlink. `creds.json` ada utuh dengan karantina `creds.json.broken-*` + test `creds-recovery.test.mjs`. Risiko kecil.

**Yang sebenarnya berbahaya: setengah berhasil.** Bot connect + online + handshake lolos, tapi pesan tidak bisa didecrypt → user lihat "Menunggu pesan ini…" selamanya. Lebih mahal dari gagal bersih.

**Definisi gate S1 diperbaiki:** S1 **gagal** kalau bot connect **DAN** pesan dari akun uji **terbaca** (bukan sekadar "connect OK"). **Dua jalur import diuji terpisah:** (a) full — creds + record sesi dari Turso; (b) creds-only — kalau (a) buntu, buang record, bangun ulang ratchet. Jalur (b) yang menjamin no-re-pair.
**Status.** Terbuka. Spike adalah gate.

## R2 — Parity interactive/buttons/list — **Blocker**

**Apa.** Oktz mensyaratkan semua pesan interaktif di-rewrite ke Rust dan **harus bisa**. Pola yang jalan sekarang (`viewOnce+nativeFlow+deviceListMetadata+relay-to-biz`) harus direplikasi persis; `whatsapp-rust` tidak punya builder untuk itu.
**Kenapa berisiko.** WA actively berubah + cara ini non-standard (ditemukan Oktz, bukan standar); sedikit beda field bisa WA tolak.
**Mitigasi.** Spike S3 (tombol) + S4 (list) kirim paket nyata ke akun uji; **paste bukti tombol bisa diklik**, bukan compile. Test golden byte-equal dari bot JS.
**Status.** Terbuka.

## R3 — RSS belum terukur vs target 60MB — **Tinggi (delegated ke orchestrator)**

**Keputusan Oktz (GATE 0):** "atur saja" — orchestrator berwenang menetapkan scope/target RSS berdasarkan angka spike, tidak perlu tanya lagi.
**Aturan yang saya pakai kalau meleset:** kalau S2 ukur RSS idle `whatsapp-rust` > 60MB, prioritaskan (1) lazy-load subsistem media (canvas/OCR/scrapers tidak di-load sampai dipakai), (2) feature flag per kategori plugin agar binary bisa dirampingkan, (3) kurangi cache global. **Naikkan angka target hanya sebagaiPALING akhir**, dan wajib dicatat alasannya di `PERF.md` — tidak diam-diam.
**Mitigasi.** Spike S2 mengukur; scope change dipilih dari angka, bukan tebakan.
**Status.** Terbuka, tapi tidak lagi blokir keputusan — orchestrator pegang kendali.

## R4 — 830 plugin = 80% volume kerja — **Tinggi**

**Apa.** Distribusi: A=396, B=262, D=103, C=44, E=5, F=20. Itu 60–80 batch subagent.
**Kenapa.** Solo, zero-budget, mesin dev 2-core/4GB. Build berat di Codespaces.
**Mitigasi.** A dulu (47.7% logika murni → cepat). Subagent batch 10–15 per kategori. Test paritas otomatis (golden output), bukan review manual.
**Status.** Territory, bukan blocker.

## R5 — 20 plugin kelas F — **SELESAI: port semua**

**Keputusan Oktz (GATE 0, 2026-10-10):** port semua 20 plugin kelas F. Tidak ada yang dibuang.
Daftar: 15 artefak palsu canvas (`fakebankjago`, `fakecall`, `fakedana`, `fakedev7`, `fakegopay`, `fakeml`, `fakeovo`, `fakestory`–`4`, `iqc`, `iqc2`, `iqcpink`, `ttqc`), `nsfw/nsfw.js`, `nsfw/paptt.js`, `jpm/jpm.js`, `pushkontak/pushkontak.js`, `tools/spamngl.js`.
Manifest sudah di-update: semua F = `oktzDecision: "port-all"`, `awaitingDecision: false`.
**Status.** ✅ Ditutup — jadi tugas biasa di gelombang G3.

## R6 — 5 plugin kelas E mati — **Rendah**

**Apa.** Semua andalkan `firefly.maiku.my.id` (530 Cloudflare). `ssyoutube.com` mati region tapi tidak dipakai plugin.
**Kenapa.** Tidak bisa di-port; harus tandai.
**Mitigasi.** Port dengan pesan jujur yang **sama persis** seperti perilaku JS sekarang (endpoint mati → user dapat pesan error yang ada). Tandai ❌-mati-di manifest. Ekuivalen gratis: riset terpisah (lihat ENDPOINTS §5, mostly belum terverifikasi).
**Status.** Terbuka, low-risk.

## R7 — `libsql` 0.9.30 tertinggal 7 bulan — **Sedang**

**Apa.** Rilis stabil terakhir 2026-03-19; 0.10 masih pre. Plan B `rusqlite` 0.40.2 (aktif, tapi butuh `spawn_blocking`).
**Mitigasi.** Dipakai parity dengan `turso.js` (Turso remote + fallback lokal). Kalau crateXY bermasalah → ganti `rusqlite`.
**Status.** Terbuka.

## R8 — `reqwest` 0.13 ganti feature TLS — **Sedang**

**Apa.** Feature `rustls-tls` **dihapus**, jadi `rustls` (default). Default provider sekarang **aws-lc-rs** (perlu cmake + C compiler), yang notoriously buruk saat cross-compile musl (panel Pterodactyl/Alpine).
**Mitigasi.** Pakai `rustls-no-provider` + provider `ring` sendiri. **Belum diuji — jadikan item spike.**
**Status.** Terbuka.

## R9 — Byte-compat DB lowdb — **Sedang**

**Apa.** 8 file JSON via lowdb, `database.js:127-153` merge default di atas data. Tanpa `serde_json` `preserve_order`, urutan key berubah tiap serialize → churn diff + risk regresi.
**Mitigasi.** `preserve_order` wajib. Test round-trip: baca file asli → tulis → `git diff` kosong.
**Status.** Mitigasi sudah dipilih.

## R10 — Wire WA berubah berkala → maintenance baru — **Sedang (terIMA)**

**Apa.** WA mengubah wire protocol; `whatsapp-rust` sudah 1.874 commit ambil ground truth dari WhatsApp Web. Pindah ke Rust = rebase berkala.
**Status.** Diterima. Baru, tapi inevitable sama seperti Baileys.

## R11 — `chat-store` dihapus dari crate — **Sedang**

**Apa.** `whatsapp-rust-chat-store` sudah di-extract ke repo terpisah → tidak ada message-history/unread bawaan.
**Mitigasi.** Kalau bot butuh history/unread → bangun sendiri di `oktz-store`. Kalau ternyata tidak dipakai (cek `config`) → tidak perlu.
**Status.** Perlu konfirmasi pemakaian.

## R12 — 41 plugin `needsRuntimeConfig` — **Rendah**

**Apa.** 23 `games.createPlugin(...)` + 18 config non-literal (loop/spread). Config di-generate runtime, tidak bisa di-parse statis.
**Mitigasi.** Port manual dengan golden-output test. `customTrigger` (`owner/eval.js`) tetap `"<function>"`, tidak di-eval.
**Status.** Terbuka, low-risk.

---

## Yang **tidak** jadi risk (sudah diputuskan)

- `panic="abort"` tidak dipakai — `unwind` + `catch_unwind` per handler, karena `tests/handler-error-visibility.test.mjs` (22 kasus) mensyaratkan error plugin terlihat, bukan bot mati.
- `oktz-signal` bukan engine — sudah ditolak (§ ENGINE-DECISION 2B).
- ffmpeg in-process — sudah ditolak (audit lama: out-of-process RSS flat).