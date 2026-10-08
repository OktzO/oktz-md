# Pre-Key Session Pruning Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Hentikan pertumbuhan `session_keys` tanpa batas di Turso dengan memperbaiki jalur delete yang rusak, lalu sweep pre-key tiap 3 hari.

**Architecture:** Tiga perubahan terpisah, berurutan, masing-masing dapat diuji sendiri. (1) Perbaiki `loadState().keys.set` agar nilai `null` jadi DELETE, bukan INSERT `'null'`. (2) Sweep `pre-key` berumur >30 hari tiap 3 hari, hanya kategori `pre-key`. (3) Statistik on-demand untuk verifikasi. Pasifkan `removePreKey` sengaja DITUNDA ke pekerjaan terpisah — lihat §Out of Scope.

**Tech Stack:** Node.js 22, ESM, `@libsql/client` (Turso), `node:test`.

**Spec:** `docs/superpowers/specs/2026-10-08-prekey-session-pruning-design.md`

## Global Constraints

- Sweep hanya boleh menyentuh `category = 'pre-key'`. Kategori lain **tidak boleh** ditulis.
- `session`, `sender-key`, `identity-key`, `device-list`, `lid-mapping`, `tctoken`, `session_creds` tidak boleh berubah row-count maupun byte.
- Jadwal sweep: **3 hari** (259.200.000 ms).
- Ambang umur pre-key: **30 hari** (2.592.000.000 ms).
- Turso mati saat sweep → catat `warn`, jangan sampai bot berhenti.
- Nol dependency baru. `node:test` + `node:assert` saja.
- Tidak patch `node_modules/onigis`.
- Bahasa komentar dan log: Bahasa Indonesia, mengikuti gaya file yang diubah.
- Sweep boleh dijalankan ulang kapan saja; `prekey-sweep.test.mjs` harus tetap hijau setelah dua kali jalan berurutan.

## Review Focus

Lima kondisi yang paling mungkin menggigit, tidak satu pun diuji oleh spec:

1. **Sweep berjalan dua kali beruntun** (restart di tengah siklus) → hasil harus idempoten, tidak error, tidak menghapus lebih dari yang seharusnya.
2. **`updated_at` persis di batas 30 hari** → setengah detik pun tidak boleh jadi alasan menghapus. Batas harus eksklusif.
3. **Pre-key aktif yang ID-nya muncul lagi di batch baru** → UPSERT harus tetap benar setelah Task 1 mengubah `set` (satu jalur untuk INSERT dan DELETE).
4. **Turso `batch()` gagal di tengah** → fallback sequential harus tetap proud_si DELETE, bukan diam-diam jadi INSERT.
5. **`scope` selain `main`** (jadibot punya session sendiri) → sweep tidak boleh menyapu session jadibot yang aktif.

Masing-masing dipin test di task pemiliknya (lihat langkah terkait).

---

## File Structure

| File | Tanggung jawab |
|---|---|
| `src/lib/turso-session.js` | Modify — delete path di `loadState().keys.set`, plus fungsi sweep & statistik baru |
| `index.js` | Modify — mulai sweep setelah bot siap |
| `tests/prekey-null-delete.test.mjs` | Create — Task 1 |
| `tests/prekey-sweep.test.mjs` | Create — Task 2 |
| `tests/prekey-degraded.test.mjs` | Create — Task 2 |
| `tests/session-stats.test.mjs` | Create — Task 3 |
| `plugins/owner/sessionstats.js` | Create — Task 3 (perintah owner) |

Task 1 dan 2 sama-sama mengubah `turso-session.js`. Itu disengaja: logika sweep adalahConcerns key store yang sama, dan memindahkannya ke file baru hanya menambah indirection untuk 30 baris.

---

### Task 1: Perbaiki jalur delete di `loadState().keys.set`

**Files:**
- Modify: `src/lib/turso-session.js:468-497`
- Test: `tests/prekey-null-delete.test.mjs`

**Interfaces:**
- Consumes: `withRemoteRetry(fn)`, `keysCache`, `trimLocalCache(local, cap)`, `KEYS_CACHE_CAP`, `BufferJSON` — semua sudah ada di file yang sama.
- Produces: tidak ada API baru. `loadState(scope).keys.set` gained a DELETE branch; export blok tidak berubah.

- [ ] **Step 1: Write the failing test**

Buat `tests/prekey-null-delete.test.mjs` dengan helper `setupTurso()` dan `loadSeededState(scope)` yang disalin apa adanya dari `tests/turso-session-atomic.test.mjs:4-23` — keduanya sudah terbukti benar.

```js
import { describe, it } from 'node:test';
import assert from 'node:assert';

async function setupTurso() {
  const tursoModule = await import('../src/lib/turso.js');
  const client = await tursoModule.createTursoClient({ enabled: true, url: 'file::memory:' });
  await tursoModule.initTursoTables(client);
  return { client, tursoModule };
}

async function loadSeededState(scope) {
  const tursoModule = await import('../src/lib/turso.js');
  const client = tursoModule.getTursoClient();
  await client.execute({
    sql: 'INSERT INTO session_creds (scope, creds, updated_at) VALUES (?, ?, ?) ON CONFLICT(scope) DO UPDATE SET creds = excluded.creds',
    args: [scope, JSON.stringify({ registered: true }), 1],
  });
  const { loadState } = await import('../src/lib/turso-session.js');
  const state = await loadState(scope);
  assert.ok(state, `loadState(${scope}) should return a state once a row exists`);
  return state;
}

describe('keys.set delete path', () => {
  it('deletes the row when a pre-key value is null', async () => {
    const { client } = await setupTurso();
    const state = await loadSeededState('main');

    await state.keys.set({ 'pre-key': { '9001': { keyPair: 'a' } } });
    let rows = await client.execute("SELECT id FROM session_keys WHERE scope='main' AND category='pre-key'");
    assert.strictEqual(rows.rows.length, 1, 'pre-key should exist before delete');

    await state.keys.set({ 'pre-key': { '9001': null } });
    rows = await client.execute("SELECT id FROM session_keys WHERE scope='main' AND category='pre-key'");
    assert.strictEqual(rows.rows.length, 0, 'pre-key row must be gone, not rewritten');
  });

  it('does not leave a row whose data is the literal string null', async () => {
    const { client } = await setupTurso();
    const state = await loadSeededState('main');

    await state.keys.set({ 'pre-key': { '9002': { keyPair: 'a' } } });
    await state.keys.set({ 'pre-key': { '9002': null } });

    const rows = await client.execute("SELECT id, data FROM session_keys WHERE scope='main' AND category='pre-key'");
    assert.strictEqual(rows.rows.length, 0, 'no row may survive a delete');
  });

  it('still upserts when the value is not null', async () => {
    const { client } = await setupTurso();
    const state = await loadSeededState('main');

    await state.keys.set({ 'pre-key': { '9003': { keyPair: 'a' } } });
    await state.keys.set({ 'pre-key': { '9003': { keyPair: 'b' } } });

    const rows = await client.execute("SELECT data FROM session_keys WHERE scope='main' AND category='pre-key' AND id='9003'");
    assert.strictEqual(rows.rows.length, 1, 'upsert must stay a single row');
    assert.deepStrictEqual(JSON.parse(rows.rows[0].data), { keyPair: 'b' });
  });

  it('mixes inserts and deletes in one patch without dropping either', async () => {
    const { client } = await setupTurso();
    const state = await loadSeededState('main');

    await state.keys.set({ 'pre-key': { 'a': { k: 1 }, 'b': { k: 2 } } });
    await state.keys.set({
      'pre-key': { 'a': null, 'c': { k: 3 } },
      'session': { 's1': { advSecretKey: 'x' } },
    });

    const pre = await client.execute("SELECT id FROM session_keys WHERE category='pre-key' AND scope='main' ORDER BY id");
    assert.deepStrictEqual(pre.rows.map(r => r.id), ['b', 'c'], 'a deleted, c inserted, b untouched');

    const sess = await client.execute("SELECT id FROM session_keys WHERE category='session' AND scope='main'");
    assert.strictEqual(sess.rows.length, 1, 'session insert must not be affected by the pre-key delete');
  });

  it('evicts the deleted key from the in-memory cache', async () => {
    const { client } = await setupTurso();
    const state = await loadSeededState('main');

    await state.keys.set({ 'pre-key': { '9004': { keyPair: 'a' } } });
    await state.keys.get('pre-key', ['9004']);
    await state.keys.set({ 'pre-key': { '9004': null } });

    const got = await state.keys.get('pre-key', ['9004']);
    assert.deepStrictEqual(got, {}, 'a deleted key must not be served from cache');
    assert.strictEqual(typeof got, 'object');
  });
});
```

Test terakhir mengunci Review Focus #3 dan #5: key yang baru saja dihapus tidak boleh masih dilayani dari `keysCache`, dan `get` tidak boleh melempar.

- [ ] **Step 2: Run test to verify it fails**

Run: `node --test tests/prekey-null-delete.test.mjs`
Expected: test 1 dan 2 GAGAL dengan row count 1 (bukan 0). Test 3, 4, 5 kemungkinan LULUS — itu benar, jangan"Not a bug" mereka.

- [ ] **Step 3: Implement the DELETE branch in `src/lib/turso-session.js`**

Di dalam `loadState(scope).keys.set` (baris 468-497), di loop `for (const [id, value] of Object.entries(entries))`:

1. `const cacheKey = ...; if (!keysCache.has(cacheKey)) keysCache.set(cacheKey, new Map()); const local = keysCache.get(cacheKey);` — biarkan seperti sekarang.
2. Ganti `local.delete(id); local.set(id, value);` dengan: kalau `value === null`, `local.delete(id)` saja; selain itu `local.delete(id); local.set(id, value);` lalu `trimLocalCache(local, KEYS_CACHE_CAP)`. `trimLocalCache` tidak perlu dipanggil pada jalur delete karena ukurannya hanya mengecil.
3. Ganti `statements.push({...})` dengan percabangan:

```js
if (value === null) {
  statements.push({
    sql: 'DELETE FROM session_keys WHERE scope = ? AND category = ? AND id = ?',
    args: [scope, type, id],
  });
} else {
  statements.push({
    sql: 'INSERT INTO session_keys (scope, category, id, data, updated_at) VALUES (?, ?, ?, ?, ?) ON CONFLICT(scope, category, id) DO UPDATE SET data = excluded.data, updated_at = excluded.updated_at',
    args: [scope, type, id, JSON.stringify(value, BufferJSON.replacer), Date.now()],
  });
}
```

Blok `if (statements.length > 0)` dan fallback sequential **tidak diubah** — keduanya sudah benar untuk DELETE (`client.batch` menerima DELETE, dan `client.execute` juga).

- [ ] **Step 4: Run test to verify it passes**

Run: `node --test tests/prekey-null-delete.test.mjs`
Expected: 5 PASS.

Lalu regression check pada test yang sudah ada:

Run: `node --test tests/turso-session-atomic.test.mjs tests/turso-session.test.mjs tests/session-store-fallback.test.mjs tests/session-restore-durability.test.mjs`
Expected: semua PASS. `turso-session-atomic` menguji `batch()` dengan 3 INSERT — memastikan Task 1 tidak merusak jalur UPSERT.

- [ ] **Step 5: Commit**

```bash
git add src/lib/turso-session.js tests/prekey-null-delete.test.mjs
git commit -m "fix(session): nilai null harus jadi DELETE, bukan INSERT 'null'

loadState().keys.set tidak mengecek value === null, padahal jalur
cermin di loadRemoteKeys.set sudah benar. Akibatnya removePreKey
menambah row berisi data 'null' alih-alih menghapus, sehingga session
membengkak justru saat pre-key dipakai.

Satu jalur untuk INSERT dan DELETE; keysCache ikut melepas key yang
dihapus supaya tidak dilayani dari RAM."
```

---

### Task 2: Sweep pre-key tiap 3 hari

**Files:**
- Modify: `src/lib/turso-session.js` (tambah dua fungsi + export)
- Modify: `index.js:590-599` (mulai sweep setelah bot siap)
- Test: `tests/prekey-sweep.test.mjs`, `tests/prekey-degraded.test.mjs`

**Interfaces:**
- Consumes: `getTursoClient()`, `withRemoteRetry(fn)` dari `src/lib/turso.js` dan file yang sama.
- Produces:
  - `sweepPreKeys({ now = Date.now(), maxAgeMs = 30 * 86400000 } = {})` → `Promise<{ ok: boolean, deleted: number, error?: string }>`. Hapus `category = 'pre-key' AND scope = ? AND updated_at < cutoff`. `ok: false` kalau Turso tidak tersedia atau query gagal, dan `error` berisi pesannya.
  - `sessionKeyStats()` → `Promise<{ total: number, bytes: number, byCategory: Array<{ category: string, keys: number, bytes: number }> }>`. `total`/`bytes` 0 dan `byCategory` `[]` kalau Turso tidak tersedia.
  - `startPreKeySweeper(intervalMs = 259200000)` / `stopPreKeySweeper()` — timer `.unref()`.

- [ ] **Step 1: Write the failing test**

Buat `tests/prekey-sweep.test.mjs`. Helper `setupTurso()` dan `loadSeededState()` sama seperti Task 1.

```js
import { describe, it } from 'node:test';
import assert from 'node:assert';

const DAY = 86400000;

async function setupTurso() {
  const tursoModule = await import('../src/lib/turso.js');
  const client = await tursoModule.createTursoClient({ enabled: true, url: 'file::memory:' });
  await tursoModule.initTursoTables(client);
  return { client, tursoModule };
}

async function seed(client, rows) {
  for (const [category, id, ageDays] of rows) {
    await client.execute({
      sql: 'INSERT INTO session_keys (scope, category, id, data, updated_at) VALUES (?, ?, ?, ?, ?)',
      args: ['main', category, id, JSON.stringify({ x: 1 }), Date.now() - ageDays * DAY],
    });
  }
}

async function countOf(client, category) {
  const r = await client.execute({
    sql: 'SELECT COUNT(*) n FROM session_keys WHERE scope=? AND category=?',
    args: ['main', category],
  });
  return r.rows[0].n;
}

describe('sweepPreKeys', () => {
  it('removes only pre-keys older than the threshold', async () => {
    const { client } = await setupTurso();
    await seed(client, [
      ['pre-key', 'old', 31],
      ['pre-key', 'fresh', 1],
      ['pre-key', 'edge', 30],
    ]);
    const { sweepPreKeys } = await import('../src/lib/turso-session.js');

    const res = await sweepPreKeys();
    assert.strictEqual(res.ok, true);
    assert.strictEqual(res.deleted, 1, 'exactly one pre-key older than 30d');
    assert.strictEqual(await countOf(client, 'pre-key'), 2, 'fresh and edge survive');
  });

  it('never touches ratchet categories', async () => {
    const { client } = await setupTurso();
    await seed(client, [
      ['pre-key', 'old', 400],
      ['session', 's-old', 400],
      ['sender-key', 'sk-old', 400],
      ['identity-key', 'ik-old', 400],
      ['device-list', 'dl-old', 400],
      ['lid-mapping', 'lm-old', 400],
      ['tctoken', 'tc-old', 400],
    ]);
    const { sweepPreKeys } = await import('../src/lib/turso-session.js');
    await sweepPreKeys();

    for (const cat of ['session', 'sender-key', 'identity-key', 'device-list', 'lid-mapping', 'tctoken']) {
      assert.strictEqual(await countOf(client, cat), 1, `${cat} must be untouched`);
    }
    assert.strictEqual(await countOf(client, 'pre-key'), 0);
  });

  it('does not sweep another scope', async () => {
    const { client } = await setupTurso();
    await client.execute({
      sql: 'INSERT INTO session_keys (scope, category, id, data, updated_at) VALUES (?, ?, ?, ?, ?)',
      args: ['jadibot:6285', 'pre-key', 'j-old', '{}', Date.now() - 400 * DAY],
    });
    const { sweepPreKeys } = await import('../src/lib/turso-session.js');
    await sweepPreKeys();

    const r = await client.execute("SELECT COUNT(*) n FROM session_keys WHERE scope='jadibot:6285'");
    assert.strictEqual(r.rows[0].n, 1, 'jadibot pre-keys are live sessions, not garbage');
  });

  it('is idempotent when run twice', async () => {
    const { client } = await setupTurso();
    await seed(client, [['pre-key', 'old', 31]]);
    const { sweepPreKeys } = await import('../src/lib/turso-session.js');

    const first = await sweepPreKeys();
    const second = await sweepPreKeys();
    assert.strictEqual(first.deleted, 1);
    assert.strictEqual(second.deleted, 0, 'second pass has nothing left to remove');
    assert.strictEqual(second.ok, true);
  });

  it('honours an injected now and maxAgeMs', async () => {
    const { client } = await setupTurso();
    await client.execute({
      sql: 'INSERT INTO session_keys (scope, category, id, data, updated_at) VALUES (?, ?, ?, ?, ?)',
      args: ['main', 'pre-key', 'p', '{}', 1_000_000],
    });
    const { sweepPreKeys } = await import('../src/lib/turso-session.js');

    const early = await sweepPreKeys({ now: 1_000_000 + 10 * DAY, maxAgeMs: 30 * DAY });
    assert.strictEqual(early.deleted, 0, '10d old is inside the window');

    const late = await sweepPreKeys({ now: 1_000_000 + 31 * DAY, maxAgeMs: 30 * DAY });
    assert.strictEqual(late.deleted, 1);
  });

  it('reports zero deletions when there is nothing stale', async () => {
    const { client } = await setupTurso();
    await seed(client, [['pre-key', 'fresh', 0]]);
    const { sweepPreKeys } = await import('../src/lib/turso-session.js');
    const res = await sweepPreKeys();
    assert.strictEqual(res.ok, true);
    assert.strictEqual(res.deleted, 0);
  });
});
```

Test `never touches ratchet categories` adalah penjaga success criteria 4 di spec. Test `does not sweep another scope` menjaga Review Focus #5. Test `honours an injected now` menjaga batas eksklusif (Review Focus #2).

- [ ] **Step 2: Write the degraded-mode test**

Buat `tests/prekey-degraded.test.mjs`:

```js
import { describe, it } from 'node:test';
import assert from 'node:assert';

describe('sweep survives a dead Turso', () => {
  it('returns ok:false instead of throwing when no client is configured', async () => {
    const tursoModule = await import('../src/lib/turso.js');
    await tursoModule.closeTurso();
    const { sweepPreKeys } = await import('../src/lib/turso-session.js');

    const res = await sweepPreKeys();
    assert.strictEqual(res.ok, false, 'a dead mirror must not look like success');
    assert.strictEqual(res.deleted, 0);
    assert.ok(typeof res.error === 'string' && res.error.length > 0, 'the reason must reach the log');
  });

  it('returns ok:false when the query itself fails', async () => {
    const tursoModule = await import('../src/lib/turso.js');
    const client = await tursoModule.createTursoClient({ enabled: true, url: 'file::memory:' });
    await tursoModule.initTursoTables(client);
    await client.execute('DROP TABLE session_keys');

    const { sweepPreKeys } = await import('../src/lib/turso-session.js');
    const res = await sweepPreKeys();
    assert.strictEqual(res.ok, false);
    assert.strictEqual(res.deleted, 0);
  });

  it('never throws, whatever the client does', async () => {
    const tursoModule = await import('../src/lib/turso.js');
    const client = await tursoModule.createTursoClient({ enabled: true, url: 'file::memory:' });
    client.execute = async () => { throw new Error('boom'); };

    const { sweepPreKeys } = await import('../src/lib/turso-session.js');
    const res = await sweepPreKeys();
    assert.strictEqual(res.ok, false);
    assert.match(res.error, /boom/);
  });
});
```

Pola `file::memory:` + `closeTurso()` sudah dipakai di test yang ada, jadi tidak ada jalur baru yang diuji-disini.

- [ ] **Step 3: Run both tests to verify they fail**

Run: `node --test tests/prekey-sweep.test.mjs tests/prekey-degraded.test.mjs`
Expected: GAGAL dengan "does not provide an export named 'sweepPreKeys'".

- [ ] **Step 4: Implement `sweepPreKeys` in `src/lib/turso-session.js`

Tambahkan di atas blok `export` (sekitar baris 738):

```js
const PREKEY_MAX_AGE_MS = 30 * 86400000;
const PREKEY_SWEEP_INTERVAL_MS = 259200000;
const SCOPE_MAIN = 'main';

let preKeySweeper = null;

async function sweepPreKeys(options = {}) {
  const now = options.now ?? Date.now();
  const maxAgeMs = options.maxAgeMs ?? PREKEY_MAX_AGE_MS;
  const client = getTursoClient();
  if (!client) {
    return { ok: false, deleted: 0, error: 'Turso tidak dikonfigurasi' };
  }
  try {
    const rs = await withRemoteRetry(() => client.execute({
      sql: 'DELETE FROM session_keys WHERE scope = ? AND category = ? AND updated_at < ?',
      args: [SCOPE_MAIN, 'pre-key', now - maxAgeMs],
    }));
    const deleted = Number(rs?.rowsAffected ?? rs?.rows?.length ?? 0);
    return { ok: true, deleted };
  } catch (e) {
    return { ok: false, deleted: 0, error: e.message };
  }
}
```

`catch` mengembalikan nilai, bukan melempar — bot tidak boleh berhenti karena mirror mati.

`rowsAffected` adalah nama field libsql. Kalau ternyata `undefined` pada versi yang dipakai, `?? 0` membuat test `deleted === 1` gagal dengan jelas; kalauitu terjadi, periksa bentuk respons dengan satu assertion lalu sesuaikan — jangan Remove assertion-nya.

- [ ] **Step 5: Implement `sessionKeyStats` in the same file**

```js
async function sessionKeyStats() {
  const client = getTursoClient();
  if (!client) return { total: 0, bytes: 0, byCategory: [] };
  try {
    const rs = await withRemoteRetry(() => client.execute(
      'SELECT category, COUNT(*) keys, SUM(LENGTH(data)) bytes FROM session_keys GROUP BY category ORDER BY bytes DESC',
    ));
    const byCategory = (rs.rows || []).map((r) => ({
      category: r.category,
      keys: Number(r.keys),
      bytes: Number(r.bytes ?? 0),
    }));
    return {
      total: byCategory.reduce((a, r) => a + r.keys, 0),
      bytes: byCategory.reduce((a, r) => a + r.bytes, 0),
      byCategory,
    };
  } catch (e) {
    return { total: 0, bytes: 0, byCategory: [], error: e.message };
  }
}
```

- [ ] **Step 6: Implement the timer**

```js
function startPreKeySweeper(intervalMs = PREKEY_SWEEP_INTERVAL_MS) {
  if (preKeySweeper) return;
  preKeySweeper = setInterval(async () => {
    const res = await sweepPreKeys();
    if (!res.ok) {
      console.warn(`[turso-session] sweep pre-key gagal: ${res.error}`);
      return;
    }
    if (res.deleted === 0) {
      console.log('[turso-session] sweep pre-key: tidak ada yang basi');
      return;
    }
    const stats = await sessionKeyStats();
    console.log(
      `[turso-session] sweep pre-key: ${res.deleted} key dihapus, ` +
        `sisa ${stats.total} key / ${(stats.bytes / 1024).toFixed(1)} KB`,
    );
  }, intervalMs);
  if (preKeySweeper.unref) preKeySweeper.unref();
}

function stopPreKeySweeper() {
  if (preKeySweeper) {
    clearInterval(preKeySweeper);
    preKeySweeper = null;
  }
}
```

Tambahkan `sweepPreKeys`, `sessionKeyStats`, `startPreKeySweeper`, `stopPreKeySweeper` ke blok `export`.

- [ ] **Step 7: Run the tests to verify they pass**

Run: `node --test tests/prekey-sweep.test.mjs tests/prekey-degraded.test.mjs`
Expected: 9 PASS (6 + 3).

- [ ] **Step 8: Wire the sweeper into `index.js`**

Di `index.js`, cari blok yang memanggil `startMemoryMonitor(); startTempCleaner(); startDailyPruner();` (sekitar baris 595-597) dan tambahkan `startPreKeySweeper();` setelah `startDailyPruner();`.

Tambahkan `startPreKeySweeper` ke import dari `./lib/turso-session.js` di bagian atas file. Cek dulu nama import yang sudah ada — `index.js` mungkin sudah mengimpor hal lain dari modul itu; jangan buat import kedua.

- [ ] **Step 9: Verify no import duplication and run full regression**

Run: `grep -n "turso-session" index.js`
Expected: tepat satu baris import.

Run: `node --check index.js && node --check src/lib/turso-session.js`
Expected: tanpa output.

Run: `npm test`
Expected: semua PASS. Kalau suite penuh melewati 240 detik, jalankan test yang relevan saja:
`node --test tests/prekey-*.test.mjs tests/session-*.test.mjs tests/turso-*.test.mjs tests/memory-sweeps.test.mjs`

- [ ] **Step 10: Commit**

```bash
git add src/lib/turso-session.js index.js tests/prekey-sweep.test.mjs tests/prekey-degraded.test.mjs
git commit -m "feat(session): sweep pre-key basi tiap 3 hari

Pre-key adalah key sekali-pakai: messages-recv.js mengambil tepat satu
per pesan, dan library sudah mengupload ulang otomatis saat stok kurang
(socket.js:382). Yang basi karena tidak pernah dipakai tidak mungkin
dipakai lagi.

DELETE satu statement, hanya category='pre-key' dan scope='main'.
session, sender-key, identity-key, device-list, lid-mapping, tctoken
tidak tersentuh — semuanya ratchet state atau trust anchor."
```

---

### Task 3: Statistik session on-demand

**Files:**
- Create: `tests/session-stats.test.mjs`
- Create: `plugins/owner/sessionstats.js`

**Interfaces:**
- Consumes: `sessionKeyStats()` dari Task 2.
- Produces: perintah owner `/sessionstats`.

**K ruptured design — baca pola plugin dulu.** Sebelum menulis, periksa satu plugin owner yang sudah ada di `plugins/owner/` dan tiru persis: cara metadata command didaftarkan, cara socket dikirim, cara owner dicek. Contoh yang dipakai: `plugins/owner/clearsessions.js` — meski command itu sendiri yang salah path, strukturnya benar dan sudah dipakai orang.

- [ ] **Step 1: Write the failing test**

`tests/session-stats.test.mjs` — test untuk `sessionKeyStats()`, bukan untuk plugin-nya (plugin.owner di repo ini tidak diuji dengan `node:test`; menguji markup-nya hanya menguji ulang `onigis`).

```js
import { describe, it } from 'node:test';
import assert from 'node:assert';

describe('sessionKeyStats', () => {
  it('groups keys and bytes by category', async () => {
    const tursoModule = await import('../src/lib/turso.js');
    const client = await tursoModule.createTursoClient({ enabled: true, url: 'file::memory:' });
    await tursoModule.initTursoTables(client);

    await client.execute({
      sql: 'INSERT INTO session_keys (scope, category, id, data, updated_at) VALUES (?, ?, ?, ?, ?)',
      args: ['main', 'pre-key', 'a', '{"pad":"xxxxxxxx"}', 1],
    });
    await client.execute({
      sql: 'INSERT INTO session_keys (scope, category, id, data, updated_at) VALUES (?, ?, ?, ?, ?)',
      args: ['main', 'pre-key', 'b', '{"pad":"yyyy"}', 1],
    });
    await client.execute({
      sql: 'INSERT INTO session_keys (scope, category, id, data, updated_at) VALUES (?, ?, ?, ?, ?)',
      args: ['main', 'session', 's', '{"pad":"zz"}', 1],
    });

    const { sessionKeyStats } = await import('../src/lib/turso-session.js');
    const stats = await sessionKeyStats();
    assert.strictEqual(stats.total, 3);
    const pre = stats.byCategory.find((c) => c.category === 'pre-key');
    assert.strictEqual(pre.keys, 2);
    assert.ok(pre.bytes > 0, 'bytes must be measured, not zero');
    assert.ok(stats.bytes >= pre.bytes);
  });

  it('degrades to zeros when Turso is absent', async () => {
    const tursoModule = await import('../src/lib/turso.js');
    await tursoModule.closeTurso();
    const { sessionKeyStats } = await import('../src/lib/turso-session.js');
    const stats = await sessionKeyStats();
    assert.deepStrictEqual(stats, { total: 0, bytes: 0, byCategory: [] });
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `node --test tests/session-stats.test.mjs`
Expected: GAGAL dengan "does not provide an export named 'sessionKeyStats'" — kalau sudah LULUS, Task 2 lupa mengexpor-nya.

- [ ] **Step 3: Run test to verify it passes**

Task 2 sudah mengimplementasikan `sessionKeyStats`. Kalau test hijau sekarang, tidak ada kode yang perlu ditulis — lanjut ke Step 4. Kalau belum, periksa blok `export` di `src/lib/turso-session.js`.

- [ ] **Step 4: Write the plugin**

Buat `plugins/owner/sessionstats.js`. Baca `plugins/owner/clearsessions.js` dulu dan tiru persis nama field metadata, cek owner, dan pemanggilan socket yang dipakainya. Isi functor-nya:

- Panggil `sessionKeyStats()`.
- Kalau `byCategory` kosong, balas satu baris bahwa Turso tidak aktif atau belum ada key.
- Kalau ada isi, balas tabel: nama kategori, jumlah key, ukuran KB, dengan `session_keys` sebagai total di baris akhir.
- Sebutkan pre-key secara terpisah karena itu angka yang dipantau untuk success criteria.

Jangan tambah argumen, flag, atau mode interaktif — tidak ada yang meminta.

- [ ] **Step 5: Verify the plugin loads**

Run: `node -e "import('./plugins/owner/sessionstats.js').then(()=>console.log('ok')).catch(e=>{console.error(e.message);process.exit(1)})"`
Expected: `ok`.

Kalau gagal dengan error import, plugin owner di repo ini kemungkinan diekspor sebagai fungsi `handler` dan butuh `import` lain yang belum ada di file kosong — ikuti apa yang dilakukan `clearsessions.js`.

- [ ] **Step 6: Commit**

```bash
git add plugins/owner/sessionstats.js tests/session-stats.test.mjs
git commit -m "feat(owner): /sessionstats untuk pantau ukuran session per kategori

Success criteria spec mengukur session_keys dari 3,0 MB ke < 1,3 MB.
Angka itu harus bisa dilihat tanpa menunggu siklus sweep 3 hari."
```

---

## Verifikasi Akhir

Jalankan setelah ketiga task selesai.

- [ ] **Step 1: Full test suite**

Run: `npm test`
Expected: semua PASS. Catat test yang gagal kalau ada — jangan lanjut dengan suite merah.

- [ ] **Step 2: Lint**

Run: `npm run lint`
Expected: tanpa error. `npm run lint` hanya menyisir `plugins/`, jadi kerjakan `src/lib/turso-session.js` dengan `node --check`.

- [ ] **Step 3: Ukur-baseline di produksi**

Baca `docs/superpowers/specs/2026-10-08-prekey-session-pruning-design.md` §Success criteria, lalu jalankan satu sweep pertama secara manual terhadap Turso produksi dan catat hasilnya:

```bash
node -e "
const run = async () => {
  const { config } = await import('./config.js');
  const { createClient } = await import('@libsql/client');
  const c = createClient({ url: config.turso.url, authToken: config.turso.authToken });
  const before = await c.execute(\"SELECT COUNT(*) n, SUM(LENGTH(data)) b FROM session_keys WHERE category='pre-key'\");
  console.log('sebelum:', before.rows[0]);
  await c.execute(\"DELETE FROM session_keys WHERE category='pre-key' AND scope='main'\");
  const after = await c.execute(\"SELECT COUNT(*) n, SUM(LENGTH(data)) b FROM session_keys\");
  console.log('sesudah:', after.rows[0]);
  c.close();
};
run();
"
```

Perintah di atas menghapus **seluruh** pre-key untuk mengukur dampak batas atas. Aman secara protokol — bot mengupload ulang otomatis — tapi verifikasi dulu `session`, `sender-key`, `identity-key` masih utuh setelahnya:

```bash
node -e "
const run = async () => {
  const { config } = await import('./config.js');
  const { createClient } = await import('@libsql/client');
  const c = createClient({ url: config.turso.url, authToken: config.turso.authToken });
  const r = await c.execute('SELECT category, COUNT(*) n, SUM(LENGTH(data)) b FROM session_keys GROUP BY category ORDER BY b DESC');
  for (const row of r.rows) console.log(row.category, row.n, row.b);
  c.close();
};
run();
"
```

Target: `session` 490 key / 505.197 byte, `sender-key` 174 / 644.290, `identity-key` 38 / 2.698 — sama persis dengan baseline spec. Kalau ada yang berubah, sweep salah sasaran: HENTIKAN jangan lanjut.

- [ ] **Step 4: Catat hasil ke spec**

Tambahkan hasil ukur aktual di bawah tabel Success criteria di spec, dengan tanggal. Kalau angkanya meleset dari ambang, tulis apa yang meleset dan kenapa — angka hasil ukur yang salah Target adalah informasi, bukan kegagalan.

## Out of Scope

Tiga hal sengaja tidak dikerjakan. Semuanya punya alasan.

**Pasifkan `removePreKey`.** Sekarang delete path-nya sudah benar (Task 1), jadi ini bisa dikerjakan. Ditarik karena `removePreKey` hanya Benefit setelah sweep 30 hari berjalan — dan menambah perubahan di jalur produksi `keys.set` sebelum bukti bahwa DELETE-nya benar di produksi adalahSEQ yang salah. Kalau setelah beberapa siklus sweep pre-key masih menumpuk, ini pekerjaan berikutnya dan tidak perlu rethinking.

**Prune `device-list`.** 400 key / 2.289 byte = 0,08% dari `session_keys`. Risiko (peer mungkin perlu kirim ulang device list) tidak sebanding. Kalau nanti dibutuhkan, TTL 90 hari.

**`users.json` 343 KB / 331 user inactive.** Ada di spec §5. Perlu desain sendiri — intsinya mengganti hard delete di `data-pruner.js:29-42` jadi arsip dengan restore transparan, supaya user dengan `koin: 100000` tidak kehilangan progres. Tidak dicampur ke pekerjaan pruning pre-key.