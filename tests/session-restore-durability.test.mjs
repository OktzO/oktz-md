// Pemulihan sesi dari Turso tidak benar-benar "dipulihkan": yang dipindah hanya
// objek creds di memori. `useDurableAuthState` memakai objek creds milik
// `useMultiFileAuthState` untuk menulis storage lokal, dan objek itu tidak pernah
// disentuh saat pemulihan terjadi. Akibatnya:
//
//   - storage/session/creds.json tetap `registered: false` selamanya, jadi
//     looksPaired salah setiap boot dan pemulihan diulang terus;
//   - "durable" hanya durable kalau Turso hidup. Token mati = identitas hilang;
//   - log boot mengsays "sesi dipulihkan dari Turso ke storage lokal" tanpa
//     satu byte pun yang ditulis ke storage lokal.
//
// State itu persis yang menghasilkan ratusan "failed to decrypt message":
// session_creds ada (bot connect, kelihatan paired) tapi session_keys kosong,
// jadi tiap pesan miss. Tidak ada kode yang pernah memeriksa kondisi itu, dan
// pemulihannya mustahil tanpa pair ulang.
import { describe, it, after } from "node:test";
import assert from "node:assert";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";

const memTurso = async () => {
  const turso = await import("../src/lib/turso.js");
  const client = await turso.createTursoClient({ enabled: true, url: "file::memory:" });
  await turso.initTursoTables(client);
  return client;
};

const seedRemoteCreds = async (client, scope, creds) => {
  await client.execute({
    sql: "INSERT INTO session_creds (scope, creds, updated_at) VALUES (?, ?, ?)",
    args: [scope, JSON.stringify(creds), 1],
  });
};

const PAIRED = {
  registered: true,
  me: { id: "628@s.whatsapp.net" },
  registrationId: 7,
};

const tmpDirs = [];
const mkSessionDir = () => {
  const d = fs.mkdtempSync(path.join(os.tmpdir(), "restore-durable-"));
  tmpDirs.push(d);
  return d;
};

after(() => {
  for (const d of tmpDirs) {
    try { fs.chmodSync(d, 0o700); } catch { }
    fs.rmSync(d, { recursive: true, force: true });
  }
});

describe("pemulihan creds dari Turso harus benar-benar mendarat di storage lokal", () => {
  it("menulis creds yang dipulihkan ke creds.json di folder sesi", async () => {
    const client = await memTurso();
    await seedRemoteCreds(client, "restore-disk", PAIRED);
    const dir = mkSessionDir();

    const { useDurableAuthState } = await import("../src/lib/turso-session.js");
    await useDurableAuthState("restore-disk", dir);

    const credsPath = path.join(dir, "creds.json");
    assert.ok(
      fs.existsSync(credsPath),
      "creds.json tidak dibuat sama sekali saat sesi dipulihkan: yang dipulihkan " +
        "hanya ingatan di proses, jadi container berikutnya mulai dari nol",
    );
    const onDisk = JSON.parse(fs.readFileSync(credsPath, "utf8"));
    assert.equal(
      onDisk.registered,
      true,
      "creds.json lokal masih menyimpan stub registered:false walau sesi dipulihkan",
    );
    assert.equal(onDisk.me?.id, "628@s.whatsapp.net");
  });

  it("objek creds yang dipulihkan adalah objek yang dipersistensi store lokal", async () => {
    const client = await memTurso();
    await seedRemoteCreds(client, "restore-obj", PAIRED);
    const dir = mkSessionDir();

    const { useDurableAuthState } = await import("../src/lib/turso-session.js");
    const res = await useDurableAuthState("restore-obj", dir);

    // Ini yang dilakukan library: auth.creds dimutasi in-place, lalu
    // event 'creds.update' memanggil saveCreds.
    res.state.creds.registrationId = 4242;
    await res.saveCreds();

    const onDisk = JSON.parse(fs.readFileSync(path.join(dir, "creds.json"), "utf8"));
    assert.equal(
      onDisk.registrationId,
      4242,
      "mutasi dari library tidak pernah masuk ke creds.json: store lokal menulis " +
        "objek creds yang berbeda, jadi setiap save menimpa disk dengan stub lama",
    );

    const mirror = await client.execute({
      sql: "SELECT creds FROM session_creds WHERE scope = ?",
      args: ["restore-obj"],
    });
    assert.equal(
      JSON.parse(mirror.rows[0].creds).registrationId,
      4242,
      "cermin Turso harus ikut nilai yang sama",
    );
  });

  it("boot kedua tetap punya sesi lokal walau Turso mati total", async () => {
    const client = await memTurso();
    await seedRemoteCreds(client, "restart", PAIRED);
    const dir = mkSessionDir();

    const { useDurableAuthState } = await import("../src/lib/turso-session.js");
    await useDurableAuthState("restart", dir);

    // Bot mati, Turso tidak bisa dijangkau (token mati / database tidak ada).
    const turso = await import("../src/lib/turso.js");
    await turso.closeTurso();

    const messages = [];
    const { setRemoteSessionLogger } = await import("../src/lib/turso-session.js");
    setRemoteSessionLogger((m) => messages.push(m));
    let res;
    try {
      res = await useDurableAuthState("restart", dir);
    } finally {
      setRemoteSessionLogger(null);
    }

    assert.equal(
      res.state.creds.registered,
      true,
      "Turso mati dan storage lokal tidak pernah diperbaiki, jadi identitas hilang " +
        "dan bot perlu pair ulang setiap kali Turso tidak bisa dijangkau",
    );
  });
});

describe("kondisi creds-ada-tapi-keys-nol harus dilaporkan", () => {
  it("memberi tahu saat creds paired tapi tidak ada satu pun key di disk atau Turso", async () => {
    const client = await memTurso();
    await seedRemoteCreds(client, "hollow", PAIRED);
    const dir = mkSessionDir();

    const messages = [];
    const { useDurableAuthState, setRemoteSessionLogger } = await import(
      "../src/lib/turso-session.js"
    );
    setRemoteSessionLogger((m) => messages.push(m));
    let res;
    try {
      res = await useDurableAuthState("hollow", dir);
    } finally {
      setRemoteSessionLogger(null);
    }

    assert.equal(res.state.creds.registered, true);
    assert.match(
      messages.join("\n"),
      /pair ulang|re-pair|scan QR/i,
      "creds paired tanpa key sama sekali = tiap pesan akan gagal decrypt. " +
        "Kondisi ini tidak bisa pulih sendiri dan harus disebut eksplisit, " +
        "bukan dibiarkan jadi ratusan error decrypt yang tidak ada artinya",
    );
  });

  it("diam saat key ada, jadi laporan tidak jadi noise", async () => {
    const client = await memTurso();
    await seedRemoteCreds(client, "healthy", PAIRED);
    await client.execute({
      sql: "INSERT INTO session_keys (scope, category, id, data, updated_at) VALUES (?, ?, ?, ?, ?)",
      args: ["healthy", "pre-key", "1", JSON.stringify({ private: "11", public: "22" }), 1],
    });
    const dir = mkSessionDir();

    const messages = [];
    const { useDurableAuthState, setRemoteSessionLogger } = await import(
      "../src/lib/turso-session.js"
    );
    setRemoteSessionLogger((m) => messages.push(m));
    try {
      await useDurableAuthState("healthy", dir);
    } finally {
      setRemoteSessionLogger(null);
    }

    assert.doesNotMatch(
      messages.join("\n"),
      /pair ulang|re-pair|scan QR/i,
      "key ada, jadi tidak ada yang perlu diperbaiki — jangan alarm yang tidak useful",
    );
  });

  it("diam saat memang belum pair, karena itu kondisi normal pairing baru", async () => {
    await memTurso();
    const dir = mkSessionDir();

    const messages = [];
    const { useDurableAuthState, setRemoteSessionLogger } = await import(
      "../src/lib/turso-session.js"
    );
    setRemoteSessionLogger((m) => messages.push(m));
    try {
      await useDurableAuthState("brand-new", dir);
    } finally {
      setRemoteSessionLogger(null);
    }

    assert.doesNotMatch(
      messages.join("\n"),
      /pair ulang|re-pair|scan QR/i,
      "belum pair + tidak ada key adalah NORMAL, bukan kerosokan",
    );
  });
});

describe("kegagalan tulis lokal tidak boleh mematikan cermin Turso", () => {
  it("masih menulis ke Turso saat storage lokal tidak bisa ditulis", async () => {
    const client = await memTurso();
    await seedRemoteCreds(client, "readonly", PAIRED);
    const dir = mkSessionDir();

    const { useDurableAuthState } = await import("../src/lib/turso-session.js");
    const { state } = await useDurableAuthState("readonly", dir);

    // Folder sesi jadi read-only: baca tetap jalan, tulis lempar EACCES.
    fs.chmodSync(dir, 0o500);
    try {
      await state.keys.set({
        "pre-key": { 3: { private: Buffer.from("11", "hex"), public: Buffer.from("22", "hex") } },
      });

      const rows = await client.execute({
        sql: "SELECT COUNT(*) AS n FROM session_keys WHERE scope = ? AND category = ?",
        args: ["readonly", "pre-key"],
      });
      assert.equal(
        rows.rows[0].n,
        1,
        "keys.set kembali begitu saja saat tulis lokal gagal, jadi tidak ada key " +
          "yang pernah sampai ke Turso: pemulihannya mustahil begitu folder sesi " +
          "tidak bisa ditulis",
      );
    } finally {
      fs.chmodSync(dir, 0o700);
    }
  });
});