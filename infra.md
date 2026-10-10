# Infra — foto-md v3.3.1

Dokumentasi teknis internal: arsitektur, alur eksekusi, routing, plugin system, database, session, security, dan library. Untuk panduan pengguna lihat `README.md`.

## Arsitektur Garis Besar

```
index.js                  → entry point, init semua subsystem
config.js                 → config tunggal + helper functions (isOwner, isPremium, dll)
case/foto.js             → case-based command handler (minor, ~5 built-in commands)
src/connection.js         → WhatsApp WebSocket connection (Baileys)
src/handler.js            → message router (pusat routing, ~2200 baris)
src/lib/*                 → 87 library modules
src/scraper/*             → 59 scraper modules
src/tiktok/*              → 8 JSON data feed asupan TikTok
plugins/<kategori>/*.js   → 829 plugin files (34 kategori)
assets/                   → media assets (images, fonts, audio, video)
database/                 → runtime data files (JSON, lowdb, lid-cache)
tests/                    → node:test (71 file) + fixtures tests/foto/ (gitignored)
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
  ├── case handler (case/foto.js) — built-in commands
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

## Plugin System (`src/lib/plugins.js`)

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

1. **Case** — `case/foto.js`: switch-case sederhana, built-in, di-handle duluan di `messageHandler()`. Tiga grup command (11 nama): `cping`/`cspeed`/`clatency`, `listallcase`/`lcase`/`caselist`/`allcase`, `listallplugin`/`lplugin`/`pluginlist`/`allplugin`.
2. **Plugin** — mayoritas command: lookup by name di `pluginStore`, eksekusi `handler()`. 829 file plugin.

## Database (`src/lib/database.js`)

- **lowdb** (JSON file) — data utama: users, groups, settings, premium, transaksi, game state
- **Turso** (libsql) — session auth state (WhatsApp credentials), sync remote
- **premium-db.js** — JSON file fallback untuk owner/premium/partner list
- **jadibot-database.js** — sub-bot session data
- **lid-cache.json** — cache resolusi LID (location ID) → JID, mencegah re-resolve berulang

#### LID (username) di user store

- User dengan username WhatsApp (JID `...@lid`) **tidak disimpan sebagai nomor**. Key di store users memakai prefix `lid:`, mis. `lid:165648885899430`.
- `getUser`/`setUser`/`deleteUser` LID-aware: `user.lid=<lid>`, `user.number=null`, `user.jid=null`. Tidak ada fabrikasi nomor `+44`/`+1` palsu untuk LID.
- Deteksi LID via `isLidLikeJid()` (digit-only + `isLidConverted`) sehingga jid sintetik game (`husbu_*`, `waifu_*`) tidak salah klasifikasi.
- `migrateLegacyUsers()` dijalankan saat boot satu kali: memindahkan key LID lama (nomor digit polos) ke `lid:<num>`, dan membersihkan record sintetik `husbu_KiseRyota`.
- Render nama: `getName`/`getNameFromParticipants` return `"Unknown"` untuk LID yang tak ter-resolve — tidak menampilkan `+<number>` palsu.

### Turso Fallback
- `flushAllToTurso()` — tulis semua key ke Turso; saat `batch()` gagal, fallback ke `execute()` sequential
- Saat Turso down → `save()` return false dan tulis ke file lokal (tidak kehilangan data)
- Session auth state: atomic write per-key batch

## Session & Auth (`src/connection.js`)

- WhatsApp Multi-Device protocol via `foto-baileys`
- Auth state: Turso (libsql) atau file-based
- Pairing code atau QR code
- Auto-reconnect dengan exponential backoff
- LID (Location ID) resolution: `src/lib/lid.js` — konversi `@lid` ↔ JID, fallback ke `@lid` bila tidak resolve

### Batas tanggung jawab session store (hasil audit 2026-10-10)

`src/lib/turso-session.js` **bukan** duplikat dari signal layer `onigis`. Keduanya
bekerja pada lapisan berbeda dan keduanya dipertahankan:

| Lapisan | Pemilik | Tugas |
|---|---|---|
| Kriptografi | `onigis` (via `oktz-signal` + `oktz-curve25519`) | session record, pre-key, encrypt/decrypt |
| Persistensi | `src/lib/turso-session.js` | nyimpan/baca auth state, prune, recovery file korup |

Bot tetap memakai `makeCacheableSignalKeyStore` dari `onigis` sebagai key store;
`makeWASocket` di dua entry point (`src/connection.js` dan
`src/lib/jadibot-manager.js`) meneruskan store itu apa adanya. Tidak ada
session store ganda yang perlu dikonsolidasikan.

## Security

- `config.js`: strict number matching (`matchesNumber`) — mencegah privilege escalation via partial match
- `isOwner()` → cek config owner, DB owner, DB ownerNumbers
- `isPremium()` → owner/partner otomatis premium, cek expiry
- `isBanned()` → owner tidak bisa di-ban
- Middleware di handler: permission check sebelum eksekusi plugin
- Rate limiter (rate-limiter-flexible): `globalRateLimiter` 8 points / 3s + `spamDelayTracker` (`src/handler.js:358-361`)
- Group protection: antilink, antitoxic, antispam, antibot, antidocument, antisticker, antimedia, anti-hidetag, anti-phishing, anti-judol, anti-remove
- Anti-crash guard: global uncaughtException + unhandledRejection handler
- Secret management: semua API key dipindah ke `.env` (tidak di-commit)

### Tiga hal yang mudah disalahpahami soal akses

1. **`user.access` bukan hanya pintu owner.** Di `src/lib/middleware.js:37-52`
   sebuah entri `user.access` yang belum kedaluwarsa di-OR ke enam gate:
   `isOwner` (`:54`), `isPartner` (`:61`), `isPremium` (`:69`), `isAdmin`
   (`:96`), matikan fitur `game` per grup (`:120`), dan matikan fitur `rpg`
   per grup (`:128`).

2. **`m.fromMe` = owner-tier.** `src/lib/serialize.js:770` menulis
   `m.isOwner = m.fromMe ? true : ...`, untuk `isPartner` dan `isPremium`
   juga. `src/connection.js:1382` memproses event bertipe `notify` **dan**
   `append`, jadi pesan yang bot kirim ke account-nya sendiri ikut masuk
   router dengan owner-tier. Jangan pakai account bot untuk chat biasa.

3. **Tidak semua plugin host pakai `isOwner`.** `plugins/vps/*` (6 file) dan
   12 dari 21 `plugins/panel/*` mendeklarasikan `isOwner: false`, jadi gate
   framework tidak menyaringnya. 11 di antaranya punya gate kedua di dalam
   handler (`hasAccess`, `hasAccessToServer`, `hasFullAccess`, `canManageRole`),
   semua default-deny. `plugins/panel/cpanel.js` tidak punya gate kedua,
   tetapi read-only.

Lihat juga bagian **Jadibot bukan sandbox** di `README.md` untuk batas
sub-bot.

## Key Libraries

| Library | Fungsi |
|---------|--------|
| `onigis` | WhatsApp MD protocol (fork of @whiskeysockets/baileys) — dinamai `foto-baileys` di versi lama. Dipin `10.1.0-rc.12` |
| `lowdb` | JSON file database |
| `@libsql/client` | Turso/libsql edge database |
| `@napi-rs/canvas` | Canvas rendering (welcome card, OCR fixture) |
| `sharp` | Image processing |
| `brat-canvas` | Render kartu brat/bratvid |
| `fluent-ffmpeg` | Audio/video processing |
| `pino` | Logging (Baileys internal) |
| `cron` | Job scheduling |
| `oktz-signal` | Signal protocol (X25519, session, pre-key) — dipin `0.3.0-rc.3` |
| `oktz-curve25519` | Native binding kripto kurva — transitif, `0.0.10`, binary per-platform via `@oktz/curve25519-*` |
| `rate-limiter-flexible` | Rate limiting |
| `lru-cache` | Performance caching (apimanager, thumb) |
| `node-cache` | Cache key/value sederhana |
| `undici` | HTTP client |
| `axios` | HTTP requests — termasuk scraper Gemini (`src/scraper/gemini.js`) |
| `cheerio` | HTML scraping |
| `tesseract.js` | OCR, fallback saat binary native `ourin_native` tidak ada |
| `ssh2` | SSH (VPS management) |
| `btch-downloader` | Media downloader (FB, CapCut, ttdl) |
| `google-tts-api` | Text-to-speech |

Tidak ada `@google/generative-ai`: integrasi Gemini memakai REST via `axios`.
Tidak ada `node-webpmux`: metadata stiker ditulis sendiri di `src/lib/exif.js`
menggunakan `fs` + `crypto`.

## Direktori Plugin (829 file, 34 kategori)

| Kategori | Jumlah | Fungsi |
|----------|-------:|--------|
| owner | 150 | Eval, exec, manage bot, cap energi/premium, sewa |
| group | 101 | Antilink, welcome, mute, warn, dll |
| rpg | 66 | RPG game system |
| tools | 56 | Utility tools |
| cek | 48 | Quiz/check personality |
| ai | 46 | AI chat integration + image gen |
| search | 46 | Search engines |
| fun | 39 | Fun commands |
| game | 36 | Interactive games |
| canvas | 31 | Image generation |
| download | 26 | Media downloaders |
| sticker | 22 | Sticker creation |
| panel | 21 | Hosting panel (Pterodactyl, DO, Linode, CPanel) |
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

Jumlah di atas adalah hitungan file `.js` per folder. Satu file bisa
mendaftarkan beberapa command lewat `name: ['a','b','c']`, jadi ini bukan
jumlah command.

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

Built-in di `case/foto.js`:
- `cping`, `cspeed`, `clatency` — ping/latency
- `listallcase`, `lcase`, `caselist`, `allcase` — daftar case
- `listallplugin`, `lplugin`, `pluginlist`, `allplugin` — daftar plugin

## Dev Mode

- `NODE_ENV=development` → dev mode aktif
- `config.dev.watchPlugins` → hot-reload plugin saat file berubah
- `config.dev.debugLog` → stack trace di error
- Anti-crash: uncaughtException + unhandledRejection handler
- SIGINT/SIGTERM: save database, exit safe (sinyal kedua = force exit)

Hanya `npm run dev` yang menyalakan `NODE_ENV=development`. `npm start` tidak,
meski `dev.watchPlugins` bernilai `true` di config — watcher hanya aktif di
mode development.

## Batas Memori Produksi

| Batas | Nilai | Sumber |
|-------|------:|--------|
| Heap ceiling | 512 MB | `package.json` → `start`: `--max-old-space-size=512` |
| RSS limit (monitor) | 550 MB | `src/lib/memory-monitor.js:3` |
| GC trigger (RSS) | 380 MB | `src/lib/memory-monitor.js:4` |
| GC trigger (heap) | 250 MB | `src/lib/memory-monitor.js:5` |
| Profiler RSS ceiling | 400 MB | `src/lib/profiler.js:42` |
| Profiler proyeksi ceiling | 800 MB | `src/lib/profiler.js:44` |

`--max-old-space-size=512` itu batas keras, bukan saran: proses OOM sebelum
host kehabisan RAM. `storage/profiling` tidak punya retensi — lihat
`docs/profiling-harness.md`.

## Testing

`npm test` → `node --experimental-test-module-mocks --test "tests/**/*.test.?(m)js"`
— 71 file, 668 test, 0 gagal. Jalankan lewat script, jangan menulis ulang
perintahnya: flag `--experimental-test-module-mocks` dipakai beberapa suite.

`npm run lint` → `eslint plugins/` (cakupan hanya `plugins/`, bukan seluruh repo).

Suite berjalan **offline terhadap stub socket** — tidak menjalankan bot, tidak
menghubungi WhatsApp, tidak menyentuh Turso. Diverifikasi dengan memblokir
`net.connect` / `tls.connect` / `dns.*` lewat preload: 668 pass, nol koneksi
keluar. Satu-satunya percobaan DNS ke domain `.invalid` (RFC 2606, memang
diservis tidak ter-resolve) di `tests/upstream-diagnostics.test.mjs`.

CI ada di `.github/workflows/ci.yml`: test + lint di Node 22 dan 24, plus
`scripts/secret-guard.mjs` yang memeriksa `.env` tidak bocor ke file ter-track.

Grup test yang penting saat menyentuh auth/session:

- `number-match.security.test.mjs` — verifikasi `isOwner` strict equality (anti privilege escalation via partial match)
- `jadibot-owner-privilege.test.mjs` — batas privilege owner jadibot (blokir seluruh kategori `owner`, prefix `>>` / `!!`)
- `onwhatsapp-gate.test.mjs` — gate provisioning admin fail-closed
- `rename-invariants.test.mjs` — merge `ourin` → `foto`: import resolve, tidak ada sisa `ourin-*.js`, kunci aset utuh
- `turso-db.test.mjs`, `turso-helper.test.mjs`, `turso-session.test.mjs`, `turso-session-atomic.test.mjs` — Turso DB + session auth state + atomic batch/sequential fallback
- `waifu-data.test.mjs`, `waifu-lib.test.mjs`, `husbu-data.test.mjs`, `husbu-lib.test.mjs`, `romance-lib.test.mjs` — gacha data pool (300+ entri), pity system, aksi/mood/jealousy
- `afk.test.mjs` — persistence + alias guard + mention throttle
- `lid-resolve.test.mjs` — LID→JID resolution + fallback
- `logger-command.test.mjs` — log command tanpa nomor user
- `memory-leaks.test.mjs` — cron job leak, cache cap (antispam, waifupool pages)
- `profiler.test.mjs`, `profiler-heap-guard.test.mjs` — kontrak profiler + ceiling heap snapshot
- `tests/foto/` — fixture untuk integration test. **Gitignored** dan tidak ada
  di checkout bersih; test yang bergantung padanya akan gagal di CI.

## Troubleshooting Umum

| Gejala | Penyebab | Solusi |
|---|---|---|
| `.tt` gagal semua | Semua provider kena Cloudflare 403 di IP server | Cek log `[tiktokDl]`, butuh proxy/residential IP atau API berbayar (TikHub) |
| Plugin tidak ke-load | Error syntax / import rusak | `node --check plugins/<file>.js`, cek log `plugin failed <file>` |
| Bot restart terus | OOM | `--max-old-space-size=512` sudah ada di `npm start`; cek memory monitor (RSS limit 550MB) |
| Turso error | Network / batch tidak didukung | Sudah ada fallback sequential + file lokal, cek `[turso]` log |
| Session hilang | Turso down / session expired | Auto-recovery main-session + restore jadibot di `connection open` |
| Disk penuh | `storage/profiling` menumpuk tanpa batas | Hapus `storage/profiling/*.cpuprofile` dan `*.heapsnapshot` secara berkala — tidak ada retensi otomatis |
| `npm test` gagal di mesin bersih | `tests/foto/` gitignored, jadi fixture tidak ada | Suite utama tidak memerlukannya; kalau ada test lokal yang gagal, itu test yang butuh fixture |

## Dokumen lain di `docs/`

Audit dan catatan desain. Semuanya analisis historis, bukan spesifikasi
yang selalu berlaku — periksa tanggalnya sebelum mengutip angka.

| Dokumen | Isi |
|---|---|
| `docs/memory-leak-audit.md` | Audit kebocoran memori & efisiensi (2026-08-25) |
| `docs/ram-safety-audit-2026-09-21.md` | Sweep keamanan RAM & free yang aman |
| `docs/profiling-audit.md` | Analisis flamegraph CPU produksi |
| `docs/profiling-harness.md` | Cara memakai profiler bawaan (termasuk batas retensinya) |
| `docs/plugin-audit-2026-09-20.md` | Audit 829 plugin |
| `docs/rust-migration-audit.md` | Evaluasi migrasi ke Rust (2026-08-29) |
| `docs/superpowers/` | Rencana & spesifikasi desain per fitur |

> Peringatan: `docs/plugin-audit-2026-09-20.md` dan dua dokumen
> `docs/superpowers/plans/` memuat nilai `APIKEY_CUKI` secara verbatim, dan
> satu spec memuat `TURSO_URL`. `.env` ter-ignore, tetapi `docs/` tidak.
> `scripts/secret-guard.mjs` mendeteksinya; rotasi dan pembersihan adalah
> keputusan maintainer.
