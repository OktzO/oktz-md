import { describe, it } from "node:test";
import assert from "node:assert";
import fs from "node:fs";
import path from "node:path";
import { Database } from "../src/lib/database.js";

function makeDb(dbPath, storeData = {}) {
  fs.rmSync(dbPath, { recursive: true, force: true });
  const db = new Database(dbPath);
  for (const [key, data] of Object.entries(storeData)) {
    db.stores[key] = {
      data,
      adapter: { filename: path.join(dbPath, `${key}.json`) },
    };
  }
  return db;
}

describe("Database with Turso backend", () => {
  it("creates stores with defaults", () => {
    const db = new Database("/tmp/test-turso-db");
    // local-only test — no Turso client needed
    assert.ok(db.stores);
    assert.equal(typeof db.getUser, "function");
  });

  it("flushAllToTurso writes all keys and returns true", async () => {
    const dbPath = "/tmp/test-turso-db-ok";
    const db = makeDb(dbPath, {
      users: { "123": { name: "x" } },
      settings: { selfMode: true },
    });
    const written = {};
    db.tursoEnabled = true;
    db.tursoClient = {
      execute: async ({ sql, args }) => {
        written[args[0]] = JSON.parse(args[1]);
        return { rows: [] };
      },
    };
    const ok = await db.flushAllToTurso();
    assert.equal(ok, true);
    assert.deepEqual(written.users, { "123": { name: "x" } });
    assert.deepEqual(written.settings, { selfMode: true });
  });

  it("save() returns false and writes local file when turso write fails", async () => {
    const dbPath = "/tmp/test-turso-db-fail";
    const db = makeDb(dbPath, { users: { "123": { name: "x" } } });
    db.tursoEnabled = true;
    db.tursoClient = {
      execute: async () => {
        throw new Error("network down");
      },
    };
    const ok = await db.save();
    assert.equal(ok, false);
    const fileContent = JSON.parse(
      fs.readFileSync(path.join(dbPath, "users.json"), "utf-8"),
    );
    assert.deepEqual(fileContent, { "123": { name: "x" } });
  });

  it("save() returns true when turso write succeeds", async () => {
    const dbPath = "/tmp/test-turso-db-save-ok";
    const db = makeDb(dbPath, { users: { "123": { name: "x" } } });
    db.tursoEnabled = true;
    db.tursoClient = {
      execute: async () => ({ rows: [] }),
    };
    const ok = await db.save();
    assert.equal(ok, true);
  });
});
