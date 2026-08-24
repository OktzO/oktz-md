# Ourin-MD v3.3.1

**WhatsApp Multi-Device Bot** — Modular plugin-based bot built on `ourin-baileys` (fork of `@whiskeysockets/baileys`). Written in ESM JavaScript, requires Node >=22.

## Fitur Utama

- **Plugin System** — 823+ plugin, 34 kategori. Hot-reload di development.
- **Dual Routing** — Plugin-based (majoritas) + Case-based (built-in, di `case/ourin.js`).
- **Multi Storage** — lowdb (JSON file) + Turso (libsql) untuk session & data.
- **Group Protection** — Antilink, anti-toxic, anti-spam, anti-hidetag, anti-viewonce, anti-remove, anti-bot, anti-document, anti-sticker, anti-media, anti-phishing, anti-judol.
- **Games** — Tebak gambar, tebak kata, family 100, RPG (adventure, mining, fishing, etc), Minecraft simulation, Fisch game, snakes & ladders, tictactoe, suit PvP, dungeon, dll.
- **AI Integration** — Gemini, Claude, DeepSeek, Qwen3, GPT5, auto-AI chat, image generation (txt2img, img2img).
- **Downloaders** — YouTube (mp3/mp4), TikTok, Instagram, Twitter, Facebook, SoundCloud, Spotify, Reddit, Terabox, DailyMotion, Likee, Douyin, dll.
- **Scrapers** — 59+ scraper modules (Google, Pinterest, GSMArena, dll).
- **Subsystems** — memory monitor, temp cleaner, data pruner, sholat/notif/JPM scheduler, OTP poller, jadibot auto-restore.
- **RPG System** — Level, XP, inventory, shop, mining, fishing, hunting, crafting, dungeon, clan.
- **Store System** — Jual-beli item, transaksi, profit.
- **Panel Management** — Pterodactyl, Vercel deploy, VPS (Linode, DigitalOcean), CPanel.
- **Jadibot** — Sub-bot system: pengguna bisa jadi bot sendiri.
- **JPM (Jadwal Pesan Massal)** — Broadcast terjadwal.
- **Scheduler** — Cron jobs, daily reset, notification scheduler, prayer time scheduler, auto-backup, auto-JPM.
- **Sewa (Rental)** — Group rental with expiry, auto-kick.
- **Auto-download** — Auto-detect link dari pesan & download otomatis.
- **Smart Triggers** — Auto-reply based on keyword.
- **Canvas** — Welcome/goodbye card generator, brat-style text, carbon code image, sticker maker.
- **Primbon** — Ramalan jodoh, watak, weton, dll.
- **Stalker** — Stalk IG, ML, FF, TikTok, dll.
- **NSFW Protection** — Anti-NSFW content detection.

## Cara Install

```bash
git clone <repo-url>
cd ourin-md
npm install
```

Edit `config.js`:
- `session.pairingNumber` — nomor WA tujuan
- `session.usePairingCode` — true = pairing code, false = QR
- `owner.number` — nomor owner

## Cara Run

```bash
npm start        # NODE_ENV=development
npm run dev      # development
node index.js    # production
```

## Testing

```bash
npm test         # node --test tests/
```

Suite `node:test`: `tests/number-match.security.test.mjs` (strict owner-number matching), `tests/turso-*.test.mjs` (Turso DB/session/helper). `tests/OURIN/` berisi fixture runtime (copy bot untuk integration test).

## Environment Variables

| Variable | Fungsi |
|----------|--------|
| `TURSO_URL` | Turso database URL |
| `TURSO_AUTH_TOKEN` | Turso auth token |
| `NODE_ENV` | Set ke `development` untuk dev mode |

## Config Penting di `config.js`

| Key | Default | Fungsi |
|-----|---------|--------|
| `mode` | `"public"` | Mode bot (public/self) |
| `command.prefix` | `"."` | Prefix command |
| `features.autoRead` | `true` | Auto-read pesan |
| `features.antiCall` | `false` | Tolak panggilan |
| `features.smartTriggers` | `false` | Auto-reply trigger |
| `registration.enabled` | `false` | Wajib daftar dulu |
| `energi.enabled` | `true` | Limit/energi system |
| `turso.enabled` | `true` | Pakai Turso session |
| `turso.url` | env/fallback | Turso URL |
| `backup.enabled` | `false` | Auto-backup |
| `dev.enabled` | auto | Dev mode (hot-reload) |

## Struktur Plugin

Setiap plugin di `plugins/<kategori>/<nama>.js` harus export:

```js
export const config = {
  name: "commandname",          // required
  alias: ["alias1", "alias2"],  // optional
  category: "kategori",         // otomatis dari folder
  description: "...",           // optional
  usage: "...",                 // optional
  isOwner: false,               // owner-only
  isPremium: false,             // premium-only
  isGroup: false,               // group-only
  isPrivate: false,             // private-only
  isAdmin: false,               // admin grup required
  isBotAdmin: false,            // bot harus admin
  cooldown: 3,                  // detik
  limit: 1,                     // energi per use
};

export async function handler(m, { sock, store, config, plugins }) {
  // handler logic
}
```

## Case System

Built-in commands di `case/ourin.js` (bukan plugin):
- `cping`, `cspeed`, `clatency` — ping test
- `listallcase`, `lcase`, `caselist`, `allcase` — lihat case list
- `listallplugin`, `lplugin`, `pluginlist`, `allplugin` — lihat plugin list

## Error Handling

Anti-crash protection aktif otomatis. Uncaught exceptions & unhandled rejections ditangkap tanpa mematikan process. Error network umum (ECONNRESET, EPIPE, dll) di-ignore.

## License

ISC — by Zann