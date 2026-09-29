<div align="center">

# ⚡ OKTZ-MD v3.3.1
### Modular WhatsApp Multi-Device Bot

[![Node Version](https://img.shields.io/badge/Node.js-%3E%3D22.0.0-339933?style=for-the-badge&logo=node.js&logoColor=white)](https://nodejs.org)
[![Tests](https://img.shields.io/badge/tests-668%20pass%20%2F%200%20fail-success?style=for-the-badge)](https://nodejs.org)
[![Engine](https://img.shields.io/badge/Engine-Onigi--Baileys%20v10.1.0--rc.6-25D366?style=for-the-badge&logo=whatsapp&logoColor=white)](https://github.com/OktzO/Onigi)
[![Database](https://img.shields.io/badge/Database-Turso%20%26%20LowDB-4ff8d2?style=for-the-badge&logo=sqlite&logoColor=black)](https://turso.tech)
[![Plugins](https://img.shields.io/badge/plugins-829%20files%20%2F%2034%20categories-blueviolet?style=for-the-badge&logo=speedtest&logoColor=white)](#-kategori-plugin)
[![License](https://img.shields.io/badge/License-ISC-orange?style=for-the-badge)](LICENSE)

<p align="center">
  <b>ES Module, plugin modular, penyimpanan hybrid LowDB + Turso LibSQL.</b><br>
  Bot WhatsApp multi-device dengan sistem plugin yang bisa dimuat ulang
  saat runtime dan penjaga memori yang aktif.
</p>

---

[Fitur](#-fitur) •
[Struktur](#-struktur-proyek) •
[Instalasi](#-instalasi--menjalankan) •
[Konfigurasi](#-konfigurasi) •
[Plugin](#-panduan-membuat-plugin) •
[Testing](#-testing) •
[Operasional](#-kebutuhan-operasional) •
[Arsitektur](#-infrastruktur--arsitektur)

---

> ## ⚠️ STATUS
>
> - **Eksperimental.** Stack E2EE memakai `oktz-signal` (Rust native, MIT)
>   melalui Onigi-Baileys. Kalau pesan tertahan sebagai "Menunggu pesan ini..."
>   atau gagal verifikasi MAC: `git pull && npm install`, lalu restart. Kalau
>   masih, lepas perangkat dan pindai ulang QR.
> - **Tidak ada benchmark publik.** Halaman ini tidak mengklaim angka performa
>   apa pun. Angka RAM yang dipakai di bawah adalah hasil pengukuran di host
>   Pterodactyl 1GB, dan sumbernya dituliskan per klaim.

---

</div>

## 🚀 Fitur

Bagian ini sengaja memakai kalimat pendek. Setiap butir di bawah adalah
fitur yang **ada di kode dan terjangkau lewat router** — bukan backlog.
Untuk rincian per kategori, lihat [tabel kategori plugin](#-kategori-plugin).
### 🧠 AI
- Multi-provider: Gemini, Claude, DeepSeek, Qwen3, GPT-5 (`src/scraper/`:
  `gemini.js`, `claudehaiku.js`, `deepseek.js`, `qwen3.js`, `gpt5.js`, `gpt52.js`).
- Image generation & image-to-image (`txt2img.js`, `txt2img2.js`, `img2img.js`).
- CMD VN: transkripsi voice note via Groq Whisper, hasilnya dieksekusi sebagai command.

### 🛡️ Proteksi Grup
- Antilink, antijudol, antiviewonce, antihidetag, antitoxic, antispam,
  antibot, antidocument, antisticker, antimedia, anti-remove
  (`src/lib/group-protection.js`).
- Sewa grup: auto-join, durasi, dan auto-kick saat kedaluwarsa.
- Anti-crash guard: handler `uncaughtException` + `unhandledRejection` global.

### 🎮 Game, RPG, Ekonomi
- RPG: dungeon, mining, fishing, hunting, crafting, clan.
- Ekonomi: limit/energi, bank, store.
- Game interaktif:Family 100, fisch, chess, tictactoe, suit pvp, ulartangga.

### 📥 Downloader & Scraper
- 26 plugin download: TikTok, YouTube, Instagram, Facebook, Twitter/X,
  Spotify, SoundCloud, Terabox, Douyin, CapCut, dan lainnya.
- 59 modul scraper di `src/scraper/`.
- TikTok memakai rantai fallback otomatis — lihat
  [catatan TikTok](#-tiktok-rantai-fallback).

### ⚙️ DevOps & Sub-Bot
- Panel hosting (Pterodactyl, cPanel, Linode) dan kontrol VPS (DigitalOcean).
- Jadibot: sub-bot multi-sesi dengan auto-restore saat koneksi terbuka.

### 🔍 Tools
- Stalker profil, primbon Jawa, random, ephoto, konverter.

---

## 📂 Struktur Proyek

```text
oktz-md/
├── assets/                  # Gambar, font, audio, video
├── case/
│   └── foto.js              # Built-in switch-case: 4 grup command
├── database/                # Data runtime (JSON), gitignored
├── docs/                    # Audit internal & catatan desain
├── native/                  # Loader + fetch prebuilt binary
├── plugins/                 # 829 file plugin (34 kategori)
│   ├── owner/               # 150 — eval, exec, broadcast, sewa, backup
│   ├── group/               # 101 — proteksi, welcome, mute, warn
│   ├── rpg/                 #  66 — dungeon, clan, inventory, levelup
│   ├── tools/               #  56 — utilitas harian
│   ├── cek/                 #  48 — kuis / cek kepribadian
│   ├── ai/                  #  46 — integrasi AI + image gen
│   ├── search/              #  46 — web search & scraper
│   ├── fun/                 #  39 — game teks ringan
│   ├── game/                #  36 — game grup interaktif
│   ├── canvas/              #  31 — kartu & gambar
│   ├── download/            #  26 — downloader media
│   ├── sticker/             #  22 — pembuat stiker
│   ├── panel/               #  21 — Pterodactyl & hosting
│   ├── main/                #  20 — menu, ping, stats
│   ├── user/                #  17 — profil user
│   ├── stalker/             #  15 — stalking profil
│   ├── store/               #  14 — toko digital
│   ├── info/                #  14 — informasi
│   ├── random/              #  12 — konten acak
│   ├── clan/                #   9
│   ├── primbon/             #   8
│   ├── vps/                 #   6 — kontrol VPS
│   └── ...                  # 15 kategori lagi, 1-4 file each
├── src/
│   ├── lib/                 # 87 modul library
│   ├── scraper/             # 59 modul scraper
│   ├── tiktok/              # 8 feed JSON
│   ├── connection.js        # Socket & event auth
│   └── handler.js           # Router pesan
├── tests/                   # 71 file test (node:test)
├── config.js                # Konfigurasi sentral
├── index.js                 # Entry point
├── infra.md                 # Dokumen arsitektur internal
└── .github/workflows/ci.yml # CI: test, lint, penjaga secret
```

---

## 📦 Kategori Plugin

Jumlah di bawah adalah hitungan `find plugins -name '*.js'` per folder, bukan
estimasi. Totalnya **829 file di 34 kategori**. Angka ini adalah jumlah *file*,
bukan jumlah command — satu file bisa mendaftarkan banyak nama lewat
`name: ['a','b','c']` (contohnya `plugins/vps/vpskontrol.js` daftarkan empat).

| Kategori | File | Kategori | File |
|---|---:|---|---:|
| `owner` | 150 | `info` | 14 |
| `group` | 101 | `random` | 12 |
| `rpg` | 66 | `clan` | 9 |
| `tools` | 56 | `primbon` | 8 |
| `cek` | 48 | `vps` | 6 |
| `search` | 46 | `religi` | 4 |
| `ai` | 46 | `asupan` | 4 |
| `fun` | 39 | `utility` | 3 |
| `game` | 36 | `anime` | 3 |
| `canvas` | 31 | `tts` | 2 |
| `download` | 26 | `nsfw` | 2 |
| `sticker` | 22 | `media` | 2 |
| `panel` | 21 | `islamic` | 2 |
| `main` | 20 | `pushkontak` | 1 |
| `user` | 17 | `jpm` | 1 |
| `stalker` | 15 | `ephoto` | 1 |
| `store` | 14 | `convert` | 1 |

Tidak semua file di `owner/` bisa dipakai semua orang. Baca
[Model Akses](#-model-akses) sebelum bot Anda diekspos.

---

## 🛠️ Instalasi & Menjalankan

### Persyaratan Sistem
- **Node.js:** `>= 22.0.0` (`engines` di `package.json`).
- **RAM:** lihat [Kebutuhan Operasional](#-kebutuhan-operasional) — angka
  minimumnya bukan sekadar "512 MB".
- **ffmpeg:** opsional di sistem. `src/lib/ffmpeg.js` mencari urutan: variabel
  `FFMPEG_PATH`/`FFPROBE_PATH` → `PATH` → binary bawaan
  `@ffmpeg-installer/ffmpeg`. Jadi instalasi tidak perlu apt-get.

### 1. Clone & Install
```bash
git clone https://github.com/OktzO/oktz-md.git
cd oktz-md
npm install
```
`postinstall` melakukan tiga hal berurutan: rebuild `sharp` bila perlu, ambil
prebuilt `ourin_native` dari GitHub Release, lalu pasang ffmpeg bila belum ada.
Ketiganya **tidak wajib** untuk menjalankan test suite — lihat
[Test](#-testing).

### 2. Konfigurasi Environment
Salin `.env.example` ke `.env`, lalu isi:
```env
TURSO_URL=libsql://<database>.turso.io
TURSO_AUTH_TOKEN=<token>

APIKEY_NEOXR=
APIKEY_FGSI=
GROQ_API_KEY=          # untuk transkripsi voice command (CMD VN)
APIKEY_COVENANT=
APIKEY_OBSCURA=
APIKEY_FIREFLY=
APIKEY_CUKI=
```
`.env` sudah ada di `.gitignore` dan tidak boleh di-commit. Nilai `.env` yang
bocor ke file ter-track akanCI tangkap oleh `scripts/secret-guard.mjs`.

> Jangan tempel nilai `.env` ke commit, issue, atau log CI. Guard mencetak
> nama variabel dan path file saja, tidak pernah nilainya.

### 3. Menjalankan
```bash
npm start          # produksi: node --max-old-space-size=512 --expose-gc
npm run dev        # mode pengembangan (hot-reload plugin)
```

`npm start` **tidak** menyalakan `NODE_ENV=development`; hanya `npm run dev`
yang melakukannya. Ini berbeda dari versi lama README yang menyatakan
sebaliknya.

**Pairing / QR:** `config.session.usePairingCode: true` memakai pairing code
(`pairingNumber`), `false` memakai QR.

---

## 🧪 Testing

```bash
npm test      # 668 test, 71 file
npm run lint  # eslint, cakupan plugins/ saja
```

Suite memakai runner bawaan Node dengan mocking modul, dan seluruhnya berjalan
**offline terhadap stub socket** — tidak ada yang menjalankan bot, memanggil
WhatsApp, atau menyentuh Turso. Ini diverifikasi, bukan diasumsikan: suite
dijalankan dengan `net.connect`, `tls.connect`, dan `dns.*` diblokir lewat
preload, dan hasilnya tetap **668 pass / 0 fail** tanpa satu pun koneksi keluar.
Satu-satunya percobaan DNS adalah ke domain `.invalid` (RFC 2606, memang
diservis untuk tidak bisa di-resolve) di `tests/upstream-diagnostics.test.mjs`.

Untuk lantai yang benar-benar diuji: **Node v22.23.3**, sesuai `engines`.
Workflow CI menguji Node 22 dan 24.

### Cakupan test (71 file, dipilih)
| Area | File |
|---|---|
| Privilege escalation | `number-match.security.test.mjs` |
| Batas privilege jadibot | `jadibot-owner-privilege.test.mjs` |
| Gate admin fail-closed | `onwhatsapp-gate.test.mjs` |
| Merge/rename | `rename-invariants.test.mjs` |
| Session & kredensial | `creds-recovery.test.mjs`, `auth-key-corruption.test.mjs`, `turso-session*.test.mjs` |
| Memori & cache | `memory-leaks.test.mjs`, `memory-monitor-limit.test.mjs`, `message-dedup-cap.test.mjs` |
| Profiler | `profiler.test.mjs`, `profiler-heap-guard.test.mjs` |
| Stiker & media | `sticker-batch.test.mjs`, `animated-sticker-alpha.test.mjs`, `exif.test.mjs` |
| Keamanan input | `anti-crash-bounds.test.mjs`, `anticustom-pattern-safety.test.mjs`, `ffmpeg-injection.test.mjs` |
| LID / user store | `user-lid.test.mjs`, `lid-resolve.test.mjs`, `group-protection-lid.test.mjs` |

---

## 🛡️ Model Akses

Ini bagian yang paling sering digeneralisasi berlebihan, jadi ditulis sesuai
`src/lib/middleware.js`.

**Tiga flag per plugin**, dievaluasi berurutan di `checkPermission()`:

| Flag | Arti | Baris |
|---|---|---|
| `isOwner` | Hanya owner | `middleware.js:54` |
| `isPremium` | Premium (owner & partner otomatis lolos) | `middleware.js:69` |
| `isAdmin` | Admin grup, hanya berlaku di grup | `middleware.js:96` |

### `user.access` melonggarkan semua gate

`middleware.js:37-52` menghitung `hasAccess` bila `user.access` punya entri
untuk command itu yang belum kedaluwarsa. `hasAccess` itu di-OR ke **enam**
gate, bukan hanya `isOwner`:

`isOwner` (`:54`) · `isPartner` (`:61`) · `isPremium` (`:69`) ·
`isAdmin` (`:96`) · matikan fitur `game` per grup (`:120`) ·
matikan fitur `rpg` per grup (`:128`)

Artinya satu entri `user.access` yang tidak kedaluwarsa membuka command owner
untuk user itu. Ini perilaku yang disengaja, tapi harus diketahui operator
sebelum memberi akses.

### Plugin yang tidak memakai `isOwner`

Tidak semua plugin host memakai flag framework:

- **`plugins/vps/*` (6 file)** dan **12 dari 21 `plugins/panel/*`**
  mendeklarasikan `isOwner: false`, jadi gate framework tidak menyaringnya.
  Semuanya—the `vps` group dan 11 `panel`—punya gate kedua sendiri di dalam
  handler (`hasAccess`, `hasAccessToServer`, `hasFullAccess`, `canManageRole`)
  yang membaca daftar di `config.digitalocean.sellers` / `ownerPanels` atau
  store peran. Default-nya kosong, jadi **deny-by-default**.
- **`plugins/panel/cpanel.js`** adalah menu read-only tanpa gate kedua: ia
  menampilkan menu dan peran milik pemanggil. Tidak menulis apa pun.
- **`plugins/search/pap.js`** memakai gate khusus di dalam handlernya sendiri
  (lihat baris 92), terpisah dari `isOwner`.

### Pesan yang dikirim sendiri

`src/lib/serialize.js:770` memberi `m.isOwner = true` pada `m.fromMe`. Jadi
pesan yang bot kirim ke account-nya sendiri terbaca sebagai owner-tier.
Persis untuk `append` maupun `notify`, keduanya ditangani di
`src/connection.js:1382`. Ini jalur yang membuat `=>` (eval) dan `$` (exec)
di `src/connection.js:1614` dan `:1652` hidup. Jangan sampai account bot
dipakai untuk chat biasa.

### Jadibot bukan sandbox

Sub-bot berbagi process, filesystem, dan `.env` dengan bot utama. Batasnya
diterapkan di `src/handler.js:1667`:

- Kategori **`owner` diblokir seluruhnya**, bukan hanya plugin yang disebut.
- Kategori **`sewa`, `panel`, `store`, `pushkontak` diblokir**.
- Command `sewa*`, `jadibot*`, `botmode`, `restart`, `shutdown` diblokir.
- Prefix body `>>` dan `!!` diblokir.

Blokir ini berlaku di **grup maupun private chat**. Private dulu tidak ikut
terblokir; itu sudah diperbaiki. Plugin `sewa` sendiri ada di kategori
`owner`, jadi tercakup dua kali.

Nilai yang tersisa: `m.isOwner` di dalam jadibot berarti "owner jadibot itu",
bukan owner host — lihat `src/handler.js:762`.

---

## 💾 Kebutuhan Operasional

| Batas | Nilai | Sumber |
|---|---|---|
| Heap ceiling | **512 MB** | `package.json` → `start`: `--max-old-space-size=512` |
| RSS limit (monitor) | 550 MB | `src/lib/memory-monitor.js:3` |
| GC trigger RSS | 380 MB | `src/lib/memory-monitor.js:4` |
| GC trigger heap | 250 MB | `src/lib/memory-monitor.js:5` |
| Profiler RSS ceiling | 400 MB | `src/lib/profiler.js:42` |
| Profiler proyeksi ceiling | 800 MB | `src/lib/profiler.js:44` |

`npm start` sengaja mengunci heap di 512 MB. Itu batas keras, bukan saran:
node akan `OOM` sebelum host kehabisan RAM, bukan sesudahnya. Kalau bot
restart berulang, itu ceilings yang bekerja.

### CPU profiler menulis tanpa batas

`src/lib/profiler.js` menulis ke `storage/profiling/`. **Tidak ada logika
retensi sama sekali** — tidak ada prune, tidak ada ttl, tidak ada `unlink`
(`profiler.js:204` satu-satunya titik tulis). Isi direktori itu tumbuh
sampai seseorang menghapusnya secara manual. Di mesin ini direktori itu sudah
135 MB dari 25 file, dengan file tertua dari 30 Agustus.

Kalau `storage/profiling` adalah volume yang tidak dibersihkan, itu akan
makan disk. Periprogram perlu menambahkan retensi; ini belum ada.

### Angka RAM

Diukur di host Pterodactyl 512MB/1GB, `src/lib/profiler.js:17-18` mencatat
idle `rss 290-375MB` dan `heapTotal 185-195MB`. Itu kondisi idle, bukan
garansi runtime. Untuk angka 24 jam dan 72 jam, lihat
`docs/profiling-audit.md` dan `docs/ram-safety-audit-2026-09-21.md`.

---

## 🎬 TikTok: Rantai Fallback

Satu provider diblokir ≠ selesai. Rantainya:

```
src/scraper/tiktok.js (ttdown)      tikwm → savett → yuulabs → musicaldown
plugins/download/tiktokdl.js  (.tt) tikwm → savett(raw) → savett(resolved)
plugins/download/tiktokdl2.js (.tt2) savett.cc (csrf token + form POST)
```

- **tikwm** cepat, tapi sering kena Cloudflare 403 di IP datacenter.
- **savett** butuh token CSRF + cookie, dan mau menerima shortlink mentah.

Kalau semua kena blokir, `.tt` memang gagal. Itu batas provider, bukan bug —
`docs/` mencatatnya.

---

## ⚙️ Konfigurasi (`config.js`)

Nilai di bawah adalah default yang benar-benar ada di `config.js`, bukan
contoh karangan.

```javascript
mode: "public",                 // "public" | "self"
session: { usePairingCode: true, pairingNumber: "628..." },
command: { prefix: "." },
features: { autoRead: true, antiCall: false, smartTriggers: false, logMessage: true },
energi:  { enabled: true, default: 99999, premium: 99999999, owner: -1 },
registration: { enabled: false },
turso: { enabled: true, syncInterval: 5000 },
dev: { watchPlugins: true, debugLog: false },
```

Catatan yang sering disalahdokumentasikan:

- `energi.default` adalah **99999**, bukan 25. Batas energi praktis datang dari
  kuota per-command (`limit`), bukan dari default ini.
- `features.logMessage` aktif secara default.
- `dev.watchPlugins` aktif di config, tapi hot-reload hanya berjalan di
  `NODE_ENV=development` — yaitu hanya lewat `npm run dev`.

Opsi lain: `features.antiCall`, `registration.rewards`, `turso.syncInterval`,
`backup.{enabled,intervalHours,retainDays}`, `errorTemplate`.

---

## 🧩 Panduan Membuat Plugin

Taruh di `plugins/<kategori>/<nama>.js`, format ES Module:

```javascript
export const config = {
  name: "ping",                   // wajib (string atau string[])
  alias: ["p"],
  category: "utility",            # diisi otomatis dari nama folder
  description: "Cek responsivitas bot",
  usage: ".ping",

  isOwner: false,                 // lihat /docs model akses
  isPremium: false,
  isGroup: false,
  isPrivate: false,
  isAdmin: false,
  isBotAdmin: false,

  cooldown: 3,                    // detik
  limit: 1,                       # konsumsi energi
  isEnabled: true,
};

export async function handler(m, { sock, store, config, plugins }) {
  await m.reply("Pong!");
}
```

Setiap field opsional di-merge dari `defaultConfig` di
`src/lib/plugins.js:61`, jadi yang wajib hanya `config.name` dan `handler`.

### Objek `m`
`m.text` / `m.body` · `m.command` / `m.args` · `m.prefix` · `m.chat` ·
`m.sender` · `m.isGroup` / `m.isPrivate` / `m.isAdmin` / `m.isOwner` ·
`m.reply(text)` · `m.react(emoji)`

### Hot-reload
Di mode dev, file yang diedit dimuat ulang tanpa restart. **Tidak ada command
`reload`** — versi lama README menyebutkannya, dan command itu tidak ada di
kode. Reload dipicu oleh watcher, atau oleh plugin owner yang memanggil
`hotReloadPlugin()` secara eksplisit (`plugins/owner/addplugin.js:114`,
`ganticode.js:156`, `gantiscraper.js:93`).

---

## 🏗️ Infrastruktur & Arsitektur

Dokumentasi teknis internal ada di **`infra.md`**. Ringkasnya:

- **Entry point:** `index.js` → anti-crash, database, asset, plugin, scheduler,
  lalu `startConnection()`.
- **Routing:** `src/handler.js` → serialize → filter → case handler →
  proteksi grup → permission check → `getPlugin(command)` → `handler()`.
- **Plugin store:** `src/lib/plugins.js` — tiga Map: `commands`, `aliases`,
  `categories`.
- **Database:** LowDB (JSON lokal) untuk data utama; Turso LibSQL untuk
  session auth.
- **Case vs plugin:** `case/foto.js` menangani 4 grup command bawaan
  (`cping`/`cspeed`/`clatency`, `listallcase`, `listallplugin`, dan alias-nya)
  sebelum plugin mencari jalan.

### ⚠️ Benchmark
Tidak ada benchmark yang bisa direproduksi di repo ini. Berkas `run-sim.mjs`
dan `sim-load.mjs` adalah harness pengembangan lokal, **keduanya gitignored**
dan bukan bagian dari repo. Referensi benchmark di README versi lama
(9× lebih cepat, 186–233×, dan sejenisnya) berasal dari audit engine
`oktz-signal`, bukan dari pengukuran bot ini, dan tidak diulang di sini.

---

## 📄 Lisensi & Kredit

- **Author:** [Zann](https://github.com)
- **Base engine:** [`Onigi-Baileys`](https://github.com/OktzO/Onigi)
  v10.1.0-rc.6 — rebase `@whiskeysockets/baileys`.
- **E2EE:** [`oktz-signal`](https://www.npmjs.com/package/oktz-signal)
  — pengganti `libsignal` (GPL) dengan Rust native (MIT).
- **License:** ISC.

> Saat berkembang, `node_modules/onigis` dan `node_modules/ourin` bisa berupa
> symlink ke checkout lokal `Onigi/`, bukan paket npm. CI memasang dari
> `package-lock.json`, bukan dari symlink itu.
