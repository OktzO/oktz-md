# Design: Local Capability Resolver (putus dependensi API agregator)

## Goal
1. Ganti panggilan langsung ke API agregator (neoxr / nexray / izuka / cuki / siputzx / azbry) dengan **satu lapisan resolusi lokal** yang punya fallback berjenjang.
2. Minimalkan downtime: aggregator gratis milik orang privat boleh mati, tapi perintah tidak boleh ikut mati.
3. Jaga RAM 1GB — tidak boleh menambah beban signifikan.

## Non-Goal
- Tidak menulis ulang sistem plugin loader.
- Tidak mengubah kontrak `src/scraper/*` yang sudah ada (63 file) — itu dipakai sebagai primitif, tidak diubah interface-nya.
- Tidak membuat model AI/image/music lokal. Mustahil di RAM 1GB → `txt2img`, `anime-gen`, `musicmaker` tetap API.
- Tidak menghapus file plugin lama; migrasi mengganti isi bodinya.

## Constraints
- **RAM 1GB**, RSS limit sekarang 550MB, GC threshold 380MB, `--max-old-space-size=512`.
- **Nol dependency npm baru.**
- **Tidak ada fallback ke data basi.** Kalau semua backend gagal → error jujur ke user.
- Cache hanya untuk kapabilitas yang hasilnya identik dalam hitungan detik, TTL 30–60 detik.
- Gaya kode: komentar dalam bahasa Indonesia, menjelaskan **kenapa** — mengikuti konvensi repo.

---

## 1. Temuan awal (dasar keputusan)

### 1.1 Agregator sudah setengah mati
Status terverifikasi 2026-09-26, sudah tercatat di `.env.example`:

| Key | Status | Plugin terdampak |
|---|---|---|
| `APIKEY_NEOXR` | `api.neoxr.eu` → "apikey is required" (key ditolak) | 35 |
| `APIKEY_CUKI` | `api.cuki.biz.id` → 401 butuh key | 14 |
| `APIKEY_FIREFLY` | `firefly.maiku.my.id` → HTTP 530, host mati | ~5 |
| `APIKEY_COVENANT` | `api.covenant.sbs` → DNS gagal | 1 |
| `APIKEY_OBSCURA` | `api.obscuraworks.org` → 404, path berubah | 1 |

Artinya ~60 dari ~100 plugin terdampak **sudah rusak sebelum task ini**. Ini bukan risiko hipotetis.

### 1.2 Peta kapabilitas
~110 endpoint unik di 6 agregator, terpetakan ke bucket:

| Bucket | Plugin | Target |
|---|---|---|
| Punya scraper lokal sudah | ~18 | rewire, nol kode baru |
| Data lokal (randai/primbon/puisi/resep) | ~35 | nol network |
| Canvas lokal | ~15 | nol network |
| AI teks | ~12 | reuse multi-LLM yang ada |
| Search & stalker | ~16 | scraper baru seperlunya |
| Sulit | ~10 | API + error jujur |

Target akhir: **~85% plugin jadi nol-network atau memakai scraper yang sudah ada.**

### 1.3 Repo `RennZSync/plugins` sudah ~90% kepakai
Dari 30 scraper di sana, 26 sudah ada di `src/scraper/`. Di bagian yang berbeda, versi lokal justru lebih baik (`httpAxios` keep-alive, `tiktok.js` +169 baris dengan fallback tikwm). Empat yang belum ada justru **menambah** dependensi eksternal. **Tidak di-import** — lihat 1.3.

---

## 2. Arsitektur

Tiga lapisan, tiap lapisan satu tanggung jawab:

| Lokasi | Tanggung jawab |
|---|---|
| `src/scraper/*` (ada, 63 file) | primitif: "cara ambil data dari situs X" |
| `src/capabilities/*` (baru) | orkestrasi: "yang bisa dilakukan perintah ini" |
| `src/lib/resolve.js` (baru) | facade, circuit breaker, budget waktu, kontrak hasil |

Batas ini yang bikin tiap unit bisa dipahami dan dites sendiri: ganti isi `capabilities/spotify.js` tanpa menyentuh scraper, dan scraper tetap bisa dipakai langsung oleh plugin yang belum dimigrasi.

### 2.1 Registry lazy

`src/lib/resolve.js` **tidak** eagerly-import apa pun. Registry cuma fungsi panah:

```js
const CAPABILITIES = {
  bola:    () => import('../capabilities/bola.js'),
  zodiak:  () => import('../capabilities/zodiak.js'),
  spotify: () => import('../capabilities/spotify.js'),
};
```

~100 kapabilitas × ~50 byte ≈ **5KB**. Modul baru dievaluasi saat pertama dipakai. Ini proteksi utama RAM: kapabilitas yang tidak pernah dipanggil tidak pernah dimuat, dan modul ESM yang tidak ter-import tidak menambah heap.

### 2.2 Bentuk file kapabilitas

```js
// src/capabilities/spotify.js
import { aggregator } from '../lib/aggregator.js';

export const stable = false;              // true hanya untuk hasil deterministik <60det
export const normalize = (raw) => ({ id: raw.id, title: raw.title, url: raw.url });
export const backends = [
  { name: 'ytmusic', kind: 'local', run: (a) => spotifySearch(a.q) },
  { name: 'neoxr',   kind: 'api',   run: (a) => aggregator.hit('neoxr', '/api/spotify', a) },
];
```

Tiap modul kapabilitas mengekspor tiga hal: `backends`, `normalize`, dan `stable`.

- Tiap backend wajib punya `name` (untuk log & circuit breaker), `kind` (`'local'` | `'api'`), dan `run(args)`.
- `normalize` memaksa satu bentuk data keluar supaya plugin tidak perlu tahu backend mana yang menjawab.
- `stable` menentukan boleh tidaknya hasil di-cache (lihat 3.4). Default `false` — harus di-set eksplisit.
- `aggregator` berasal dari `src/lib/aggregator.js` (baru, kecil): satu-satunya pintu ke API agregator. Dibuat lewat `createAggregatorClient({ http, keyOf })` yang mengembalikan `{ hit }`, default `export const aggregator = createAggregatorClient()`. `hit` memakai `httpAxios` dan melempar `AggregatorError` kalau nama agregator tidak dikenal, key-nya kosong, atau host balas non-2xx. Semua panggilan agregator di dalam `src/capabilities/` lewat sini — tidak boleh ada `axios` langsung ke domain agregator.

### 2.3 Kontrak hasil

```js
{ ok: true, source: 'ytmusic', data: {...}, meta: { tookMs: 3, cached: false } }
```

Kegagalan **melempar**, bukan return `ok: false` — supaya plugin tidak bisa lupa mengecek:

```js
throw new CapabilityError('Spotify sedang tidak bisa dihubungi.', { capability, tried: [...] })
```

`CapabilityError.tried` menyimpan daftar backend yang sudah dicoba + alasannya — ini yang dipakai pesan error ke user dan log.

### 2.4 Injeksi untuk test

`resolve()` dan circuit breaker tidak di-bind ke state modul. State-nya dibuat lewat `createResolver({ capabilities, breaker, cache, now, budget })`, yang mengembalikan objek `{ resolve, breaker, cache, capabilities }`. Default `export const resolver = createResolver()` dipakai runtime; test bikin instance sendiri dengan `overrides` untuk mengganti kapabilitas/backend palsu.

`budget` menerima `{ localMs, totalMs }` supaya test bisa mengecilkan budget tanpa menunggu detik sungguhan.

Ini mencerminkan `createUploadProviders(overrides)` di `src/lib/upload-providers.js:192` — pola yang sudah terbukti di repo ini.

---

## 3. Resolusi, fallback, error

### 3.1 Urutan

1. Ambil modul kapabilitas (`await import`). Kalau nama tidak ada di registry → lempar `CapabilityError` dengan `code: 'unknown-capability'`.
2. Saring backend: buang yang breaker-nya `OPEN`. Kalau semua backend terbuang, breaker yang paling dekat ke `CLOSED` dicoba satu kali (mencegah 100% gagal permanent setelah semua host mati).
3. Coba sisanya: `kind: 'local'` dulu (urutan array), lalu `kind: 'api'` (urutan array).
4. Backend pertama yang sukses → `normalize` → kembali.
5. Semua gagal → `CapabilityError` dengan `tried[]`.

**Tidak ada langkah membaca cache lama.** Kalau upstream berubah, user dapat error, bukan data basi.

### 3.2 Circuit breaker

Tabel state per backend, **64 slot LRU** (bukan `Map` yang tumbuh unbounded).

| Kondisi | State | Aksi |
|---|---|---|
| 3 gagal berturut-turut | `OPEN` selama 60 detik | skip tanpa request |
| lewat 60 detik | `HALF_OPEN` | izinkan 1 request |
| `HALF_OPEN` sukses | `CLOSED`, reset hitung | — |
| `CLOSED` gagal | `OPEN` ulang 60 detik | — |

Tanpa breaker, satu host mati membuat tiap perintah menunggu timeout penuh. Dengan breaker, backend mati di-skip dalam mikrodetik setelah 3 kegagalan.

### 3.3 Budget waktu

Total **8 detik** per panggilan `resolve()`:
- backend `local` dibatasi 3 detik total, dibagi rata dengan jumlah backend local yang tersedia
- sisa ~5 detik untuk backend `api`
- `AbortController` per backend, jadi satu backend tidak bisa melewati porsinya

Dipakai `httpAxios` dari `src/lib/http.js` (pool keep-alive + agent global undici) — tidak membuat koneksi sendiri.

### 3.4 Cache respons

Hanya untuk kapabilitas yang ditandai `stable: true` di definisinya — yaitu yang hasilnya deterministik dalam hitungan detik (scrape fixed, quote, chord, template canvas).

- TTL 30–60 detik
- Maks **200 entri** (`lru-cache`, sudah jadi dependency)
- ~200 × 2KB ≈ **400KB** batas atas keras

Kapabilitas volatil (primbon, jadwal, info game, hasil search) **tidak pernah** di-cache. Inilah yang dijaga test #5.

---

## 4. Memori (batas 1GB)

| Aturan | Angka | Kenapa |
|---|---|---|
| Lazy import per kapabilitas | ~0,1–1,3 MB per kapabilitas di produksi | 829 plugin + 63 scraper sudah numpuk di registry ESM; jangan tambah |
| Circuit breaker | 64 slot tetap, LRU | batas atas pasti, tidak pernah tumbuh mengikuti jumlah backend |
| Response cache | 200 entri, ~400KB | `lru-cache` sudah jadi dependency |
| Data JSON | per-file on-demand | `src/data` 2.5MB total; jangan pernah load semua |
| Dependency baru | 0 | hindari native module baru |
| Overhead resolver | ≤ 8 MB RSS untuk kedelapan kapabilitas di produksi | diukur test #6, angkanya ada di §4.1 |

Yang **tidak** berubah: `memory-monitor.js` (RSS 550MB / GC 380MB) tetap jadi penjaga. Resolver tidak menambah ambang apa pun.

### 4.1 Angka RAM yang diukur

Kriteria RAM versi pertama berbunyi "import semua kapabilitas → delta RSS < 50KB". Angka itu tidak pernah diukur, dan tidak bisa dipenuhi implementasi apa pun: `src/lib/aggregator.js:1` mengimpor `config.js`, dan `config.js` sendiri sudah 74–76 MB. Angka 50KB melenceng tiga orde besaran dan menyisakan kesan bahwa biayanya pernah dicek. Angka itu diganti dengan hasil ukur di bawah, tanpa sisa.

Diukur dengan `node --expose-gc` pada commit `dbba3d2`, masing-masing di proses `node` yang baru start. Lima baris pertama adalah **delta RSS dari proses kosong** (baseline 43,8 MB); dua baris terakhir adalah **delta dari baseline produksi**, jadi angkanya tidak boleh dibandingkan langsung dengan lima baris di atasnya — itulah sumber angka 110MB yang terlihat di harness test tapi tidak pernah terjadi di bot yang sedang jalan.

| Skenario | Angka |
|---|---|
| `src/lib/resolve.js` saja (delta dari kosong) | 1,9–2,0 MB |
| `config.js` saja (delta dari kosong) | 74–76 MB |
| `resolve.js` + kedelapan kapabilitas (delta dari kosong) | 110–113 MB |
| Satu kapabilitas saja, `hd` — yang paling murah — di atas `config.js` + `resolve.js` (delta dari kosong) | 87–90 MB |
| Baseline produksi saja — `cheerio` + `ytmusic-api` + `axios` + `config.js` (delta dari kosong) | 133–142 MB |
| Baseline produksi + `resolve.js` + kedelapan kapabilitas (delta dari kosong) | 139–147 MB |
| **Marginal kedelapan kapabilitas di atas baseline produksi** | **4,9–7,0 MB** |

Dua hal yang tidak terlihat dari angka cold:

- **Biaya marginal produksi 4,9–7,0 MB** untuk kedelapan kapabilitas, yaitu 0,1–1,3 MB masing-masing — bukan 112 MB. Alasannya `index.js` memuat seluruh plugin sebelum kapabilitas pertama menyentuh apa pun, dan plugin-plugin itu sudah mengimpor `axios` (100 berkas), `config.js`, `cheerio`, dan `ytmusic-api`. Yang benar-benar baru dibayar resolver adalah modul scraper dan `sharp`; sisanya sudah resident. Angka 110–113 MB hanya terlihat di harness test yang belum memuat apa pun, dan itu bukan keadaan produksi.
- **Deferral itu benar-benar bekerja**: memuat satu kapabilitas (87–90 MB) jauh lebih murah daripada kedelapan (110–113 MB), jadi sekitar 23 MB tidak pernah dibayar kalau prosesnya tidak memakai backend yang bermasalah. Inilah yang di-hardcode oleh baris "lazy import per kapabilitas" di tabel di atas.

Kriteria yang dipakai berikutnya: **≤ 8 MB** untuk kedelapan kapabilitas dalam bentuk produksi (baseline 133–142 MB → dengan seluruh kapabilitas 139–147 MB). Diukur 4,9–7,0 MB, jadi ada 1–3 MB ruang untuk satu scraper yang belum terduga. Fase berikutnya butuh batas yang benar; batas yang dikarang lebih buruk daripada tidak ada batas, karena membuat orang berikutnya percaya bahwa angkanya pernah dicek.

---

## 5. Migrasi plugin (6 fase, independen)

Tiap fase bisa dikirim dan dihentikan sendiri. Urut dari payoff tertinggi.

### Fase 1 — Rewire ke scraper yang ada (~18 plugin)
Nol kode scraper baru, cuma ganti import.
`download/spotifydl`+`search/spotify` → `scraper/spotify.js` · `download/pindl`+`search/pin`+`search/pap` → `scraper/pindl.js` · `download/douyindl` → `scraper/douyin.js` · `download/sfiledl`+`download/videy` → `scraper/sfiledl.js`/`videy.js` · `download/ytmp3`+`ytmp4` → `scraper/ytdl.js`/`youtube.js` · `search/applemusic`+`search/playvid`+`playcall`+`playch` → `ytmusic-api` · `search/spotplay` → `scraper/spotify.js` · `tools/hd2`+`hd3` → `scraper/hd.js`/`hdvid.js`/`hdvid2.js`

### Fase 2 — Data lokal, nol network (~35 plugin) ⭐
`primbon/*` ×8 · `random/cecan*` ×6 + `barandom` · `fun/senja`,`puisi`,`fuckmylife` · `random/meme`,`quotesimage` · `search/resep`,`chords` · `fun/gachahusbu`,`gachawaifu` (pakai `asahotak.json` + `waifu.js`) · `anime/topanime` · `religi/islami` · `info/harilibur` (JSON, di-generate) · `sticker/attp`,`linesticker`,`emojimix`
→ Pemutus downtime terbesar: mustahil mati karena tidak ada network.

### Fase 3 — Canvas lokal (~15 plugin)
`canvas/*` (gura, musiccard, iqc2, fakeml, balogo, fakedana, fakebankjago, starboy, pakustad, watercolortext, tiktokchat, iqc, applemusic) + `tools/invoicemaker` → `@napi-rs/canvas` + `brat-canvas` + `src/lib/brat.js`

### Fase 4 — AI teks ke multi-LLM (~12 plugin)
`ai/characterai`,`sologo`,`matematika`,`quillbot`,`simi`,`tools/caribug` → `gpt52`/`gpt5`/`qwen3`/`deepseek`/`gemini`/`unlimitedai` yang sudah ada
→ `txt2img`, `anime-gen`, `musicmaker` **tetap API** (Non-Goal).

### Fase 5 — Search & stalker (~16 plugin)
`search/lyrics`,`pixiv`,`mangatoon`,`mcpedl`,`tiktokfoto`,`carigrup`,`sticker/stickerly`,`pinpack`, 6× stalker → scraper baru seperlunya, extend `src/lib/stalker-fallback.js`

### Fase 6 — Sulit (~10 plugin)
`tools/phisingataubukan`,`hitungwrmlbb`,`info/gag`,`gagwatch`,`tools/spamngl` → API + error jujur, kecuali diminta lokal.

---

## 6. Testing

Pola `node:test` + `mock.module` yang sudah dipakai `tests/fallback-providers.test.mjs`. Injeksi backend lewat `createResolver(overrides)`, mencerminkan `createUploadProviders(overrides)` di `src/lib/upload-providers.js`.

| # | Test | Yang dibuktikan |
|---|---|---|
| 1 | **Offline** | stub semua network jadi throw → tiap kapabilitas `kind: 'local'` tetap sukses. Ini pembuktian langsung bahwa tujuan "mandiri" tercapai. |
| 2 | **Circuit breaker** | 3 gagal → `OPEN` → panggilan berikutnya tidak invoke backend mati |
| 3 | **Urutan fallback** | local gagal → API dipanggil → hasil balik dengan `source: 'api:…'` |
| 4 | **Kontrak** | semua kapabilitas balik `{ok, source, data, meta}` |
| 5 | **Anti-basi** | kapabilitas volatil tidak pernah membaca cache (assert tidak ada hit) |
| 6 | **Memori** | impor kedelapan kapabilitas di atas baseline produksi (`cheerio`, `ytmusic-api`, `axios`, `config.js`) → delta RSS **≤ 8 MB**. Angka ini hasil ukur (4,9–7,0 MB), bukan target pilihan; cold totalnya 110–113 MB dan tidak bisa ditekan karena `aggregator.js` wajib mengimpor `config.js`. Lihat §4.1 |
| 7 | **Regresi per fase** | plugin tiap fase tetap berfungsi |

### 6.1 Audit harness

`scripts/audit-offline.mjs` — menjalankan seluruh kapabilitas dengan network diblokir, mencetak tabel kapabilitas mandiri vs masih bergantung API. Jadi dasbord untuk mengukur progres nyata tiap fase, bukan klaim.

---

## 7. Cakupan & urutan pengerjaan

Enam fase di atas **tidak** masuk satu rencana implementasi. Satu rencana = satu fase, karena tiap fase menyentuh 10–35 file plugin dan punya kriteria selesai yang berbeda. Rencana dibuat per fase, mulai dari Fase 1.

Ketergantungan: Fase 2 butuh Fase 1 (butuh `src/lib/resolve.js` + `src/lib/aggregator.js`). Fase 3–6 butuh Fase 1–2.

## 8. Yang harus terbukti sebelum dianggap selesai

- `npm test` hijau, termasuk 7 test baru.
- `npm run lint` bersih.
- `node scripts/audit-offline.mjs` → ≥85% kapabilitas bertanda lokal.
- RSS bot tidak naik signifikan setelah semua kapabilitas terpakai (dibanding baseline sebelum migrasi).
- Tidak ada pemanggilan `api.neoxr.eu` / `nexray` / `izuka` / `cuki` / `siputzx` / `azbry` di luar `src/capabilities/`.
- Tidak ada `axios`/fetch langsung ke domain agregator di dalam `plugins/`.
