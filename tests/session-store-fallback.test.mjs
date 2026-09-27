import { describe, it, before } from "node:test";
import assert from "node:assert";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";

describe("turso session load must not fabricate an authoritative session", () => {
  let m;
  before(async () => {
    m = await import("../src/lib/turso-session.js");
  });

  it("returns null when session_creds has no row for the scope", async () => {
    const turso = await import("../src/lib/turso.js");
    const client = await turso.createTursoClient({
      enabled: true,
      url: "file::memory:",
    });
    await turso.initTursoTables(client);

    assert.equal(
      await m.loadState("no-such-scope"),
      null,
      "empty session_creds must yield null so the caller falls back to local files",
    );
  });

  it("returns a real state once a creds row exists", async () => {
    const turso = await import("../src/lib/turso.js");
    const client = await turso.createTursoClient({
      enabled: true,
      url: "file::memory:",
    });
    await turso.initTursoTables(client);
    await client.execute({
      sql: "INSERT INTO session_creds (scope, creds, updated_at) VALUES (?, ?, ?)",
      args: [
        "main",
        JSON.stringify({ registered: true, me: { id: "1@s.whatsapp.net" } }),
        1,
      ],
    });

    const state = await m.loadState("main");
    assert.ok(state, "a stored row must still load");
    assert.equal(state.creds.registered, true);
    assert.equal(state.creds.me.id, "1@s.whatsapp.net");
  });

  it("useTursoAuthState reports state:null so connection.js uses local files", async () => {
    const turso = await import("../src/lib/turso.js");
    const client = await turso.createTursoClient({
      enabled: true,
      url: "file::memory:",
    });
    await turso.initTursoTables(client);

    const res = await m.useTursoAuthState("empty-scope");
    assert.equal(res.state, null, "caller must see null -> local-file fallback");
    assert.equal(typeof res.saveCreds, "function");
  });
});

describe("durable session store", () => {
  it("persists the session to the local folder even when Turso has nothing", async () => {
    const turso = await import("../src/lib/turso.js");
    const client = await turso.createTursoClient({
      enabled: true,
      url: "file::memory:",
    });
    await turso.initTursoTables(client);

    const dir = fs.mkdtempSync(path.join(os.tmpdir(), "sess-durable-"));
    const { useDurableAuthState } = await import("../src/lib/turso-session.js");

    const res = await useDurableAuthState("durable-a", dir);
    assert.ok(res.state, "a local state must always be produced");
    assert.equal(res.state.creds.registered, false);

    // simulate a successful pairing
    res.state.creds.registered = true;
    res.state.creds.me = { id: "628@s.whatsapp.net" };
    await res.saveCreds();

    const onDisk = JSON.parse(
      fs.readFileSync(path.join(dir, "creds.json"), "utf-8"),
    );
    assert.equal(
      onDisk.registered,
      true,
    );
  });

  it("restores a paired session from Turso into the local folder", async () => {
    const turso = await import("../src/lib/turso.js");
    const client = await turso.createTursoClient({
      enabled: true,
      url: "file::memory:",
    });
    await turso.initTursoTables(client);
    await client.execute({
      sql: "INSERT INTO session_creds (scope, creds, updated_at) VALUES (?, ?, ?)",
      args: [
        "restore-me",
        JSON.stringify({ registered: true, me: { id: "628@s.whatsapp.net" } }),
        1,
      ],
    });

    const dir = fs.mkdtempSync(path.join(os.tmpdir(), "sess-restore-"));
    const { useDurableAuthState } = await import("../src/lib/turso-session.js");
    const res = await useDurableAuthState("restore-me", dir);

    assert.equal(res.state.creds.registered, true, "session restored from mirror");
    assert.equal(res.state.creds.me.id, "628@s.whatsapp.net");
  });

  it("an aborted pairing leftover does not block restoring a good Turso session", async () => {
    const turso = await import("../src/lib/turso.js");
    const client = await turso.createTursoClient({
      enabled: true,
      url: "file::memory:",
    });
    await turso.initTursoTables(client);
    await client.execute({
      sql: "INSERT INTO session_creds (scope, creds, updated_at) VALUES (?, ?, ?)",
      args: [
        "leftover",
        JSON.stringify({ registered: true, me: { id: "628@s.whatsapp.net" } }),
        1,
      ],
    });

    const dir = fs.mkdtempSync(path.join(os.tmpdir(), "sess-leftover-"));
    fs.writeFileSync(
      path.join(dir, "creds.json"),
      JSON.stringify({
        registered: false,
        pairingCode: "OKTZZLAH",
        me: { id: "628@s.whatsapp.net", name: "~" },
      }),
    );

    const { useDurableAuthState } = await import("../src/lib/turso-session.js");
    const res = await useDurableAuthState("leftover", dir);

    assert.equal(
      res.state.creds.registered,
      true,
      "a half-pairing leftover must not shadow a real session in the mirror",
    );
  });

  it("a dead Turso does not orphan the session (no client at all)", async () => {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), "sess-dead-"));
    const { useDurableAuthState } = await import("../src/lib/turso-session.js");

    const res = await useDurableAuthState("dead-scope", dir);
    assert.ok(res.state, "must still produce a usable local state");
    res.state.creds.registered = true;
    await res.saveCreds();

    const onDisk = JSON.parse(
      fs.readFileSync(path.join(dir, "creds.json"), "utf-8"),
    );
    assert.equal(onDisk.registered, true);
  });
});
