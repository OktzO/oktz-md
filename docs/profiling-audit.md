# Profiling Audit — Ourin MD (kandidat Rust: Go/No-go)

Date: 2026-08-30 · Data: window CPU produksi pertama (Pterodactyl panel,
Node v22.23.2) + catatan status heap. Metode: analisis flamegraph
(`.cpuprofile`, self-time + total-time), BUKAN baca kode manual.

> **Status doc ini: AUDIT AWAL — data terbatas.** Hanya SATU window CPU
> (idle) yang sudah terkumpul. Belum ada heap snapshot produksi (guard RSS
> abort terus — ceiling terlalu rendah buat baseline panel). Verdict di bawah
> valid untuk data yang ADA, tapi syarat "muncul konsisten di multiple
> window" belum bisa dipenuhi penuh. Lihat section "Butuh data lagi".

---

## TL;DR

| Kandidat | Self-time kumulatif (window 600s idle) | Frekuensi | Verdict | Alasan satu baris |
|---|---|---|---|---|
| `curve25519-js` (Signal crypto) | 178ms self / 541ms total (0.03%/0.09%) | per-message signal decrypt | **No-go** | Operasi per-call kecil (~µs), FFI overhead > hemat; total cuma 0.09% CPU |
| `luxon` (datetime) | 179ms self / 586ms total (0.03%/0.098%) | per-schedule/per-format | **No-go** | Sudah di batas noise; per-call microsecond, FFI gak worth |
| `cron` scheduler | 7ms self / 346ms total | per-menit (10+ CronJob) | **No-go** | Total-time dari children I/O/timer; self cuma 7ms |
| `WABinary` encode/decode | 64ms self / 179ms total | per-message baileys | **No-go** | Bagian `ourin-baileys` (out of scope, fork terpisah) |
| Signal protocol (baileys) | 23ms self / 33ms total | per-message | **No-go** | `ourin-baileys` out of scope |
| `src/*` repo (lid, serialize, handler) | 57ms self / 116ms total | per-command | **No-go** | SavePersistentCache 17ms = disk I/O, bukan CPU |
| baileys Utils (crypto, promiseTimeout) | 31ms self / 141ms total | per-message | **No-go** | Out of scope + native crypto internal |

**Kesimpulan sementara: berdasarkan window idle 600s (06:57-07:15 UTC /
13:47-13:57 WIB), TIDAK ADA kandidat yang lolos syarat Go.** Semua kandidat
<0.1% CPU kumulatif. Tapi data belum lengkap — butuh window ramai + heap diff
untuk verdict final.

---

## 1. Data yang tersedia

### 1.1 CPU profile — window idle (satu-satunya yang ada)

- File: `storage/profiling/cpu-20260830-065717-cmd-6285608953677.cpuprofile`
- 600.4s, 527,099 samples, interval sampling 1.14ms
- Beban saat window: SEpi — log menunjukkan hanya beberapa command owner
  (.menu, .menucat, .heapsnap, .cpuprofile, .allmenu dari user lain, .ping2,
  .afk). Bot 99.6% idle.
- `(idle)` = 598,145ms (99.6%). Semua kerja nyata kumulatif < 2.5s.

### 1.2 Heap snapshot — BELUM ADA

- `.heapsnap` dicoba 2x di panel (13:26:06, 13:26:51) → **ABORT dua-duanya**:
  - `RSS 374.5MB > ceiling 300.0MB`
  - `RSS 337.3MB > ceiling 300.0MB`
- Baseline RSS panel ternyata 290-375MB — JAUH di atas asumsi 150-230MB di
  audit lama (mesin audit ≠ panel produksi).
- **Temuan penting:** ceiling 300MB TIDAK cocok buat panel. Guard jalan benar
  (bukti safety net kerja), tapi gak akan pernah lolos di panel. Perlu
  penyesuaian threshold sebelum bisa ambil heap snapshot produksi.

---

## 2. Kandidat & verdict

### 2.1 `curve25519-js` (Signal protocol crypto) — **No-go**

| Metrik | Angka |
|---|---|
| Self-time total | 178ms (0.03%) |
| Total-time (with children) | 541ms (0.09%) |
| Node puncak self-time | `M` 24ms/23ms, `gf` 19ms — curve25519-js/lib/index.js:189, :5 |
| Frekuensi | per-message signal decrypt (group messages) |
| Konsistensi | 1 window saja — belum bisa konfirmasi multiple window |

Alasan No-go (khusus, bukan generic):

- **Per-call cost sangat kecil.** `gf`/`M` adalah field arithmetic 25519 —
  operasi single-function dalam hitungan mikrodetik. Di window 600s, total
  self-time 178ms = ~0.3ms per detik rata-rata. Bahkan kalau Rust ngehabisin
  semuanya jadi 0, hematnya 0.178s per 10 menit = <0.03% CPU.
- **FFI overhead > hemat.** Operasi per-call µs; napi-rs boundary + data
  crossing (angka/buffer) sudah memakan lebih dari yang dihemat. Ini persis
  kasus yang di-screening-out di audit lama (`ourin-serialize`, `ourin-lid`).
- **Juga out of scope:** `curve25519-js` dipanggil dari `ourin-baileys`
  (node_modules/ourin/lib/Signal/...) — bukan repo ini. Menulis ulang butuh
  fork baileys, keputusan terpisah.

### 2.2 `luxon` (datetime) — **No-go**

| Metrik | Angka |
|---|---|
| Self-time total | 179ms (0.03%) |
| Total-time | 586ms (0.098%) |
| Node puncak | `shiftTo` 30ms, `toObject` 21ms, `normalizeObject` 15ms |
| Pemicu | cron scheduling (per-menit), format waktu, sholat/jadwal |

Alasan No-go:

- Self-time 179ms/600s = 0.03%. Walaupun total-time 586ms terlihat besar,
  mayoritas itu children (cron/scheduler I/O & async), bukan CPU luxon.
- Per-call microsecond (satu shiftTo/toObject ≈ 0.02-0.03ms). Rust FFI
  overhead per call > isi call. Tidak ada agregasi yang cukup besar.
- Dua audit sebelumnya sudah mencatat luxon sebagai non-masalah; data
  profiling idle mengonfirmasi — bukan kandidat.

### 2.3 `cron` scheduler — **No-go**

Total-time 346ms tapi self cuma 7ms. 346ms itu dari children: `callbackWrapper`
→ handler async (I/O, DB). Scheduler sendiri hampir gratis. Bukan CPU JS yang
bisa di-Rust-kan.

### 2.4 `WABinary` + Signal protocol (baileys) — **No-go**

Semuanya di `node_modules/ourin/` = `ourin-baileys`. Per audit lama section
2.6, repo ini tidak menyentuh baileys (fork terpisah). Self-time WABinary 64ms
(0.011%), Signal 23ms (0.004%) — kecil dan out of scope. Double No-go.

### 2.5 `src/*` repo — lid, serialize, handler — **No-go**

| File | Self-time | Catatan |
|---|---|---|
| `src/lib/ourin-lid.js:32` `savePersistentCache` | 17ms | **Disk I/O** (fs.writeFileSync), bukan CPU — Rust tidak membantu |
| `src/handler.js:577` `messageHandler` | 7ms | Entry point dispatch, bukan hotspot |
| `src/lib/ourin-serialize.js:517` `serialize` | 3ms | Per-command; audited sebelumnya No-go |

Total `src/*` self 57ms (0.009%). Tidak ada hotspot JS murni di repo ini yang
muncul. Bahkan serialize (yang paling sering dipanggil per-message) cuma 3ms.

---

## 3. Konfirmasi status M1-M18 dari heap diff

**TIDAK BISA DILAKUKAN — heap snapshot produksi belum ada.**

Guard RSS abort 2x di panel (RSS 337-374MB vs ceiling 300MB). Jadi tidak ada
`.heapsnapshot` produksi untuk di-diff, tidak ada retainer growth data.
M1-M18 tidak bisa dikonfirmasi dari data profiling.

---

## 4. Temuan tambahan (bukan kandidat Rust, tapi penting)

### 4.1 Ceiling RSS heap snapshot salah asumsi

- Audit lama: baseline RSS 150-230MB (diukur di mesin audit IDX 8GB).
- Panel produksi: baseline 290-375MB (dari memory monitor + guard abort log).
- Perbedaan ~100-150MB — kemungkinan dari: memory monitor `global.gc()` yang
  beda behavior, jumlah plugin/state termuat (829 plugin), baileys store,
  native `ourin_native` (181MB peak di audit native), dll.
- **Aksi (DITERAPKAN):** ceiling `HEAP_SNAP_RSS_CEILING` dinaikkan 300MB →
  **400MB**, `HEAP_SNAP_PROJECTED_CEILING` 480MB → **800MB**, berdasarkan
  angka nyata panel (idle rss 330MB + 2×heapTotal 190MB ≈ 710MB; kontainer
  1GB). Test guard RSS tetap hijau (abort terbukti jalan saat RSS
  di-inflasi). Perlu verifikasi lanjutan di panel: ambil snapshot saat idle
  tengah malam (rss ~290MB) paling aman.
- Ini BUKAN kandidat Rust — ini fix config harness (threshold salah asumsi).

---

## 5. Butuh data lagi (untuk verdict final)

Data sekarang tidak cukup untuk verdict final yang solid:

1. **Window CPU ramai** — perlu `.cpuprofile start 10` saat traffic chat
   padat (jam ramai), biar kandidat yang cuma muncul di idle bisa beda.
   Kandidat yang muncul konsisten di idle + ramai baru layak dipertimbangkan.
2. **Heap snapshot produksi** — perlu fix ceiling dulu (section 4.1), lalu
   minimal 2 snapshot jeda ≥2 jam buat diff & konfirmasi M1-M18.
3. **Multi-window konsistensi** — syarat "muncul konsisten di multiple
   window" belum terpenuhi (baru 1 window).

**Kalau mau lanjut cari kandidat:** tambah window spesifik operasi berat —
`.cpuprofile start 5` lalu `.ocr` / sticker / broadcast berulang-ulang. Itu
skenario di mana hotspot per-call (kalau ada) bakal terlihat agregat.

---

## Implementation log

### 2026-08-30 — Window pertama (idle) dianalisis

- CPU profile produksi `cpu-20260830-065717-...` di-deploy user dari panel,
  dianalisis. Verdict awal: semua No-go (semua kandidat <0.1% CPU).
- Heap snapshot produksi belum bisa diambil — guard RSS abort 2x (RSS 337-374MB
  vs ceiling 300MB). Guard terbukti jalan (safety net kerja), ceiling salah
  asumsi buat panel.
- Actionable next: fix ceiling RSS di `src/lib/ourin-profiler.js` + ambil
  window ramai + heap snapshot.

### 2026-08-30 — Ceiling RSS disesuaikan dengan data nyata panel

- `HEAP_SNAP_RSS_CEILING` 300MB → **400MB**; `HEAP_SNAP_PROJECTED_CEILING`
  480MB → **800MB** (idle panel: rss 330MB + 2×heapTotal 190MB ≈ 710MB).
- Test `tests/profiler.test.mjs`: 5 pass, 0 fail (guard abort tetap terbukti).
  Full suite: 104 pass, 0 fail.
- Belum di-deploy — user perlu `git pull` + restart, lalu coba `.heapsnap`
  pas RSS terendah buat ambil snapshot pertama.
