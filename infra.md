# Infra — Ourin-MD v3.3.1

## Arsitektur Garis Besar

```
index.js                  → entry point, init semua subsystem
config.js                 → config tunggal + helper functions (isOwner, isPremium, dll)
case/ourin.js             → case-based command handler (minor, ~5 built-in commands)
src/connection.js         → WhatsApp WebSocket connection (Baileys)
src/handler.js            → message router (2254 line, pusat routing)
src/lib/*                 → 68 library modules
src/scraper/*             → 59 scraper modules
plugins/<kategori>/*.js   → 823 plugin files (34 kategori)
assets/                   → media assets (images, fonts, audio, video)
database/                 → runtime data files (JSON, lowdb)
session/                  → WhatsApp session files
tests/                    → node:test (4 suite) + fixtures tests/OURIN/
```

## Alur Eksekusi

```
1. index.js: main()
   ├── setupAntiCrash()     → global error handlers
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

## Message Routing (`handler.js`)

```
messageHandler(msg, sock)
  ├── serialize()          → raw WA msg → object `m` (command, args, body, dll)
  ├── filter spam/rate-limit
  ├── case handler (case/ourin.js) — built-in commands
  ├── if handled? → done
  ├── group protection (antilink, antitoxic, antidocument, dll)
  ├── game answer handler (sulap, tictactoe, suitpvp, ulartangga, family100, dll)
  ├── auto AI (auto-chat)
  ├── auto download (detect URL → download)
  ├── smart triggers (auto-reply by keyword)
  ├── sticker command handler
  ├── check permission: owner? premium? banned? group? admin? cooldown? energi?
  ├── getPlugin(command) → lookup di pluginStore
  └── plugin.handler(m, { sock, store, config, plugins })
```

## Plugin System (`src/lib/ourin-plugins.js`)

- **PluginStore**: 3 Maps — `commands` (name→plugin), `aliases` (alias→name), `categories` (kategori→plugin[])
- `loadPlugins(dir)` → scan subdirectory per kategori, `import()` tiap file `.js`, register
- `registerPlugin(plugin)` → masukkan ke store berdasarkan name + alias + category
- `getPlugin(name)` → lookup by name, fallback ke alias
- `hotReloadPlugin(filePath)` → `import()` dengan cache bust, ganti di store
- `unloadPlugin(name)` → hapus dari store
- Setiap plugin wajib export `config` (metadata) + `handler` (async function)

## Dual Routing

1. **Case** — `case/ourin.js`: switch-case sederhana, built-in, di-handle duluan di `messageHandler()`
2. **Plugin** — mayoritas command: lookup by name di `pluginStore`, eksekusi `handler()`

## Database (`src/lib/ourin-database.js`)

- **lowdb** (JSON file) — data utama: users, groups, settings, premium, transaksi, game state
- **Turso** (libsql) — session auth state (WhatsApp credentials)
- **ourin-premium-db.js** — JSON file fallback untuk owner/premium/partner list
- **ourin-jadibot-database.js** — sub-bot session data

## Session & Auth (`src/connection.js`)

- WhatsApp Multi-Device protocol via `ourin-baileys`
- Auth state: Turso (libsql) atau file-based
- Pairing code atau QR code
- Auto-reconnect dengan exponential backoff
- `ourin-reconnect.js` — reconnect logic

## Security

- `config.js`: strict number matching (`matchesNumber`) — mencegah privilege escalation via partial match
- `isOwner()` → cek config owner, DB owner, DB ownerNumbers
- `isPremium()` → owner/partner otomatis premium, cek expiry
- `isBanned()` → owner tidak bisa di-ban
- Middleware di handler: permission check sebelum eksekusi plugin
- Rate limiter (rate-limiter-flexible)
- Group protection: antilink, antitoxic, antispam, antibot, dll

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
| `lru-cache` | Performance caching |
| `undici` | HTTP client |
| `axios` | HTTP requests |
| `cheerio` | HTML scraping |
| `tesseract.js` | OCR |
| `ssh2` | SSH (VPS management) |

## Direktori Plugin

| Kategori | Jumlah | Fungsi |
|----------|--------|--------|
| owner | 147 | Eval, exec, manage bot, cap energi/premium |
| group | 101 | Antilink, welcome, mute, warn, dll |
| rpg | 67 | RPG game system |
| tools | 55 | Utility tools |
| cek | 48 | Quiz/check personality |
| ai | 47 | AI chat integration + image gen |
| search | 45 | Search engines |
| game | 36 | Interactive games |
| fun | 35 | Fun commands |
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

`npm test` → `node --test tests/`

- `number-match.security.test.mjs` — verifikasi `isOwner` strict equality (anti privilege escalation via partial match)
- `turso-db.test.mjs`, `turso-helper.test.mjs`, `turso-session.test.mjs` — Turso DB + session auth state
- `tests/OURIN/` — fixture: full copy struktur bot + database sampel untuk integration test