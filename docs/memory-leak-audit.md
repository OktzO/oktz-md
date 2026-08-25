# Memory-Leak & Efficiency Audit — Ourin MD

Date: 2026-08-25 · Host: 512MB heap, long-lived, frequent WhatsApp reconnects

Audit method: read every hot file (`index.js`, `src/connection.js`, `src/handler.js`, all `src/lib/*`), dispatched systematic sweeps of all 825 plugins across 34 categories, and verified every claim against the actual code path before patching. Regression tests added; full `node --test tests/` green before (44) and after (48).

---

## CRITICAL — patched

### C1. Orphaned CronJob on every reconnect — `src/lib/ourin-scheduler.js:417`
`initScheduler()` created `new CronJob("*/5 * * * *", …)` and never stored it in `activeCronJobs`, with no dedup. `initScheduler(config, sock)` runs from `index.js`'s `onConnectionUpdate` on **every** `connection === "open"` (plus once at boot) → one new live 5-minute job per reconnect, forever. Each fires `saveScheduledMessages()` every 5 min.

**Fix:** register under key `messageSaverTick` with the same stop-before-replace guard as `startDailyLimitReset()`. `activeCronJobs` exported for the regression test.

### C2. Orphaned CronJob per reconnect — `plugins/group/giveaway.js:361`
`startGiveawayChecker()` (called from `src/connection.js:572` on every open) created a `new CronJob("* * * * *", …)` every call, never stored/stopped → N per-minute jobs after N reconnects, each scanning all giveaways.

**Fix:** module-level `_giveawayChecker` ref; stop-before-replace.

### C3. Pruner never ran — `src/lib/ourin-data-pruner.js:13`
`startDailyPruner()` calls `getDatabase()` but the module never imports it → `ReferenceError` every 6h, swallowed by try/catch. `users`/`groups`/`premium`/`partner` in the DB (memory + disk) grew unbounded.

**Fix:** add `import { getDatabase } from './ourin-database.js'`.

---

## MODERATE — patched

| # | File:line | Problem | Growth bound / trigger | Fix |
|---|-----------|---------|------------------------|-----|
| M1 | `plugins/group/antispam.js:19,119` | `spamTracker` only `.delete()`d on kick path | 1 entry per `chat_sender` per message in antispam groups; never shrinks | Port slowmode sweep: when `size > 5000`, delete entries with `lastMessage` older than 10 min |
| M2 | `plugins/fun/waifupool.js:19` | `pages` only ever `.set()` | 1 entry per `sender:query` per command, forever | Store `{page, ts}`; when `size > 500`, sweep entries older than 30 min |
| M3 | `index.js:379-399` | On every reconnect: `getAllJadibotSessions()` re-read from disk, loops all sessions, `restartJadibotSession()` sleeps 3s each then fails on `jadibotSessions.has(id)` guard | Per reconnect, O(sessions) × 3s | Filter with `isJadibotActive(s.jid)` before the loop |
| M4 | `plugins/group/cekonline.js:46` | `sock.ev.on('presence.update')` with `.off()` only on happy path | 1 stacked listener per failed `.cekonline` (shared emitter) | `try/finally` around subscribe+wait so `.off()` always runs |
| M5 | `plugins/ai/ai.js:21` | `sessions` object only ever written | 1 entry per user who ever ran `.ai`, forever | Convert to capped `Map` (evict oldest past 500) |
| M6 | `plugins/fun/tembak.js:125` | `global.tembakSessions` never evicted if proposal unanswered (1h check at read time only) | 1 entry per abandoned proposal | `setTimeout` delete with timestamp guard |
| M7 | `plugins/jpm/jpm.js:618` | `jpmSessions` holds a media buffer until mode picked; abandoned flow leaks it | 1 buffer per abandoned owner flow | 10-min TTL with timestamp guard |
| M8 | `plugins/owner/swgc.js:275`, `swgcv2.js:248`, `swgcall.js:178` | pending maps hold media buffers + (swgcall) full group list; deleted only on confirm/error | 1 entry per abandoned owner flow | 10-min TTL with timestamp guard |
| M9 | `plugins/search/apkmod.js:70` | `global.apkmodSession` dead store (no reader anywhere; `apkmod-get` re-queries API) | 1 entry per `.apkmod` user, forever | Removed dead store entirely |
| M10 | `plugins/search/film.js:20,52` | `filmSessions` dead (filmget uses URL via button id) + un-cleared 5-min timers per call | 1 timer per invocation, up to 5 min | Removed dead map + timers |
| M11 | `src/lib/ourin-jadibot-manager.js:138` | `rateLimit` never evicted | 1 entry per `.jadibot` attempt, forever | Delete expired on next check + sweep past 100 entries |
| M12 | `src/lib/ourin-lid.js:5` | `lidCache` unbounded, persisted to `database/lid-cache.json` across restarts | grows monotonically, ~2 entries per contact seen | Cap at 10k (evict oldest), trim at load/save/population |
| M13 | `src/lib/ourin-serialize.js:836,1148` | Every `m.reply()` with `srtEnabled` does `readdirSync` + `readFileSync` | per bot reply | Module-cached file list + per-file buffer cache |
| M14 | `src/lib/ourin-auto-ai.js:10,186,1036` | `userCooldowns` never evicted; `autoai.sessions` per-sender history never pruned; `db.save()` full write per AI reply | per message | Cooldown sweep past 1k; session prune past 200 idle>24h; drop `db.save()` (autoai lives on `db.data`, not a store — `save()` never persisted it) |
| M15 | `plugins/main/menu.js:451` (+`allmenu.js`, `menucat.js`) | `fs.readFileSync` of static assets (`ourin`, `ourin2`) on every invocation of the hottest commands | per command | Use cached `getAssetBuffer("ourin"/"ourin2")` (preloaded at boot) |
| M16 | `index.js:366-377` | Dead imports `initSahurCron` (`plugins/religi/autosahur.js` — file doesn't exist), `startOrderPoller` (never defined), `startOtpPoller` (`src/lib/ourin-otp-poller.js` — file doesn't exist). All failed silently in try/catch on every reconnect | per reconnect | Removed |
| M17 | `plugins/group/giveaway.js:413` | `createSessions` wizard entry persists if admin abandons mid-flow | 1 entry per abandoned wizard | 10-min TTL (guard on `step===q1` + `chatId`) |
| M18 | `src/lib/ourin-plugins.js:593` | Dead `// require.cache removed` comment from CJS era | n/a | Replaced with accurate ESM documentation (see ESM note below) |

---

## Moderate / Minor — accepted, documented (no patch)

| File:line | Issue | Why accepted |
|-----------|-------|--------------|
| `src/lib/ourin-jadibot-database.js:5` | `jadibotDatabases` cache never deleted on `stopJadibot` | Needs export + stopJadibot integration; entries bounded by distinct jadibots ever started. Suggested follow-up: `deleteJadibotDb(id)` called from `stopJadibot`. |
| `src/lib/ourin-turso-session.js:4` | `keysCache` leaves ~6 scope keys per deleted session | **FIXED in follow-up audit (2026-08-25): `deleteTurboSession` now clears every category key for the scope, not just `:session`.** |
| `src/lib/ourin-jadibot-manager.js:377` | per-child `groupMetadataCache` never cleared | Dies with the child socket/session; bounded by groups per active jadibot. |
| `plugins/main/carifitur.js:66` | `loadAllPlugins()` uses `__dirname` (undefined in ESM) → always throws → generic error reply; per-invocation scan is therefore **never actually executed** | Pre-existing bug, not a leak. Fix (if feature wanted): `fileURLToPath(import.meta.url)` + module-level cache. |
| `plugins/rpg/breeding.js:119` | `petStorage.push` no sell/release path | DB array, bounded by active players × level≥5 pets; rate-limited. |
| `plugins/group/afk.js` | `global.afkStorage` lingers for users who never return | Low memory, small entries. Optional TTL. |
| `src/lib/ourin-socket.js:361` | `global.stickerPackCache` only grows | Low frequency (per sticker pack save). Optional LRU. |
| `src/lib/ourin-game-queue.js` | stale queue keys not auto-removed | Bounded by groups × game commands. |
| `src/lib/ourin-ffmpeg.js:54` | unbounded queue under sticker flood | Drains eventually; cap is optional hardening. |
| `src/lib/ourin-auto-anime.js:17-47` | sync fs per 5-min interval run | Low frequency. |
| `plugins/main/ping2.js:75` | `execSync` blocks event loop per `.ping2` | Cooldown 5s mitigates. |
| `plugins/main/leaderboard.js:94` | `readFileSync(ourin.png)` per invocation | Small file. |
| `plugins/canvas/kalender.js:572` | re-registers 7 fonts per command | Per-command, bounded. Optional `fontsReady` flag. |
| `plugins/canvas/fakestory*.js` | default PP `readFileSync` on profile-pic-fetch failure | Fallback path only. |

---

## Baileys reconnect listener verification (CLEAN)

- `store.bind(sock.ev)` (`src/connection.js:330`) registers listeners on the **per-socket** `sock.ev`. On reconnect `startConnection()` calls `connectionState.sock.end()` then sets `connectionState.sock = null` (lines 255-261) — the old socket (and its emitter + listeners) is fully dereferenced and GC-able. No double registration on a shared emitter.
- `startWatchdog`, `startTempCleaner`, `startMemoryMonitor`, `startDailyPruner`, `startAutoBioChecker`, `initNotifScheduler`, `initSholatScheduler`, `initAutoJpmScheduler`, `startGroupScheduleChecker`, `startSewaChecker`, `initAutoBackup` — all verified to have dedup guards (`if (timer) return`, stop-before-replace, or `clearTimeout` before reschedule).
- `flushInterval` in `connection.js` self-clears when `isConnected` goes false.

## ESM hot-reload limitation (finding #2)

`loadPlugin(filePath, bustCache=true)` (used only by `hotReloadPlugin`) imports with `?t=` cache-busting. Node's ESM loader keeps every distinct module graph alive for the process lifetime — there is no unload API. Old module code is dropped from `pluginStore`, but its code + captured state is only reclaimed on process restart.

- **Gating:** the bust-cache path only runs when `config.dev.enabled && config.dev.watchPlugins`. Note `config.dev.enabled = (NODE_ENV === "development")`, and the `npm start` script sets `NODE_ENV=development` — so **hot reload is live under `npm start`**, not only under `npm run dev`. If a deployment uses `npm start`, edits to plugins accumulate module instances until restart.
- **Accepted:** not patched per scope (dev-only tradeoff, restart reclaims). If it ever matters in production, isolate hot-reloaded plugins in a `worker_thread` that can be `.terminate()`d, or set `NODE_ENV` without `development` in prod.

---

## Changes made

- `src/lib/ourin-scheduler.js` — messageSaverTick tracked; `activeCronJobs` exported.
- `plugins/group/antispam.js` — tracker sweep; `spamTracker` exported.
- `plugins/fun/waifupool.js` — pages cap/TTL; `pages` exported.
- `index.js` — jadibot active-session filter; dead imports removed.
- `plugins/group/giveaway.js` — checker singleton; createSessions TTL.
- `src/lib/ourin-data-pruner.js` — missing import.
- `plugins/group/cekonline.js` — try/finally `.off()`.
- `plugins/ai/ai.js`, `plugins/fun/tembak.js`, `plugins/jpm/jpm.js`, `plugins/owner/{swgc,swgcv2,swgcall}.js` — TTL/caps.
- `plugins/search/{apkmod,film}.js` — dead stores removed.
- `src/lib/{ourin-jadibot-manager,ourin-lid,ourin-serialize,ourin-auto-ai}.js` — eviction/cache fixes.
- `plugins/main/{menu,allmenu,menucat}.js` — cached asset buffers.
- `tests/memory-leaks.test.mjs` — 4 new regression tests (scheduler, antispam, waifupool).

Test status: `node --test tests/` → **48 pass, 0 fail** (was 44).

---

## Follow-up 2026-08-25: WhatsApp auth session lifecycle (main scope)

Session invalidation previously dead-ended: `loggedOut`/401 only logged "hapus folder storage lalu restart" and returned, leaving the process alive on a dead socket until a human intervened. Also `logout()` and `deleteTurboSession()` never cleaned up fully, and per-key Turso writes were non-atomic.

### S1. Main-session auto-recovery — `src/connection.js`

**Before (silent dead-end):** disconnect with `loggedOut`/401 → log message telling a human to manually delete storage and restart → `return`. No local folder removal, no Turso row removal, no reconnect. Process stayed alive, disconnected, forever.

**After:** `loggedOut`/401 now:
1. Deletes local `storage/<session folder>` (same path `logout()` uses).
2. Calls `deleteTurboSession("main")` when `config.turso.enabled && config.turso.url` (mirrors `stopJadibot(jid, true)`).
3. Schedules `startConnection(options)` after `config.session.reconnectInterval || 15s` → new pairing code / QR generated automatically.

Retry-storm guard: reuses the bounded convention from the 440 branch — counts up through `maxReconnectAttempts` (default 5), and after the cap logs a clear "needs manual intervention (banned / re-register)" state instead of looping forever. On successful `connection === "open"`, `reconnectAttempts` resets to 0 as before.

**Full end-to-end flow now:** disconnect reason received → session folder + Turso row (all categories) wiped → `startConnection()` re-inits auth state (fresh, unregistered) → `usePairingCode`? pairing code : QR printed → user pairs. No manual restart.

### S2. `logout()` also clears Turso — `src/connection.js`

**Before:** removed local folder + `sock.logout()`, but left stale `session_creds`/`session_keys` rows for `scope: "main"` in Turso. Next boot, `loadState("main")` loaded those stale creds → immediate re-`loggedOut`.

**After:** `logout()` additionally calls `deleteTurboSession("main")` (guarded on `config.turso.enabled && config.turso.url`, same as `stopJadibot`). Stale creds never survive a deliberate logout.

### S3. Atomic per-key Turso writes — `src/lib/ourin-turso-session.js`

**Before:** `keys.set()` looped `await client.execute()` once per key — a crash / dropped connection / Turso hiccup mid-loop left Signal ratchet state (session/pre-key/sender-key) torn between DB and WhatsApp's expectations. Plausible real cause of unexplained invalid sessions.

**After:** builds one `statements[]` array (all types × ids) and runs `client.batch(statements, "write")` — atomic all-or-nothing. In-memory `keysCache` update logic unchanged. `saveCreds()` verified single-row upsert → no change needed.

### S4. `deleteTurboSession` cache invalidation — `src/lib/ourin-turso-session.js`

**Before:** only `keysCache.delete(scope + ":session")` — left `pre-key`, `sender-key`, `app-state-sync-*`, etc. cached in memory. A re-paired session under the same scope could be served stale in-memory state instead of fresh empty state.

**After:** iterates `keysCache` and deletes every key whose prefix matches `` `${scope}:` ``. Safe when Turso disabled (`getTurboClient()` returns null → no-op, cache wipe still correct). Shared with `stopJadibot` — that caller re-verified, no change needed.

### Changes made

- `src/lib/ourin-turso-session.js` — `keys.set()` batches; `deleteTurboSession()` wipes all scope categories from `keysCache`.
- `src/connection.js` — `loggedOut`/401 auto-clears session + Turso + reconnects with bounded retry; `logout()` deletes Turso row.
- `tests/turso-session-atomic.test.mjs` — new: batch atomicity (single `batch()` call, all rows land), failure atomicity (no partial rows), scope-wide cache clear.

Test status: `node --test tests/` → **83 pass, 0 fail** (was 48).
