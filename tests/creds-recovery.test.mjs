import { describe, it, beforeEach, afterEach } from "node:test";
import assert from "node:assert";
import {
  mkdtempSync,
  writeFileSync,
  readdirSync,
  readFileSync,
  rmSync,
  existsSync,
} from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import {
  loadAuthStateWithRecovery,
  classifyAuthStateFailure,
  quarantineBrokenCreds,
} from "../src/connection.js";

let dir;
let logs;

beforeEach(() => {
  dir = mkdtempSync(path.join(tmpdir(), "a4-session-"));
  logs = [];
});

afterEach(() => rmSync(dir, { recursive: true, force: true }));

const put = (name, body) => writeFileSync(path.join(dir, name), body);
const msg = (m) => logs.join("\n");

/* ------------------------------------------------------------------ */
/* A4a - creds.json rusak: dulu boot loop tanpa penjelasan             */
/* ------------------------------------------------------------------ */

describe("creds.json ada tapi tidak bisa dipakai", () => {
  it("boot tetap jalan: file dikarantina, sesi baru dibuat", async () => {
    put("creds.json", '{"noiseKey":0.5,"me":'); // JSON terpotong
    const res = await loadAuthStateWithRecovery(dir, (m) => logs.push(m));

    assert.ok(res.state, "state harus ada, boot tidak boleh gagal");
    assert.ok(res.state.creds, "creds harus ada");
    assert.equal(res.recovered, true, "harus ditandai sebagai pemulihan");
  });

  it("file ASLI tidak hilang: ada di karantina, utuh isinya", async () => {
    put("creds.json", '{"noiseKey":0.5,"me":');
    await loadAuthStateWithRecovery(dir, (m) => logs.push(m));

    const files = readdirSync(dir);
    const q = files.filter((f) => /^creds\.json\.broken-/.test(f));
    assert.equal(q.length, 1, `harus ada 1 karantina, dapat: ${files.join()}`);
    assert.equal(
      readFileSync(path.join(dir, q[0]), "utf8"),
      '{"noiseKey":0.5,"me":',
      "isi asli harus utuh",
    );
    assert.ok(!files.includes("creds.json"), "nama asli harus vacated");
  });

  it("TIDAK ADA file yang dihapus sama sekali", async () => {
    put("creds.json", '{"me":');
    put("pre-key-1.json", '{"a":"AQAB"}');
    put("app-state-sync-key-main.json", '{"b":1}');
    const beforeFiles = readdirSync(dir);
    await loadAuthStateWithRecovery(dir, (m) => logs.push(m));
    const after = readdirSync(dir);

    // rename = 1 file jadi 1 file, jadi jumlahnya tidak boleh bertambah
    assert.equal(after.length, beforeFiles.length, "tidak boleh ada file hilang/tambah");
    for (const f of beforeFiles) {
      if (f === "creds.json") continue; // sudah dipindah ke karantina
      assert.ok(after.includes(f), `${f} harus tetap ada -- jangan hapus data lain`);
    }
    assert.equal(
      readFileSync(path.join(dir, "pre-key-1.json"), "utf8"),
      '{"a":"AQAB"}',
      "isi file lain tidak boleh berubah",
    );
  });

  it("pesan operator jelas: kenapa, di mana, dan apa yang dilakukan", async () => {
    put("creds.json", '"hello"');
    await loadAuthStateWithRecovery(dir, (m) => logs.push(m));
    const out = msg();
    assert.match(out, /creds\.json/, "sebut file yang bermasalah");
    assert.match(out, /karantina|broken/i, "sebut tindakan yang dilakukan");
    assert.match(out, /tidak dihapus|tetap ada|pemulihan|session/i, "sebut konsekuensi");
  });

  it("semua bentuk tidak-bisa-dipakai ditangani", async () => {
    for (const [label, body] of [
      ["truncated", '{"me":'],
      ["array", '[{"me":null}]'],
      ["string", '"hello"'],
      ["me-null", '{"me":null,"registered":true}'],
      ["null", "null"],
    ]) {
      const d = mkdtempSync(path.join(tmpdir(), "a4-x-"));
      writeFileSync(path.join(d, "creds.json"), body);
      const res = await loadAuthStateWithRecovery(d, () => {});
      assert.ok(res && res.state, `${label}: harus tetap boot`);
      assert.equal(res.recovered, true, `${label}: harus ditandai dipulihkan`);
      rmSync(d, { recursive: true, force: true });
    }
  });
});

/* ------------------------------------------------------------------ */
/* A4b - corrupt TIDAK BOLEH tercampur dengan "belum pernah pair"      */
/* ------------------------------------------------------------------ */

describe("corrupt vs belum-pair tidak boleh tercampur", () => {
  it("creds.json yang SAH tapi belum pair: TIDAK dikarantina, TIDAK scary", async () => {
    // persis bentuk yang ditulis initAuthCreds()
    put("creds.json", JSON.stringify({ noiseKey: 0.5, registered: false }));
    const res = await loadAuthStateWithRecovery(dir, (m) => logs.push(m));

    assert.equal(res.recovered, false, "bukan pemulihan -- ini kondisi normal");
    assert.ok(!readdirSync(dir).some((f) => f.includes("broken")), "tidak ada karantina");
    assert.doesNotMatch(msg(), /karantina|rusak/i, "tidak boleh ada report rusak");
  });

  it("creds.json belum-pair lalu TERISI me tapi belum registered = bukan korup", async () => {
    // sisa pairing yang gagal: punya me/pairingCode tapi registered false
    put(
      "creds.json",
      JSON.stringify({ me: { id: "6281:1@s.whatsapp.net" }, registered: false }),
    );
    const res = await loadAuthStateWithRecovery(dir, (m) => logs.push(m));
    assert.equal(res.recovered, false);
    assert.ok(!readdirSync(dir).some((f) => f.includes("broken")));
  });

  it("tidak ada creds.json sama sekali = fresh install, bukan korup", async () => {
    const res = await loadAuthStateWithRecovery(dir, (m) => logs.push(m));
    assert.equal(res.recovered, false);
    assert.ok(res.state.creds, "identitas baru harus difabrikasi");
    assert.ok(!readdirSync(dir).some((f) => f.includes("broken")));
  });

  it("file kosong (0 byte) diperlakukan korup, bukan 'belum pair'", async () => {
    // write yang terpotong bisa menghasilkan ini
    put("creds.json", "");
    const res = await loadAuthStateWithRecovery(dir, (m) => logs.push(m));
    assert.equal(res.recovered, true, "0 byte bukan state kosong yang sah");
    assert.ok(res.state.creds);
  });
});

/* ------------------------------------------------------------------ */
/* A4c - kegagalan lain tidak boleh salah dikarantina                  */
/* ------------------------------------------------------------------ */

describe("kegagalan yang bukan creds.json tidak boleh dikarantina", () => {
  it("folder sesi bukan direktori -> lempar, jangan pura-pura self-heal", async () => {
    const d = mkdtempSync(path.join(tmpdir(), "a4-y-"));
    const f = path.join(d, "notadir");
    writeFileSync(f, "x");
    await assert.rejects(
      () => loadAuthStateWithRecovery(f, (m) => logs.push(m)),
      /directory|tidak bisa|is not/i,
      "harus menolak, bukan mengarang pemulihan",
    );
    rmSync(d, { recursive: true, force: true });
  });

  it("classify hanya mau 'corrupt' kalau fileNYA ada", () => {
    const missing = classifyAuthStateFailure(new Error("failed to read auth state file \"/x/creds.json\": EACCES"), {
      credsExists: false,
      credsPath: "/x/creds.json",
    });
    assert.equal(missing.kind, "other", "tidak boleh dikarantina tanpa bukti file ada");
  });

  it("classify 'corrupt' kalau creds.json ada dan errornya tentang file itu", () => {
    const kind = classifyAuthStateFailure(
      new Error('auth state file "/x/creds.json" parsed but is not a usable credentials object'),
      { credsExists: true, credsPath: "/x/creds.json" },
    );
    assert.equal(kind.kind, "corrupt");
  });

  it("classify 'other' untuk error yang tidak menyebut creds.json", () => {
    const kind = classifyAuthStateFailure(new Error("found something that is not a directory"), {
      credsExists: true,
      credsPath: "/x/creds.json",
    });
    assert.equal(kind.kind, "other");
  });
});

/* ------------------------------------------------------------------ */
/* A4d - karantina tidak menumpuk tanpa batas                          */
/* ------------------------------------------------------------------ */

describe("quarantineBrokenCreds tidak menumpuk tanpa batas", () => {
  it("karantina lama dibuang hanya setelah batas, tidak lebih dulu", async () => {
    for (let i = 0; i < 8; i++) {
      put("creds.json", `{"broken":${i}`);
      await loadAuthStateWithRecovery(dir, () => {});
    }
    const backups = readdirSync(dir).filter((f) => /^creds\.json\.broken-/.test(f));
    assert.ok(backups.length > 0, "harus ada karantina");
    assert.ok(backups.length <= 5, `karantina melapuk: ${backups.length}`);
  });
});

/* ------------------------------------------------------------------ */
/* A4e - jalur TURSO (yang hidup di produksi) ikut terlindungi         */
/* ------------------------------------------------------------------ */

describe("useDurableAuthState (jalur Turso) juga self-heal", () => {
  it("creds.json rusak tidak lagi menggagalkan boot", async () => {
    const ts = await import("../src/lib/turso-session.js");
    ts.setRemoteSessionLogger((m) => logs.push(m));
    try {
      put("creds.json", '{"me":');
      put("pre-key-1.json", '{"a":"AQAB"}');
      const res = await ts.useDurableAuthState("main", dir);
      assert.ok(res.state.creds, "harus tetap punya creds");
      assert.equal(
        readdirSync(dir).filter((f) => /^creds\.json\.broken-/.test(f)).length,
        1,
        "karantina harus terjadi di jalur ini juga",
      );
    } finally {
      ts.setRemoteSessionLogger(null);
    }
  });

  it("key file lain tetap utuh dan terbaca setelah pemulihan", async () => {
    const ts = await import("../src/lib/turso-session.js");
    ts.setRemoteSessionLogger(() => {});
    try {
      put("creds.json", '{"me":');
      put("pre-key-1.json", '{"a":"AQAB"}');
      const res = await ts.useDurableAuthState("main", dir);
      const got = await res.state.keys.get("pre-key", ["1"]);
      assert.deepEqual(got["1"], { a: "AQAB" }, "pre-key lain tidak boleh ikut hilang");
    } finally {
      ts.setRemoteSessionLogger(null);
    }
  });

  it("creds belum-pair di jalur Turso TIDAK dikarantina", async () => {
    const ts = await import("../src/lib/turso-session.js");
    ts.setRemoteSessionLogger((m) => logs.push(m));
    try {
      put("creds.json", JSON.stringify({ noiseKey: 0.5, registered: false }));
      await ts.useDurableAuthState("main", dir);
      assert.ok(
        !readdirSync(dir).some((f) => f.includes("broken")),
        "pairing yang belum selesai bukan korup",
      );
      assert.doesNotMatch(msg(), /karantina|tidak bisa dipakai/i);
    } finally {
      ts.setRemoteSessionLogger(null);
    }
  });
});
