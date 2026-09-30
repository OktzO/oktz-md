// Mirror key ke Turso hanyadibuat kalau remote di-load, dan remote hanya di-load
// kalau creds lokal terlihat belum paired. Setelah wipe (atau container
// replacement, karena storage/ gitignored) lokal baru pair sendiri, Turso sudah
// kosong, jadi remote = null dan remoteKeys null selamanya.
//
// Akibatnya setiap keys.set melewati `if (!remoteKeys) return;` dan mirror
// dibuang tanpa satu warning pun. Bot tetap jalan, prekeys tetap terkirim ke
// WhatsApp, tapi tidak ada satu pun key yang sampai ke Turso — dan pemulihannya
//mustahil, karena tidak ada yang pernah ditulis. Pola device 52 lalu 53 lalu
// 54 adalah akibatnya, bukan sebabnya.
//
// Bukti di produksi: setelah wipe + pair ulang, session_creds = 1 tapi
// session_keys = 0 di seluruh kategori, sementara log boot tetap melaporkan
// "pre-keys udah dikirim".
import { describe, it, before } from "node:test";
import assert from "node:assert";
import { mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

let dir;

before(() => {
  dir = mkdtempSync(join(tmpdir(), "turso-durable-"));
});

describe("useDurableAuthState: mirror key harus hidup setelah pair lokal", () => {
  it("keys.set tetap masuk Turso walau creds lokal sudah paired dan Turso kosong", async () => {
    const tursoModule = await import("../src/lib/turso.js");
    const client = await tursoModule.createTursoClient({ enabled: true, url: "file::memory:" });
    await tursoModule.initTursoTables(client);
    const { useDurableAuthState } = await import("../src/lib/turso-session.js");

    // Local session folder paired, Turso has no rows at all — exactly the state
    // after a wipe: the operator re-paired on the panel, and the mirror has to
    // re-establish itself from that moment on.
    const { state } = await useDurableAuthState("main", dir);
    state.creds.registered = true;
    state.creds.account = { id: "6285608953677@s.whatsapp.net" };

    await state.keys.set({
      "pre-key": { 7: { private: Buffer.from("11", "hex"), public: Buffer.from("22", "hex") } },
    });
    // Let the mirror's queued write settle.
    await new Promise((r) => setTimeout(r, 200));

    const rows = await client.execute({
      sql: "SELECT COUNT(*) AS n FROM session_keys WHERE scope = ? AND category = ?",
      args: ["main", "pre-key"],
    });

    assert.strictEqual(
      rows.rows[0].n,
      1,
      "pre-key written while local creds look paired never reached Turso — " +
      "the mirror is permanently dead after a wipe, so the next container " +
      "replacement loses every session again",
    );

    rmSync(dir, { recursive: true, force: true });
  });

  it("kunci pre-key yang hilang ikut terpulihkan dari Turso", async () => {
    const tursoModule = await import("../src/lib/turso.js");
    const client = await tursoModule.createTursoClient({ enabled: true, url: "file::memory:" });
    await tursoModule.initTursoTables(client);
    const { useDurableAuthState, loadState } = await import("../src/lib/turso-session.js");

    const folder = mkdtempSync(join(tmpdir(), "turso-durable2-"));
    // Seed a paired mirror by hand, the way a previous run would have left it.
    await client.execute({
      sql: "INSERT INTO session_creds (scope, creds, updated_at) VALUES (?, ?, ?)",
      args: ["main", JSON.stringify({ registered: true }), 1],
    });
    await client.execute({
      sql: "INSERT INTO session_keys (scope, category, id, data, updated_at) VALUES (?, ?, ?, ?, ?)",
      args: ["main", "pre-key", "7", JSON.stringify({ private: "11", public: "22" }), 1],
    });

    const seeded = await loadState("main");
    assert.ok(seeded?.keys, "loadState must expose the mirrored keys");

    const { state } = await useDurableAuthState("main", folder);
    const got = await state.keys.get("pre-key", ["7"]);

    assert.ok(got["7"], "a pre-key that only exists in Turso must still be readable");
    rmSync(folder, { recursive: true, force: true });
  });
});
