import { describe, it, before, after } from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { Database } from "../src/lib/database.js";

const G1 = "1203630000001@g.us";
const G2 = "1203630000002@g.us";

function makeDb() {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), "group-plugin-iso-"));
  const db = new Database(dir);
  db.tursoEnabled = false;
  return { db, dir };
}

describe("isolasi setting plugin per-grup", () => {
  it("setGroup untuk grup 1 tidak menyentuh grup 2", () => {
    const { db, dir } = makeDb();
    db.setGroup(G1, { onlyAdmin: true, autoSholat: true, anticulik: "on" });
    const g1 = db.getGroup(G1);
    const g2 = db.getGroup(G2);
    assert.equal(g1.onlyAdmin, true);
    assert.equal(g1.autoSholat, true);
    assert.equal(g1.anticulik, "on");
    assert.equal(g2, null);
    fs.rmSync(dir, { recursive: true, force: true });
  });

  it("fallback: grup yang belum diatur mengikuti setting global lama", () => {
    const { db, dir } = makeDb();
    db.setting("onlyAdmin", true);
    db.setting("autoSholat", true);
    db.setGroup(G1, {});
    db.setGroup(G2, {});
    // grup 1 dimatikan secara eksplisit
    const g1 = db.getGroup(G1);
    g1.onlyAdmin = false;
    db.setGroup(G1, g1);

    const fresh1 = db.getGroup(G1);
    const fresh2 = db.getGroup(G2);
    // eksplisit di grup 1
    assert.equal(fresh1.onlyAdmin, false);
    // grup 2 tidak ikut berubah
    assert.equal(fresh2.onlyAdmin, undefined);
    // fallback manual: belum diatur berarti ikut global
    assert.equal(fresh2.onlyAdmin ?? db.setting("onlyAdmin"), true);
    assert.equal(fresh1.onlyAdmin ?? db.setting("onlyAdmin"), false);
    fs.rmSync(dir, { recursive: true, force: true });
  });

  it("autoSholat/notifSholat per grup tidak saling memengaruhi", () => {
    const { db, dir } = makeDb();
    db.setting("autoSholat", true);
    db.setGroup(G1, { notifSholat: false });
    db.setGroup(G2, { notifSholat: true, autoSholatKota: { id: "1301", nama: "KOTA JAKARTA" } });
    assert.equal(db.getGroup(G1).notifSholat, false);
    assert.equal(db.getGroup(G2).notifSholat, true);
    assert.equal(db.getGroup(G1).autoSholatKota, undefined);
    assert.equal(db.getGroup(G2).autoSholatKota.nama, "KOTA JAKARTA");
    fs.rmSync(dir, { recursive: true, force: true });
  });

  it("anticulik per grup tersimpan mandiri", () => {
    const { db, dir } = makeDb();
    db.setGroup(G1, { anticulik: "on" });
    db.setGroup(G2, { anticulik: "off" });
    assert.equal(db.getGroup(G1).anticulik, "on");
    assert.equal(db.getGroup(G2).anticulik, "off");
    fs.rmSync(dir, { recursive: true, force: true });
  });
});
