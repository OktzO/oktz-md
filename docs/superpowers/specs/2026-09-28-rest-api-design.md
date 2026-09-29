# REST API terpisah (stalker/maker) — desain

Tanggal: 2026-09-28 · Status: menunggu review pemilik
Konteks: proyek baru, terpisah dari bot `ourin-md`. Bot tetap independen (punya fallback lokal).

## Latar belakang & tujuan

Bot `ourin-md` sudah punya 59 scraper + 827 plugin (stalker, downloader, canvas).
Idenya: ekstrak kemampuan itu ke **REST API publik** dengan web dokumentasi,
supaya bisa dipakai siapa pun tanpa WhatsApp.

Keputusan pemilik (dari brainstorming):

| Aspek | Keputusan |
|---|---|
| Akses | Publik terbuka, tanpa API key di MVP |
| Traffic | ~ratusan ribu request/bulan; prioritas: **stabil, tahan lama, minim maintenance** |
| Budget | **$0** total |
| Scraper | **Hybrid** — API pihak ketiga dulu, scrape sendiri sebagai fallback |
| Stack | Backend **Rust**, web modern, terpisah |
| Hosting | Web di Cloudflare **Workers** (static assets), API di **Vercel** (Rust runtime) |
| Round-robin multi-akun Vercel | **Ditunda** — disiapkan via indireksi DNS, bukan dijalankan di v1 (lihat "Risiko & keputusan") |

**Definisi sukses:** API jalan tanpa intervensi harian; provider mati tidak
men-down endpoint; traffic naik tetap muat di free tier; kalau limit benar-benar
tercapai, migrasi tanpa mengubah URL client.

## Hasil riset (dicek 2026-09-28)

| Fakta | Sumber | Implikasi |
|---|---|---|
| Vercel punya **Rust runtime resmi** (beta, semua plan, Fluid compute) | vercel.com/docs/functions/runtimes/rust | Backend Rust di Vercel legal & didukung |
| Hobby: durasi 300s, 2GB RAM / 1 vCPU, 1 jt invokasi, 100GB bandwidth | vercel.com/docs/functions/limitations | Cukup; video/bratvid ketat → phase 2 |
| Request/response body maks **4.5MB** | idem | Media hasil harus berupa URL, bukan biner |
| AUP Vercel **melarang multi-akun** untuk circumvent limit | vercel.com/legal/acceptable-use-policy | Round-robin multi-akun = risiko ban semua akun → ditunda |
| IP datacenter sering di-block Cloudflare (TikTok/IG/FB) | komunitas webscraping | Scrape langsung hanya fallback, bukan andalan |
| Nano Banana (`gemini-2.5-flash-image`) punya **free tier** ~500 gambar/hari **per project** | ai.google.dev + dokumentasi tier | Rotasi banyak API key di 1 akun Google = legal |
| Cloudflare arahkan project baru ke **Workers (static assets)**, bukan Pages | developers.cloudflare.com | Web = Workers; statis free & unlimited |
| Workers/Pages Functions free: 100k request/hari | developers.cloudflare.com/workers/platform/limits | Cukup untuk web + (nanti) gateway |
| Repo `RennZSync/plugins`: ~85% scraper = panggil API pihak ketiga tipis | analisis repo | Porting ke Rust mudah; tapi jangan port 36 file — pakai provider chain config-driven |

## Section 1 — Arsitektur & topologi

```
                          ┌─────────────────────────────────────────┐
 User / Browser ────────► │  WEB (Cloudflare Workers, static SPA)  │
                          │  Dokumentasi + Playground               │
                          │  botwaku.pages.dev / custom domain      │
                          └─────────────────────────────────────────┘

                          ┌─────────────────────────────────────────┐
 Client API ────────────► │  API (Vercel, Rust runtime, 1 akun)     │
  api.<domain> ──CNAME──► │  Cargo.toml, [[bin]] per endpoint       │
                          │  ┌─ handler ─────────────────────────┐  │
                          │  │ 1. rate-limit (per-IP, in-mem)    │  │
                          │  │ 2. route → provider chain         │  │
                          │  │ 3. respon JSON + URL media        │  │
                          │  └───────────────────────────────────┘  │
                          └───────┬─────────────────┬───────────────┘
                    ┌─────────────▼──────┐  ┌───────▼────────────────┐
                    │ PROVIDER TIER-1     │  │ PRODUKSI MEDIA          │
                    │ API pihak ketiga    │  │ - brat: resvg/tiny-skia │
                    │ fallback: scrape    │  │   (pure Rust)           │
                    │ langsung (reqwest)  │  │ - nano banana: Gemini   │
                    └─────────────────────┘  │   free tier, rotasi key │
                                             └───────┬────────────────┘
                                             ┌───────▼────────────────┐
                                             │ CLOUDFLARE R2 (free)   │
                                             │ bucket media hasil      │
                                             │ → URL publik            │
                                             └────────────────────────┘
```

Keputusan kunci:

1. **Stateless, tanpa database.** Tidak ada user table, tidak ada sesi.
   Satu-satunya state: rate-limit counter (in-memory per instance) + cache CDN.
   Tidak ada yang di-backup, tidak ada yang bisa corrupt.
2. **Caching 2 lapis:** CDN Vercel (`s-maxage`) + `stale-while-revalidate`.
   URL yang sama di-request 1000 orang = 1 invokasi.
3. **Media tidak lewat Vercel.** Hasil brat/nano banana → upload R2 → API
   balas URL. R2: 10GB free, egress $0. Vercel hanya lewat JSON puluhan KB.
4. **R2 diakses dari Rust** via S3-compatible API (crate `rust-s3` atau
   reqwest + AWS SigV4) — bukan SDK berat.
5. **Round-robin disiapkan tapi tidak dijalankan.** `api.<domain>` CNAME ke
   Vercel sekarang. Kalau limit jadi masalah → ganti target DNS / taruh
   gateway di Cloudflare Workers (project web yang sama), client tidak perlu
   mengubah apa pun.

Domain: asumsi punya 1 domain murah untuk `botwaku.web.id` +
`api.botwaku.web.id`. Tanpa domain: `*.pages.dev` + `*.vercel.app`
(custom CNAME Vercel tidak jalan tanpa domain sendiri → fitur round-robin
future sulit).

## Section 2 — Endpoint & format respon

Semua endpoint **GET** (cacheable), prefix `/api/v1/`.

Format standar:

```jsonc
// Sukses
{ "status": true, "code": 200, "creator": "botwaku", "result": { } }

// Gagal
{ "status": false, "code": 404,
  "error": "provider_unavailable",     // enum stabil, bukan teks bebas
  "message": "Semua provider untuk TikTok sedang gagal" }
```

Enum `error`: `not_found`, `invalid_url`, `provider_unavailable`,
`rate_limited`, `upstream_timeout`, `internal`.

Endpoint MVP:

```
── STALKER ──────────────────────────────────────────────
GET /api/v1/stalk/tiktok?username=xxx
GET /api/v1/stalk/instagram?username=xxx
GET /api/v1/stalk/youtube?channel=@xxx
GET /api/v1/stalk/github?username=xxx
GET /api/v1/stalk/threads?username=xxx
GET /api/v1/stalk/facebook?username=xxx          (best-effort)

── DOWNLOADER ──────────────────────────────────────────
GET /api/v1/download/tiktok?url=<link>
GET /api/v1/download/instagram?url=<link>
GET /api/v1/download/facebook?url=<link>
GET /api/v1/download/threads?url=<link>
GET /api/v1/download/twitter?url=<link>

── MAKER ───────────────────────────────────────────────
GET /api/v1/brat?text=halo&width=512&height=512
GET /api/v1/ai/image?prompt=kucing&size=1024     (Nano Banana)

── INFORMASI ───────────────────────────────────────────
GET /api/v1/status     (uptime, versi, kesehatan & latency provider)
GET /api/v1/           (daftar endpoint, auto dari registry)
```

Catatan:

- **Media → URL** (upload R2), bukan biner — jauh di bawah limit 4.5MB dan
  cacheable.
- URL masuk di-encode (`?url=` di-encode) supaya `&` link asal tidak pecah
  query.
- **Tanpa API key di MVP.** Rate-limit per-IP saja. Kalau nanti butuh tier
  premium: tambah `?key=` tanpa mengubah endpoint yang ada.
- Brat pakai GET agar `text+size` yang sama = 1 render (cache). Panjang URL
  praktis ~2KB — cukup untuk brat (teks pendek, cap 300 karakter).
- `/api/v1/` → ruang breaking change v2 tanpa mematikan client lama.

## Section 3 — Provider chain & strategi scraper

Pola: **chain per platform, urutan & status di konfigurasi data, bukan di kode.**

```
Request: GET /api/v1/download/tiktok?url=...
     ┌─ provider chain (TikTok) ─────────────────┐
     │  1. yuulabs      → API luar  (utama)      │
     │  2. tikwm        → API luar  (fallback 1) │
     │  3. vibetik      → API luar  (fallback 2) │
     │  4. scrape_tiktok → sendiri   (fallback 3) │
     └───────────────────────────────────────────┘
     tiap step: timeout 8–10s → gagal? lanjut.
     semua gagal → 503 provider_unavailable
```

Rust:

```rust
trait Provider {
    fn name(&self) -> &'static str;
    async fn fetch(&self, input: &Input) -> Result<Raw, ProviderError>;
}
// registry: Vec<(Platform, Vec<Box<dyn Provider>>)>
// urutan dibaca dari config saat startup; provider mati → matiin 1 baris
// di config, redeploy (nanti: config di KV → matiin tanpa redeploy)
```

- Setiap respon provider dinormalisasi ke **satu struct per platform**
  (`TiktokResult { title, author, media: Vec<Media> }`) — client tidak pernah
  tahu provider mana yang menang.
- Dari 36 file `RennZSync/plugins` + 59 scraper bot, yang di-port **hanya yang
  dipakai MVP** (~10 platform), dan mayoritas tipis: `reqwest` + `serde`.
- **Tanpa headless browser** (Playwright/Puppeteer tidak jalan di Vercel) —
  yang butuh itu tetap lewat API luar.

Budget waktu worst-case: 4 provider × 10s ≈ 40s < 300s. Praktis: provider
pertama jawab < 2s.

Nano Banana & Brat jalur berbeda:

- **Brat:** render pure Rust (resvg/tiny-skia + font bundle) — tidak
  tergantung pihak ketiga. Cache: hash(text+size) → URL R2 yang sama.
- **Nano Banana:** Gemini API free tier, rotasi beberapa API key (banyak
  project di 1 akun Google, legal, kuota per-project). Kena 429 → coba key
  berikutnya → quota reset harian otomatis.

## Section 4 — Rate limit, caching & anti-abuse

**Caching (pertahanan utama):**

| Endpoint | s-maxage |
|---|---|
| stalk/download | 300s (5 menit) |
| brat | 86400s (24 jam, deterministik) |
| status | 60s |

- Prasyarat: semua GET + **URL kanonikal** (query diurutkan) supaya cache key
  tidak kembar.
- `stale-while-revalidate`: respon lama dikirim sambil refresh diam-diam.

**Rate limit per-IP (in-memory, tanpa Redis):**

- 10 request/menit per IP per grup endpoint; burst 20.
- Implementasi: sliding window di `DashMap<Ip, VecDeque<Timestamp>>` dengan
  auto-cleanup. Tanpa DB/KV.
- Limit kena → 429 + `rate_limited` + header `Retry-After`.
- Kelemahan jujur: counter per-instance → limit perkiraan, bukan presisi.
  Cukup untuk MVP (menahan scraper nakal, bukan billing). Kalau butuh presisi:
  pindah KV/Upstash — komponen baru = maintenance → tunda.

**Anti-abuse lain:**

- Validasi URL: whitelist domain (`tiktok.com`, `instagram.com`, …) — cegah
  SSRF (`?url=http://169.254.169.254/`).
- Teks brat maks 300 karakter.
- Vercel Firewall rate-limit rule (bawaan Hobby) sebagai lapis kedua.
- Tanpa auth MVP → abuse parah: blokir IP via Vercel Firewall.

**Observability minimal:**

- `/api/v1/status`: per provider `healthy | degraded | down` + latency p50
  (probe internal tiap 10 menit, disimpan di memori).
- Log: Vercel log bawaan (3 hari Hobby). Tanpa Sentry/Grafana di MVP —
  tiap tool = maintenance.

## Section 5 — Error handling & testing

```
ProviderError ──normalize──► ApiError { code, error_enum, message, status }
                                   │
                                   ▼
                    respon JSON standar + logging
                    (platform, provider, durasi, err kind)
```

1. Enum `error` stabil; `message` boleh berubah, enum tidak.
2. Provider gagal ≠ error ke client — lanjut chain. Error hanya dikirim saat
   **seluruh chain habis** (503) atau input salah (400/404).
3. Handler dibungkus `catch_unwind` → 500 `internal`. Panic tidak boleh
   bocor (crash = 504 ke user).
4. Timeout per provider 8–10s, total dibatasi ~40s.
5. Upload R2 gagal → retry 1x → tetap gagal: 502 `internal` (lebih jujur
   daripada base64 5MB yang kena limit).

**Testing:**

| Lapis | Tool | Cakupan |
|---|---|---|
| Unit | `cargo test` | Normalisasi respon provider (fixture), validasi URL (SSRF), rate-limit window, kanonikal URL |
| Contract | `cargo test` + fixture | Replay JSON asli → assert struct output. Provider live **tidak** ditest di CI |
| Integration | `cargo test --features live` | Smoke provider asli — manual/on-demand |
| API e2e | `vercel dev` lokal | Format respon sesuai Section 2, enum error benar, header cache ada |
| Web | vitest + Playwright smoke | Playground vs API mock (MSW) |

Prinsip: **CI tidak pernah tergantung API orang lain.** Provider live berubah
format → ketahuan di `/status` (degraded), bukan CI merah tiap pagi.

**Definisi selesai MVP:** unit + contract hijau, e2e lokal lolos, deploy ke
Vercel + Workers, 5 endpoint utama respon benar dari production.

## Risiko & keputusan

| Risiko | Status keputusan |
|---|---|
| Multi-akun Vercel round-robin melanggar AUP → ban semua akun | **Ditunda.** Desain v1 single-akun + cache; gateway C di Cloudflare Workers bisa ditambah via DNS switch nanti tanpa ubah client |
| IP Vercel di-block untuk scrape langsung | Diterima — scrape hanya fallback tier terakhir; andalan API luar |
| ffmpeg/bratvid di Vercel (bundle 250MB, 1vCPU, 4.5MB payload) | **Phase 2**, wajib spike dulu sebelum di-commit |
| Rust runtime Vercel masih beta; ada limit env var ~4KB | Diterima — hemat env var, monitor changelog Vercel |
| Provider API luar mati mendadak (pola nyata, cf. `.env.example`) | Chain fallback + `/status` + config-driven kill-switch |
| Free tier Gemini image bisa berubah (quota 0 tiba-tiba, pola terlihat di forum) | Rotasi multi-project + pesan jelas di respon `provider_unavailable` |

## Di luar scope MVP (phase 2)

- Bratvid / render video (butuh ffmpeg — spike wajib)
- Auth / API key tier premium
- Round-robin multi-origin (Approach C) — hanya disiapkan via indireksi DNS
- Rate limit presisi (KV/Upstash)
- Bot `ourin-md` migrasi memakai API ini (v1: independen, bot tetap punya
  fallback lokal)
- Endpoint tambahan: Spotify, Pinterest, TikTok search, dll (tambah provider
  ke registry, bukan ubah arsitektur)
