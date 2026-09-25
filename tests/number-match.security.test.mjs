import { describe, it, before, after } from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";

import config, { isOwner, isSelf } from "../config.js";

// Nilai dari config.js (identitas Oktz Bot)
const OWNER_NUM = "6285608953677";
const BOT_NUM = "6285143885645";

describe("isOwner — strict number matching (security)", () => {
  it("exact match nomor owner di config => true", () => {
    assert.equal(isOwner(OWNER_NUM), true);
  });

  it("format JID WhatsApp => true", () => {
    assert.equal(isOwner(`${OWNER_NUM}@s.whatsapp.net`), true);
  });

  it("JID dengan device suffix (:44) => true", () => {
    assert.equal(isOwner(`${OWNER_NUM}:44@s.whatsapp.net`), true);
  });

  it("format bebas dengan +/-/- => true", () => {
    assert.equal(isOwner(`+${OWNER_NUM.slice(0, 2)} ${OWNER_NUM.slice(2, 5)}-${OWNER_NUM.slice(5, 9)}-${OWNER_NUM.slice(9)}`), true);
  });

  it("nomor bot sendiri => true (self = owner by design)", () => {
    assert.equal(isOwner(BOT_NUM), true);
  });

  it("subset/suffix attack: sender punya extra digit di belakang => false", () => {
    // dulu: cleanNumber.endsWith(c) bikin ini LOLAS sebagai owner
    assert.equal(isOwner(`${OWNER_NUM}1`), false);
    assert.equal(isOwner(`${OWNER_NUM}99999`), false);
  });

  it("prefix attack: sender kekurangan digit => false", () => {
    // dulu: c.endsWith(cleanNumber) bikin ini LOLAS sebagai owner
    assert.equal(isOwner(OWNER_NUM.slice(0, -1)), false);
    assert.equal(isOwner(OWNER_NUM.slice(0, 8)), false);
  });

  it("prefix+suffix attack pada nomor bot => false", () => {
    assert.equal(isOwner(`1${BOT_NUM}`), false);
    assert.equal(isOwner(`${BOT_NUM}00`), false);
  });

  it("nomor lain yang tidak berhubungan => false", () => {
    assert.equal(isOwner("628999999999"), false);
    assert.equal(isOwner("08123456789"), false);
  });

  it("input kosong / tanpa digit => false", () => {
    assert.equal(isOwner(null), false);
    assert.equal(isOwner(""), false);
    assert.equal(isOwner("abc@lid"), false);
  });

  it("tetap mengenali owner walau database belum di-init (boot order)", () => {
    // getDatabase() melempar error sebelum initDatabase();
    // pengecekan owner config harus tetap jalan.
    assert.equal(isOwner(OWNER_NUM), true);
  });
});

describe("isSelf — strict matching", () => {
  it("nomor bot exact => true", () => {
    assert.equal(isSelf(BOT_NUM), true);
  });

  it("JID bot dengan device suffix => true", () => {
    assert.equal(isSelf(`${BOT_NUM}:12@s.whatsapp.net`), true);
  });

  it("substring/prefix/suffix bukan match => false", () => {
    assert.equal(isSelf(`${BOT_NUM}1`), false);
    assert.equal(isSelf(`1${BOT_NUM}`), false);
    assert.equal(isSelf(BOT_NUM.slice(0, 10)), false);
  });
});

describe("premium-db — matchJid strict equality", () => {
  let tmpDir;
  let originalCwd;
  let premiumDb;

  before(async () => {
    originalCwd = process.cwd();
    tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "premium-db-test-"));
    process.chdir(tmpDir);

    const dbDir = path.join(tmpDir, "src", "database");
    fs.mkdirSync(dbDir, { recursive: true });
    fs.writeFileSync(
      path.join(dbDir, "owner.json"),
      JSON.stringify({ owners: ["628555666777"] }),
    );
    fs.writeFileSync(
      path.join(dbDir, "premium.json"),
      JSON.stringify({ premium: ["628111222333"] }),
    );

    // Cache-bust: module ini sudah ter-load via import chain config.js
    // (dengan DB_PATH frozen ke cwd asli), jadi paksa instance baru
    // yang membaca cwd temp.
    premiumDb = await import(
      `../src/lib/premium-db.js?fixture=${Date.now()}`
    );
  });

  it("premium exact match => true", () => {
    assert.equal(premiumDb.isPremium("628111222333"), true);
  });

  it("premium via JID & device suffix => true", () => {
    assert.equal(premiumDb.isPremium("628111222333@s.whatsapp.net"), true);
    assert.equal(premiumDb.isPremium("628111222333:7@s.whatsapp.net"), true);
  });

  it("premium subset/prefix attack => false", () => {
    assert.equal(premiumDb.isPremium("6281112223339"), false);
    assert.equal(premiumDb.isPremium("1628111222333"), false);
    assert.equal(premiumDb.isPremium("62811122233"), false);
  });

  it("owner exact => true, subset/prefix => false", () => {
    assert.equal(premiumDb.isOwner("628555666777"), true);
    assert.equal(premiumDb.isOwner("628555666777@s.whatsapp.net"), true);
    assert.equal(premiumDb.isOwner("62855566677788"), false);
    assert.equal(premiumDb.isOwner("628555666"), false);
  });

  after(() => {
    process.chdir(originalCwd);
    fs.rmSync(tmpDir, { recursive: true, force: true });
  });
});
