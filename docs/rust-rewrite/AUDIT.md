# AUDIT — `oktz-md` (read-only, faktual)

Generated: **2026-10-10** · Angka dihitung dengan `find`/`wc`/parse AST, bukan asumsi. Zaman ini mengoreksi sebagian angka di brief (lihat §0).

---

## 0. Koreksi terhadap brief

| Brief | Aktual (terukur) |
|---|---|
| ±169.000 baris JS | **143.186** (plugins 95.922 + src 46.264 + index/config ~1.000) |
| 829 plugin | **830** file, **33** kategori (bukan 34) |
| 87 modul `src/lib` | **92** file |

Nomor **832** di sini = 830 plugin + `case/foto.js` + satu file lagi. Fine.

## 1. Plugin — 830 file, 95.922 LOC

Distribusi per kategori (terbesar): owner 151, group 104, rpg 67, tools 56, cek 48, ai 46, search 46, fun 40, game 36, canvas 30, download 24, lalu 26 kategori kecil.

Klasifikasi lengkap di [`PLUGIN-CLASSIFICATION.md`](./PLUGIN-CLASSIFICATION.md) + `plugin-manifest.json` (machine-readable).

**Kontrak `pluginConfig`** (dari `src/lib/plugins.js:61-77`): loader wajib `{config, handler}` dengan `handler: function`. Config di-merge dengan default; `name` kosong → fallback `basename(file)`.

Field yang **benar-benar muncul** di dataset (frequency): `name` 797, `category` 797, `description` 797, `usage` 797, `alias` 795, `cooldown` 793, `isEnabled` 793, `energi` 791, `example` 772, `isOwner` 761, `isGroup` 748, `isPremium` 737, `isPrivate` 735, `isAdmin` 88, `isBotAdmin` 52, `skipRegistration` 2, `isBanned` 1, `noPrefix` 1, `customTrigger` 1. **Field `limit` ada di default tapi 0 plugin override.**

Pola ekspor: 733 `export { pluginConfig as config, handler }`, 65 + ekstra, 8 `export { config, handler }`, 1 `export default {config, handler}` (`main/menu.js`), 23 `games.createPlugin(...)`.

**Signature handler:** `handler(m, ctx)`. `m.*` paling sering: `reply` 3694, `prefix` 1706, `react` 1698, `chat` 1040, `sender` 806, `quoted` 774, `command` 433, `pushName` 424, `args` 316, `text` 233, `mentionedJid` 162, `download` 102. `serialize()` assign **83 field** ke `m`.

**`isEnabled: false` di kode: 1** — `plugins/search/playcall.js`. Tapi `owner/disableplugin.js:82` menulis `isEnabled:false` ke source saat runtime, jadi state bisa ada di luar git.

**15 plugin paling kompleks** (LOC + state/import): `jpm/jpm.js` 1490, `main/menu.js` 1412, `game/werewolf.js` 1145, `search/play2.js` 1131, `game/minecraft.js` 1018, `search/ytplay.js` 914, `owner/schedule.js` 908, `game/fisch.js` 872, `pushkontak.js` 842, `group/giveaway.js` 614 (satu-satunya pakai `cron.CronJob`), `user/daftar.js` 590, `game/ulartangga.js` 600, `sticker/bratvid2.js` 612. **49 file menyentuh `global.*`**; 21 file punya state module-level.

## 2. src/ — 161 file, 46.264 LOC

`src/lib` 92 file / 32.628 · `src/scraper` 59 / 7.542 · `src/capabilities` 8 / 1.695 · root 2 (`handler.js` 2.405, `connection.js` 1.994).

File >500 LOC (paling mahal di-port): `handler.js` 2405, `builder.js` 2263, `group-protection.js` 2198, `connection.js` 1994, `serialize.js` 1828, `rich-messages.js` 1637, `socket.js` 1343, `apimanager.js` 1259, `auto-ai.js` 1189, `jadibot-manager.js` 1153, `database.js` 938, `turso-session.js` 854, `scheduler.js` 815, `lid.js` 654, `welcome-card.js` 600, `plugins.js` 598, `minecraft.js` 591, `formatter.js` 540, `utils.js` 507.

**Graf dependensi:** `config.js` di-import 34 file · `lib/logger.js` 22 · `lib/database.js` 17 · `lib/http.js` 11 · `lib/aggregator.js` 8. Cluster: core (`handler → plugins, database, group-protection, serialize, context, lid`), io (`serialize/rich-messages/builder/asset-manager`), net (`http/aggregator/apimanager/circuit-breaker/async-pool`), persistence (`database ↔ turso, turso-session, premium-db, jadibot-database, auto-backup`), scheduler (9 modul).

Eksternal by file-count: `axios` 55, `onigis` 21, `cheerio` 16, `sharp` 15, `form-data` 13, `@napi-rs/canvas` 5, `cron` 5, `btch-downloader` 4, `qrcode`/`archiver` 3, `tesseract.js` 1, `@libsql/client` 1.

## 3. Data & config

`config.js` 577 baris, `export default` + 8 helper (`isOwner`, `isPremium`, `isPartner`, `isBanned`, `setBotNumber`, `isSelf`, `getOwnerName`). `config.session`: `pairingNumber`, `usePairingCode`, `pairingRateLimitCooldownMs`, `pairingFallbackToQr`, `qrDir`, `qrPngWidth`, `qrMaxPrints`, `keepAliveIntervalMs`. `config.energi`: `{enabled, default, premium, owner:-1}`.

`.env.example` 19 var (nilai disensor): TURSO_URL/AUTH_TOKEN, APIKEY_{NEOXR,FGSI,COVENANT,OBSCURA,FIREFLY,CUKI}, FGSI_API_KEY, GROQ_API_KEY, TOPMEDIA_API_KEY, TERMAI_UPLOAD_KEY, PASTEBIN_DEV_KEY, LINODE_API_TOKEN, IMGDROP_COOKIE, BRAT_FONT_URL, BRAT_EMOJI_URL, FFMPEG_PATH, FFPROBE_PATH. **Kode juga baca `DEBUG_PLUGINS`, `PATH`, `NODE_TEST_CONTEXT` yang TIDAK ada di `.env.example`.**

File runtime (kompat byte-level):
- `database/main/users.json` — **604 key × 35 field**, 343KB
- `groups.json` 16 key · `settings.json` 64 key · `stats.json` 416 key
- `database/lid-cache.json` 457KB
- `storage/session/creds.json` 22 key + `pre-key-*.json` + `device-list-*.json` + `session.broken-*`
- `storage/qr/qr-*.png`, `storage/load-audit.json`
- `data/` asset (`Roboto_Medium.ttf`, `quizbattle.json`, husbu/waifu/Demo_fonts)
- `src/data/` 33 JSON (tebak*, family100 **1.5MB**, nsfw/)
- `assets/` 21 file / 7.4MB, **21/21 entri `config.assets` verifikasi ADA**
- `tmp/` 23 file / 28.9MB (`emoji-apple.json` **28.5MB**)

## 4. Tests

**102 file, 1.258 kasus** (`node:test` + `node:assert`, tanpa framework eksternal). Terbesar: `capabilities-phase1` 263, `pairing-state` 46, `youtube-capability` 43, `fallback-providers` 33, `resolve-core` 33, `antiswgc` 32.

**51 file / 444 kasus relevan port 1:1:** pairing/session 13 file / 135 · router/handler 13 / 124 · group-protection 11 / 99 · database/memory 14 / 86. Ini bahan test paritas utama.

**Sulit pindah:** 23 file pakai `mock.module` (butuh flag Node, tak ada di Rust) → trait mock DI. 13 file kena API network → fixture offline. 2 file override waktu (`Date.now`) → clock trait. Sisanya ~68 file assert murni → port langsung.

## 5. Native & Rust yang sudah ada (untuk reuse)

- `ourin-md/native/` — crate `ourin-native` (napi-rs 3.12 + tesseract-rs 0.4 + image 0.25), 74 LOC (`ocr_init`/`ocr_recognize`/`ocr_shutdown`), prebuilt gnu-x64 10.2MB. **Dipakai ulang di Rust**, dilepas dari napi. Tidak ada file LICENSE di `native/` (root ISC).
- `/home/user/noddjs/oktz-signal/` — `oktz-signal` 0.3.0-rc.3, **MIT**, 4.343 LOC Rust murni (`ratchet` 1732, `proto` 686, `x3dh` 630, `session` 522, `curve` 487). **Bukan engine** (SenderKey/group E2EE "not implemented by design"), tapi Rust murni tanpa napi → kalau butuh, bisa dipakai sebagai lib. Tidak dihapus.
- `/home/user/noddjs/Onigi/` — **source repo** paket `onigis` v10.1.0-rc.12 yang dipakai bot. MIT. Description: "Onigi-Baileys — rebase Baileys 7.0.0-rc14 + E2EE via oktz-signal". `lib/` 103 file / 42.909 LOC. Punya crate `curve25519-rs` (381 LOC) + **bridge ke `whatsapp-rust-bridge` (author sama dgn kandidat engine (A))**.

## 6. Baseline RAM (dari audit lama, untuk target Rust)

| Skenario | RSS median |
|---|---|
| baseline Node | ~84MB |
| canvas card | 99MB peak |
| OCR worker-per-call | **216MB peak** (paling buruk, sudah diganti native → 181MB flat) |
| convert via ffmpeg (subprocess) | flat (152→152) |

Target Rust: **idle < 60MB, puncak < +40MB** — **belum terverifikasi achievable** (lihat RISKS R3, ENGINE-DECISION §5).

---

## Tidak terverifikasi / catatan
- Ada worktree `.worktrees/fase1-resolver/` = salinan `src/`,`plugins/` dengan LOC identik. Angka di sini **tidak termasuk** worktree itu.
- `eng.traineddata` 5.0MB di root — dipakai OCR native, di luar `config.assets`.
- Hanya `linux-x64-gnu` native yang terpasang (musl belum ada untuk `ourin-native`).