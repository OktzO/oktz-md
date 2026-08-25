<div align="center">

# ⚡ OURIN-MD v3.3.1
### Next-Generation Modular WhatsApp Multi-Device Bot

[![Node Version](https://img.shields.io/badge/Node.js-%3E%3D22.0.0-339933?style=for-the-badge&logo=node.js&logoColor=white)](https://nodejs.org)
[![Engine](https://img.shields.io/badge/Baileys-ourin--baileys%20v9-25D366?style=for-the-badge&logo=whatsapp&logoColor=white)](https://github.com/whiskeysockets/Baileys)
[![Database](https://img.shields.io/badge/Database-Turso%20%26%20LowDB-4ff8d2?style=for-the-badge&logo=sqlite&logoColor=black)](https://turso.tech)
[![Plugins](https://img.shields.io/badge/Plugins-825%2B%20Loaded-blueviolet?style=for-the-badge&logo=speedtest&logoColor=white)](#-kategori-plugin-34-kategori)
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
[Testing](#-testing--keamanan)

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

### 🛡️ Group Security & Protection
- **Full Guard System:** Anti-Link, Anti-Toxic, Anti-Spam, Anti-Bot.
- **Media Protection:** Anti-ViewOnce, Anti-Sticker, Anti-Document.
- **Safety Enforcement:** Anti-Hidetag, Anti-Phishing, Anti-Judol.
- **Rental (Sewa Grup):** Auto-join, durasi sewa, & auto-kick expired.

### 🎮 Gaming, RPG & Economy
- **Full RPG System:** Dungeon, Mining, Fishing, Hunting, Crafting, Clan.
- **Economy:** Limit/Energi system, Bank, Store, Market transaksi.
- **Casual & Interactive:** Tebak Gambar, Tebak Kata, Family 100, Fisch, Chess, TicTacToe, Suit PvP.

</td>
<td width="50%">

### 📥 Media Downloaders & Scrapers
- **Social Downloader:** YouTube (Audio/Video), TikTok (No WM), IG, FB, Twitter/X, Spotify, SoundCloud, Terabox, Douyin.
- **59+ Scraper Modules:** Pinterest, Google Search, GSMArena, Anime info, lirik lagu, dan scraper kustom.

### ⚙️ DevOps & Cloud Management
- **Panel Hosting:** Pterodactyl server manager & Vercel deployment.
- **Server Control:** VPS Management (Linode, DigitalOcean, CPanel).
- **Sub-Bot:** Jadibot multi-session system dengan auto-restore.

### ⚡ Infrastructure & Resiliency
- **Dual Routing Engine:** Plugin-based (dynamic) + Case-based fallback.
- **Hybrid Storage:** LowDB (local JSON) + Turso LibSQL (remote cloud).
- **Anti-Crash Guard:** Uncaught exception filter & auto network recover.

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
│   └── ourin.js               # Direct switch-case execution
├── 📁 database/               # Local JSON storage fallback
│   ├── 📁 autoreply_media/    # Storage media respon otomatis
│   ├── 📁 cpanel/             # Database akun & order cpanel
│   └── 📁 main/               # User state, inventory, & global data
├── 📁 docs/                   # Spesifikasi arsitektur & rencana proyek
├── 📁 plugins/                # 825+ Dynamic Plugins (34 Kategori)
│   ├── 📁 ai/                 # OpenAI, Gemini, Claude, prompt generator
│   ├── 📁 anime/              # Anime info, picture stream, tracer
│   ├── 📁 download/           # TikTok, YT, IG, Spotify downloaders
│   ├── 📁 game/               # Tebak-tebakan, kuis, multiplayer mini-games
│   ├── 📁 group/              # Admin tools, group protection, settings
│   ├── 📁 owner/              # Evaluator, exec, broadcast, backup
│   ├── 📁 panel/              # Pterodactyl & hosting automation
│   ├── 📁 rpg/                # Mining, clan, inventory, levelup
│   ├── 📁 search/             # Web search, scraper query
│   ├── 📁 sticker/            # Sticker converter, meme maker, brat
│   └── ...                    # (Lihat tabel kategori lengkap di bawah)
├── 📁 src/                    # Bot Core Engine
│   ├── 📁 data/               # Data statis (Tebakan, Asmaul Husna, NSFW)
│   ├── 📁 database/           # Schema handler & database sync adapter
│   ├── 📁 lib/                # Memory cleaner, scheduler, image renderer
│   ├── 📁 scraper/            # Scraper functions & extraction modules
│   ├── connection.js          # Socket connection & auth event handler
│   └── handler.js             # Message router, middleware, spam blocker
├── 📁 tests/                  # Test suite (Node native test runner)
├── config.js                  # Konfigurasi sentral bot
├── index.js                   # Application entrypoint & worker supervisor
└── package.json               # Manifest dependencies & project scripts
```

### 📦 Kategori Plugin (34 Kategori)

| Kategori | Deskripsi | Kategori | Deskripsi |
|---|---|---|---|
| `ai` | Artificial Intelligence & LLM | `media` | Audio/video processing |
| `anime` | Info anime, komik & wallpaper | `nsfw` | Filter & NSFW tools |
| `asupan` | Short video & media feeder | `owner` | Command khusus owner |
| `canvas` | Image rendering & dynamic card | `panel` | Panel Pterodactyl automation |
| `cek` | Tracker & checker tool | `primbon` | Weton, ramalan & primbon Jawa |
| `clan` | Sistem guild/clan RPG | `pushkontak` | Bulk contact helper |
| `convert` | Media converter & format changer | `random` | Random generator & quotes |
| `download` | Media downloader multi-platform | `religi` | Doa & info keagamaan |
| `ephoto` | Text generator & visual effect | `rpg` | Sistem petualangan RPG |
| `fun` | Mini text game & hiburan | `search` | Multi-engine web scraper |
| `game` | Game grup & interaktif | `stalker` | Profil & username stalker |
| `group` | Manajemen & proteksi grup | `sticker` | Pembuat stiker & watermark |
| `info` | Bot information & stats | `store` | Toko digital & transaksi |
| `islamic` | Al-Qur'an, Jadwal Sholat | `tools` | Utilities harian & converter |
| `jpm` | Jadwal Pesan Massal (Broadcast) | `tts` | Text-to-speech multi-language |
| `main` | Menu & core navigation | `user` | Profil & pendaftaran user |
| `utility` | System toolkit & diagnostics | `vps` | VPS / cloud server control |

---

## 🛠️ Instalasi & Menjalankan

### Persyaratan Sistem
- **Node.js:** `>= 22.0.0`
- **FFmpeg:** Terpasang pada sistem (atau otomatis via binary package)
- **RAM:** Minimal 512 MB (Rekomendasi 1 GB+)

### 1. Clone & Install Dependencies
```bash
git clone https://github.com/username/ourin-md.git
cd ourin-md
npm install
```

### 2. Konfigurasi Environment (`.env`)
Buat file `.env` jika menggunakan cloud database Turso:
```env
TURSO_URL=libsql://your-database-name.turso.io
TURSO_AUTH_TOKEN=your-auth-token
NODE_ENV=production
```

### 3. Menjalankan Bot

```bash
# Mode Development (Hot-Reload & Garbage Collection Exposed)
npm run dev

# Mode Start Development
npm start

# Mode Production
node index.js
```

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

---

## 🧪 Testing & Keamanan

Project ini menggunakan Node.js native test runner:

```bash
# Jalankan seluruh unit test
npm test
```

### Arsitektur Keamanan & Stabilitas
- **Strict Number Matching:** Verifikasi ketat JID WhatsApp owner tanpa bypass spoofing format.
- **Anti-Crash Guard:** Global uncaught exception handler memastikan bot tetap aktif saat terjadi transient network error.
- **Memory & Resource Monitor:** Automatic temp cleaner dan Garbage Collection watcher mencegah kebocoran memori (OOM).

---

## 📄 Lisensi & Kredit

- **Author:** [Zann](https://github.com)
- **Base Engine:** [`ourin-baileys`](https://www.npmjs.com/package/ourin-baileys) (Fork of `@whiskeysockets/baileys`)
- **License:** Distributed under the **ISC License**.

<div align="center">
  <sub>Dibuat dengan ❤️ untuk ekosistem bot WhatsApp yang lebih andal dan efisien.</sub>
</div>
