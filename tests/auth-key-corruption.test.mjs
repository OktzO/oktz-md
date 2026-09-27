import { describe, it, beforeEach, afterEach } from "node:test";
import assert from "node:assert";
import { readFile } from "node:fs/promises";
import { mkdtempSync, writeFileSync, readFileSync, readdirSync, rmSync, existsSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { useDurableAuthState, setRemoteSessionLogger } from "../src/lib/turso-session.js";

const CONNECTION = new URL("../src/connection.js", import.meta.url).pathname;

const PAIRED = JSON.stringify({
  me: { id: "628111:1@s.whatsapp.net" },
  registered: true,
  account: { id: "628111:1@s.whatsapp.net" },
});

let dir;
const logs = [];

beforeEach(() => {
  dir = mkdtempSync(path.join(tmpdir(), "a2-session-"));
  logs.length = 0;
  setRemoteSessionLogger((m) => logs.push(m));
  writeFileSync(path.join(dir, "creds.json"), PAIRED);
});

afterEach(() => {
  setRemoteSessionLogger(null);
  rmSync(dir, { recursive: true, force: true });
});

const corrupt = (name, body) => writeFileSync(path.join(dir, name), body);
const good = (name, obj) => writeFileSync(path.join(dir, name), JSON.stringify(obj));

/* ------------------------------------------------------------------ */
/* A2a - key file rusak: dulu throw, tidak pernah self-heal            */
/* ------------------------------------------------------------------ */

describe("keys.get dengan key file rusak", () => {
  it("TIDAK melempar; berubah jadi miss supaya sesi bisa re-negotiate", async () => {
    corrupt("pre-key-1.json", '{"abc":'); // JSON terpotong
    const { state } = await useDurableAuthState("main", dir);
    const got = await state.keys.get("pre-key", ["1"]);
    assert.deepEqual(got, {}, "key rusak harus jadi miss, bukan error fatal");
  });

  it("file rusak dikarantina, bukan dihapus (bisa dipulihkan operator)", async () => {
    corrupt("pre-key-1.json", '{"abc":');
    const { state } = await useDurableAuthState("main", dir);
    await state.keys.get("pre-key", ["1"]);

    const files = readdirSync(dir);
    const quarantined = files.filter((f) => /^pre-key-1\.json\.broken-/.test(f));
    assert.equal(quarantined.length, 1, `file harus pindah ke .broken-*: ${files}`);
    assert.ok(!files.includes("pre-key-1.json"), "nama asli harus vacated");
    assert.equal(
      readFileSync(path.join(dir, quarantined[0]), "utf8"),
      '{"abc":',
      "isi asli harus utuh di file karantina",
    );
  });

  it("korupsinya TERDIAGNOSIS: nama file + langkah pemulihan ada di log", async () => {
    corrupt("pre-key-1.json", '{"abc":');
    const { state } = await useDurableAuthState("main", dir);
    await state.keys.get("pre-key", ["1"]);

    assert.ok(logs.length > 0, "tidak boleh ada yang diam");
    const line = logs.join("\n");
    assert.match(line, /pre-key-1\.json/, "log harus menyebut file yang rusak");
    assert.match(line, /karantina|broken/i, "log harus menyebut karantina");
  });

  it("karantinanya sekali per file, tidak spam tiap panggilan berikutnya", async () => {
    corrupt("pre-key-1.json", '{"abc":');
    const { state } = await useDurableAuthState("main", dir);
    await state.keys.get("pre-key", ["1"]);
    // file sudah dikarantina di panggilan pertama; tiga panggilan berikutnya
    // harus bersih dan diam (file tidak ada lagi, jadi ini miss biasa)
    await state.keys.get("pre-key", ["1"]);
    await state.keys.get("pre-key", ["1"]);
    await state.keys.get("pre-key", ["1"]);

    const events = logs.filter((l) => l.includes("dikarantina"));
    assert.equal(events.length, 1, `harus tepat 1 karantina, dapat: ${events.length}`);
    assert.match(events[0], /pre-key-1\.json/);
  });
});

/* ------------------------------------------------------------------ */
/* A2b - key yang sehat tidak boleh ikut|population kтьори              */
/* ------------------------------------------------------------------ */
/* ------------------------------------------------------------------ */
/* A2b - key yang sehat tidak boleh ikut korbankan              */
/* ------------------------------------------------------------------ */

describe("keys.get tidak boleh mencederai key yang sehat", () => {
  it("satu id rusak tidak boleh menenggelamkan id lain di batch yang sama", async () => {
    good("pre-key-1.json", { a: "AQAB" });
    corrupt("pre-key-2.json", '{"b":');
    const { state } = await useDurableAuthState("main", dir);
    const got = await state.keys.get("pre-key", ["1", "2"]);
    assert.deepEqual(Object.keys(got), ["1"], "id sehat harus tetap kembali");
    assert.deepEqual(got["1"], { a: "AQAB" });
  });

  it("key yang valid dikembalikan apa adanya, tanpa karantina", async () => {
    good("pre-key-7.json", { a: "AQAB" });
    const { state } = await useDurableAuthState("main", dir);
    const got = await state.keys.get("pre-key", ["7"]);
    assert.deepEqual(got["7"], { a: "AQAB" });
    assert.ok(!readdirSync(dir).some((f) => f.includes("broken")), "tidak ada karantina");
    assert.equal(logs.length, 0, "tidak boleh ada log untuk key sehat");
  });

  it("file yang memang tidak ada = miss biasa, BUKAN korup", async () => {
    const { state } = await useDurableAuthState("main", dir);
    const got = await state.keys.get("pre-key", ["999"]);
    assert.deepEqual(got, {});
    assert.equal(logs.length, 0, "tidak boleh ada log untuk key yang absen");
    assert.ok(!readdirSync(dir).some((f) => f.includes("broken")));
  });

  it("pre-key dan sender-key rusak dikarantina terpisah", async () => {
    corrupt("pre-key-1.json", '{"a":');
    corrupt("sender-key-co-1.json", '{"b":');
    const { state } = await useDurableAuthState("main", dir);
    await state.keys.get("pre-key", ["1"]);
    await state.keys.get("sender-key-co", ["1"]);

    const files = readdirSync(dir);
    assert.ok(files.some((f) => /^pre-key-1\.json\.broken-/.test(f)), files.join());
    assert.ok(
      files.some((f) => /^sender-key-co-1\.json\.broken-/.test(f)),
      files.join(),
    );
  });

  it("id ber-slash (dipakai app-state) tidak merusak logika", async () => {
    corrupt("app-state-sync-key-abc-def.json", "{oops");
    const { state } = await useDurableAuthState("main", dir);
    const got = await state.keys.get("app-state-sync-key", ["abc/def"]);
    assert.deepEqual(got, {});
  });
});

/* ------------------------------------------------------------------ */
/* A2c - jalur tanpa Turso di connection.js ikut terlindungi           */
/* ------------------------------------------------------------------ */

describe("useMultiFileAuthState mentah (jalur tanpa Turso) ikut terlindungi", () => {
  it("connection.js membungkus keys store mentah dengan recovery", async () => {
    const src = await readFile(CONNECTION, "utf8");
    assert.match(
      src,
      /withCorruptKeyRecovery\(/,
      "jalur useMultiFileAuthState harus dibungkus juga",
    );
    assert.doesNotMatch(
      src,
      /state\s*=\s*result\.state;\s*\n\s*saveCreds\s*=\s*result\.saveCreds;/,
      "state mentah tidak boleh dipakai apa adanya",
    );
  });
});
