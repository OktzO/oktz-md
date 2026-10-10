# RISKS — rewrite Rust `oktz-md`

Generated: **2026-10-10** · Skala: **Blocker** (hentikan proyek) / **Tinggi** / **Sedang** / **Rendah**. Diurutkan besaran, bukan nomor.

---

## R1 — Impor sesi lama mungkin mustahil — **Blocker**

**Apa.** `creds.json` Baileys/onigis + file `SessionRecord` v6 dari `oktz-signal` harus dibaca `whatsapp-rust` (decoder v1, feature `legacy-session-interop`) tanpa pairing ulang.
**Kenapa credibel.** README `oktz-signal` sendiri mengakui "wire compatibility with real WhatsApp clients is not guaranteed in every edge case" dan tidak pernah diaudit. `DeviceStore` butuh `login_counter` dsb yang belum terbukti ada.
**Dampak kalau gagal.** Syarat mutlak Oktz "user tidak scan ulang" gugur.
**Mitigasi.** Spike S1 **sebelum** kode produksi. Fallback berjenjang: (1) `legacy-session-interop`, (2) skrip re-encode v6→v1, (3) **keputusan Oktz** re-pair / tunda.
**Status.** Terbuka. Spike adalah gate.

## R2 — Parity interactive/buttons/list — **Blocker**

**Apa.** Oktz mensyaratkan semua pesan interaktif di-rewrite ke Rust dan **harus bisa**. Pola yang jalan sekarang (`viewOnce+nativeFlow+deviceListMetadata+relay-to-biz`) harus direplikasi persis; `whatsapp-rust` tidak punya builder untuk itu.
**Kenapa berisiko.** WA actively berubah + cara ini non-standard (ditemukan Oktz, bukan standar); sedikit beda field bisa WA tolak.
**Mitigasi.** Spike S3 (tombol) + S4 (list) kirim paket nyata ke akun uji; **paste bukti tombol bisa diklik**, bukan compile. Test golden byte-equal dari bot JS.
**Status.** Terbuka.

## R3 — RSS belum terukur vs target 60MB — **Tinggi**

**Apa.** `wacore`+libsignal+SQLite in-process. Kalau >60MB sendirian, target idle <60MB mustahil tanpa scope change.
**Mitigasi.** Spike S2. Kalau meleset → ajukan scope change ke Oktz (lazy-load media, feature flag), jangan diam-diam naikkan target.
**Status.** Terbuka.

## R4 — 830 plugin = 80% volume kerja — **Tinggi**

**Apa.** Distribusi: A=396, B=262, D=103, C=44, E=5, F=20. Itu 60–80 batch subagent.
**Kenapa.** Solo, zero-budget, mesin dev 2-core/4GB. Build berat di Codespaces.
**Mitigasi.** A dulu (47.7% logika murni → cepat). Subagent batch 10–15 per kategori. Test paritas otomatis (golden output), bukan review manual.
**Status.** Territory, bukan blocker.

## R5 — 20 plugin kelas F menunggu keputusan — **Tinggi (blokir sebagian batch)**

**Apa.** 15 artefak palsu canvas (fakedana/fakegopay/fakecall/…), `nsfw/*`, `jpm/jpm.js`, `pushkontak/pushkontak.js`, `tools/spamngl.js`.
**Kenapa.** Diminta brief: kategori F → **tanya Oktz, jangan porting otomatis.**
**Mitigasi.** Tandai `awaitingDecision` di manifest; **jangan sentuh** sampai Oktz memutuskan. Semua dicatat di manifest, bukan dilewati diam-diam.
**Status.** Menunggu keputusan Oktz.

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