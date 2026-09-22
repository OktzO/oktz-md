# RAM Safety & Safe Freeing Audit — Ourin MD

Date: 2026-09-21
Goal: full sweep of RAM usage safety + safe memory freeing. Not a leak-redo of the
2026-08-25 audit — verify its fixes still hold, then find what is NEW (plugins since
f281394: bratvid2, play2, brattheme, crm + brat theme work).

Global constraints:
- No new dependencies. Stdlib/node:buffer first.
- Every freeing path must be side-effect safe: no user data loss, no reply broken,
  TTL over blind delete where state matters. Caps + sweeps; never unbounded Maps/arrays.
- Runtime guards already exist: ourin-memory-monitor (RSS limit 550MB, GC at 380MB/250MB,
  check 2min), ourin-profiler heap-snap RSS ceiling 400MB.
- Do not touch eval/exec eslint-disable bindings. Keep all public plugin contracts.
- Verified against working code path; every finding file:line + mechanism + fix.
- Prior audit (docs/memory-leak-audit.md) is the baseline — spot-verify at least
  M1/M2/M13/M14/M15 caps still hold; do NOT re-report already-fixed findings.

Tasks (phase A = analysis, parallel; phase B = fixes, SDD per-task):
A1  src/connection.js, src/handler.js, index.js, src/lib/ourin-socket.js, ourin-serialize.js:
    reconnect listener stacking, per-message retained state, hot-path allocations.
A2  All src/lib/ourin-*.js module caches: unbounded Maps/arrays, dead stores, DB
    in-memory indexes, session/asset caches.
A3  Schedulers/timers/queues: ourin-scheduler, autobackup, keepAlive, group schedule,
    ourin-ffmpeg queue, game queue, periodic intervals in plugins.
A4  plugins/ crawl (groups, main, fun, owner, search, ai, rpg, jpm): sessions/cooldown/
    tracker/dedupe Maps with and without TTL + cap; media-buffer-holding pending maps.
A5  Media/download/file paths: download.stream buffers, temp files, asset caches,
    srt cache, sticker/media buffers held past use.
A6  Recent commits since f281394 (bratvid2, play2, brattheme, crm, jadibot, helper):
    new code for leaks + verify memory monitor whoop wiring index.js + restart paths.

# Phase A — HASIL (2026-09-21, 4 subagent baca paralel: findings-b1..b4)

REGRESI: 0. Semua cap lama HOLD: M1 (antispam 5000/10min), M2 (waifupool
500/30min), M13 (srt module-cache LRU50 — no per-reply readdir), M14 (auto-ai
sweep 1k / prune 200-idle24h / db.save gone), M15 (asset cache; sisa
readFileSync mp4/mp3 by-design), M16 (dead import gone), M17 (giveaway 10min),
M11 (jadibot rateLimit evicted), M12 (lidCache 10k). Reconnect listener
stacking bersih; scheduler crons dedupe; ffmpeg queue bersih; auto-download
URL-passthrough bersih; game queue bersih.

## Temuan (bidang → prioritas)

### C1 Critical — plugins/sticker/bratvid2.js:51,88
`emojiImageCache` module-Map unbounded, simpan decoded canvas images FOREVER,
key attacker-controlled (emoji dari prompt). Fix: cap 128 + FIFO evict — mirror
`src/lib/ourin-brat.js` `BRAT_EMOJI_CACHE_MAX=128`.

### C2 Critical — plugins/sticker/bratvid.js:10-16
Lazy import `brat-canvas/video` add ±200MB RSS PERMANENT setelah pemakaian
pertama; tak recoverable di bot heap 512MB. Fix: RSS gate — kalau
`process.memoryUsage().rss > 480MB` saat akan render bratvid, tolak render
(throw ramah "bot kurang RAM") daripada import gemuk.

### I1 Important — ioqueue banjir: connection.js:35,1275 + index.js:321
AsyncPool(8) antrian tali tak terbatas saat banjir; timeout `Promise.race`
60s tidak membatalkan handler gantung → 1 lane + protobuf penuh kepegang
selamanya. Fix: cap antrian (≥64, drop-newest + warn) — tidak usah abort
handler jalan (mustahil aman), queue-bound cukup.

### I2 Important — global.groupMetadataCache (connection.js:886, serialize.js:829)
Tanpa TTL di bawah 1000, survive reconnect, tiap entry pegang
`participants[]` penuh → jutaan byte monoton. Fix: age-sweep (drop entry
diakses >30 menit lalu) saat growth.

### I3 Important — src/lib/ourin-turso-session.js:27-48
`keysCache` cap 1000 hanya di `set`; `get()` load key DB ke Map per-scope
uncapped. Fix: cap 1000 FIFO pada load.

### I4 Important — src/lib/ourin-scheduler.js:555-557
`notifiedGroups` clear per-jam keliru: butuh callback tepat `HH:00:00`; drift
cron luput → Set tumbuh 1/group/menit selamanya. Fix: sweep by AGE (>60 menit)
bukan jam pas.

### I5 Important — src/lib/ourin-lid.js:528-533
`cacheLidJid` (dipanggil per pesan via serialize) bypass `trimLidCache()`;
`savePersistentCache` tak pernah trim → bisa tembus 10k. Fix: panggil
trim/cap-check di setter-hot path.

### I6 Important — src/lib/ourin-jadibot-manager.js:388,409
Per-session `groupMetadataCache` tak pernah evict. Fix: cap 200 FIFO/TTL
(main-bot pun sudah di-cap).

### I7 Important — src/lib/ourin-socket.js:529 + src/handler.js:602
`global.stickerPackCache` — cap entry bukan byte; body = SEMUA buffer sticker;
auto-save tiap pack masuk. Fix: byte-cap (e.g. 200MB sederhana: hitung total,
evict oldest) + jangan persist tiap pack (debounce/mark dirty).

### I8 Important — plugins/group/afk.js:3-47
`afkStorage` unbounded: entry dihapus cuma saat user AFK bales lagi; user yang
tak kembali bocor selamanya. Fix: sweep >5000 drop `since` >24 jam.

### I9 Important — plugins/group/absen.js:17
`global.absensi` per-chat, tak disapu, cuma `.hapusabsen` manual. Fix: sweep
24 jam saat akses.

### I10 Important — emoji-apple.json parse ganda (src/lib/ourin-brat.js:201,287 + bratvid2.js:50,63)
27.2MB JSON di-`JSON.parse` dua kali → dua objek module FOREVER (±60-100MB).
Fix: shared helper + cap.

### M-j Important — src/lib/ourin-memory-monitor.js:2,38
Limit RSS 550MB dekoratif; cuma GC 380/250 ter-wire. Fix: wire action RSS
>550MB (N check beruntun) → warn owner + restart bersih via mekanisme restart
yang ada di index, atau owner-warn saja kalau tak ada restart path aman.

### Minor (batch di Task 6, atau deferred)
- auto-ai cap 200 cuma prune idle>24h → bisa tembus saat >200 sender aktif.
  Ikut Task 3 (prune by size juga).
- socket.js:858 getName tulis `store.contacts[jid]` tanpa cek cap 500. Ikut Task 5.
- index.js:95,133 watcher dev `fileStatCache` uncapped (pakai npm start). Task 6.
- scheduler `repeat:true` tak pernah expire (106-137). Task 4.
- swgc.js:271 / swgcv2.js:244 timeout temp `tempFile` bocor (swgcall.js:185
  benar). Task 6.
- srt.js:24-54 session per-chat tanpa TTL. Task 6.
- play2.js:375,395 size cap sesudah buffer penuh + ffmpeg di luar `queueFFmpeg`
  gate. DEFERRED (butuh judgment; bukan leak RAM murni).
- smemevid.js:138-147 temp leak path error. Task 6.
- os.tmpdir temp di luar scan cleaner. DEFERRED (info).
- serialize.js:1025-1307 per-reply sharp resize churn (2-3x resize+2 upload).
  DEFERRED (churn bukan retention).

# Fase B — Fix tasks (SDD)

Semua fix: no dep baru, stdlib first, cap+sweep TTL (tak pernah blind delete
data user), jaga kontrak plugin publik, jangan sentuh binding eval/exec.

## Task 1: bratvid2 emoji-json cache dibatasin (C1 + I10)
File: plugins/sticker/bratvid2.js, src/lib/ourin-brat.js.
- `emojiImageCache` Module-Map bratvid2: cap 128 + FIFO evict (mirror
  `BRAT_EMOJI_CACHE_MAX=128` di ourin-brat.js).
- Dedupe `emoji-apple.json`: sekali parse di shared helper bermodul-import
  (json masih `import json with {type:'json'}`? kalau ya—sip, impor ganda
  tetap satu instans per-module; kalau masih `readFileSync`+JSON.parse dobel,
  pindah ke shared module import ESM). Hasil akhir: tidak ada dua objek
  27MB hidup bareng di module singleton.

## Task 2: bratvid RSS gate (C2)
File: plugins/sticker/bratvid.js (juga cek bratvid2.js kalau import crossover).
- Sebelum lazy-import `brat-canvas/video`: kalau
  `process.memoryUsage().rss > 480`MB → tolak render ramah ("bot lagi low RAM,
  coba nanti") — JANGAN import modul gemuk.
- Import gemuk tetap lazy alias cuma sekali saat perlu.

## Task 3: sweep-batch unbounded maps (I3, I5, I6, I8, I9, auto-ai)
Satu implementer, pola sama (cap+sweep TTL). File:
- src/lib/ourin-turso-session.js — cap 1000 FIFO pada `get()` load.
- src/lib/ourin-lid.js — hot setter `cacheLidJid` dipanggil ke trim/cap 10k.
- src/lib/ourin-jadibot-manager.js:388 — per-session meta cache cap 200 evict oldest.
- plugins/group/afk.js — sweep >5000, drop `since` >24 jam; tetap jangan
  hapus AFK aktif (<24 jam).
- plugins/group/absen.js — sweep global.absensi >1 jam saat akses (hapus chat
  yang lastUpdate >24 jam).
- src/lib/ourin-auto-ai.js — prune by size: saat >250, drop oldest (jaga juga
  prune idle-24 jam existing). Jangan sentuh logika AI.
TTL value + cap angka eksak ada di atas; jangan cari-cari nilai sendiri.

## Task 4: scheduler + io-queue (I1, I4, repeat-expire)
File: src/lib/ourin-scheduler.js, src/connection.js, index.js (hanya watch).
- `notifiedGroups`: ganti clear-pas-jam → sweep entry umur >60 menit di tiap run.
- `repeat:true` tasks: dukung `expiresAt` (ms) opsional; kalau ada dan lewat →
  berhenti + lapor. Tanpa `expiresAt` = perilaku lama (tak expire). Default
  AMAN: tidak berubah untuk task existing.
- AsyncPool antrian: cap 64, drop-newest + warn. JANGAN ubah timeout 60s
  behavior selain itu.

## Task 5: tanda-taman chat metadata + sticker pack (I2, I7, M-b)
File: src/lib/ourin-serialize.js, src/lib/ourin-socket.js, src/handler.js, src/connection.js.
- `global.groupMetadataCache`: age-sweep entry keakses >30 menit lalu saat
  size >800 (jangan berubah saat <800). Full participants tetap disimpan.
- `global.stickerPackCache`: byte-cap 200MB — total ukuran diancer-kecek di
  tiap set; lewat → evict oldest pack. Persist: jangan `save()` tiap pack
  masuk (debounce/mark-dirty, persist saat shutdown/schedule).
- `getName` socket.js:858: cek cap 500 sebelum tulis `store.contacts[jid]`.

## Task 6: monitor-limit + minor cleanup (M-j + swgc/smemevid/srt/index-watch)
File: src/lib/ourin-memory-monitor.js, index.js, plugins/owner/swgc.js,
plugins/owner/swgcv2.js, plugins/sticker/smemevid.js, plugins/owner/srt.js,
plugins/owner/helper.js (kalau fileStatCache di index).
- memory-monitor: RSS >550MB selama 3 check beruntun → kirim warn owner +
  restart bersih pakai mekanisme restart yang ADA (teliti index/connection:
  `process.exit`? baileys restart? pakai yang ada, jangan baru). Kalau tidak
  ada restart path, cukup warning keras.
- swgc/swgcv2 timeout path: `unlink(tempFile)` (copy swgcall.js:185), jangan
  double-unlink error path.
- smemevid error paths: `unlink` siocle temp.
- srt.js session: idle TTL 10 menit saat akses.
- index watcher dev `fileStatCache`: cap 800.
Deferred (catat, bukan task): play2 size-cap & ffmpeg gate, os.tmpdir cleaner
scope, serialize per-reply churn.
