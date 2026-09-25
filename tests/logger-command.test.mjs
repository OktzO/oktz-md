import { describe, it, before } from "node:test";
import assert from "node:assert";

// Regression test: command log box tampil untuk command, TANPA nomor sender
// (anti-doxx), dan chat non-command tidak log sama sekali (dijamin handler).
// Run: node --test tests/logger-command.test.mjs

const SENDER = "6281234567890@s.whatsapp.net";

describe("ourin-logger logCommand", () => {
  let mod;
  let logs;

  before(async () => {
    const origLog = console.log;
    const captured = [];
    console.log = (...a) => captured.push(a.map(String).join(" "));
    try {
      mod = await import("../src/lib/logger.js");
    } finally {
      console.log = origLog;
    }
    logs = (fn) => {
      const buf = [];
      const orig = console.log;
      console.log = (...a) => buf.push(a.map(String).join(" "));
      try {
        fn();
      } finally {
        console.log = orig;
      }
      return buf;
    };
  });

  it("menampilkan satu baris command dengan nama user, tanpa nomor sender", () => {
    const lines = logs(() =>
      mod.logCommand({
        prefix: ".",
        command: "ping",
        pushName: "Zann",
        sender: SENDER,
        chatType: "private",
      }),
    );
    assert.strictEqual(lines.length, 1, "satu baris saja");
    const joined = lines[0];
    assert.ok(joined.includes(".ping"), `missing command: ${joined}`);
    assert.ok(joined.includes("Zann"), `missing name: ${joined}`);
    assert.ok(!joined.includes("6281234567890"), `number leaked: ${joined}`);
  });

  it("fallback nama generik saat pushName kosong, tetap tanpa nomor", () => {
    const lines = logs(() =>
      mod.logCommand({
        prefix: ".",
        command: "menu",
        pushName: "",
        sender: SENDER,
        chatType: "private",
      }),
    );
    const joined = lines.join("\n");
    assert.ok(!joined.includes("6281234567890"), `number leaked: ${joined}`);
    assert.ok(joined.includes(".menu"));
    assert.ok(joined.includes("Pengguna"));
    assert.strictEqual(lines.length, 1);
  });

  it("menampilkan nama grup untuk chat grup", () => {
    const lines = logs(() =>
      mod.logCommand({
        prefix: ".",
        command: "sticker",
        pushName: "Zann",
        sender: SENDER,
        chatType: "group",
        groupName: "Test Group",
      }),
    );
    const joined = lines.join("\n");
    assert.ok(joined.includes("Test Group"), `missing group: ${joined}`);
    assert.ok(!joined.includes("6281234567890"));
    assert.strictEqual(lines.length, 1);
  });

  it("skip command kosong", () => {
    const lines = logs(() =>
      mod.logCommand({ prefix: ".", command: "", sender: SENDER }),
    );
    assert.strictEqual(lines.length, 0);
  });
});
