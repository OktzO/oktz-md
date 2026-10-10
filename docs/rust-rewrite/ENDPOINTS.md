# ENDPOINTS — audit endpoint eksternal `oktz-md`

Generated: **2026-10-10** · Metode: inventarisasi `grep` host dari `plugins/`, `src/`, `src/capabilities/`, `src/scraper/`, lalu 1 request per host via Firecrawl (`maxAge: 0`).
**Verdict berbasis response nyata, bukan komentar `.env.example`.** Semua yang tidak bisa diverifikasi ditulis `tidak terverifikasi` — tidak ditebak.

> Penting untuk porting Rust: tiap host di sini butuh **fixture offline** di `rust/tests/fixtures/<scraper>/` supaya test tidak bergantung jaringan.

---

## 0. Koreksi terhadap `.env.example`

`.env.example` punya komentar status host bertanggal 2026-09-26. **Empat komentar salah/kedaluwarsa:**

| Komentar lama | Kenyataan terverifikasi |
|---|---|
| `APIKEY_COVENANT -> api.covenant.sbs` "DNS gagal (fetch failed)" | **Salah.** Host hidup, balas JSON. Path `/melolo` sudah hilang → `404` problem+json |
| `api-faa.my.id` "WAF Cloudflare 403, perlu whitelist IP" | **Kedaluwarsa.** Tidak kena Cloudflare. `404` HTML dari aplikasinya sendiri |
| `APIKEY_FGSI -> fgsi.dpdns.org` "belum dicek" | **Hidup.** `500 {"Method GET not allowed"}` — endpoint POST-only |
| `APIKEY_NEOXR -> api.neoxr.eu` "key ditolak" | Host hidup, tapi branding pindah → `api.neoxr.my.id` |

**Aksi:** proses porting tidak boleh mempercayai `.env.example` sebagai sumber kebenaran status endpoint. `ENDPOINTS.md` ini yang jadi acuan.

---

## 1. API butuh key / auth

| host | plugin (contoh) | status | bukti | catatan |
|---|---|---|---|---|
| `api.neoxr.eu` | chords, txt2img, emojimix, apkmod, senja, puisi, pixiv, film, sfiledl, musikapaini | **berubah** | `GET /api/chord?apikey=test` → `200 {"creator":"@neoxr.js – Wildan Izzudin","status":false,"msg":"Sorry, apikey is not registered. © https://api.neoxr.my.id"}` | Prefix path `/api/`. Branding → `api.neoxr.my.id`; cek apakah `.eu` redirect |
| `api.cuki.biz.id` | caribug, spamngl, lahelu, mangatoon, tiktokfoto, mcpedl, watercolortext, starboy | **butuh-key** | `GET /api/random/lahelu` → `401 {"success":false,"error":"API key required. Get your API key from /auth/dashboard"}` | ✅ sesuai `.env`. Prefix `/api/{ns}/{slug}` |
| `api.covenant.sbs` | melolo | **berubah** | `GET /melolo?url=x` → `404 {"kind","title","status","code","detail","instance","request_id"}` | Host hidup, path mati. Auth header `x-api-key` |
| `fgsi.dpdns.org` | hdvid, hdvid2, img2img, enchantVideo | **butuh-key** | `GET /api/ai/image/img2img` → `500 {"developer","status","message":"Method GET not allowed","data"}` | Semua endpoint POST. Header `apikey` |
| `api.obscuraworks.org` | am-data (nikparser sudah fallback) | **berubah** | `GET /api/v2/tools/nik?nik=…` → `404` | Host hidup, path lama mati |
| `api.overchat.ai` | gpt5, qwen3, claudehaiku | **butuh-key** | `POST /v1/chat/completions` → `403 {"message","error","statusCode"}` | OpenAI-compatible |
| `api-pddikti.kemdiktisaintek.go.id` | pddikti | **berubah** | `GET /search?q=tipes` → `401 {"message":"Unauthorized"}` | Dulu API publik, kini butuh token |

## 2. API gratis tanpa key — **sumber utama fixture test**

| host | plugin | status | shape response (untuk fixture) |
|---|---|---|---|
| `api.nexray.eu.cc` | gag, matematika, quillbot, simi, sologo, lyrics, stalker-*, tiktoksearch | **hidup** | envelope: `{status: bool, author, result, timestamp, response_time}` → `200 {"status":true,...}` |
| `api.open-meteo.com` | menucat, menu, allmenu | **hidup** | `{latitude, longitude, generationtime_ms, utc_offset_seconds, timezone, current_units, current}` |
| `lrclib.net` | play2 | **hidup** | `{id, name, trackName, artistName, duration, plainLyrics, syncedLyrics, ...}` |
| `ipwho.is` | ipwho | **hidup** | `{ip, success, country_code, city, latitude, longitude, timezone, ...}` |
| `data.bmkg.go.id` | gempa | **hidup** | `{"Infogempa":{...}}` |
| `api.myquran.com` | architectures (`sholat-api.js`) | **hidup** | `{status, request, data}` |
| `api.siputzx.my.id` | primbon/* (9 plugin), islami, pinpack | **hidup** | `{status, result, ...}` / error `400 {status, error, code}` |
| `my.izuka-api.xyz` | topanime, stickerly, animeapaini, applemusic, iqc, fakecall, hd, youtube | ** hidup** | envelope `{status, creator, result, timestamp, developer}` |
| `api.theresav.eu` | brat | **hidup** | **biner** `image/png` mentah, tanpa key. Fixture paling gampang |
| `api.hackertarget.com` | lookup | **hidup** | **`text/plain`**, bukan JSON. Baris `A:`, `AAAA:`, `MX:`, `NS:`, `TXT:`, `SOA:` |
| `api.memegen.link` | smeme | **hidup** | **biner** `image/png` |
| `www.tikwm.com/api/` | tiktokdl, tiktokdl2, aio | **hidup** | `{code, msg, processed_time}` → sukses `code:0` + `data.*` |

## 3. Scraping HTML

| host | plugin | status | bukti |
|---|---|---|---|
| `media.savetube.vip` | play2, youtube | **hidup** | `GET /api/random-cdn` → `200 {"cdn":"…"}`. kecil & stabil, fixture bagus |
| `savett.cc` | tiktokdl, tiktokdl2, aio | **berubah** | `/en1/download?url=…` → **redirect ke `/en1/`** (homepage SaveTT). Path download perlu verifikasi dengan URL TikTok valid |
| `ssyoutube.com` | — | **mati (region)** | redirect ke `/blocked`: "Services in the United States have been discontinued". `media.ssyoutube.com` ikut mati |
| `api.waifu.im` | mywaifu | **tidak terverifikasi** | `404` `application/problem+json` — kemungkinan param/path berubah |

> Catatan: `ssyoutube.com` **tidak dipakai plugin mana pun** — hanya muncul di `src/scraper/yt.js`, dan tidak ada plugin yang meng-import helper itu. Perlu dicek terpisah saat porting `src/scraper/yt.js`.

**Tidak terverifikasi** (perlu batch lanjutan): `yt.savetube.me`, `save-tube.com`, `imglarger.com`, `get1.imglarger.com`, `photoai.imglarger.com`, `imgupscaler.com`, `www.nerdfonts.com`, `rednote.savevideodown.com`, `code.rifkyshre.biz.id`, `cors.rifkyshre.biz.id`, `chat.dphn.ai`, `api.termai.cc`, `c.termai.cc`, `telegra.ph`, `api.internal.temp-mail.io`, `api.proactor.ai`, `api.unwatermark.ai`, `api.imgupscaler.ai`, `pastebin.com`, `registry.npmjs.org`, `api.github.com`, `api.yuulabs.web.id`, `api.yupra.my.id`, `api.zenzxz.my.id`, `api.ourin.my.id`, `api.nexray.web.id`, plus uploader `uguu.se`/`kappa.lol`/`litterbox.catbox.moe`/`cdn.nekohime.site`/`leopard.hosting.pecon.us`/`www.upload.ee`.

**Bukan endpoint runtime** (file data, jangan dipindah ke tabel ini): `www.cademedia.com` (1000 hit), `telegra.ph` (793), `konachan.com`, `top4top.io` — ini `src/data/*.json` lokal.

---

## 4. Mati / percuma

| host | plugin | bukti |
|---|---|---|
| `firefly.maiku.my.id` | igstalk, ytstalk, text2img, pinvid, autoai | `530` Cloudflare `Error 1033` — Argo tunnel down, mati total |
| `videy.co` / `cdn.videy.co` | videy | `522 Connection timed out` |

## 5. Ekuivalen gratis untuk yang mati / butuh-key

1. **Chat AI** (`api.overchat.ai`) → self-host router OpenAI-compatible: https://github.com/openfreerouter/freerouter — expose `/v1/chat/completions`, satu provider bisa ganti gpt5/qwen3/claudehaiku sekaligus.
2. **TTS** (`api.emiliabot.my.id`) → Puter.js free unlimited TTS: https://developer.puter.com/tutorials/free-unlimited-text-to-speech-api/ — tanpa signup, tapi JS SDK bukan REST murni. Perlu cek CLI/spawn headless.
3. **text2img** (`firefly`, sudah mati total) → **tidak terverifikasi**. Kandidat self-host (ComfyUI/Peebles gateway) harus riset terpisah sebelum dipakai. Jangan karang.
4. **PDDikti** (kini 401) → **tidak terverifikasi** ada mirror publik. Jangan diasumsikan.

---

## 6. Bentuk response — implikasi desain klien HTTP

Tiga shape berbeda wajib ditangani eksplisit di `oktz-scrapers` (jangan pakai `responseType: json` global):

| Shape | Contoh | Rust |
|---|---|---|
| Envelope generik | nexray, izuka: `{status, creator, result, timestamp}` | `ApiEnvelope<T>` generik dengan `serde` |
| Problem+json | covenant, waifu.im: `{kind, title, status, code, detail, instance}` (Fastly) | `ProblemDetail` — **beda** dari error neoxr (`{status:false,msg}`) dan cuki (`{success:false,error}`) |
| Non-JSON | hackertarget = `text/plain` | parser plain text |
| Biner | memegen.link, theresav.eu = `image/png` | `bytes()`, bukan `json()` |

Tiga enum error HTTP yang berbeda di atas harus dibedakan di enum respons — jangan disatukan jadi satu `ApiError`, karena plugin JS saat ini menampilkan `msg`/`error` yang berbeda-beda ke user dan itu bagian dari paritas.