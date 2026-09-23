import { describe, it } from "node:test";
import assert from "node:assert";
import fs from "node:fs";
import { Database } from "../src/lib/ourin-database.js";
import {
  collectClearTargets,
  isClearcableJid,
  isAutoClearEnabled,
  setAutoClearEnabled,
} from "../src/lib/ourin-chat-cleaner.js";

function makeDb(dbPath) {
  fs.rmSync(dbPath, { recursive: true, force: true });
  const db = new Database(dbPath);
  db.tursoEnabled = false;
  return db;
}

describe("Auto-clear chat classifier", () => {
  it("accepts group and private-chat jids", () => {
    assert.equal(isClearcableJid("1203630123456@g.us"), true);
    assert.equal(isClearcableJid("6281234567890@s.whatsapp.net"), true);
  });

  it("rejects status, broadcast, newsletter, and bogus jids", () => {
    assert.equal(isClearcableJid("status@broadcast"), false);
    assert.equal(isClearcableJid("15189899000@broadcast"), false);
    assert.equal(isClearcableJid("1203630123456@newsletter"), false);
    assert.equal(isClearcableJid("nonsense"), false);
    assert.equal(isClearcableJid(""), false);
    assert.equal(isClearcableJid(null), false);
    assert.equal(isClearcableJid("globo@comedia"), false);
  });
});

describe("Auto-clear target collection", () => {
  it("merges store chats and participating groups, dedupes, filters, sorts", async () => {
    const storeChats = new Map([
      ["1203630111111@g.us", { id: "1203630111111@g.us" }],
      ["62811111111@s.whatsapp.net", { id: "62811111111@s.whatsapp.net" }],
      ["status@broadcast", { id: "status@broadcast" }],
    ]);
    const fakeSock = {
      store: { chats: storeChats },
      groupFetchAllParticipating: async () => ({
        "1203630111111@g.us": { id: "1203630111111@g.us" },
        "1203630222222@g.us": { id: "1203630222222@g.us" },
      }),
    };

    const targets = await collectClearTargets(fakeSock);
    assert.deepEqual(targets, [
      "1203630111111@g.us",
      "1203630222222@g.us",
      "62811111111@s.whatsapp.net",
    ]);
  });

  it("returns empty array when there is nothing to clear", async () => {
    const targets = await collectClearTargets({
      store: { chats: new Map() },
      groupFetchAllParticipating: async () => ({}),
    });
    assert.deepEqual(targets, []);
  });

  it("survives a failing group fetch by still using store chats", async () => {
    const storeChats = new Map([
      ["62822222222@s.whatsapp.net", { id: "62822222222@s.whatsapp.net" }],
    ]);
    const targets = await collectClearTargets({
      store: { chats: storeChats },
      groupFetchAllParticipating: async () => {
        throw new Error("boom");
      },
    });
    assert.deepEqual(targets, ["62822222222@s.whatsapp.net"]);
  });

  it("keeps status/newsletter entries out of the final list", async () => {
    const storeChats = new Map([
      ["status@broadcast", { id: "status@broadcast" }],
      ["1203630333333@newsletter", { id: "1203630333333@newsletter" }],
    ]);
    const targets = await collectClearTargets({
      store: { chats: storeChats },
      groupFetchAllParticipating: async () => ({}),
    });
    assert.deepEqual(targets, []);
  });
});

describe("Auto-clear enable setting", () => {
  it("defaults to disabled", () => {
    const db = makeDb("/tmp/test-autoclear-default");
    assert.equal(isAutoClearEnabled(db), false);
  });

  it("can be turned on and persisted", () => {
    const db = makeDb("/tmp/test-autoclear-on");
    setAutoClearEnabled(true, db);
    assert.equal(isAutoClearEnabled(db), true);
  });

  it("can be turned off again", () => {
    const db = makeDb("/tmp/test-autoclear-off");
    setAutoClearEnabled(true, db);
    setAutoClearEnabled(false, db);
    assert.equal(isAutoClearEnabled(db), false);
  });
});