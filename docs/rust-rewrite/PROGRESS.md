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
| **GATE 0 — approval Oktz** | ⏳ **MENUNGGU** |
| Spike S1–S4 | ⏳ setelah GATE 0 |

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

## Reproduction

```bash
# regenerate manifest
node scripts/extract-plugin-manifest.mjs

# (akan datang) spike & test — butuh Codespaces
cd rust && cargo test
```

## Perintah penting

- Build/test berat: **GitHub Codespaces (`gh cs ssh`)**, bukan lokal (2-core/4GB).
- Branch: `rust-rewrite`. **Jangan push ke main.**
- JS lama = oracle + fallback. **Jangan hapus** sebelum Fase 9 approved.

## Catatan styling repo

- Bahasa komentar & dokumen: Indonesia (casual-teknis). Identifier kode: Inggris.
- Commit kecil & sering, pesan konvensional (`feat(core): …`, `test(plugin/rpg): …`).
- Tanpa klaim "selesai/berhasil" tanpa bukti (`cargo test`, `clippy -D warnings`, ukuran binary, RSS terukur).