# Profiling Harness — Ourin MD

Date: 2026-08-30 · Tujuan: kumpulkan data empiris nyata (CPU flamegraph + heap
snapshot) dari proses yang jalan sungguhan di produksi, BUKAN baca kode manual
lagi (dua audit sebelumnya sudah exhausted metode baca source).

Harness ini murni instrumentasi observability — tidak menyentuh logic bisnis,
plugin, handler, atau database. Zero dependency tambahan (`node:inspector` dan
`node:v8` keduanya built-in).

---

## 1. Cara trigger

Semua trigger owner-only (non-owner otomatis ditolak middleware via
`isOwner: true` di config plugin). Profiling **default OFF** — tidak ada yang
nyala otomatis saat boot.

### CPU profiling — `.cpuprofile`

| Command | Fungsi |
|---|---|
| `.cpuprofile start [menit]` | Mulai profiling CPU (default 5 menit, max 10 menit) |
| `.cpuprofile stop` | Stop lebih awal & tulis file |
| `.cpuprofile status` | Cek status CPU profiler + RSS saat ini |

Ambil beberapa window di jam berbeda (pagi sepi vs jam ramai chat) biar data
representatif — satu snapshot kondisi tertentu tidak cukup.

### Heap snapshot — `.heapsnap`

| Command | Fungsi |
|---|---|
| `.heapsnap [label]` | Ambil heap snapshot (label opsional, misal `.heapsnap before-ocr`) |
| `.heapsnap status` | Cek RSS saat ini + apakah aman untuk snapshot |

Ambil **minimal 2 snapshot dengan jeda 2-6 jam** (misal `.heapsnap pagi` lalu
`.heapsnap sore`) supaya bisa di-diff untuk nemuin leak/growth. Satu snapshot
tidak ada gunanya.

---

## 2. Lokasi output

```
storage/profiling/
  cpu-YYYYMMDD-HHMMSS-<name>.cpuprofile
  heap-YYYYMMDD-HHMMSS-<label>.heapsnapshot
```

- `storage/` di-gitignore dan **persist melewati restart** (folder ini tempat
  session WhatsApp — diverifikasi: session hidup kembali setelah restart).
- File diberi timestamp biar gak timpa-timpakan.

### Cara ambil file keluar dari panel

1. **Panel Pterodactyl → File Manager** → navigasi ke `storage/profiling/`
   → download file (cara paling simpel & dianjurkan).
2. Alternatif via WhatsApp: `plugins/owner/get.js` bisa ambil path lokal
   (`.get` didesain untuk URL, tapi owner bisa taruh file ke path publik atau
   pakai tool panel). Rekomendasi: pakai File Manager panel.

---

## 3. Safety threshold (angka & alasan)

Host produksi: Pterodactyl, heap 512MB (`--max-old-space-size=512`), total RAM
kontainer 1GB. Baseline RSS bot normal: **150-230MB** (lihat
`memory-leak-audit.md`; memory monitor `startMemoryMonitor` log RSS tiap 2
menit, `RSS_LIMIT = 550MB`).

| Threshold | Nilai | Alasan |
|---|---|---|
| `HEAP_SNAP_RSS_CEILING` | **400MB** | Baseline RSS panel produksi 290-375MB (terukur dari memory monitor, 2026-08-30). Ceiling 400MB memberi margin ~70MB di atas idle; abort kalau RSS mendekati zona bahaya OOM. |
| `HEAP_SNAP_PROJECTED_CEILING` | **800MB** | Proyeksi puncak `rss + 2×heapTotal` (estimasi konservatif overhead serialisasi). Di panel idle: 330 + 2×190 = 710MB. Ceiling 800MB < 1GB total RAM kontainer (margin 200MB). Dua kondisi, abort kalau salah satu dilanggar. |
| `CPU_MAX_DURATION_MS` | **10 menit** | Auto-stop safety net — kalau CPU profiler lupa dimatiin, berhenti sendiri. Mencegah sampling overhead numpuk berjam-jam & file `.cpuprofile` membengkak. |
| Default durasi CPU | **5 menit** | Window pendek cukup buat flamegraph; gak numpuk overhead. |

**Perilaku saat guard memicu:** snapshot TIDAK dipaksa jalan — abort, log
alasan (`Heap snapshot ABORT · <reason> · trigger <siapa>`), user dikasih tahu
lewat balasan command. Tidak ada retensi data snapshot di memory setelah file
ditulis ke disk (hanya path file yang di-return, data hasil `writeHeapSnapshot`
tidak disimpan reference).

---

## 4. Observability & kill-switch

- **Log setiap start/stop** dengan konteks lengkap:
  - CPU start: `CPU profiling START · <name> · <durasi>s · trigger <siapa> · rss <X>MB`
  - CPU stop: `CPU profiling STOP · <name> · <durasi aktual>s · trigger <siapa> · <path>`
  - CPU auto-stop (kalau lupa): `CPU auto-stop (durasi max tercapai, trigger <siapa>)`
  - Heap start/done/abort: masing-masing log.
- **Auto-stop timer** di-clear saat `stop` manual (kill-switch) dan saat
  auto-stop memicu. `cpuTimer.unref()` biar timer gak nahan proses exit.
- **`startMemoryMonitor`**: tidak perlu diberitahu soal profiling. Memory
  monitor hanya log + panggil `global.gc()` saat RSS > 400MB — tidak ada
  alarm/restart otomatis. Snapshot heap menaikkan RSS sementara tapi di bawah
  ambang gc (guard 300MB), jadi tidak ada false alarm. Profiling CPU tidak
  menyentuh RSS. **Watchdog** (di `src/connection.js`) memantau pesan masuk,
  bukan memory — tidak terpengaruh.

---

## 5. Validasi & uji coba

- `tests/profiler.test.mjs` — `node --test tests/profiler.test.mjs`:
  - CPU: start → busy-work → stop, file `.cpuprofile` valid (nodes +
    samples/timeDeltas); start ganda ditolak; durasi > max di-cap ke 10 menit.
  - Heap: snapshot saat RSS rendah sukses & file `.heapsnapshot` valid (JSON
    dengan `snapshot.meta`); guard RSS terbukti abort saat RSS di-inflasi
    (bukan diasumsikan) — snapshot harus abort dengan `reason`.
- **Jangan langsung trigger di proses produksi utama tanpa validasi.** Uji dulu
  di lingkungan murah:
  1. Test suite di atas (jalankan di dev/Codespaces).
  2. Kalau ada session jadibot yang bisa dipakai buat trial, jalankan
     `.cpuprofile start 1` / `.heapsnap test` di situ dulu.
  3. Baru sentuh proses utama panel.

---

## 6. File yang disentuh

- `src/lib/ourin-profiler.js` — core harness (baru)
- `plugins/owner/cpuprofile.js` — command CPU (baru)
- `plugins/owner/heapsnap.js` — command heap snapshot (baru)
- `tests/profiler.test.mjs` — regression/bukti (baru)

Tidak ada file production lain yang diubah.

---

## Implementation log

### 2026-08-30 — Harness dibangun + test hijau

- `src/lib/ourin-profiler.js` selesai: CPU via `inspector.Session` +
  `Profiler.enable/start/stop`; heap via `v8.writeHeapSnapshot()` dengan guard
  RSS dua kondisi (`rss > 300MB` ATAU `rss + 2*heapTotal > 480MB`).
- Plugin `cpuprofile.js` + `heapsnap.js`: owner-only, trigger terpisah.
- `tests/profiler.test.mjs`: **5 pass, 0 fail** (`node --test tests/profiler.test.mjs`).
  - CPU: file `.cpuprofile` valid ditulis, start ganda ditolak, cap durasi
    max terbukti.
  - Heap: snapshot sukses saat RSS rendah (file `.heapsnapshot` valid), guard
    abort terbukti jalan saat RSS di-inflasi (snapshot di-abort dengan reason,
    bukan dipaksa jalan).
- Validasi runtime aktual (child process / dev, di luar proses produksi):
  - CPU window 2s (simulasi beban JSON+sqrt loop): `.cpuprofile` ditulis
    17.5KB, valid (nodes + samples), log START/STOP lengkap.
  - Heap snapshot: `.heapsnapshot` 4.8MB ditulis, valid (JSON, `meta.sample_rate`
    ada), RSS saat ambil 63.6MB — jauh di bawah ceiling 300MB. Guard RSS
    di-inflasi di test → abort dengan reason (bukan dipaksa jalan).
  - Kedua file ada di `storage/profiling/` — siap dianalisis di Prompt B.

Catatan jujur: test guard RSS membuktikan abort saat RSS tinggi di proses
test; angka produksi aktual (RSS saat .heapsnap di panel) tetap harus dicatat
di log saat eksekusi nyata, karena nilai RSS bergantung beban panel saat itu.
