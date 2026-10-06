import { test, mock } from "node:test";
import assert from "node:assert";

let clientImpl = null;
mock.module("../src/lib/turso.js", {
  namedExports: {
    getTursoClient: () => clientImpl,
    createTursoClient: async () => null,
    initTursoTables: async () => {},
    closeTurso: async () => {},
  },
});

const { loadState, useDurableAuthState } = await import("../src/lib/turso-session.js");
const config = (await import("../config.js")).default;

const RETRIEVABLE = /fetch failed|network|ECONN|ETIMEDOUT|ECONNRESET|socket|disconnect/i;

function failingThenOk(realResult, failures) {
  let calls = 0;
  return {
    calls: () => calls,
    execute: async () => {
      calls++;
      if (calls <= failures) throw new Error("fetch failed");
      return realResult;
    },
  };
}

test("loadState menahan putus jaringan sesaat: retry lalu sukses", async () => {
  const creds = { account: { lid: "123" }, registered: true };
  const flapping = failingThenOk({ rows: [{ creds: JSON.stringify(creds) }] }, 2);
  clientImpl = { execute: flapping.execute, batch: async () => {} };

  const state = await loadState("main");
  assert.ok(state, "state harus ada setelah retry");
  assert.equal(state.creds.registered, true);
  assert.equal(flapping.calls(), 3, "dua gagal + satu sukses");
  clientImpl = null;
});

test("error non-jaringan langsung lempar, tanpa retry", async () => {
  let calls = 0;
  clientImpl = {
    execute: async () => {
      calls++;
      throw new Error("syntax error in SQL");
    },
    batch: async () => {},
  };
  const state = await loadState("main");
  assert.equal(state, null);
  assert.equal(calls, 1);
  clientImpl = null;
});

test("koneksi Turso production hanya dicek kalau kredensial diisi", async (t) => {
  if (!config.turso?.enabled || !config.turso?.url) {
    t.skip("TURSO_URL tidak diisi — lewati probe jaringan");
    return;
  }
  const { createClient } = await import("@libsql/client");
  const client = createClient({ url: config.turso.url, authToken: config.turso.authToken });
  try {
    const rs = await client.execute("SELECT 1 AS ok");
    assert.equal(Number(rs.rows[0].ok), 1);
    const tables = await client.execute(
      "SELECT name FROM sqlite_master WHERE type='table' AND name IN ('session_keys','session_creds')",
    );
    assert.ok(tables.rows.length >= 1, "tabel session minimal satu ada");
  } finally {
    client.close();
  }
});
