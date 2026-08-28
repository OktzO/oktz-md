# Infra — Ourin-MD v3.3.1

Dokumentasi teknis internal: arsitektur, alur eksekusi, routing, plugin system, database, session, security, dan library. Untuk panduan pengguna lihat `README.md`.

## Arsitektur Garis Besar

```
index.js                  → entry point, init semua subsystem
config.js                 → config tunggal + helper functions (isOwner, isPremium, dll)
case/ourin.js             → case-based command handler (minor, ~5 built-in commands)
src/connection.js         → WhatsApp WebSocket connection (Baileys)
src/handler.js            → message router (pusat routing, ~2200 baris)
src/lib/*                 → 71 library modules
src/scraper/*             → 59 scraper modules
src/tiktok/*              → 8 JSON data feed asupan TikTok
plugins/<kategori>/*.js   → 827 plugin files (34 kategori)
assets/                   → media assets (images, fonts, audio, video)
database/                 → runtime data files (JSON, lowdb, lid-cache)
tests/                    → node:test (14 suite) + fixtures tests/OURIN/
infra.md                  → dokumentasi ini
```

## Alur Eksekusi

```
1. index.js: main()
   ├── setupAntiCrash()     → global error handlers (uncaughtException, unhandledRejection)
   ├── initDatabase()       → lowdb + Turso
   ├── preloadAssets()      → cache assets ke memory
   ├── loadPlugins()        → scan plugins/*, load + register tiap plugin
   ├── initScheduler()      → cron jobs
   └── startConnection()     → WhatsApp WebSocket
        ├── onRawMessage     → anti-tag status
        ├── onMessage        → messageHandler() (core router)
        ├── onGroupUpdate    → groupHandler()
        ├── onMessageUpdate  → messageUpdateHandler()
        ├── onGroupSettingsUpdate → groupSettingsHandler()
        ├── onStubMessage    → anti-remove
        └── onConnectionUpdate
             └── connection "open" → loadScheduledMessages, startGroupScheduleChecker,
                 startSewaChecker, initScheduler, initAutoJpmScheduler,
                 initSholatScheduler, initNotifScheduler, initSahurCron,
                 restore jadibot sessions, startMemoryMonitor, startTempCleaner,
                 startDailyPruner
```

## Message Routing (`src/handler.js`)

```
messageHandler(msg, sock)
  ├── serialize()          → raw WA msg → object `m` (command, args, body, dll)
  ├── filter spam/rate-limit (globalRateLimiter + spamDelayTracker)
  ├── case handler (case/ourin.js) — built-in commands
  ├── if handled? → done
  ├── group protection (antilink, antitoxic, antidocument, antisticker, dll)
  ├── game answer handler (sulap, tictactoe, suitpvp, ulartangga, family100, dll)
  ├── auto AI (auto-chat), auto download (detect URL → download)
  ├── smart triggers (auto-reply by keyword)
  ├── sticker command handler
  ├── CMD VN handler (voice note → transkripsi Groq → eksekusi command)
  ├── check permission: owner? premium? banned? group? admin? cooldown? energi?
  ├── getPlugin(command) → lookup di pluginStore
  └── plugin.handler(m, { sock, store, config, plugins })
```

## Plugin System (`src/lib/ourin-plugins.js`)

- **PluginStore**: 3 Maps — `commands` (name→plugin), `aliases` (alias→name), `categories` (kategori→plugin[])
- `loadPlugins(dir)` → scan subdirectory per kategori, `import()` tiap file `.js`, register
- `registerPlugin(plugin)` → masukkan ke store berdasarkan name + alias + category
- `getPlugin(name)` → lookup by name, fallback ke alias
- `hotReloadPlugin(filePath)` → `import()` dengan cache bust, ganti di store (mode dev)
- `unloadPlugin(name)` → hapus dari store
- Setiap plugin wajib export `config` (metadata) + `handler` (async function)
- Default config di-merge dari `defaultConfig` — field opsional tidak wajib didefinisikan

### Field Plugin Config

| Field | Tipe | Default | Fungsi |
|---|---|---|---|
| `name` | string \| string[] | (nama file) | Command utama / multi-name |
| `alias` | string[] | `[]` | Alias command |
| `category` | string | `"uncategorized"` | Kategori (diisi otomatis dari folder) |
| `description` | string | `"No description"` | Deskripsi |
| `usage` / `example` | string | `""` | Panduan pemakaian |
| `isOwner` / `isPremium` | boolean | `false` | Pembatasan akses |
| `isGroup` / `isPrivate` | boolean | `false` | Pembatasan konteks chat |
| `isAdmin` / `isBotAdmin` | boolean | `false` | Pembatasan role grup |
| `cooldown` | number | `3` | Cooldown detik |
| `limit` | number | `1` | Konsumsi energi |
| `isEnabled` | boolean | `true` | Aktif/nonaktif |

## Dual Routing

1. **Case** — `case/ourin.js`: switch-case sederhana, built-in, di-handle duluan di `messageHandler()`. Hanya ~5 command (ping, latency, listcase, listplugin).
2. **Plugin** — mayoritas command: lookup by name di `pluginStore`, eksekusi `handler()`. ~827 plugin.

## Database (`src/lib/ourin-database.js`)

- **lowdb** (JSON file) — data utama: users, groups, settings, premium, transaksi, game state
- **Turso** (libsql) — session auth state (WhatsApp credentials), sync remote
- **ourin-premium-db.js** — JSON file fallback untuk owner/premium/partner list
- **ourin-jadibot-database.js** — sub-bot session data
- **lid-cache.json** — cache resolusi LID (location ID) → JID, mencegah re-resolve berulang

### Turso Fallback
- `flushAllToTurso()` — tulis semua key ke Turso; saat `batch()` gagal, fallback ke `execute()` sequential
- Saat Turso down → `save()` return false dan tulis ke file lokal (tidak kehilangan data)
- Session auth state: atomic write per-key batch

## Session & Auth (`src/connection.js`)

- WhatsApp Multi-Device protocol via `ourin-baileys`
- Auth state: Turso (libsql) atau file-based
- Pairing code atau QR code
- Auto-reconnect dengan exponential backoff
- LID (Location ID) resolution: `src/lib/ourin-lid.js` — konversi `@lid` ↔ JID, fallback ke `@lid` bila tidak resolve

## Security

- `config.js`: strict number matching (`matchesNumber`) — mencegah privilege escalation via partial match
- `isOwner()` → cek config owner, DB owner, DB ownerNumbers
- `isPremium()` → owner/partner otomatis premium, cek expiry
- `isBanned()` → owner tidak bisa di-ban
- Middleware di handler: permission check sebelum eksekusi plugin
- Rate limiter (rate-limiter-flexible): `globalRateLimiter` 8 points / 3s + `spamDelayTracker`
- Group protection: antilink, antitoxic, antispam, antibot, antidocument, antisticker, antimedia, anti-hidetag, anti-phishing, anti-judol, anti-remove
- Anti-crash guard: global uncaughtException + unhandledRejection handler
- Secret management: semua API key dipindah ke `.env` (tidak di-commit)

## Key Libraries

| Library | Fungsi |
|---------|--------|
| `ourin-baileys` | WhatsApp MD protocol (fork of @whiskeysockets/baileys) |
| `lowdb` | JSON file database |
| `@libsql/client` | Turso/libsql edge database |
| `@napi-rs/canvas` | Canvas rendering (welcome card, dll) |
| `sharp` | Image processing |
| `fluent-ffmpeg` | Audio/video processing |
| `pino` | Logging (Baileys internal) |
| `cron` | Job scheduling |
| `rate-limiter-flexible` | Rate limiting |
| `node-webpmux` | Webp EXIF (sticker metadata) |
| `lru-cache` | Performance caching (apimanager, thumb) |
| `undici` | HTTP client |
| `axios` | HTTP requests |
| `cheerio` | HTML scraping |
| `tesseract.js` | OCR |
| `ssh2` | SSH (VPS management) |
| `btch-downloader` | Media downloader (FB, CapCut, ttdl) |
| `@google/generative-ai` | Gemini AI |
| `google-tts-api` | Text-to-speech |

## Direktori Plugin (827 file, 34 kategori)

| Kategori | Jumlah | Fungsi |
|----------|-------:|--------|
| owner | 147 | Eval, exec, manage bot, cap energi/premium |
| group | 101 | Antilink, welcome, mute, warn, dll |
| rpg | 67 | RPG game system |
| tools | 55 | Utility tools |
| cek | 48 | Quiz/check personality |
| ai | 47 | AI chat integration + image gen |
| search | 45 | Search engines |
| fun | 39 | Fun commands |
| game | 37 | Interactive games |
| canvas | 31 | Image generation |
| download | 26 | Media downloaders |
| sticker | 22 | Sticker creation |
| panel | 22 | Hosting panel (Pterodactyl, DO, Linode, CPanel) |
| main | 20 | Core commands (menu, ping, stats) |
| user | 17 | User profile |
| stalker | 15 | Profile stalking |
| store | 14 | Store system |
| info | 14 | Information |
| random | 12 | Random content |
| clan | 9 | Clan system |
| primbon | 8 | Fortune telling |
| vps | 6 | VPS management |
| religi | 4 | Religious content |
| asupan | 4 | Social media content |
| utility | 3 | Notifikasi makan/tidur, inspect |
| anime | 3 | Top anime, waifu, auto-anime |
| tts | 2 | Text-to-speech |
| nsfw | 2 | NSFW (gated) |
| media | 2 | Media processing |
| islamic | 2 | Quran, murrotal |
| pushkontak | 1 | Push contact massal |
| jpm | 1 | Jadwal pesan massal |
| ephoto | 1 | Ephoto templates |
| convert | 1 | Audio converter |

## Scraper & Downloader

### TikTok Downloader (`.tt`, `.tt2`, `.ttmp3`)
Fallback chain otomatis agar tetap bekerja saat satu provider diblokir:

```
src/scraper/tiktok.js (ttdown — dipakai ttmp3 & internal)
  tikwm → savett → yuulabs → musicaldown

plugins/download/tiktokdl.js (tiktokDl — dipakai .tt)
  tikwm → savett(raw) → savett(resolved)

plugins/download/tiktokdl2.js (tt2 — savett)
  savett.cc (csrf token + form POST + cheerio parse)
```

- **tikwm**: `POST https://www.tikwm.com/api/` — cepat, tapi sering kena Cloudflare 403 di IP datacenter
- **savett**: `https://savett.cc/en1/download` — butuh token CSRF + cookie, toleran ke raw shortlink (`vt.tiktok.com`)
- **Shortlink resolution**: `resolveTikTokUrl()` ikuti redirect `vt/vm.tiktok.com` → full URL
- **Photo/slide**: parse `data-data` tiap carousel, ambil 1 URL terbaik per slide + MP3 dari formatselect

## Case Commands

Built-in di `case/ourin.js`:
- `cping`, `cspeed`, `clatency` — ping/latency
- `listallcase`, `lcase`, `caselist`, `allcase` — daftar case
- `listallplugin`, `lplugin`, `pluginlist`, `allplugin` — daftar plugin

## Dev Mode

- `NODE_ENV=development` → dev mode aktif
- `config.dev.watchPlugins` → hot-reload plugin saat file berubah
- `config.dev.debugLog` → stack trace di error
- Anti-crash: uncaughtException + unhandledRejection handler
- SIGINT/SIGTERM: save database, exit safe

## Testing

`npm test` → `node --test tests/` (14 suite, 97 assertions)

- `number-match.security.test.mjs` — verifikasi `isOwner` strict equality (anti privilege escalation via partial match)
- `turso-db.test.mjs`, `turso-helper.test.mjs`, `turso-session.test.mjs`, `turso-session-atomic.test.mjs` — Turso DB + session auth state + atomic batch/sequential fallback
- `waifu-data.test.mjs`, `waifu-lib.test.mjs`, `husbu-data.test.mjs`, `husbu-lib.test.mjs`, `romance-lib.test.mjs` — gacha data pool (300+ entri), pity system, aksi/mood/jealousy
- `afk.test.mjs` — persistence + alias guard + mention throttle
- `lid-resolve.test.mjs` — LID→JID resolution + fallback
- `logger-command.test.mjs` — log command tanpa nomor user
- `memory-leaks.test.mjs` — cron job leak, cache cap (antispam, waifupool pages)
- `tests/OURIN/` — fixture: full copy struktur bot + database sampel untuk integration test

## Troubleshooting Umum

| Gejala | Penyebab | Solusi |
|---|---|---|
| `.tt` gagal semua | Semua provider kena Cloudflare 403 di IP server | Cek log `[tiktokDl]`, butuh proxy/residential IP atau API berbayar (TikHub) |
| Plugin tidak ke-load | Error syntax / import rusak | `node --check plugins/<file>.js`, cek log `plugin failed <file>` |
| Bot restart terus | OOM | Aktifkan `--max-old-space-size=512` (sudah di npm scripts), cek memory monitor |
| Turso error | Network / batch tidak didukung | Sudah ada fallback sequential + file lokal, cek `[turso]` log |
| Session hilang | Turso down / session expired | Auto-recovery main-session + restore jadibot di `connection open` |
