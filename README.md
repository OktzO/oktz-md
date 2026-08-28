<div align="center">

# ⚡ OURIN-MD v3.3.1
### Next-Generation Modular WhatsApp Multi-Device Bot

[![Node Version](https://img.shields.io/badge/Node.js-%3E%3D22.0.0-339933?style=for-the-badge&logo=node.js&logoColor=white)](https://nodejs.org)
[![Engine](https://img.shields.io/badge/Baileys-ourin--baileys%20v9-25D366?style=for-the-badge&logo=whatsapp&logoColor=white)](https://github.com/whiskeysockets/Baileys)
[![Database](https://img.shields.io/badge/Database-Turso%20%26%20LowDB-4ff8d2?style=for-the-badge&logo=sqlite&logoColor=black)](https://turso.tech)
[![Plugins](https://img.shields.io/badge/Plugins-827%20Loaded-blueviolet?style=for-the-badge&logo=speedtest&logoColor=white)](#-kategori-plugin-34-kategori)
[![License](https://img.shields.io/badge/License-ISC-orange?style=for-the-badge)](LICENSE)

<p align="center">
  <b>Modern, Ultra-Fast, Memory-Optimized WhatsApp Bot Architecture</b><br>
  Dibangun menggunakan ES Module native, hot-reload dynamic plugin loader, sistem dual-routing, serta manajemen memori canggih dengan Turso LibSQL persistence.
</p>

---

[Fitur Utama](#-fitur-utama) •
[Struktur File](#-struktur-proyek) •
[Instalasi](#-instalasi--menjalankan) •
[Konfigurasi](#-konfigurasi) •
[Panduan Plugin](#-panduan-membuat-plugin) •
[Testing](#-testing--keamanan) •
[Infrastruktur](#-infrastruktur--arsitektur)

---

</div>

## 🚀 Fitur Utama

<table>
<tr>
<td width="50%">

### 🧠 AI & Intelligent Chat
- **Multi-Model Provider:** Integrasi Gemini, Claude, DeepSeek, Qwen3, GPT-5.
- **Image Generation:** Text-to-Image & Image-to-Image engine.
- **Smart Triggers:** Auto-reply cerdas berbasis context & keyword.
- **Voice Command (CMD VN):** Transkripsi voice note via Groq Whisper, langsung dieksekusi sebagai command.

### 🛡️ Group Security & Protection
- **Full Guard System:** Anti-Link, Anti-Toxic, Anti-Spam, Anti-Bot.
- **Media Protection:** Anti-ViewOnce, Anti-Sticker, Anti-Document.
- **Safety Enforcement:** Anti-Hidetag, Anti-Phishing, Anti-Judol.
- **Rental (Sewa Grup):** Auto-join, durasi sewa, & auto-kick expired.
- **Anti-Remove:** Deteksi hapus pesan anggota (anti-revoke).

### 🎮 Gaming, RPG & Economy
- **Full RPG System:** Dungeon, Mining, Fishing, Hunting, Crafting, Clan.
- **Economy:** Limit/Energi system, Bank, Store, Market transaksi.
- **Casual & Interactive:** Tebak Gambar, Tebak Kata, Family 100, Fisch, Chess, TicTacToe, Suit PvP, Ulartangga.

### 🥰 Waifu & Husbando Gacha
- **Gacha System:** Roll waifu/husbando dengan pity system & tier (Common→Mythic).
- **Interaksi Romance:** Aksi, event, mood, affection, jealousy check, married system.
- **Koleksi:** Album, pool browsing, daily claim, neglekt/anger system.

</td>
<td width="50%">

### 📥 Media Downloaders & Scrapers
- **Social Downloader:** YouTube (Audio/Video), TikTok (`.tt` no-WM + fallback `savett`), IG, FB, Twitter/X, Spotify, SoundCloud, Terabox, Douyin.
- **TikTok Multi-Provider:** `tikwm → savett → yuulabs → musicaldown` automatic fallback chain saat satu provider kena blokir (Cloudflare 403).
- **59+ Scraper Modules:** Pinterest, Google Search, GSMArena, Anime info, lirik lagu, dan scraper kustom.

### ⚙️ DevOps & Cloud Management
- **Panel Hosting:** Pterodactyl server manager & Vercel deployment.
- **Server Control:** VPS Management (Linode, DigitalOcean, CPanel).
- **Sub-Bot:** Jadibot multi-session system dengan auto-restore.

### ⚡ Infrastructure & Resiliency
- **Dual Routing Engine:** Plugin-based (dynamic) + Case-based fallback.
- **Hybrid Storage:** LowDB (local JSON) + Turso LibSQL (remote cloud).
- **Anti-Crash Guard:** Uncaught exception filter & auto network recover.
- **Memory Monitor:** Garbage collection watcher, temp cleaner, daily pruner mencegah OOM.

### 🔍 Stalker & Tools
- **Profile Stalker:** Instagram, ML, Discord, TikTok, dll.
- **Utility Tools:** Konverter, status cek, primbon Jawa, random content, ephoto.

</td>
</tr>
</table>

---

## 📂 Struktur Proyek

```text
ourin-md/
├── 📁 assets/                 # Asset statis bot
│   ├── 📁 audio/              # Sound effects & voice prompts
│   ├── 📁 fonts/              # Custom typography & canvas fonts
│   ├── 📁 image/              # Banner, avatar default, menu thumbnails
│   ├── 📁 kertas/             # Template magernulis / canvas note
│   └── 📁 video/              # Video intros & template media
├── 📁 case/                   # Built-in fast commands handler
│   └── ourin.js               # Direct switch-case execution (~5 command)
├── 📁 database/               # Runtime data (local JSON fallback)
│   ├── 📁 autoreply_media/    # Storage media respon otomatis
│   ├── 📁 cpanel/             # Database akun & order cpanel
│   ├── 📁 main/               # User state, inventory, & global data
│   └── lid-cache.json         # Cache resolusi LID → JID
├── 📁 docs/                   # Spesifikasi arsitektur & rencana proyek
├── 📁 plugins/                # 827 Dynamic Plugins (34 Kategori)
│   ├── 📁 ai/                 # OpenAI, Gemini, Claude, prompt generator (47)
│   ├── 📁 anime/              # Anime info, picture stream, tracer (3)
│   ├── 📁 download/           # TikTok, YT, IG, Spotify downloaders (26)
│   ├── 📁 game/               # Tebak-tebakan, kuis, multiplayer mini-games (37)
│   ├── 📁 group/              # Admin tools, group protection, settings (101)
│   ├── 📁 owner/              # Evaluator, exec, broadcast, backup (147)
│   ├── 📁 panel/              # Pterodactyl & hosting automation (22)
│   ├── 📁 rpg/                # Mining, clan, inventory, levelup (67)
│   ├── 📁 search/             # Web search, scraper query (45)
│   ├── 📁 sticker/            # Sticker converter, meme maker, brat (22)
│   └── ...                    # (Lihat tabel kategori lengkap di bawah)
├── 📁 src/                    # Bot Core Engine
│   ├── 📁 data/               # Data statis (Tebakan, Asmaul Husna, NSFW)
│   ├── 📁 database/           # Schema handler & database sync adapter
│   ├── 📁 lib/                # 71 library modules (memory, scheduler, dll)
│   ├── 📁 scraper/            # 59 scraper functions & extraction modules
│   ├── 📁 tiktok/             # Data feed asupan TikTok (JSON)
│   ├── connection.js          # Socket connection & auth event handler
│   └── handler.js             # Message router, middleware, spam blocker
├── 📁 tests/                  # Test suite (Node native test runner, 14 file)
├── config.js                  # Konfigurasi sentral bot
├── index.js                   # Application entrypoint & worker supervisor
├── infra.md                   # Dokumentasi infrastruktur internal
└── package.json               # Manifest dependencies & project scripts
```

### 📦 Kategori Plugin (34 Kategori, 827 Plugin)

| Kategori | Jml | Deskripsi | Kategori | Jml | Deskripsi |
|---|---:|---|---|---:|---|
| `owner` | 147 | Eval, exec, manage bot, cap energi/premium | `game` | 37 | Interactive group games |
| `group` | 101 | Antilink, welcome, mute, warn, dll | `canvas` | 31 | Image generation & card |
| `rpg` | 67 | RPG petualangan, dungeon, clan | `download` | 26 | Media downloaders |
| `tools` | 55 | Utility harian & converter | `sticker` | 22 | Sticker creation |
| `cek` | 48 | Quiz / personality check | `panel` | 22 | Hosting panel (Pterodactyl dll) |
| `ai` | 47 | AI chat integration + image gen | `main` | 20 | Menu, ping, stats |
| `search` | 45 | Multi-engine web scraper | `user` | 17 | User profile & daftar |
| `fun` | 39 | Mini text game & hiburan | `stalker` | 15 | Profile stalking |
| `store` | 14 | Toko digital & transaksi | `random` | 12 | Random generator |
| `info` | 14 | Informasi & statistik | `clan` | 9 | Sistem guild/clan |
| `primbon` | 8 | Weton & ramalan Jawa | `vps` | 6 | VPS / cloud control |
| `religi` | 4 | Konten keagamaan | `asupan` | 4 | Short video feed |
| `utility` | 3 | Notifikasi makan/tidur, inspect | `anime` | 3 | Top anime, waifu |
| `tts` | 2 | Text-to-speech | `nsfw` | 2 | NSFW (gated) |
| `media` | 2 | Media processing | `islamic` | 2 | Quran & jadwal sholat |
| `pushkontak` | 1 | Push contact massal | `jpm` | 1 | Jadwal pesan massal |
| `ephoto` | 1 | Ephoto templates | `convert` | 1 | Audio converter |

---

## 🛠️ Instalasi & Menjalankan

### Persyaratan Sistem
- **Node.js:** `>= 22.0.0`
- **FFmpeg:** Terpasang pada sistem (atau otomatis via binary package)
- **RAM:** Minimal 512 MB (Rekomendasi 1 GB+)

### 1. Clone & Install Dependencies
```bash
git clone https://github.com/OktzO/ourin-md.git
cd ourin-md
npm install
```
> `npm install` otomatis menjalankan `postinstall` yang me-rebuild `sharp` bila diperlukan.

### 2. Konfigurasi Environment (`.env`)
Copy `.env.example` jadi `.env`, lalu isi:
```env
# Database Turso (remote DB + session)
TURSO_URL=libsql://your-database-name.turso.io
TURSO_AUTH_TOKEN=your-auth-token

# API key untuk fitur yang pakai layanan eksternal
APIKEY_NEOXR=
APIKEY_FGSI=
GROQ_API_KEY=          # Dipakai untuk transkripsi voice command (CMD VN)
APIKEY_COVENANT=
APIKEY_OBSCURA=
APIKEY_FIREFLY=
APIKEY_CUKI=

NODE_ENV=production
```
File `.env` tidak di-commit (sudah ada di `.gitignore`).

### 3. Menjalankan Bot

```bash
# Mode Development (Hot-Reload & Garbage Collection Exposed)
npm run dev

# Mode Start Development
npm start

# Mode Production
node index.js
```

**Pairing / QR:** Set `config.session.usePairingCode: true` + isi `pairingNumber` untuk pairing code, atau `false` untuk QR code. Scan dengan WhatsApp → Linked Devices.

---

## ⚙️ Konfigurasi (`config.js`)

Pengaturan sentral bot terdapat pada file `config.js`:

```javascript
export default {
  // Mode Operasional
  mode: "public",                // "public" (semua user) | "self" (hanya owner)

  // Sesi & Pairing
  session: {
    usePairingCode: true,        // true = Pairing Code, false = QR Code
    pairingNumber: "628xxx",     // Nomor WhatsApp bot (format: 628xxx)
  },

  // Owner Setting
  owner: {
    name: "Zann",
    number: "628xxx",            // Nomor owner utama
  },

  // Command & Behavior
  command: {
    prefix: ".",                 // Simbol prefix command bot
  },

  features: {
    autoRead: true,              // Otomatis read pesan masuk
    antiCall: false,             // Blokir/tolak panggilan otomatis
    smartTriggers: false,        // Auto-reply berbasis kata kunci
  },

  energi: {
    enabled: true,               // Sistem batas limit/energi
    default: 25,                 // Default energi user baru
  },

  turso: {
    enabled: true,               // Aktifkan sinkronisasi Turso LibSQL
  }
};
```

### Konfigurasi Tambahan (opsional)
- **`registration.enabled`** — wajibkan user daftar (`daftar`) sebelum pakai command.
- **`features.logMessage`** — log command per-fitur ke console (tanpa nomor user).
- **`dev.watchPlugins`** — hot-reload plugin otomatis saat file berubah (mode dev).
- **`dev.debugLog`** — tampilkan stack trace lengkap saat error.
- **`errorTemplate`** — template pesan error global command.

---

## 🧩 Panduan Membuat Plugin

Setiap plugin baru diletakkan di `plugins/<kategori>/<nama-plugin>.js` menggunakan format ES Module:

```javascript
/**
 * Plugin Template - Ourin-MD
 */
export const config = {
  name: "ping",                         // Nama command utama (wajib)
  alias: ["p", "speed", "test"],        // Alias command (opsional)
  category: "utility",                  // Kategori plugin
  description: "Cek responsivitas bot", // Deskripsi command
  usage: ".ping",                       // Cara penggunaan

  // Permission & Flags
  isOwner: false,                       // Khusus owner bot
  isPremium: false,                     // Khusus user premium
  isGroup: false,                       // Khusus pesan grup
  isPrivate: false,                     // Khusus private chat (PC)
  isAdmin: false,                       // User harus admin grup
  isBotAdmin: false,                    // Bot harus jadi admin grup

  // Rate Limit & Cost
  cooldown: 3,                          // Cooldown per-eksekusi (detik)
  limit: 1                              // Konsumsi energi/limit per-use
};

export async function handler(m, { sock, store, config, plugins }) {
  const start = Date.now();
  await m.reply("Pong!");
  const latency = Date.now() - start;
  await m.reply(`⚡ Kecepatan respon: *${latency}ms*`);
}
```

### Objek `m` (Serialized Message)
Setelah di-`serialize`, objek `m` berisi field berguna:
- `m.text` / `m.body` — teks penuh (tanpa prefix)
- `m.command` / `m.args` — command + argumen
- `m.prefix` — prefix yang dipakai
- `m.chat` — JID chat, `m.sender` — JID pengirim
- `m.isGroup` / `m.isPrivate` / `m.isAdmin` / `m.isOwner`
- `m.reply(text)` — reply text, `m.react(emoji)` — react ke pesan

### Hot-Reload
Saat mode dev + `dev.watchPlugins`, file plugin yang diedit otomatis di-load ulang tanpa restart bot. Manual: gunakan command `reload` (owner) bila tersedia.

---

## 🧪 Testing & Keamanan

Project ini menggunakan Node.js native test runner:

```bash
# Jalankan seluruh unit test
npm test
```

### Cakupan Test (14 file)
- **`number-match.security.test.mjs`** — verifikasi `isOwner` strict equality (anti privilege escalation via partial match)
- **`turso-db/helper/session.test.mjs`** — Turso DB, helper, session auth state + atomic writes
- **`waifu-data/lib.test.mjs`**, **`husbu-data/lib.test.mjs`** — data pool & gacha logic (300+ entri)
- **`romance-lib.test.mjs`** — shared romance engine (diminish, fatigue, neglekt, anger)
- **`afk.test.mjs`**, **`lid-resolve.test.mjs`**, **`logger-command.test.mjs`** — AFK persistence, LID resolve, log
- **`memory-leaks.test.mjs`** — deteksi kebocoran cron job / cache cap

### Arsitektur Keamanan & Stabilitas
- **Strict Number Matching:** Verifikasi ketat JID WhatsApp owner tanpa bypass spoofing format.
- **Anti-Crash Guard:** Global uncaught exception handler memastikan bot tetap aktif saat terjadi transient network error.
- **Memory & Resource Monitor:** Automatic temp cleaner dan Garbage Collection watcher mencegah kebocoran memori (OOM).
- **Rate Limiter:** Anti-spam per-user (rate-limiter-flexible).
- **Group Protection:** Antilink, antitoxic, antispam, antibot, antidocument, antisticker, antimedia, antidetele.

---

## 🏗️ Infrastruktur & Arsitektur

Untuk dokumentasi teknis internal (alur eksekusi, message routing, plugin system, database, session, key libraries) lihat **`infra.md`**.

Ringkasan cepat:
- **Entry Point:** `index.js` → setup anti-crash, init database, preload assets, load plugins, scheduler, lalu start connection.
- **Routing:** `src/handler.js` → serialize → filter → case handler → group protection → permission check → `getPlugin(command)` → eksekusi `handler()`.
- **Plugin Store:** `src/lib/ourin-plugins.js` — 3 Map (`commands`, `aliases`, `categories`).
- **Storage:** LowDB (JSON lokal) untuk data utama + Turso LibSQL (cloud) untuk session auth.
- **Downloader TikTok:** fallback chain `tikwm → savett → yuulabs → musicaldown` supaya tetap jalan saat satu provider kena blokir.

---

## 📄 Lisensi & Kredit

- **Author:** [Zann](https://github.com)
- **Base Engine:** [`ourin-baileys`](https://www.npmjs.com/package/ourin-baileys) (Fork of `@whiskeysockets/baileys`)
- **License:** Distributed under the **ISC License**.

<div align="center">
  <sub>Dibuat dengan ❤️ untuk ekosistem bot WhatsApp yang lebih andal dan efisien.</sub>
</div>
