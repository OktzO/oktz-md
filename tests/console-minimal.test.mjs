import { describe, it, before } from "node:test";
import assert from "node:assert";

// Minimal-log: tabel plugin ringkas (1 baris) & log command 1 baris.
// Run: node --test tests/console-minimal.test.mjs

function capture(fn) {
  const buf = [];
  const orig = console.log;
  console.log = (...a) => buf.push(a.map(String).join(" "));
  try {
    fn();
  } finally {
    console.log = orig;
  }
  return buf;
}

describe("printPluginTable minimal", () => {
  let mod;
  let origLog;

  before(async () => {
    origLog = console.log;
    console.log = () => {};
    try {
      mod = await import("../src/lib/plugins.js");
    } finally {
      console.log = origLog;
    }
  });

  it("mencetak satu baris total saja, tanpa baris per kategori", () => {
    const plugins = [
      { name: "a", category: "owner" },
      { name: "b", category: "owner" },
      { name: "c", category: "group" },
      { name: "d", category: "tools" },
      { name: "e", category: "tools" },
      { name: "f", category: "rpg" },
    ];
    const lines = capture(() => mod.printPluginTable(plugins));
    assert.strictEqual(lines.length, 1, `expect 1 line, got ${lines.length}`);
    const line = lines[0];
    assert.ok(line.includes("6"), `missing total: ${line}`);
    assert.ok(line.includes("4 kategori"), `missing catCount: ${line}`);
    assert.ok(!line.includes("owner"), `kategori bocor: ${line}`);
    assert.ok(!line.includes("─"), `border bocor: ${line}`);
  });

  it("skip diam-diam saat daftar kosong", () => {
    const lines = capture(() => mod.printPluginTable([]));
    assert.strictEqual(lines.length, 0);
  });
});

describe("logger logCommand minimal", () => {
  let mod;

  before(async () => {
    const origLog = console.log;
    console.log = () => {};
    try {
      mod = await import("../src/lib/logger.js");
    } finally {
      console.log = origLog;
    }
  });

  it("log command jadi satu baris dengan nama user, tanpa nomor sender", () => {
    const lines = capture(() =>
      mod.logCommand({
        prefix: ".",
        command: "ping",
        pushName: "Zann",
        sender: "6281234567890@s.whatsapp.net",
        chatType: "private",
      }),
    );
    assert.strictEqual(lines.length, 1, `expect 1 line, got ${lines.length}`);
    const joined = lines[0];
    assert.ok(joined.includes(".ping"), `missing command: ${joined}`);
    assert.ok(joined.includes("Zann"), `missing name: ${joined}`);
    assert.ok(!joined.includes("6281234567890"), `number leaked: ${joined}`);
  });

  it("fallback nama generik saat pushName kosong, tanpa nomor", () => {
    const lines = capture(() =>
      mod.logCommand({
        prefix: ".",
        command: "menu",
        pushName: "",
        sender: "6281234567890@s.whatsapp.net",
        chatType: "private",
      }),
    );
    const joined = lines[0];
    assert.ok(!joined.includes("6281234567890"), `number leaked: ${joined}`);
    assert.ok(joined.includes(".menu"));
    assert.ok(joined.includes("Pengguna"));
    assert.strictEqual(lines.length, 1);
  });

  it("menampilkan nama grup untuk chat grup dalam satu baris", () => {
    const lines = capture(() =>
      mod.logCommand({
        prefix: ".",
        command: "sticker",
        pushName: "Zann",
        sender: "6281234567890@s.whatsapp.net",
        chatType: "group",
        groupName: "Test Group",
      }),
    );
    const joined = lines[0];
    assert.ok(joined.includes("Test Group"), `missing group: ${joined}`);
    assert.ok(!joined.includes("6281234567890"));
    assert.strictEqual(lines.length, 1);
  });

  it("skip command kosong", () => {
    const lines = capture(() =>
      mod.logCommand({ prefix: ".", command: "", sender: "6281@s.whatsapp.net" }),
    );
    assert.strictEqual(lines.length, 0);
  });
});