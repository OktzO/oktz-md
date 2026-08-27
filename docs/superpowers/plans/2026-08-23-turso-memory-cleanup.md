# Turso DB+Session, Memory Audit, Deep Cleanup — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Migrate DB + session storage to Turso (libsql), reduce memory pressure, and improve cleanup coverage.

**Architecture:** DB layer keeps JSON blob per store in Turso (same in-memory API for 267 plugins). Session uses custom `useTursoAuthState()` replacing `useMultiFileAuthState()`. All with fallback to file mode if Turso disabled/unavailable.

**Tech Stack:** Node 20, `@libsql/client`, `@napi-rs/canvas` (already), `ourin-baileys@^9.0.11`, `lowdb` (fallback only)

**Spec:** `docs/superpowers/specs/2026-08-23-turso-memory-cleanup-design.md`

## Global Constraints
- `@libsql/client` must be added to `package.json` dependencies.
- Config section `turso` with `url`, `authToken`, `syncInterval`, `enabled`.
- All existing npm test (`node --test tests/`) must pass.
- Bot boot with 791 plugins must not error.
- No behavior change to ANY plugin — `db.data.*` API identical.
- Fallback: `config.turso.enabled = false` → full file mode.

---

### Task 1: Install dep + Config

**Files:**
- Modify: `package.json`
- Modify: `config.js`

**Interfaces:**
- Consumes: nothing
- Produces: `config.turso` object, `@libsql/client` available

- [ ] **Step 1: Add `@libsql/client` to package.json**

Edit `package.json` dependencies:
```json
"@libsql/client": "^0.14.0",
```

- [ ] **Step 2: Add turso config block to config.js**

After line 196 (`database: { path: "./database/main" }`), add:
```js
turso: {
  url: process.env.TURSO_URL || "libsql://your-database-name.turso.io",
  authToken: process.env.TURSO_AUTH_TOKEN || "",
  syncInterval: 5000,
  enabled: true,
},
```

- [ ] **Step 3: Install dep**

```bash
cd /home/user/noddjs/ourin-md && npm install
```

- [ ] **Step 4: Verify config loads**

```bash
node --check config.js
```

- [ ] **Step 5: Commit**

```bash
git add package.json config.js
git commit -m "feat: add @libsql/client + turso config"
```

---

### Task 2: Create Turso Client Helper

**Files:**
- Create: `src/lib/ourin-turso.js`

**Interfaces:**
- Consumes: `config.turso`
- Produces: `getTursoClient()`, `initTursoTables()`, `backupToTurso()`, `closeTurso()`

This module is a thin wrapper around `@libsql/client`. It creates tables if they don't exist, provides a shared client, and handles connection errors gracefully.

- [ ] **Step 1: Write the test**

Create `tests/turso-helper.test.mjs`:
```js
import { describe, it, before, after } from 'node:test';
import assert from 'node:assert';

describe('turso helper', () => {
  it('returns null client when turso disabled', async () => {
    process.env.TURSO_ENABLED = 'false';
    const { createTursoClient } = await import('../src/lib/ourin-turso.js');
    const client = createTursoClient({ enabled: false });
    assert.strictEqual(client, null);
  });
});
```

- [ ] **Step 2: Write the implementation**

`src/lib/ourin-turso.js`:
```js
import config from "../../config.js";

let client = null;

async function createTursoClient(cfg) {
  if (!cfg || !cfg.enabled || !cfg.url) return null;
  try {
    const { createClient } = await import("@libsql/client");
    client = createClient({
      url: cfg.url,
      authToken: cfg.authToken,
    });
    return client;
  } catch (e) {
    console.warn("[turso] failed to create client:", e.message);
    return null;
  }
}

function getTursoClient() {
  return client;
}

async function initTursoTables(client) {
  if (!client) return;
  const sql = `
    CREATE TABLE IF NOT EXISTS stores (
      key TEXT PRIMARY KEY,
      data TEXT NOT NULL,
      updated_at INTEGER NOT NULL
    );
    CREATE TABLE IF NOT EXISTS session_creds (
      scope TEXT PRIMARY KEY,
      creds TEXT NOT NULL,
      updated_at INTEGER NOT NULL
    );
    CREATE TABLE IF NOT EXISTS session_keys (
      scope TEXT NOT NULL,
      category TEXT NOT NULL,
      id TEXT NOT NULL,
      data TEXT NOT NULL,
      updated_at INTEGER NOT NULL,
      PRIMARY KEY (scope, category, id)
    );
    CREATE TABLE IF NOT EXISTS backup_snapshots (
      id INTEGER PRIMARY KEY AUTOINCREMENT,
      data TEXT NOT NULL,
      created_at INTEGER NOT NULL
    );
  `;
  await client.executeMultiple(sql);
}

async function closeTurso() {
  if (client) { try { client.close(); } catch {} client = null; }
}

export { createTursoClient, getTursoClient, initTursoTables, closeTurso };
```

- [ ] **Step 3: Run test**

```bash
node --test tests/turso-helper.test.mjs
```

- [ ] **Step 4: Commit**

```bash
git add src/lib/ourin-turso.js tests/turso-helper.test.mjs
git commit -m "feat: add turso client helper"
```

---

### Task 3: Migrate DB Layer to Turso

**Files:**
- Modify: `src/lib/ourin-database.js`

**Interfaces:**
- Consumes: `createTursoClient`, `initTursoTables`, `getTursoClient` from `ourin-turso.js`
- Produces: same `Database` class API — nothing changes for consumers

This is the core change. The Database class gets a Turso backend. On boot:
1. If Turso enabled, init tables.
2. Try to read stores from Turso. If empty, fall back to local JSON files (migrate seed).
3. Write-behind: `_asyncWrite` writes to Turso instead of JSON file.
4. On error → silent fallback to local file.

- [ ] **Step 1: Write test**

```js
// tests/turso-db.test.mjs
import { describe, it, before, after } from 'node:test';
import assert from 'node:assert';

describe('Database with Turso backend', () => {
  it('creates stores with defaults', () => {
    const db = new Database('/tmp/test-turso-db');
    // local-only test — no Turso client needed
    assert.ok(db.stores);
    assert.equal(typeof db.getUser, 'function');
  });
});
```

- [ ] **Step 2: Modify `ourin-database.js`**

Key changes to the `Database` class:

```js
import { createTursoClient, initTursoTables, getTursoClient, closeTurso } from './ourin-turso.js';

class Database {
  constructor(dbPath) {
    // ... existing fields ...
    this.tursoClient = null;
    this.tursoEnabled = false;
  }

  async init() {
    // LOWDB STORES DULU (buat adapter + baca file lokal, validasi JSON).
    // Ini tetap dijalankan walau Turso aktif — jadi kalau Turso kosong,
    // data lokal yang jadi seed, dan store.data siap di-overlay.
    this.migrateFromOldPath();
    await this.migrateFromSingleFile();

    const fileMap = {
      users: { file: "users.json", defaults: defaultUsers },
      groups: { file: "groups.json", defaults: defaultGroups },
      settings: { file: "settings.json", defaults: defaultSettings },
      stats: { file: "stats.json", defaults: defaultStats },
      sewa: { file: "sewa.json", defaults: defaultSewa },
      premium: { file: "premium.json", defaults: [] },
      owner: { file: "owner.json", defaults: [] },
      partner: { file: "partner.json", defaults: [] },
    };

    for (const [key, { file, defaults }] of Object.entries(fileMap)) {
      const filePath = path.join(this.dbPath, file);
      this.validateJsonFile(filePath, defaults, file);
      const adapter = new JSONFileSync(filePath);
      const store = new LowSync(adapter, defaults);
      store.read();
      if (!store.data) store.data = defaults;
      if (Array.isArray(defaults)) {
        if (!Array.isArray(store.data)) store.data = defaults;
      } else {
        store.data = { ...defaults, ...store.data };
      }
      store.write();
      this.stores[key] = store;
    }

    // TURSO: coba init client + tabel, lalu overlay data dari Turso.
    // Jika data Turso ada, store.data di-replace dari blob Turso.
    this.tursoClient = await createTursoClient(config.turso);
    this.tursoEnabled = !!this.tursoClient;
    if (this.tursoEnabled) {
      try {
        await initTursoTables(this.tursoClient);
        const loaded = await this.loadFromTurso();
        if (loaded) {
          logger.info("database", "data dimuat dari Turso");
        } else {
          // Turso kosong — seed dari file lokal, lalu tulis ke Turso
          await this.flushAllToTurso();
          logger.info("database", "seed data lokal ke Turso");
        }
      } catch (e) {
        console.warn('[turso] init failed, falling back to files:', e.message);
        this.tursoEnabled = false;
      }
    }

    this.db.data = {
      users: this.stores.users.data,
      groups: this.stores.groups.data,
      settings: this.stores.settings.data,
      stats: this.stores.stats.data,
      sewa: this.stores.sewa.data,
      premium: this.stores.premium.data,
      owner: this.stores.owner.data,
    };
    if (this.stores.partner) this.db.data.partner = this.stores.partner.data;

    this.db.write = () => this.flushAll();
    this.db.read = () => this.readAll();

    this.startFlushTimer();
    this.registerShutdownHooks();

    // ... sisa init existing (energi sync, ready=true, log) tetap sama
  }
```

Add these methods:
```js
refreshDbData() {
  this.db.data = {
    users: this.stores.users.data,
    groups: this.stores.groups.data,
    settings: this.stores.settings.data,
    stats: this.stores.stats.data,
    sewa: this.stores.sewa.data,
    premium: this.stores.premium.data,
    owner: this.stores.owner.data,
  };
  if (this.stores.partner) this.db.data.partner = this.stores.partner.data;
}

async loadFromTurso() {
  const rs = await this.tursoClient.execute('SELECT key, data FROM stores');
  if (!rs.rows || rs.rows.length === 0) return false;
  for (const row of rs.rows) {
    const key = row.key;
    const data = JSON.parse(row.data);
    if (!this.stores[key]) continue;
    this.stores[key].data = data;
  }
  this.refreshDbData();
  return true;
}

async writeToTurso(key) {
  // called from _asyncWrite instead of writing to file
  const data = JSON.stringify(this.stores[key].data);
  const now = Date.now();
  await this.tursoClient.execute({
    sql: 'INSERT INTO stores (key, data, updated_at) VALUES (?, ?, ?) ON CONFLICT(key) DO UPDATE SET data = excluded.data, updated_at = excluded.updated_at',
    args: [key, data, now],
  });
}

async flushAllToTurso() {
  for (const key of Object.keys(this.stores)) {
    try { await this.writeToTurso(key); } catch {}
  }
}
```

Modify `_asyncWrite`:
```js
async _asyncWrite(key) {
  if (!this.stores[key]) return;
  if (this.tursoEnabled) {
    try {
      await this.writeToTurso(key);
      this.dirty[key] = false;
      return;
    } catch (e) {
      // fallback to file write
    }
  }
  // existing file write logic
}
```

Modify `flushAll()`, `save()`:
```js
flushAll() {
  if (this.tursoEnabled) {
    this.flushAllToTurso().catch(() => {});
    return;
  }
  // existing file flush
}

async save() {
  if (this.tursoEnabled) {
    try { await this.flushAllToTurso(); return true; } catch { return false; }
  }
  // existing file save
}
```

- [ ] **Step 3: Run existing tests**

```bash
npm test
```

- [ ] **Step 4: Boot test (no WA connection — just verify plugin load passes)**

```bash
timeout 15 node index.js 2>&1 | head -30
```
Expected: 791 plugins loaded, no import errors.

- [ ] **Step 5: Commit**

```bash
git add src/lib/ourin-database.js
git commit -m "feat: add turso backend to database layer"
```

---

### Task 4: Migrate Session to Turso

**Files:**
- Create: `src/lib/ourin-turso-session.js`
- Modify: `src/connection.js`
- Modify: `src/lib/ourin-jadibot-manager.js`

**Interfaces:**
- Consumes: `getTursoClient()` from `ourin-turso.js`
- Produces: `useTursoAuthState(scope)` — same signature as `useMultiFileAuthState`

- [ ] **Step 1: Create `useTursoAuthState`**

`src/lib/ourin-turso-session.js`:
```js
import { getTursoClient } from './ourin-turso.js';

const credsCache = new Map();
const keysCache = new Map();

async function loadState(scope) {
  const client = getTursoClient();
  if (!client) return null;
  // load creds
  const credsRs = await client.execute({
    sql: 'SELECT creds FROM session_creds WHERE scope = ?',
    args: [scope],
  });
  let creds = null;
  if (credsRs.rows && credsRs.rows.length > 0) {
    creds = JSON.parse(credsRs.rows[0].creds);
  } else {
    const { initAuthCreds } = await import('ourin');
    creds = initAuthCreds();
  }
  // keys are loaded lazily on demand
  return {
    creds,
    keys: {
      get: async (type, ids) => {
        if (!Array.isArray(ids) || ids.length === 0) return {};
        const cacheKey = `${scope}:${type}`;
        if (!keysCache.has(cacheKey)) keysCache.set(cacheKey, new Map());
        const local = keysCache.get(cacheKey);
        const missing = ids.filter(id => !local.has(id));
        if (missing.length > 0) {
          const placeholders = missing.map(() => '?').join(',');
          const rs = await client.execute({
            sql: `SELECT id, data FROM session_keys WHERE scope = ? AND category = ? AND id IN (${placeholders})`,
            args: [scope, type, ...missing],
          });
          for (const row of rs.rows) {
            local.set(row.id, JSON.parse(row.data));
          }
        }
        const result = {};
        for (const id of ids) {
          if (local.has(id)) result[id] = local.get(id);
        }
        return result;
      },
      set: async (data) => {
        const client = getTursoClient();
        if (!client) return;
        for (const [type, entries] of Object.entries(data)) {
          for (const [id, value] of Object.entries(entries)) {
            const cacheKey = `${scope}:${type}`;
            if (!keysCache.has(cacheKey)) keysCache.set(cacheKey, new Map());
            keysCache.get(cacheKey).set(id, value);
            await client.execute({
              sql: 'INSERT INTO session_keys (scope, category, id, data, updated_at) VALUES (?, ?, ?, ?, ?) ON CONFLICT(scope, category, id) DO UPDATE SET data = excluded.data, updated_at = excluded.updated_at',
              args: [scope, type, id, JSON.stringify(value), Date.now()],
            });
          }
        }
      },
      getMany: async (type) => {
        const rs = await client.execute({
          sql: 'SELECT id, data FROM session_keys WHERE scope = ? AND category = ?',
          args: [scope, type],
        });
        const result = {};
        for (const row of rs.rows) {
          result[row.id] = JSON.parse(row.data);
        }
        return result;
      },
    },
  };
}

async function saveCreds(scope, creds) {
  const client = getTursoClient();
  if (!client) return;
  credsCache.set(scope, creds);
  await client.execute({
    sql: 'INSERT INTO session_creds (scope, creds, updated_at) VALUES (?, ?, ?) ON CONFLICT(scope) DO UPDATE SET creds = excluded.creds, updated_at = excluded.updated_at',
    args: [scope, JSON.stringify(creds), Date.now()],
  });
}

async function useTursoAuthState(scope = 'main') {
  const state = await loadState(scope);
  if (!state) {
    // Turso client unavailable — caller must fall back
    return { state: null, saveCreds: () => {} };
  }
  return {
    state,
    saveCreds: () => saveCreds(scope, state.creds),
  };
}

export { useTursoAuthState, loadState, saveCreds };
```

- [ ] **Step 2: Modify connection.js**

Change line 4 (`import { useMultiFileAuthState } from 'ourin'`) → keep both imports.

Change `startConnection`:
```js
const TURSO_ENABLED = config.turso?.enabled && config.turso?.url;

let state, saveCreds;
if (TURSO_ENABLED) {
  const { useTursoAuthState } = await import('./lib/ourin-turso-session.js');
  const result = await useTursoAuthState('main');
  if (!result.state) {
    // turso client gagal — fallback ke file
    const sessionPath = path.join(process.cwd(), 'storage', config.session?.folderName || 'session');
    if (!fs.existsSync(sessionPath)) fs.mkdirSync(sessionPath, { recursive: true });
    const res = await useMultiFileAuthState(sessionPath);
    state = res.state;
    saveCreds = res.saveCreds;
  } else {
    state = result.state;
    saveCreds = result.saveCreds;
  }
} else {
  const sessionPath = path.join(process.cwd(), 'storage', config.session?.folderName || 'session');
  if (!fs.existsSync(sessionPath)) fs.mkdirSync(sessionPath, { recursive: true });
  const result = await useMultiFileAuthState(sessionPath);
  state = result.state;
  saveCreds = result.saveCreds;
}
```

- [ ] **Step 3: Modify jadibot-manager.js**

Add import at top of `src/lib/ourin-jadibot-manager.js` (after line 6):
```js
import config from "../../config.js";
```

Then add to `startJadibot` (replace the `const { state, saveCreds } = await useMultiFileAuthState(authPath);` block at line 324):
```js
const TURSO_ENABLED = config.turso?.enabled && config.turso?.url;
let state, saveCreds;
if (TURSO_ENABLED) {
  const { useTursoAuthState } = await import('./ourin-turso-session.js');
  const result = await useTursoAuthState('jadibot:' + userJid.replace(/@.+/g, ''));
  state = result.state;
  saveCreds = result.saveCreds;
} else {
  const authPath = getJadibotAuthPath(userJid);
  if (!fs.existsSync(authPath)) fs.mkdirSync(authPath, { recursive: true });
  const result = await useMultiFileAuthState(authPath);
  state = result.state;
  saveCreds = result.saveCreds;
}
```

- [ ] **Step 4: Boot test (syntax check)**

```bash
node --check src/connection.js && node --check src/lib/ourin-jadibot-manager.js && timeout 15 node index.js 2>&1 | head -30
```

- [ ] **Step 5: Commit**

```bash
git add src/lib/ourin-turso-session.js src/connection.js src/lib/ourin-jadibot-manager.js
git commit -m "feat: migrate session to turso auth state"
```

---

### Task 5: Memory Audit

**Files:**
- Modify: `src/connection.js` (store caps)
- Modify: `src/lib/ourin-memory-monitor.js` (GC threshold)

- [ ] **Step 1: Reduce store message cap**

In `connection.js` line 82:
```js
chat.size > 200 → chat.size > 50
```

- [ ] **Step 2: Add LRU cap to chats Map**

After `chats.set(chat.id, chat)` (line 111), add:
```js
if (this.chats.size > 500) {
  const first = this.chats.keys().next().value;
  if (first) this.chats.delete(first);
}
```

- [ ] **Step 3: Reduce groupCache TTL**

Line 27: `stdTTL: 5 * 60` → `stdTTL: 3 * 60`

- [ ] **Step 4: Conditional GC in memory monitor**

In `src/lib/ourin-memory-monitor.js`:
```js
if (global.gc && mem.heapUsed > 500 * 1024 * 1024) global.gc();
```

- [ ] **Step 5: Verify**

```bash
node --check src/connection.js && node --check src/lib/ourin-memory-monitor.js
```

- [ ] **Step 6: Commit**

```bash
git add src/connection.js src/lib/ourin-memory-monitor.js
git commit -m "perf: reduce memory caps, conditional GC"
```

---

### Task 6: Deep Cleanup

**Files:**
- Modify: `src/lib/ourin-temp-cleaner.js`
- Modify: `src/lib/ourin-data-pruner.js`

- [ ] **Step 1: Upgrade temp cleaner**

Rewrite `src/lib/ourin-temp-cleaner.js`:
```js
import fs from 'fs'
import path from 'path'
import { logger } from './ourin-logger.js'

const CLEAN_INTERVAL = 30 * 60 * 1000
const MAX_AGE_MS = 60 * 60 * 1000 // 1 hour
const MIN_AGE_MS = 5 * 60 * 1000   // don't touch files < 5min old

let cleanerTimer = null

function scanDir(dirPath, ageThreshold) {
  let total = 0
  let size = 0
  if (!fs.existsSync(dirPath)) return { total, size }
  try {
    const entries = fs.readdirSync(dirPath, { withFileTypes: true })
    for (const entry of entries) {
      const fullPath = path.join(dirPath, entry.name)
      try {
        if (entry.isDirectory()) {
          const sub = scanDir(fullPath, ageThreshold)
          total += sub.total
          size += sub.size
          if (fs.readdirSync(fullPath).length === 0) fs.rmdirSync(fullPath)
        } else if (entry.isFile()) {
          const stat = fs.statSync(fullPath)
          const age = Date.now() - stat.mtimeMs
          if (age > ageThreshold) {
            size += stat.size
            fs.unlinkSync(fullPath)
            total++
          }
        }
      } catch {}
    }
  } catch {}
  return { total, size }
}

function startTempCleaner() {
  if (cleanerTimer) return
  cleanerTimer = setInterval(() => {
    const now = Date.now()
    const ageThreshold = now - MAX_AGE_MS
    let grandTotal = 0, grandSize = 0
    for (const dir of ['temp', 'tmp']) {
      const dirPath = path.join(process.cwd(), dir)
      if (!fs.existsSync(dirPath)) continue
      const result = scanDir(dirPath, ageThreshold)
      grandTotal += result.total
      grandSize += result.size
    }
    if (grandTotal > 0) {
      const sizeMB = (grandSize / 1024 / 1024).toFixed(2)
      logger.system('temp', `cleaned ${grandTotal} file(s) (${sizeMB}MB)`)
    }
  }, CLEAN_INTERVAL)
  if (cleanerTimer.unref) cleanerTimer.unref()
  logger.success('temp', `auto-clean age >${MAX_AGE_MS/60000}m recursive, every ${CLEAN_INTERVAL/60000}m`)
}

function stopTempCleaner() {
  if (cleanerTimer) { clearInterval(cleanerTimer); cleanerTimer = null }
}

export { startTempCleaner, stopTempCleaner }
```

- [ ] **Step 2: Upgrade data pruner**

Extend `src/lib/ourin-data-pruner.js`:
Add after the user/group pruning (and declare the counters at the top with `prunedUsers`/`prunedGroups`):
```js
let prunedPremium = 0
let prunedPartner = 0

// expired premium/partner cleanup
if (Array.isArray(db.data.premium)) {
  const preLen = db.data.premium.length;
  db.data.premium = db.data.premium.filter(p => {
    if (typeof p === 'string') return true;
    const expire = p.expired || (p.expiredAt ? new Date(p.expiredAt).getTime() : 0);
    return !expire || expire > now;
  });
  prunedPremium += preLen - db.data.premium.length;
}
if (Array.isArray(db.data.partner)) {
  const partLen = db.data.partner.length;
  db.data.partner = db.data.partner.filter(p => {
    if (typeof p === 'string') return true;
    const expire = p.expired || (p.expiredAt ? new Date(p.expiredAt).getTime() : 0);
    return !expire || expire > now;
  });
  prunedPartner += partLen - db.data.partner.length;
}
```
Update the final `if` to include the new counters and save:
```js
if (prunedUsers > 0 || prunedGroups > 0 || prunedPremium > 0 || prunedPartner > 0) {
  db.save()
  logger.system('pruner', `removed ${prunedUsers} users, ${prunedGroups} groups, ${prunedPremium} premium, ${prunedPartner} partner (>${INACTIVE_THRESHOLD / 86400000}d inactive)`)
}
```

- [ ] **Step 3: Verify**

```bash
node --check src/lib/ourin-temp-cleaner.js && node --check src/lib/ourin-data-pruner.js
```

- [ ] **Step 4: Commit**

```bash
git add src/lib/ourin-temp-cleaner.js src/lib/ourin-data-pruner.js
git commit -m "feat: upgrade temp cleaner recursive + age filter, pruner expired premium"
```

---

### Task 7: Final Integration & Test

- [ ] **Step 1: Full test run**

```bash
npm test && timeout 20 node index.js 2>&1 | head -30
```

Expected: tests pass, 791 plugins load, no import errors.

- [ ] **Step 2: Verify all syntax**

```bash
for f in plugins/canvas/fakebankjago.js plugins/main/allmenu.js src/lib/ourin-database.js src/lib/ourin-turso.js src/lib/ourin-turso-session.js; do node --check "$f" || echo "FAIL: $f"; done
```

- [ ] **Step 3: Final commit**

```bash
git add -A
git commit -m "feat: turso db+session migration, memory audit, deep cleanup"
```