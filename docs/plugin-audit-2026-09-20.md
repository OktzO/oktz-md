# Plugin Audit 2026-09-20

## Tujuan

Memastikan seluruh plugin (832 file, 34 kategori) di OKTZ-MD bekerja normal tanpa
masalah, dan menemukan kerentanan: secret leak, API error, kode usang, command
injection, SSRF, privilege misconfiguration, dsb.

## Lingkup

- `plugins/**/*.js` (832 file)
- Supplier plugin: `src/lib/ourin-plugins.js`, `src/handler.js` (routing/permission)
- Dep/API yang dipakai plugin (package.json)

## Fase A — Analisis (research only, parallel subagents)

Setiap agen menghasilkan file laporan temuan di workspace SDD plan ini.
Agen TIDAK mengubah kode — hanya baca + catat.

| # | Zona | Metode | Output |
|---|------|--------|--------|
| A1 | Load-verification semua plugin | Harness script: import tiap plugin via `loadPlugin`, catat gagal load + bentrok nama/alias | `a1-load-report.md` |
| A2 | Command injection & shell/exec/eval | Baca 23 situs `child_process`/`eval`; cek aliran input user → shell tanpa escape; cek plugin non-owner | `a2-exec-report.md` |
| A3 | Secret & hardcode | Scan hardcoded token/key/bearer, URL endpoint mati, placeholder `your-api-key`, kredensial panel | `a3-secret-report.md` |
| A4 | Kebablasan plugin load (owner 147) | Plugin owner: eval/exec/getplugin/delplugin/addplugin, bot management, backup — cek OWASP-ish + verify API benar | `a4-owner-report.md` |
| A5 | Group & RPG (101+67) | Group protection, welcome, warn, antilink, rpg economy — cek logic bug, missing await, collision command | `a5-group-rpg-report.md` |
| A6 | AI, Download, Search, Sticker (47+26+45+22) | Provider API call salah, outdated endpoint, SSRF, missing API key handling | `a6-ai-dl-search-report.md` |
| A7 | Sisa kategori (game, canvas, tools, cek, fun, main, panel, stalker, user, store, dsb) | Scan cepat error kasar: undefined var, import salah, object kosong, config rusak | `a7-misc-report.md` |

## Fase B — Konsolidasi temuan

- Gabungkan laporan A1–A7 → satu daftar temuan severity (Critical/High/Medium/Low).
- Verifikasi temuan kunci di repo (jangan percaya laporan buta).
- Laporkan ke user, tentukan scope fix.

## Fase C — Perbaikan (via SDD implement loop)

- Bagi temuan Critical/High yang disetujui menjadi task fix.
- Satu implementer subagent per task (atau batch same-shape), lalu task review,
  fix loop, dan final review.

## Task Fix (Fase C)

| Task | File | Temuan | Fix |
|---|---|---|---|
| T1 | `plugins/rpg/shop.js:97` | C1 infinite money (buy & sell negative amount) | clamp `amount = Math.max(1, parseInt(args[2]) || 1)` |
| T1 | `plugins/rpg/gift.js:30` | H1 item dupe | reject `amount <= 0` |
| T1 | `plugins/rpg/duel.js:26` | LOW non-numeric bet | validasi bet numeric |
| T1 | `plugins/tools/invoicemaker.js:60-76` | LOW negative price/total | reject negative |
| T1 | `plugins/store/editproduk.js:112-115` | LOW negative stock | reject `< -1` |
| T1 | `src/handler.js:1726,1807` | M3 cooldown alias bypass | key cooldown pakai primary command name |
| T2 | `plugins/owner/getplugin.js:119-125` | H2 path traversal | sanitasi `..`; validasi path di dalam plugins/ |
| T2 | `src/lib/ourin-middleware.js:38-59` | M4 hasAccess → semua owner cmd | scope hasAccess per command (whitelist) |
| T2 | `src/lib/ourin-roles-cpanel.js:149` + `panel/rolemanager.js:80-90` | M9 ceo→owner escalation | reject targetRole owner utk non-owner |
| T2 | `plugins/owner/backupsc.js:21-64` | M5 backup bocor storage+database | tambah exclude |
| T2 | `plugins/owner/stop.js` + `restart.js` | A2-LOW access-gate | internal `config.isOwner(m.sender)` gate |
| T3 | `plugins/tools/pastebin.js:38` | H4 dev key hardcoded | `process.env.PASTEBIN_DEV_KEY` |
| T3 | `src/lib/ourin-tmpfiles.js`, `ourin-uploader.js`, tourl, qrcustom, musikapaini, canvas/applemusic | H5 termai key ×7 | env `TERMAI_UPLOAD_KEY` |
| T3 | `src/lib/ourin-apimanager.js:1006,979,982` | M2 key fallback hardcoded | fallback `""` + warn |
| T3 | mangatoon/mcpedl/tiktokfoto/pin2/spamngl/lahelu/pakustad | M- cuki-x dead key | fallback `''`, guard |
| T3 | `.env.example` | env drift | tambah PASTEBIN_DEV_KEY, TERMAI_UPLOAD_KEY, IMGDROP_COOKIE, BRAT_*, FFMPEG_PATH, FFPROBE_PATH |
| T3 | `config.js` + `plugins/panel/linode.js` | linode slot hilang | tambah `APIkey.linode` |
| T4 | `plugins/nsfw/nsfw.js` | H6 ungated | isPremium: true (atau wajib verifikasi umur token) |
| T4 | `random/anime.js` (loli), `asupan/bocil.js` | A7-LOW ungated sensitive | gating konsisten dgn nsfw |
| T4 | `plugins/ai/gita.js:30-33` | H8 dead provider + unguarded | guard null + friendly error |
| T4 | `plugins/ai/text2img.js:32-35` | H9 unguarded | guard chain + key check |
| T4 | `plugins/ai/txt2img.js`, `to*.js` (10) | M15 unguarded | guard `result?.image/url` + react dalam try |
| T4 | `plugins/ai/dpsteai.js:53-68` | H7 IP-spoof | hapus spoof headers |
| T5 | collision 86 | berbagai collision | resolve: rename/hapus alias losser; jaga pemenang |
| T5 | `plugins/panel/seller.js` | A7-M8 dup | hapus file duplikat |
| T5 | `plugins/search/pin2.js` | dead cuki key + collision | hapus/alias ke pin |
| T5 | `plugins/search/gpt4o.js` | outdated model endpoint | alias/hapus |
| T6 | `plugins/sticker/attp2.js:82` | M11 axios tak di-import | import axios atau hapus path dead |
| T6 | `plugins/search/searchthatsong.js:24` | M12 no timeout | tambah timeout |
| T6 | `plugins/sticker/bratvid.js:48-57` | M13 temp leak | unlink di finally |
| T6 | `plugins/sticker/bratextra.js:177-184` | M14 UA-spoof | hapus spoof + validasi buffer |
| T6 | `plugins/owner/cekschedule.js:61-62` | H3 destructuring salah | pakai `.jadwal` |
| T6 | `plugins/tts/tts.js:24-42` | M swallow error | rethrow + guard |
| T6 | `plugins/fun/cekkhodam.js:88-98` | M silent TTS | reply di catch |
| T6 | `plugins/download/mediafiredl.js`, `sfiledl.js`, `search/nerdfont-ambil.js`, `carigrup.js` | unguarded nested | guard |
| T7 | eslint config | lint rusak | tambah `eslint.config.js`

## Global Constraints

- JANGAN menulis hardcoded secret baru; semua key lewat `process.env.*` / `config`.
- JANGAN mengubah kontrak publik plugin (`config` + `handler(m, { sock, ... })`).
- JANGAN menambah dependency baru tanpa izin.
- Pertahankan kompatibilitas: plugin yang sekarang load harus tetap load setelah fix.
- Fix harus diuji: `npm test` hijau dan plugin tetap termuat.