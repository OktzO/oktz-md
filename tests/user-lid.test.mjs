import { describe, it } from "node:test";
import assert from "node:assert";
import fs from "node:fs";
import { Database } from "../src/lib/ourin-database.js";

function makeDb(dbPath) {
  fs.rmSync(dbPath, { recursive: true, force: true });
  const db = new Database(dbPath);
  db.tursoEnabled = false;
  return db;
}

const LID = "165648885899430";
const PN2 = "6281234567890";

describe("Database LID-aware user store", () => {
  it("stores a LID user under a lid: key, not the fake number", () => {
    const db = makeDb("/tmp/test-user-lid");
    const user = db.setUser(`${LID}@lid`, { name: "Munorsh" });
    assert.equal(user.number, null);
    assert.equal(user.lid, LID);
    assert.equal(user.jid, null);
    assert.deepEqual(Object.keys(db.db.data.users), [`lid:${LID}`]);
  });

  it("read back a LID user by raw @lid, converted, and prefixed forms", () => {
    const db = makeDb("/tmp/test-user-lid-2");
    db.setUser(`${LID}@lid`, { name: "Munorsh" });

    const byLid = db.getUser(`${LID}@lid`);
    assert.equal(byLid.lid, LID);
    assert.equal(byLid.number, null);

    const byConverted = db.getUser(`${LID}@s.whatsapp.net`);
    assert.equal(byConverted.lid, LID);
    assert.equal(byConverted.number, null);
  });

  it("keeps PN users on plain-number keys with number set", () => {
    const db = makeDb("/tmp/test-user-lid-pn");
    const user = db.setUser("6281234567890@s.whatsapp.net", { name: "Budi" });
    assert.equal(user.number, "6281234567890");
    assert.equal(user.lid, null);
    assert.equal(db.getUser("6281234567890@s.whatsapp.net").name, "Budi");
  });

  it("deleteUser removes a LID user via any form", () => {
    const db = makeDb("/tmp/test-user-lid-del");
    db.setUser(`${LID}@lid`, { name: "Munorsh" });
    assert.equal(db.deleteUser(`${LID}@s.whatsapp.net`), true);
    assert.equal(db.getUser(`${LID}@lid`), null);
  });

  it("does not treat gacha husbu/waifu synthetic jids as LIDs", () => {
    const db = makeDb("/tmp/test-user-lid-gacha");
    const jid = "husbu_KiseRyota@s.whatsapp.net";
    db.setUser(jid, { fun: { pasangan: "" } });
    const user = db.getUser(jid);
    assert.ok(user, "gacha record still stored as before");
    assert.equal(user.lid, undefined);
    assert.equal(user.number, "husbu_KiseRyota");
    assert.ok(!Object.keys(db.db.data.users).some((k) => k === "lid:husbu_KiseRyota"));
  });

  it("migrateLegacyUsers moves plain-number LID keys to lid: and drops husbu_KiseRyota", () => {
    const db = makeDb("/tmp/test-user-lid-migrate");
    db.db.data.users = {
      [LID]: { name: "Munorsh", number: LID },
      [PN2]: { name: "True user", number: PN2 },
      husbu_KiseRyota: { fun: { pasangan: `${PN2}@s.whatsapp.net` } },
      status: { free: true },
    };
    const n = db.migrateLegacyUsers();
    assert.equal(n, 2);
    assert.ok(db.db.data.users[`lid:${LID}`]);
    assert.equal(db.db.data.users[`lid:${LID}`].lid, LID);
    assert.equal(db.db.data.users[`lid:${LID}`].number, null);
    assert.ok(db.db.data.users[PN2]);
    assert.ok(!("husbu_KiseRyota" in db.db.data.users));
    assert.ok(db.db.data.users.status);
  });

  it("migrateLegacyUsers is idempotent", () => {
    const db = makeDb("/tmp/test-user-lid-migrate2");
    db.db.data.users = {
      [`lid:${LID}`]: { name: "Munorsh", lid: LID, number: null },
    };
    assert.equal(db.migrateLegacyUsers(), 0);
    assert.ok(db.db.data.users[`lid:${LID}`]);
  });
});