# Fase 1 — Rewire Plugin ke Scraper Lokal Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Menghapus panggilan ke 6 API agregator dari 17 plugin download/search, menggantinya dengan scraper lokal yang sudah ada di `src/scraper/`, melalui satu lapisan resolusi (`resolve`) yang punya fallback berjenjang dan circuit breaker.

**Architecture:** `src/lib/resolve.js` jadi satu-satunya pintu masuk. Setiap kapabilitas = satu modul di `src/capabilities/` yang mengekspor `backends` (lokal dulu, agregator cadangan), `normalize` (satu bentuk data keluar), dan `stable` (boleh di-cache atau tidak). Modul kapabilitas dimuat `await import()` saat pertama dipakai, jadi yang tidak dipakai tidak pernah menambah heap.

**Tech Stack:** Node.js 22 ESM, `node:test` + `mock.module`, `lru-cache` (dependency existing), `httpAxios` dari `src/lib/http.js`. **Nol dependency baru.**

**Spec:** `docs/superpowers/specs/2026-09-30-local-capability-resolver-design.md` — baca sebelum mulai; plan ini mengimplementasi Bagian 2, 3, 4, dan Fase 1 dari spec tersebut.

## Global Constraints

- **RAM 1GB.** RSS limit 550MB, GC threshold 380MB (`src/lib/memory-monitor.js`) tidak boleh diubah.
- **Nol dependency npm baru.** `lru-cache` dan `axios` sudah ada.
- **Nol fallback ke data basi.** Kalau semua backend gagal → lempar `CapabilityError`. Jangan pernah mengembalikan hasil request lama.
- **Cache hanya untuk kapabilitas `stable: true`**, TTL 30–60 detik, maks 200 entri.
- **Tidak ada `axios`/fetch langsung ke domain agregator di dalam `plugins/`.** Semua lewat `src/lib/aggregator.js`.
- **Budget waktu 8 detik berlaku untuk resolusi metadata saja** — bukan untuk transfer byte media. `resolve()` mengembalikan URL; pengiriman file tetap lewat `sock.sendMessage` tanpa budget.
- Komentar kode dalam bahasa Indonesia, menjelaskan **kenapa** — mengikuti konvensi repo.
- Test: `npm test` (node:test). Lint: `npm run lint` (hanya memindai `plugins/`).
- **Path dari `tests/` satu level ke atas, bukan dua.** `tests/` ada di root repo, jadi `../src/...` benar dan `../../src/...` keluar dari repo. Ini yang sudah dipakai `tests/wink.test.mjs`.
- **Saat mocking, mock transport yang benar-benar dipakai kode yang diuji.** Kalau modul target meng-`import axios` mentah, mock `axios` — bukan `httpAxios` dari `src/lib/http.js`. Mocking wrapper yang tidak dipakai tidak hanya membuat test hijau palsu, tapi juga membiarkan test menembak jaringan sungguhan.
- **Setiap task yang membuat modul kapabilitas wajib mencabut barisnya dari `GUARDED_MISSING_IMPORTS` di `tests/rename-invariants.test.mjs`.** Invariant I1 di sana memindai semua import relatif termasuk `import()` dinamis dan menolak yang file-nya belum ada. Task 2 membuka jalan dengan menambah 8 entri ke escape hatch itu; setiap modul yang benar-benar ada harus mencabut barisnya, kalau tidak jaring pengaman itu mati diam untuk sisa repo.
- **Nama backend di circuit breaker bersifat per-host, tidak pernah per-URL atau per-user.** Breaker punya 64 slot LRU. Kalau suatu kapabilitas memakai URL atau user sebagai nama, satu host yang sehat bisa ter-evict dan langsung dianggap `CLOSED` — breaker diam-diam kehilangan daya tepat ketika paling dibutuhkan. Nama yang benar: nama backend yang dideklarasikan di `backends`, misalnya `'ilovepin'`, `'neoxr'`, `'ytdl-native'`.

## Review Focus

Lima kondisi yang paling mungkin menggigit pengguna nyata, dan perilaku yang wajar. Masing-masing sudah punya test-nya di task pemilik kodenya.

1. **URL bentuk lain yang tidak dikenali scraper** — `spotify:track:xxx`, `youtu.be/xxx`, `vm.tiktok.com/xxx`, `pin.it/xxx`. Bot harus balas pesan validasi yang jelas, bukan `Cannot read properties of undefined`. → test di Task 3, 4, 8
2. **Backend balas HTTP 200 dengan body kosong/aneh** — scraper lama hanya cek `res.data` truthy, jadi `{}` lolos dan user dapat pesan kosong. Resolver harus perlakukan "berhasil tapi tidak ada data usable" sebagai kegagalan, supaya lanjut ke backend berikutnya. → test di Task 2
3. **Validasi yang hilang saat rewire** — `plugins/download/pindl.js` sekarang punya guard `url.includes("pinterest")`. Kalau hilang saat rewire, input acak diteruskan ke scraper. → test di Task 4
4. **Aggregator key kosong di `.env`** — `aggregator.hit()` harus melempar `AggregatorError` yang jelas tanpa mengirim request sia-sia. → test di Task 2
5. **Host lokal lambat tapi sehat** — backend `local` yang butuh melebihi `LOCAL_BUDGET_MS` akan dibunuh budget. `resolve()` harus pindah ke backend lokal berikutnya atau ke agregator, bukan menggantung. → test di Task 2

---

## Struktur File

| File | Tanggung jawab (satu saja) |
|---|---|
| `src/lib/circuit-breaker.js` (baru) | Status `OPEN`/`HALF_OPEN`/`CLOSED` per backend, 64 slot LRU |
| `src/lib/capability-cache.js` (baru) | Cache respons bounded 200 entri, TTL 30–60 detik |
| `src/lib/aggregator.js` (baru) | Satu-satunya pintu ke API agregator, melempar `AggregatorError` |
| `src/lib/resolve.js` (baru) | Facade: registry lazy, urutan fallback, budget, kontrak hasil |
| `src/capabilities/spotify.js` (baru) | Spotify track download + search |
| `src/capabilities/pinterest.js` (baru) | Pinterest pin scrape + search |
| `src/capabilities/douyin.js` (baru) | Douyin download |
| `src/capabilities/sfile.js` (baru) | Sfile.mobi download |
| `src/capabilities/videy.js` (baru) | Videy.co download |
| `src/capabilities/youtube.js` (baru) | YouTube audio (mp3) + video (mp4) |
| `src/capabilities/ytmusic.js` (baru) | Apple Music / Play Audio search lewat ytmusic-api |
| `src/capabilities/hd.js` (baru) | Imglarger + unblur |
| `src/scraper/spotify.js` (modifikasi) | Diubah dari skrip CLI jadi modul yang bisa di-import |
| `scripts/audit-offline.mjs` (baru) | Harness: jalankan semua kapabilitas dengan network diblokir |
| `tests/*.test.mjs` (baru) | Test per concern, ikut konvensi `node:test` |

Plugin yang dimodifikasi (17): `download/spotifydl.js`, `search/spotify.js`, `search/spotplay.js`, `download/pindl.js`, `search/pin.js`, `search/pap.js`, `download/douyindl.js`, `download/sfiledl.js`, `download/videy.js`, `download/ytmp3.js`, `download/ytmp4.js`, `search/playvid.js`, `search/playcall.js`, `search/playch.js`, `search/applemusic.js`, `tools/hd2.js`, `tools/hd3.js`.

---

## Konstanta yang dikunci

Dipakai lintas task. Jangan diubah tanpa update test.

```
src/lib/resolve.js
  LOCAL_BUDGET_MS   = 3000
  TOTAL_BUDGET_MS   = 8000
  STABLE_CACHE_TTL_MS = 45_000

src/lib/circuit-breaker.js
  max         = 64     // slot LRU — batas jumlah, bukan batas umur
  threshold   = 3      // gagal berturut-turut sebelum OPEN
  cooldownMs  = 60_000

src/lib/capability-cache.js
  max   = 200
  ttlMs = 45_000
```

---

### Task 1: Jadikan `src/scraper/spotify.js` modul yang bisa di-import

**Files:**
- Modify: `src/scraper/spotify.js`
- Test: `tests/spotify-scraper-module.test.mjs`

**Interfaces:**
- Consumes: `httpAxios` dari `../lib/http.js` (sudah ada, `src/lib/http.js:20`)
- Produces: `downloadSpotify(spotifyUrl: string) -> Promise<{ title: string, artist: string, url: string, mime: string }>` — melempar `Error` kalau gagal. Diekspor named. CLI block dipindah ke blok `if (import.meta.url === pathToFileURL(process.argv[1]).href)` supaya `node src/scraper/spotify.js <url>` tetap jalan.

Masalah yang diperbaiki: file sekarang tidak punya `export` sama sekali dan memanggil HTTP di module scope (baris 32-34). Plugin mana pun yang meng-import-nya akan menembak request saat load.

- [ ] **Step 1: Tulis test yang gagal**

```js
// tests/spotify-scraper-module.test.mjs
import { test, mock } from 'node:test';
import assert from 'node:assert';

const calls = [];
mock.module('../src/lib/http.js', {
  namedExports: {
    httpAxios: {
      post: async (url, body) => { calls.push({ url, body }); return { data: SPOTYLOADER_OK }; },
    },
  },
});

const { downloadSpotify } = await import('../src/scraper/spotify.js');
```

Isi `SPOTYLOADER_OK` di `beforeEach`:
```js
{ downloadLink: 'https://cdn/a.mp3', post: { name: 'Judul', artist: 'Artis', mime: 'audio/mpeg' } }
```

Test:
1. `calls.length === 0` sesudah import — **ini yang gagal sekarang**, karena module scope menembak request. Ini assertion inti dari task ini: meng-import modul tidak boleh menembak jaringan. Setelah blok CLI di-guard, `process.argv[1]` saat test adalah path file test, bukan `src/scraper/spotify.js`, jadi blok itu tidak jalan dan `calls` tetap kosong.
2. `downloadSpotify('https://open.spotify.com/track/x')` mengembalikan `{ title: 'Judul', artist: 'Artis', url: 'https://cdn/a.mp3', mime: 'audio/mpeg' }`
3. body POST berisi `{ url: <input> }`
4. response tanpa `downloadLink` → `downloadSpotify` menolak (assert.rejects)
5. `http.post` melempar → `downloadSpotify` menolak, bukan swallow ke `console.error`

- [ ] **Step 2: Jalankan test, pastikan gagal**

Run: `node --experimental-test-module-mocks --test tests/spotify-scraper-module.test.mjs`
Expected: FAIL — `downloadSpotify is not a function` (tidak ada export), dan `calls.length` sudah 1 sebelum test apa pun jalan karena module-scope side effect.

- [ ] **Step 3: Ubah `src/scraper/spotify.js`**

Ganti `import axios from "axios"` jadi `import { httpAxios as axios } from "../lib/http.js"`.

Ubah `downloadSpotify` supaya: (a) **melempar** `Error` di kedua cabang catch, bukan `console.error`; (b) kalau `data.downloadLink` tidak ada, lempar `Error` yang menyebut responsnya; (c) `return { title: data.post.name, artist: data.post.artist, url: data.downloadLink, mime: data.post.mime }`.

Hapus blok pemanggilan baris 32-34, ganti dengan blok guarded:
```js
if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  const url = process.argv[2];
  if (url) downloadSpotify(url).then((r) => console.log(r)).catch((e) => console.error(e.message));
}
```
Tambahkan `import { pathToFileURL } from "node:url";` di bagian atas.

Jangan ubah apa pun yang lain di file ini — header, Referer, Origin, dan URL endpoint sudah benar dan terverifikasi.

- [ ] **Step 4: Jalankan test, pastikan lulus**

Run: `node --experimental-test-module-mocks --test tests/spotify-scraper-module.test.mjs`
Expected: PASS 5 test.

- [ ] **Step 5: Commit**

```bash
git add src/scraper/spotify.js tests/spotify-scraper-module.test.mjs
git commit -m "fix(scraper): jadikan spotify.js modul, bukan skrip CLI"
```

---

### Task 2: Inti resolver — circuit breaker, cache, aggregator, facade

Task fondasi. Semua task berikutnya bergantung pada file di sini.

**Files:**
- Create: `src/lib/circuit-breaker.js`
- Create: `src/lib/capability-cache.js`
- Create: `src/lib/aggregator.js`
- Create: `src/lib/resolve.js`
- Test: `tests/circuit-breaker.test.mjs`
- Test: `tests/capability-cache.test.mjs`
- Test: `tests/resolve-core.test.mjs`

**Interfaces:**
- Consumes: `httpAxios` dari `src/lib/http.js:20`; `config.APIkey` dari `config.js:326`
- Produces:
  - `createBreaker({ max, threshold, cooldownMs, now }) -> { isOpen(name), recordSuccess(name), recordFailure(name), snapshot() }`
  - `createCapabilityCache({ max, ttlMs, now }) -> { get(capability, key), set(capability, key, value), size() }`
  - `AggregatorError extends Error` dengan `.aggregator` dan `.status`
  - `createAggregatorClient({ http, keyOf }) -> { hit(name, path, opts) }`
  - `CapabilityError extends Error` dengan `.capability`, `.tried`, `.code`
  - `createResolver({ capabilities, breaker, cache, now, budget }) -> { resolve, breaker, cache, capabilities }`
  - `budget` opsional: `{ localMs, totalMs }`, default `{ localMs: LOCAL_BUDGET_MS, totalMs: TOTAL_BUDGET_MS }`. Ada supaya test bisa mengecilkan budget dan tidak harus menunggu detik sungguhan.
  - `export const resolver = createResolver()` (instance default)
  - `resolve(capability: string, args?: object, opts?: object) -> Promise<{ ok: true, source: string, data: any, meta: { tookMs: number, cached: boolean } }>`

Peta agregator → key, untuk `keyOf` di `aggregator.js` (dari `config.js:326`):

| Nama | `config.APIkey` | Base URL |
|---|---|---|
| `neoxr` | `neoxr` | `https://api.neoxr.eu` |
| `nexray` | — (tanpa key) | `https://api.nexray.eu.cc` |
| `izuka` | — (tanpa key) | `https://my.izuka-api.xyz` |
| `cuki` | `cuki` | `https://api.cuki.biz.id` |
| `siputzx` | — (tanpa key) | `https://api.siputzx.my.id` |
| `azbry` | — (tanpa key) | `https://api.azbry.com` |

- [ ] **Step 1: Tulis test circuit breaker yang gagal**

```js
// tests/circuit-breaker.test.mjs
import { test } from 'node:test';
import assert from 'node:assert';
import { createBreaker } from '../src/lib/circuit-breaker.js';
```

Test dengan `now` fake (bilangan bulat yang dikontrol manual, bukan `Date.now()` langsung — test harus deterministik):
1. `isOpen('neoxr')` → `false` di awal
2. dua kali `recordFailure('neoxr')` → masih `false` (threshold 3 belum tercapai)
3. `recordFailure('neoxr')` ketiga → `isOpen('neoxr')` → `true`
4. majukan `now` sebesar `cooldownMs` → `isOpen('neoxr')` → `false` (sudah `HALF_OPEN`)
5. `recordSuccess('neoxr')` setelah `OPEN` → reset, `isOpen` `false`
6. `snapshot()` berisi `neoxr`
7. **bounded**: isi 100 nama berbeda, `snapshot().length` → `64` (slot terlama evict)
8. nama berbeda punya state terpisah: `recordFailure` 3× pada `a` tidak membuat `b` jadi `OPEN`

- [ ] **Step 2: Jalankan, pastikan gagal**

Run: `node --test tests/circuit-breaker.test.mjs`
Expected: FAIL — `Cannot find module '../src/lib/circuit-breaker.js'`

- [ ] **Step 3: Implementasikan `createBreaker` di `src/lib/circuit-breaker.js`**

Struktur state per nama: `{ failures: number, openedAt: number | null }`.

`isOpen(name)`: `true` hanya kalau `openedAt !== null` **dan** `now() - openedAt < cooldownMs`. Kalau sudah lewat, kembalikan `false` (percobaan berikutnya lolos sebagai `HALF_OPEN`).

`recordFailure(name)`: `failures++`; kalau `failures >= threshold`, set `openedAt = now()`.
`recordSuccess(name)`: `failures = 0`, `openedAt = null`.

`snapshot()`: kembalikan array `{ name, failures, openedAt }` untuk slot yang ada.

Bounding: pakai `lru-cache` (`import { LRUCache } from "lru-cache"`) dengan `max: 64` dan `ttl: undefined` — jadi **hanya** batas jumlah yang membatasi, bukan umur. LRU menempelkan entri yang baru dipakai, jadi nama yang aktif tidak ter-evict. Evict lalu delete dari state.

Jangan pakai `Map` polos: itu tumbuh unbounded dan bertentangan dengan batas RAM.

- [ ] **Step 4: Jalankan test breaker, pastikan lulus**

Run: `node --test tests/circuit-breaker.test.mjs`
Expected: PASS 8 test.

- [ ] **Step 5: Tulis test cache yang gagal**

```js
// tests/capability-cache.test.mjs
import { createCapabilityCache } from '../src/lib/capability-cache.js';
```

1. `set('bola', 'a', 'v')` lalu `get('bola', 'a')` → `'v'`
2. `get('bola', 'b')` → `undefined` (key berbeda tidak bocor)
3. `get('teman', 'a')` → `undefined` (kapabilitas berbeda tidak bocor)
4. TTL: `set`, majukan `now` sebesar `ttlMs + 1`, `get` → `undefined`
5. tepat di `ttlMs` masih hidup (batas inklusif)
6. bounded: `set` 250 key, `size()` → `200`
7. yang di-evict adalah yang paling lama tidak dipakai, bukan yang paling lama disimpan

- [ ] **Step 6: Jalankan, pastikan gagal**

Expected: FAIL — `Cannot find module '../src/lib/capability-cache.js'`

- [ ] **Step 7: Implementasikan `createCapabilityCache` di `src/lib/capability-cache.js`**

`LRUCache` dengan `max: 200`, `ttl: ttlMs`. Key gabungan dari `capability` dan `key` — pakai `\u0000` sebagai pemisah supaya `('a\u0000b','c')` tidak bentrok dengan `('a','b\u0000c')`.

`get` mengembalikan `undefined` kalau tidak ada atau sudah kedaluwarsa — `LRUCache` sudah melakukan expiredasi, jangan tambah pemeriksaan manual.
`set(capability, key, value)` menyalin nilai sebelum disimpan supaya pemanggil yang mengubah objeknya tidak merusak cache.
`size()` mengembalikan `cache.size`.

- [ ] **Step 8: Jalankan test cache, pastikan lulus**

Expected: PASS 7 test.

- [ ] **Step 9: Tulis test aggregator yang gagal**

```js
// tests/resolve-core.test.mjs
import { AggregatorError, createAggregatorClient } from '../src/lib/aggregator.js';
```

1. `hit('neoxr', '/api/sfile', { params: { url: 'u' } })` memanggil `http.get` dengan URL `https://api.neoxr.eu/api/sfile?url=u` dan header `apikey` berisi key dari `keyOf('neoxr')`
2. `nexray` tidak mengirim header key sama sekali (tidak punya key)
3. `keyOf` mengembalikan `''` untuk `neoxr` → `hit` melempar `AggregatorError` dengan `.aggregator === 'neoxr'`, dan **`http.get` tidak terpanggil sama sekali**
4. respons 4xx/5xx → `AggregatorError` dengan `.status` terisi
5. error jaringan → `AggregatorError` dengan `.status === 0`
6. `hit('nama-asing', '/x')` → `AggregatorError`, bukan `TypeError` dari `undefined`

- [ ] **Step 10: Jalankan, pastikan gagal**

Expected: FAIL — `Cannot find module '../src/lib/aggregator.js'`

- [ ] **Step 11: Implementasikan `src/lib/aggregator.js`**

Isi modul: `const AGGREGATORS = { neoxr: { base, key: 'neoxr' }, ... }` dari tabel di blok Interfaces.

`keyOf(name)` default: `AGGREGATORS[name]?.key ? config.APIkey[AGGREGATORS[name].key] : ''`.

`hit(name, path, { params, method = 'GET', data, timeout = 5000 } = {})`:
- nama tidak dikenal → lempar `AggregatorError` (Review Focus #4)
- key dibutuhkan tapi kosong → lempar `AggregatorError` **sebelum** ada request
- pakai `http` yang di-inject, dengan `timeout`
- `catch` → `AggregatorError` dengan `status` dari `error.response?.status ?? 0`
- respons di luar 2xx → `AggregatorError` dengan status tersebut

- [ ] **Step 12: Jalankan test aggregator, pastikan lulus**

Expected: PASS 6 test.

- [ ] **Step 13: Tulis test facade yang gagal**

Tambahkan ke `tests/resolve-core.test.mjs`. `createResolver` menerima `capabilities` berupa object `nama -> () => import(...)`, jadi test bisa memberi modul palsu tanpa file.

Kapabilitas palsu yang dipakai test (didefinisikan inline, tidak di file terpisah):
```js
const okCap   = { stable: false, normalize: (r) => ({ v: r }), backends: [{ name: 'a', kind: 'local', run: async () => ({ raw: 1 }) }] };
const fallCap = { stable: false, normalize: (r) => ({ v: r }), backends: [
  { name: 'a', kind: 'local', run: async () => { throw new Error('mati'); } },
  { name: 'b', kind: 'api',   run: async () => ({ raw: 2 }) },
]};
const noDataCap = { stable: false, normalize: (r) => ({ v: r }), backends: [{ name: 'a', kind: 'local', run: async () => null }] };
const stableCap = { stable: true, normalize: (r) => ({ v: r }), backends: [{ name: 'a', kind: 'local', run: async () => ({ raw: 3 }) }] };
```

Test:
1. sukses lokal → `{ ok: true, source: 'a', data: { v: 1 }, meta: { tookMs, cached: false } }`
2. **Review Focus #2** — backend balas `null`: resolver **melempar** `CapabilityError` (bukan mengembalikan `{ ok: true, data: { v: null } }`), supaya lanjut ke backend berikutnya
3. fallback: `fallCap` → `source === 'b'` (membuktikan `api` dicoba setelah `local`)
4. `api` dicoba **setelah** semua `local`, bukan_before: kapabilitas dengan `[{kind:'api'...}, {kind:'local'...}]` tetap menghasilkan `source` dari backend `local`
5. semua gagal → `CapabilityError` dengan `.capability` benar dan `.tried` berisi 2 entri
6. nama tak dikenal → `CapabilityError` dengan `.code === 'unknown-capability'`
7. **anti-basi**: `okCap` dipanggil 2× → backend dipanggil 2×, `meta.cached` selalu `false`
8. `stableCap` dipanggil 2× dengan key sama → backend dipanggil **1×**, panggilan kedua `meta.cached === true`
9. `stableCap` dipanggil dengan key berbeda → backend dipanggil 2×
10. **budget** (Review Focus #5): backend `local` yang **tidak pernah selesai** (`run: () => new Promise(() => {})`). Buat resolver dengan `budget: { localMs: 120, totalMs: 400 }`. Assert: `resolve` melempar dalam waktu < 1500ms (bukan menggantung), dan backend `api` **tetap sempat dipanggil** (counter > 0). Ini membuktikan budget berlaku juga untuk backend yang hang, bukan cuma backend yang melempar error.
11. **breaker**: gagalkan `fallCap` 3× berurutan, panggilan ke-4 **tidak** memanggil backend `a` sama sekali — hitung invocation dengan counter yang bertambah di dalam `run`
12. **bailout**: semua 4 backend di `fallCap` breaker-nya `OPEN` → resolver tetap mencoba **satu** backend (bukan langsung gagal)

- [ ] **Step 14: Jalankan, pastikan gagal**

Expected: FAIL — `Cannot find module '../src/lib/resolve.js'`

- [ ] **Step 15: Implementasikan `src/lib/resolve.js`**

Isi:
- `CapabilityError` dengan `.capability`, `.tried`, `.code`
- `CAPABILITIES` — object **fungsi panah** ke `import('../capabilities/<nama>.js')`, mendaftarkan 8 nama dari Task 3–10 (`spotify`, `pinterest`, `douyin`, `sfile`, `videy`, `youtube`, `ytmusic`, `hd`). Jangan import apa pun di module scope.
- `createResolver({ capabilities = CAPABILITIES, breaker = createBreaker(), cache = createCapabilityCache(), now = Date.now, budget = { localMs: LOCAL_BUDGET_MS, totalMs: TOTAL_BUDGET_MS } })`
- `export const resolver = createResolver()`

`resolve(capability, args = {}, opts = {})`:
1. `cap = await capabilities[capability]()` — nama tak ada → `CapabilityError` `code: 'unknown-capability'`. Tangkap kegagalan import dan bungkus jadi `CapabilityError` dengan pesan yang menyebut nama file, bukan stack mentah.
2. Kunci cache = `JSON.stringify(args)` hanya kalau `cap.stable === true`
3. Kalau kunci ada dan `cache.get` mengembalikan nilai → kembali dengan `meta.cached: true`
4. Saring backend: buang yang `breaker.isOpen(name)`. Kalau hasil saringan kosong, ambil **satu** backend yang `openedAt`-nya paling baru (paling dekat ke `CLOSED`) supaya kapabilitas tidak mati permanen — ini yang dijaga test #12.
5. Bagi budget: `local` mendapat `budget.localMs` total dibagi rata antar backend local yang tersisa; `api` mendapat sisa `budget.totalMs`
6. Loop backend dalam urutan: `local` dulu (urutan array), lalu `api` (urutan array). Untuk tiap backend, jalankan `run(args)` di bawah `AbortController` dengan `Promise.race` terhadap timeout
7. Sukses tapi hasilnya `null`/`undefined` → `recordFailure` dengan alasan "tanpa data", lanjut ke backend berikutnya (test #2)
8. Sukses → `breaker.recordSuccess(name)`, `normalize(raw)`, simpan ke cache kalau `stable`, kembali `{ ok: true, source: name, data, meta: { tookMs, cached: false } }`
9. Gagal → `breaker.recordFailure(name)`, kumpulkan `{ name, reason }` ke `tried`
10. Semua gagal → lempar `CapabilityError` dengan `tried`

`now` yang di-inject dipakai untuk `tookMs`. Jangan pernah pakai `Date.now()` langsung di dalam `resolve`.

`export function resetResolver()` untuk test — mengosongkan breaker + cache instance default.

- [ ] **Step 16: Jalankan test facade, pastikan lulus**

Run: `node --experimental-test-module-mocks --test tests/resolve-core.test.mjs`
Expected: PASS 18 test (6 aggregator + 12 facade).
- [ ] **Step 17: Jalankan seluruh test suite**

Run: `npm test`
Expected: semua lulus. Total suite naik dari 684 (setelah Task 1) menjadi 717 — tambahan 33 test, yaitu 8 breaker + 7 cache + 18 resolve-core. Angka baseline di bagian header plan yang menyebut 678/696 salah: 678 itu baseline sebelum Task 1, dan 18 itu hanya hitungan `resolve-core`.

Tambahkan juga satu test yang belum ada di brief ini: **kalau semua backend gagal, `resolve()` tidak boleh membaca cache dan mengembalikan hasil lama.** Ini janji sentral spec ("tidak ada fallback ke data basi") dan saat ini tidak diuji. Bentuknya: kapabilitas `stable: true` yang sukses sekali (cache terisi), lalu dibuat gagal di panggilan kedua, lalu assert `resolve` melempar `CapabilityError` — bukan mengembalikan hasil pertama.

- [ ] **Step 18: Commit**

```bash
git add src/lib/circuit-breaker.js src/lib/capability-cache.js src/lib/aggregator.js src/lib/resolve.js tests/circuit-breaker.test.mjs tests/capability-cache.test.mjs tests/resolve-core.test.mjs
git commit -m "feat(resolve): inti resolver dengan circuit breaker dan cache bounded"
```

---

### Task 3: Kapabilitas `spotify` + rewire 3 plugin

**Files:**
- Create: `src/capabilities/spotify.js`
- Modify: `plugins/download/spotifydl.js`, `plugins/search/spotify.js`, `plugins/search/spotplay.js`
- Test: `tests/capabilities-phase1.test.mjs`

**Interfaces:**
- Consumes: `downloadSpotify(url)` dari `src/scraper/spotify.js` (Task 1); `resolver` dari `src/lib/resolve.js` (Task 2)
- Produces: `src/capabilities/spotify.js` mengekspor `backends`, `normalize`, `stable`

Kontrak `resolve('spotify', { url })` → `data: { title, artist, url, mime }` (download) dan `resolve('spotify', { q })` → `data: { tracks: [{ title, artist, url, cover }] }` (search). Mode ditentukan backend mana yang cocok; `normalize` harus bisa menerima kedua bentuk dan selalu mengembalikan `{ ...meta, tracks? }`.

- [ ] **Step 1: Tulis test yang gagal**

```js
// tests/capabilities-phase1.test.mjs
import { test } from 'node:test';
import assert from 'node:assert';
```

Test #1 — bentuk normalisasi (pakai `normalize` langsung, tanpa network):
- `normalize({ title, artist, url, mime })` → objek dengan 4 field itu
- `normalize({ tracks: [...] })` → `{ tracks: [...] }` dengan tiap track punya `title`, `artist`, `url`, `cover`
- `normalize(null)` → melempar (Review Focus #2)

Test #2 — Review Focus #1, validasi URL:
- `resolve('spotify', { url: 'spotify:track:abc' })` → melempar `CapabilityError`, dan **tidak ada** request HTTP (stub `httpAxios` dengan yang melempar kalau dipanggil)
- `resolve('spotify', { url: 'https://open.spotify.com/track/abc' })` → tidak melempar

Letakkan validasi di dalam backend `local` sebagai guard, sebelum memanggil scraper. Source code untuk guard: `/^https?:\/\/open\.spotify\.com\/(track|album|playlist)\//i` — kalau tidak cocok, lempar `Error` dengan pesan yang menyebut format yang diterima.

- [ ] **Step 2: Jalankan, pastikan gagal**

Expected: FAIL — `Cannot find module '../src/capabilities/spotify.js'`

- [ ] **Step 3: Implementasikan `src/capabilities/spotify.js`**

```js
import { downloadSpotify } from '../scraper/spotify.js';
import { aggregator } from '../lib/aggregator.js';

export const stable = false;
```

`backends`: dua backend `kind: 'local'` (`spotyloader` untuk `{ url }` via `downloadSpotify`, `ytmusic-search` untuk `{ q }` lewat `ytmusic-api` yang sudah jadi dependency), lalu satu `kind: 'api'` (`nexray`) sebagai cadangan.

`normalize` menerima kedua bentuk dan selalu mengembalikan objek dengan `title`/`artist`/`url`/`mime` atau `{ tracks }`. Kalau input tidakrecognized salah satu dari keduanya, lempar.

Set `stable = false` — judul lagu bisa berubah (mis. metadata Spotify diperbarui), jadi jangan pernah di-cache.

- [ ] **Step 4: Jalankan, pastikan lulus**

Expected: PASS

- [ ] **Step 5: Rewire `plugins/download/spotifydl.js`**

Ganti blok `axios.get(apiUrl)` + parsing `data.result` dengan:
```js
const { data } = await resolver.resolve('spotify', { url: text });
const filename = `${data.artist || "Spotify"} - ${data.title || "Audio"}.mp3`;
```
Hapus `import axios from "axios"` kalau tidak dipakai lagi di file itu. Pertahankan guard validasi link yang sudah ada di `handler` (baris ~19) dan pesan error/reaksi yang sudah ada.

- [ ] **Step 6: Rewire `plugins/search/spotify.js`**

Ganti pemanggilan `nexray/search/spotify` dengan `resolver.resolve('spotify', { q: query })`, lalu pakai `data.tracks`. Hapus `import axios` kalau tidak dipakai.

- [ ] **Step 7: Rewire `plugins/search/spotplay.js`**

File ini memanggil dua endpoint (search lalu download). Ganti keduanya: search → `resolver.resolve('spotify', { q })`, download → `resolver.resolve('spotify', { url })`.

- [ ] **Step 8: Verifikasi tidak ada domain agregator tersisa di 3 file**

Run: `grep -nE "nexray|neoxr|izuka|cuki|siputzx|azbry" plugins/download/spotifydl.js plugins/search/spotify.js plugins/search/spotplay.js`
Expected: tidak ada output.

- [ ] **Step 9: Jalankan test + lint**

Run: `npm test && npm run lint`
Expected: lulus.

- [ ] **Step 10: Commit**

```bash
git add src/capabilities/spotify.js plugins/download/spotifydl.js plugins/search/spotify.js plugins/search/spotplay.js tests/capabilities-phase1.test.mjs
git commit -m "refactor(spotify): pindah 3 plugin ke resolver lokal"
```

---

### Task 4: Kapabilitas `pinterest` + rewire 3 plugin

**Files:**
- Create: `src/capabilities/pinterest.js`
- Modify: `plugins/download/pindl.js`, `plugins/search/pin.js`, `plugins/search/pap.js`
- Test: `tests/capabilities-phase1.test.mjs`

**Interfaces:**
- Consumes: `scrapePinterest(pinUrl)` default export dari `src/scraper/pindl.js:13`; `resolver` (Task 2)
- Produces: `resolve('pinterest', { url })` → `data: { type, media: [{ type, url }] }`; `resolve('pinterest', { q })` → `data: { pins: [{ title, image, link }] }`

Struktur `media` sengaja dibuat sama dengan yang sudah dipakai `plugins/download/pindl.js:41-59`: plugin itu sudah menyusun `mediaList` berisi `{ type: 'video' | 'image', url }`, lalu mengganti `.mp4` → `.m3u8` untuk HLS. Reuse struktur itu supaya logika HLS di plugin tidak perlu ditulis ulang.

- [ ] **Step 1: Tulis test yang gagal**

Test #1 — `normalize` menerima bentuk aggregator yang lama dan mengonversi ke bentuk `media`:
- `normalize({ type: 'video', videos: [{ url: 'https://x/a720p.mp4' }] })` → `{ type: 'video', media: [{ type: 'video', url: 'https://x/a720p.mp4' }] }`
- `normalize({ type: 'image', images: [{ name: 'orig', url: 'https://x/o.jpg' }] })` → `media` berisi `{ type: 'image', url: 'https://x/o.jpg' }`
- `normalize({ type: 'image', download: 'https://x/d.jpg' })` → `media` berisi `{ type: 'image', url: 'https://x/d.jpg' }`
- `normalize({ type: 'image', images: [] })` → **melempar**, bukan `media: []` (Review Focus #2 — plugin akan mengirim 0 file tapi tetap tampil centang hijau kalau ini lolos)
- `normalize({ pins: [] })` → `{ pins: [] }` (search kosong itu sah, bedakan dari download gagal)

Test #2 — Review Focus #3, guard validasi:
- `resolve('pinterest', { url: 'https://example.com/bukan-pinterest' })` → melempar, tanpa request HTTP

- [ ] **Step 2: Jalankan, pastikan gagal**

Expected: FAIL — `Cannot find module '../src/capabilities/pinterest.js'`

- [ ] **Step 3: Implementasikan `src/capabilities/pinterest.js`**

`backends`: `kind: 'local'` (`ilovepin` via `scrapePinterest`), lalu `kind: 'api'` (`azbry`) sebagai cadangan.
`stable = false` — pin bisa dihapus kapan saja.
Guard URL: terima `pinterest.com/pin/`, `pin.it/`, dan `pinterest.*/pin/`. Tolak yang lain dengan pesan yang menyebut format yang diterima.

`normalize` mengimplementasikan keempat+1 aturan di test #1, dalam urutan: cek `pins` dulu (search), lalu `type: 'video'`, lalu `images`/`download`.

- [ ] **Step 4: Jalankan, pastikan lulus**

Expected: PASS

- [ ] **Step 5: Rewire `plugins/download/pindl.js`**

Ganti `axios.get('https://api.azbry.com/api/download/pinterest?url=...')` + blok parsing `res.data.result` dengan `const { data } = await resolver.resolve('pinterest', { url })`, lalu iterate `data.media` untuk mengisi `mediaList`.

**Pertahankan** guard `url.includes("pinterest") || url.includes("pin.it")` yang sudah ada di `handler` (baris ~33) — jangan dihapus, dan jangan digantikan dengan guard yang lebih longgar.

Hapus `import axios from "axios"` kalau tidak dipakai lagi.

- [ ] **Step 6: Rewire `plugins/search/pin.js` dan `plugins/search/pap.js`**

Keduanya memanggil `azbry/search/pinterest`. Ganti dengan `resolver.resolve('pinterest', { q })` dan pakai `data.pins`.

- [ ] **Step 7: Verifikasi + test + lint**

Run: `grep -nE "nexray|neoxr|izuka|cuki|siputzx|azbry" plugins/download/pindl.js plugins/search/pin.js plugins/search/pap.js` → kosong.
Run: `npm test && npm run lint` → lulus.

- [ ] **Step 8: Commit**

```bash
git add src/capabilities/pinterest.js plugins/download/pindl.js plugins/search/pin.js plugins/search/pap.js tests/capabilities-phase1.test.mjs
git commit -m "refactor(pinterest): pindah 3 plugin ke resolver lokal"
```

---

### Task 5: Kapabilitas `douyin` + rewire 1 plugin

**Files:**
- Create: `src/capabilities/douyin.js`
- Modify: `plugins/download/douyindl.js`
- Test: `tests/capabilities-phase1.test.mjs`

**Interfaces:**
- Consumes: `DouyinDL(url)` dari `src/scraper/douyin.js:3`; `resolver` (Task 2)
- Produces: `resolve('douyin', { url })` → `data: { title, url, thumbnail }`

- [ ] **Step 1: Tulis test yang gagal**

`normalize` menerima bentuk `DouyinDL` dan bentuk aggregator `azbry/downloader/douyin`, keduanya menghasilkan `{ title, url, thumbnail }`. `normalize(null)` dan `normalize({})` melempar.

Guard URL: terima `douyin.com`/`iesdouyin.com`/`v.douyin.com`; tolak yang lain.

- [ ] **Step 2: Jalankan, pastikan gagal**

- [ ] **Step 3: Implementasikan `src/capabilities/douyin.js`**

`backends`: `local` (`douyin-web` via `DouyinDL`), lalu `api` (`azbry`). `stable = false`.

- [ ] **Step 4: Jalankan, pastikan lulus**

- [ ] **Step 5: Rewire `plugins/download/douyindl.js`**

Ganti pemanggilan `azbry/downloader/douyin` dengan `resolver.resolve('douyin', { url })`. Pertahankan guard URL dan pesan error yang sudah ada.

- [ ] **Step 6: Verifikasi + test + lint**

Run: `grep -nE "azbry|nexray|neoxr" plugins/download/douyindl.js` → kosong. `npm test && npm run lint` → lulus.

- [ ] **Step 7: Commit**

```bash
git add src/capabilities/douyin.js plugins/download/douyindl.js tests/capabilities-phase1.test.mjs
git commit -m "refactor(douyin): pindahkan downloader ke resolver lokal"
```

---

### Task 6: Kapabilitas `sfile` + rewire 1 plugin

**Files:**
- Create: `src/capabilities/sfile.js`
- Modify: `plugins/download/sfiledl.js`
- Test: `tests/capabilities-phase1.test.mjs`

**Interfaces:**
- Consumes: default export `sfile(url)` dari `src/scraper/sfiledl.js:3`; `resolver` (Task 2)
- Produces: `resolve('sfile', { url })` → `data: { filename, url, size }`

- [ ] **Step 1: Tulis test yang gagal**

`normalize` menerima `{ download_url, filename, size }` (bentuk scraper lokal) dan `{ url, filename }` (bentuk `neoxr/sfile`) → selalu `{ filename, url, size }`. `normalize({})` melempar.

Guard URL: terima `sfile.mobi`; tolak yang lain.

- [ ] **Step 2: Jalankan, pastikan gagal**

- [ ] **Step 3: Implementasikan `src/capabilities/sfile.js`**

`backends`: `local` (`sfile-mobi`), lalu `api` (`neoxr`). `stable = false`.

- [ ] **Step 4: Jalankan, pastikan lulus**

- [ ] **Step 5: Rewire `plugins/download/sfiledl.js`**

Ganti `neoxr/api/sfile` dengan `resolver.resolve('sfile', { url })`.

- [ ] **Step 6: Verifikasi + test + lint**

Run: `grep -nE "neoxr|nexray|izuka" plugins/download/sfiledl.js` → kosong. `npm test && npm run lint` → lulus.

- [ ] **Step 7: Commit**

```bash
git add src/capabilities/sfile.js plugins/download/sfiledl.js tests/capabilities-phase1.test.mjs
git commit -m "refactor(sfile): pindahkan downloader ke resolver lokal"
```

---

### Task 7: Kapabilitas `videy` + rewire 1 plugin

**Files:**
- Create: `src/capabilities/videy.js`
- Modify: `plugins/download/videy.js`
- Test: `tests/capabilities-phase1.test.mjs`

**Interfaces:**
- Consumes: default export `videy(file)` dari `src/scraper/videy.js:6`; `resolver` (Task 2)
- Produces: `resolve('videy', { url })` → `data: { title, thumbnail, formats: [{ quality, url }] }`

- [ ] **Step 1: Tulis test yang gagal**

`normalize` menerima `{ title, thumbnail, video: {...} }` dan bentuk `neoxr/videy` → selalu `{ title, thumbnail, formats }` dengan `formats` berupa array. `normalize({})` melempar. `normalize` dengan `formats: []` melempar (jangan kembalikan array kosong — plugin akan kirim tidak ada file).

Guard URL: terima `videy.co`; tolak yang lain.

- [ ] **Step 2: Jalankan, pastikan gagal**

- [ ] **Step 3: Implementasikan `src/capabilities/videy.js`**

`backends`: `local` (`videy-co`), lalu `api` (`neoxr`). `stable = false`.

- [ ] **Step 4: Jalankan, pastikan lulus**

- [ ] **Step 5: Rewire `plugins/download/videy.js`**

Ganti `neoxr/api/videy` dengan `resolver.resolve('videy', { url })`, lalu iterate `data.formats`.

- [ ] **Step 6: Verifikasi + test + lint**

Run: `grep -nE "neoxr|nexray" plugins/download/videy.js` → kosong. `npm test && npm run lint` → lulus.

- [ ] **Step 7: Commit**

```bash
git add src/capabilities/videy.js plugins/download/videy.js tests/capabilities-phase1.test.mjs
git commit -m "refactor(videy): pindahkan downloader ke resolver lokal"
```

---

### Task 8: Kapabilitas `youtube` + rewire 5 plugin

**Files:**
- Create: `src/capabilities/youtube.js`
- Modify: `plugins/download/ytmp3.js`, `plugins/download/ytmp4.js`, `plugins/search/playvid.js`, `plugins/search/playcall.js`, `plugins/search/playch.js`
- Test: `tests/capabilities-phase1.test.mjs`

**Interfaces:**
- Consumes: `ytdl(url, format)` dari `src/scraper/ytdl.js:68`; `downloadWithFallback(url, format)` dan `getYoutubeDirectUrl(url, format)` dari `src/scraper/youtube.js:369` dan `:355`; `resolver` (Task 2)
- Produces: `resolve('youtube', { url, format })` → `data: { title, thumbnail, format, url }`, dengan `format` `'mp3'` atau `'mp4'`

Ini grup terbesar Fase 1 — 5 plugin, tapi semuanya backend `'mp3'` yang sama, jadi cukup satu kapabilitas.

- [ ] **Step 1: Tulis test yang gagal**

Test #1 — `normalize` menerima hasil `ytdl` (`{ title, thumbnail, format, url }`) dan bentuk agregator (`{ result: { title, url, ... } }`) → selalu `{ title, thumbnail, format, url }`. `normalize({})` melempar.

Test #2 — **Review Focus #1**, ini yang paling penting untuk YouTube. Bot WA Saddari banyak short link:
- `resolve('youtube', { url: 'https://youtu.be/dQw4w9WgXcQ', format: 'mp3' })` → **tidak melempar**, dan diteruskan sebagai `https://www.youtube.com/watch?v=dQw4w9WgXcQ`
- `https://m.youtube.com/watch?v=...` → diteruskan
- `https://music.youtube.com/watch?v=...` → diteruskan
- `https://www.youtube.com/shorts/...` → diteruskan
- `https://vimeo.com/12345` → melempar, tanpa request

Letakkan normalisasi short-link di backend `local` sebelum memanggil scraper: regex `/^https?:\/\/youtu\.be\/([\w-]{6,})/` → ubah jadi `https://www.youtube.com/watch?v=$1`. Regex `m.youtube.com` dan `music.youtube.com` → ganti host-nya. `youtube.com/shorts/<id>` → `watch?v=<id>`.

Test #3 — `format` default `'mp3'` kalau `args.format` tidak diberikan; `'mp4'` diteruskan apa adanya.

- [ ] **Step 2: Jalankan, pastikan gagal**

Expected: FAIL — `Cannot find module '../src/capabilities/youtube.js'`

- [ ] **Step 3: Implementasikan `src/capabilities/youtube.js`**

```js
import { ytdl } from '../scraper/ytdl.js';
import { downloadWithFallback } from '../scraper/youtube.js';
import { aggregator } from '../lib/aggregator.js';

export const stable = false;
```

`backends`: `local` pertama (`ytdl-native` untuk mp3, `youtube-fallback` untuk mp4), lalu `api` (`izuka` untuk mp3/mp4, `neoxr` untuk mp3 sebagai cadangan kedua).

`stable = false` — URL unduhan YouTube punya masa berlaku pendek, meng-cache-nya akan menghasilkan URL mati.

- [ ] **Step 4: Jalankan, pastikan lulus**

- [ ] **Step 5: Rewire 5 plugin**

| Plugin | Baris | Args |
|---|---|---|
| `download/ytmp3.js` | `izuka/downloader/ytmp3` | `{ url, format: 'mp3' }` |
| `download/ytmp4.js` | `izuka/downloader/ytmp4` | `{ url, format: 'mp4' }` |
| `search/playvid.js` | `izuka/downloader/ytmp4` | `{ url, format: 'mp4' }` |
| `search/playcall.js` | `izuka/downloader/ytmp3` | `{ url, format: 'mp3' }` |
| `search/playch.js` | `azbry/download/ytmp3` | `{ url, format: 'mp3' }` |

Di tiap file: ganti blok request aggregator + parsing dengan satu panggilan `resolver.resolve('youtube', args)`, lalu pakai `data.title` untuk nama file dan `data.url` untuk media. Hapus `import axios` kalau tidak dipakai lagi. Jangan ubah format pesan, reaksi emoji, maupun nama command.

- [ ] **Step 6: Verifikasi + test + lint**

Run: `grep -nE "nexray|neoxr|izuka|cuki|siputzx|azbry" plugins/download/ytmp3.js plugins/download/ytmp4.js plugins/search/playvid.js plugins/search/playcall.js plugins/search/playch.js` → kosong.
Run: `npm test && npm run lint` → lulus.

- [ ] **Step 7: Commit**

```bash
git add src/capabilities/youtube.js plugins/download/ytmp3.js plugins/download/ytmp4.js plugins/search/playvid.js plugins/search/playcall.js plugins/search/playch.js tests/capabilities-phase1.test.mjs
git commit -m "refactor(youtube): pindahkan 5 plugin ke resolver lokal"
```

---

### Task 9: Kapabilitas `ytmusic` + rewire 1 plugin

**Files:**
- Create: `src/capabilities/ytmusic.js`
- Modify: `plugins/search/applemusic.js`
- Test: `tests/capabilities-phase1.test.mjs`

**Interfaces:**
- Consumes: `YTMusic` dari dependency existing `ytmusic-api`; `resolver` (Task 2)
- Produces: `resolve('ytmusic', { q })` → `data: { tracks: [{ title, artist, durationSec, cover, url }] }`

**WAJIB baca `plugins/search/play2.js` sebelum menulis task ini** — itu satu-satunya pemakai `ytmusic-api` yang sudah terbukti jalan di repo ini. Salin polanya persis. Dua jebakan yang sudah ketahuan di sana dan akan menggigit kalau diabaikan:

1. **`initialize()` wajib dipanggil** setelah `new YTMusic()`, dan harus sekali saja (singleton), kalau tidak instance-nya tidak siap.
2. **`search()` mengembalikan campuran tipe.** Entry `ARTIST` dan `PLAYLIST` muncul lebih dulu dan **tidak punya `videoId`**. Harus disaring: `songs.find((s) => s.type === "SONG" && s.videoId) || songs.find((s) => s.videoId)`. Kalau tidak disaring, hasil pertama yang dipakai adalah artist, bukan lagu.

Nama field yang benar (dari `play2.js:985-995`): `track.name || track.title` untuk judul; `track.artists.map((a) => a.name).join(", ")` dengan fallback `track.artist?.name` (catatan: `artists` itu **array**, `artist` itu objek); `track.duration` dalam **detik**; `track.thumbnails[track.thumbnails.length - 1].url` untuk cover (ambil yang terakhir = paling besar); `track.videoId` untuk URL.

- [ ] **Step 1: Tulis test yang gagal**

`normalize` menerima array mentah dari `ytmusic.search()`:
- entri `{ type: 'SONG', videoId, name, artists: [{ name }], duration, thumbnails: [{ url }, { url }] }` → `{ tracks: [{ title, artist, durationSec, cover, url }] }`, dengan `cover` = URL thumbnail terakhir dan `url` = `https://www.youtube.com/watch?v=<videoId>`
- entri dengan `title` alih-alih `name` → tetap jalan
- entri `ARTIST` tanpa `videoId` **di-lewat**, bukan jadi error
- array kosong → `{ tracks: [] }` (search tanpa hasil itu sah, bukan error)
- `normalize(null)` melempar

- [ ] **Step 2: Jalankan, pastikan gagal**

Expected: FAIL — `Cannot find module '../src/capabilities/ytmusic.js'`

- [ ] **Step 3: Implementasikan `src/capabilities/ytmusic.js`**

`import YTMusic from "ytmusic-api"`. Singleton lazy di module scope (`let instance = null` + fungsi `getYTMusic()` yang memanggil `initialize()` di percobaan pertama — persis pola `play2.js:221-229`), lalu `backends: [{ name: 'ytmusic', kind: 'local', run }]`, lalu `{ name: 'nexray', kind: 'api', run }` sebagai cadangan. `stable = false`.

Saring entri di dalam `normalize`, bukan di `run`, supaya penyaringan ikut ditebak test dan konsisten untuk kedua bentuk input.

- [ ] **Step 4: Jalankan, pastikan lulus**

- [ ] **Step 5: Rewire `plugins/search/applemusic.js`**

Ganti `nexray.web.id/search/applemusic` dengan `resolver.resolve('ytmusic', { q })`, lalu render `data.tracks`.

- [ ] **Step 6: Verifikasi + test + lint**

Run: `grep -nE "nexray|neoxr" plugins/search/applemusic.js` → kosong. `npm test && npm run lint` → lulus.

- [ ] **Step 7: Commit**

```bash
git add src/capabilities/ytmusic.js plugins/search/applemusic.js tests/capabilities-phase1.test.mjs
git commit -m "refactor(ytmusic): pindahkan pencarian Apple Music ke resolver lokal"
```

---

### Task 10: Kapabilitas `hd` + rewire 2 plugin

**Files:**
- Create: `src/capabilities/hd.js`
- Modify: `plugins/tools/hd2.js`, `plugins/tools/hd3.js`
- Test: `tests/capabilities-phase1.test.mjs`

**Interfaces:**
- Consumes: `upload(filePath)` dan `get(code)` dari `src/scraper/hd.js:4` dan `:35`; `createEnhanceTask`, `pollEnhanceTask` dari `src/scraper/hdvid.js:175`; `resolver` (Task 2)
- Produces: `resolve('hd', { kind, media })` → `data: { url }`, dengan `kind` `'imglarger'` atau `'unblur'`

`hd2.js` pakai imglarger (butuh file path untuk `upload`), `hd3.js` pakai unblur. Bedakan lewat `args.kind`, jangan buat dua kapabilitas terpisah.

- [ ] **Step 1: Tulis test yang gagal**

Test #1 — `normalize` menerima `{ url }` dari `get(code)`, `{ result: { url } }` dari agregator, dan `{ data: { url } }` dari `hdvid2` → selalu `{ url }`. Semua bentuk lain melempar.

Test #2 — dispatch:
- `kind: 'imglarger'` → memakai `upload` + `get` dari `src/scraper/hd.js`
- `kind: 'unblur'` → memakai `createEnhanceTask` + `pollEnhanceTask` dari `src/scraper/hdvid.js`
- `kind` lain → melempar dengan pesan yang menyebut `'imglarger'` dan `'unblur'`

**Penting:** `pollEnhanceTask` adalah pollingberulang. Pastikan `resolve()` tidak mem-budget seluruh durasi polling — hanya panggilan `createEnhanceTask` yang masuk budget. Jelaskan di komentar kenapa: polling memang butuh waktu, mem-budgetnya akan membakar seluruh 8 detik sebelum sempat mencoba backend lain.

- [ ] **Step 2: Jalankan, pastikan gagal**

- [ ] **Step 3: Implementasikan `src/capabilities/hd.js`**

`backends`: `local` (`imglarger` atau `unblur` sesuai `args.kind`), lalu `api` (`izuka`) sebagai cadangan. `stable = false`.

- [ ] **Step 4: Jalankan, pastikan lulus**

- [ ] **Step 5: Rewire 2 plugin**

- `plugins/tools/hd2.js` → `resolver.resolve('hd', { kind: 'imglarger', media })`
- `plugins/tools/hd3.js` → `resolver.resolve('hd', { kind: 'unblur', media })`

Hapus `import axios` kalau tidak dipakai lagi di tiap file.

- [ ] **Step 6: Verifikasi + test + lint**

Run: `grep -nE "izuka|nexray|neoxr" plugins/tools/hd2.js plugins/tools/hd3.js` → kosong. `npm test && npm run lint` → lulus.

- [ ] **Step 7: Commit**

```bash
git add src/capabilities/hd.js plugins/tools/hd2.js plugins/tools/hd3.js tests/capabilities-phase1.test.mjs
git commit -m "refactor(hd): pindahkan 2 tool enhancement ke resolver lokal"
```

---

### Task 11: Test kontrak, guard anti-regresi, dan audit harness

Menutup Fase 1. Mengubah dua hal jadi permanen: tidak boleh ada domain agregator yang bocor lagi, dan harus ada cara mengukur kemajuan.

**Files:**
- Create: `tests/capability-contract.test.mjs`
- Create: `tests/no-aggregator-in-plugins.test.mjs`
- Create: `tests/offline-local.test.mjs`
- Create: `scripts/audit-offline.mjs`
- Test: keempat file di atas

**Interfaces:**
- Consumes: `resolver` (Task 2); 8 modul kapabilitas (Task 3–10)
- Produces: `npm run audit:offline` script di `package.json`

- [ ] **Step 1: Tulis guard test yang gagal**

```js
// tests/no-aggregator-in-plugins.test.mjs
import { test } from 'node:test';
import assert from 'node:assert';
import { readdir, readFile } from 'node:fs/promises';
import { join } from 'node:path';
```

Rekursif daftarkan semua `.js` di `plugins/`. Untuk tiap file, baca isinya dan assert tidak mengandung `api.neoxr.eu`, `api.nexray.`, `my.izuka-api.xyz`, `api.cuki.biz.id`, `api.siputzx.my.id`, `api.azbry.com`.

Kumpulkan **semua** nama file yang melanggar lalu assert sekali di akhir dengan daftar lengkap — jangan assert di dalam loop, supaya pesan kegagalan menunjukkan seluruh sisa dalam satu kali jalan.

Tambahkan test kedua: `src/capabilities/` boleh mengandung domain agregator, tapi **harus** lewat `aggregator.hit(...)`, bukan `axios`/`httpAxios` langsung ke domain agregator. Assert tidak ada `import axios` di file mana pun di `src/capabilities/`.

- [ ] **Step 2: Jalankan guard test, pastikan lulus**

Run: `node --test tests/no-aggregator-in-plugins.test.mjs`
Expected: PASS — Task 1–10 sudah membersihkan semua target Fase 1. Kalau FAIL, ada plugin Fase 1 yang terlewat; perbaiki, jangan longgarkan test.

- [ ] **Step 3: Tulis contract test yang gagal**

```js
// tests/capability-contract.test.mjs
```

Untuk tiap 8 kapabilitas, import modulnya langsung (bukan lewat `resolve()`, supaya test tidak butuh network) dan assert:
- `typeof backends === 'object'` dan `backends.length >= 1`
- setiap backend punya `name` (string, tidak kosong), `kind` (`'local'` atau `'api'`), `run` (function)
- **setiap kapabilitas punya minimal satu backend `kind: 'local'`** — inilah yang menjamin "mandiri"
- `typeof normalize === 'function'` dan `typeof stable === 'boolean'`
- `new Set(backends.map(b => b.name)).size === backends.length` — nama unik, kalau tidak maka circuit breaker akan salah menghitung
- `normalize(null)` melempar untuk semua kapabilitas

Test #2 — Review Focus #2 di layer kontrak: untuk setiap kapabilitas, `normalize` harus melempar pada `null`, `undefined`, `{}`, dan `[]`. Ini yang mencegah plugin menampilkan pesan kosong.

- [ ] **Step 4: Jalankan, pastikan lulus**

Expected: PASS untuk 8 kapabilitas.

- [ ] **Step 5: Tulis offline test**

```js
// tests/offline-local.test.mjs
```

Stub `src/lib/http.js` supaya `httpAxios` **selalu melempar** (`new Error('offline')`) dan `get/post/request` semuanya menolak.

Lalu untuk tiap kapabilitas, panggil `resolver.resetResolver()` lalu coba `resolve(nama, { url: 'https://contoh.test/x', q: 'x' })` dan catat hasilnya. **Jangan assert semua lulus** — yang diuji adalah **proporsi**, dan Fase 1 memang belum 85%.

Assert:
- minimal 5 dari 8 kapabilitas berhasil **tanpa network** — itu bukti bahwa resolver benar-benar memakai backend lokal
- kapabilitas yang gagal melempar `CapabilityError` (bukan `TypeError`, `ReferenceError`, atau error import) — kegagalan yang rapi, bukan crash
- tidak ada request HTTP yang bocor keluar: hitung invocation stub, dan pastikan jumlah itu 0 untuk kapabilitas yang sukses

Tambahkan print ringkasan ke output test: `mandiri tanpa network: N/8`.

- [ ] **Step 6: Jalankan offline test, pastikan lulus**

Run: `node --experimental-test-module-mocks --test tests/offline-local.test.mjs`
Expected: PASS, dengan baris ringkasan di output.

- [ ] **Step 7: Tulis `scripts/audit-offline.mjs`**

Skrip yang bisa dijalankan manual: `node scripts/audit-offline.mjs`.

Isinya: monkey-patch global `fetch`, `http.request`, `https.request`, dan `net.connect` supaya **semua** koneksi dilog dan diblokir (pakai `net.Socket.prototype.connect` untuk mencegat axios/undici yang mungkin menyimpan reference). Jalankan setiap kapabilitas dengan `resolver.resetResolver()` di antaranya. Kumpulkan hasil per kapabilitas.

Cetak tabel:
```
KAPABILITAS        MANDIRI  BACKEND DIPAKAI   CATATAN
spotify            ya      spotyloader        ok
pinterest          ya      ilovepin           ok
youtube            ya      ytdl-native        ok
...
```

Ringkasan di akhir: `${mandiri}/${total} kapabilitas mandiri tanpa network (${persen}%)`.

Exit code **0** kalau semua kapabilitas yang punya backend `local` berhasil, **1** kalau ada yang bocor ke jaringan. Ini yang dipakai CI nanti.

Tambahkan ke `package.json` scripts: `"audit:offline": "node scripts/audit-offline.mjs"`.

- [ ] **Step 8: Jalankan audit harness**

Run: `node scripts/audit-offline.mjs`
Expected: tabel tercetak, ringkasan ≥ 5/8, exit code 0.

- [ ] **Step 9: Jalankan seluruh suite + lint**

Run: `npm test && npm run lint`
Expected: semua lulus, tidak ada regresi dari 70 test file yang sudah ada.

- [ ] **Step 10: Commit**

```bash
git add tests/capability-contract.test.mjs tests/no-aggregator-in-plugins.test.mjs tests/offline-local.test.mjs scripts/audit-offline.mjs package.json
git commit -m "test: guard kontrak kapabilitas + harness audit offline"
```

---

## Verifikasi akhir Fase 1

Jalankan semua, semuanya harus benar:

```bash
npm test
npm run lint
npm run audit:offline
grep -rnE "api\.neoxr\.eu|api\.nexray\.|my\.izuka-api\.xyz|api\.cuki\.biz\.id|api\.siputzx\.my\.id|api\.azbry\.com" plugins/ || echo "BERSIH: tidak ada domain agregator di plugins/"
```

Yang harus benar:
1. `npm test` hijau
2. `npm run lint` bersih
3. `npm run audit:offline` → ≥5/8 mandiri, exit 0
4. `grep` terakhir → `BERSIH`
5.RSS bot tidak naik signifikan dibanding sebelum migrasi — cek dengan `node --expose-gc -e` yang meng-import semua kapabilitas lalu print `process.memoryUsage().rss`; delta harus di bawah 50KB per Global Constraints spec

Setelah itu, lanjut ke **Fase 2** (data lokal, nol network) dengan plan terpisah.
