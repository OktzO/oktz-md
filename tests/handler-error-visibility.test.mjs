import { describe, it } from "node:test";
import assert from "node:assert";
import { readFile } from "node:fs/promises";
import { makeEventBuffer } from "onigis";
import pino from "pino";
import {
  createEventErrorReporter,
  attachEventErrorReporter,
  createLibraryLogger,
} from "../src/connection.js";

const CONNECTION = new URL("../src/connection.js", import.meta.url).pathname;
const tick = (ms = 30) => new Promise((r) => setTimeout(r, ms));

function collector() {
  const seen = [];
  const report = (err, events) => seen.push({ err, events });
  return { seen, report };
}

/* ------------------------------------------------------------------ */
/* A1a — break-nya: handler gagal, tidak ada yang melihat               */
/* ------------------------------------------------------------------ */

describe("event buffer: kegagalan handler tanpa listener = tak terlihat", () => {
  it("rejection TIDAK lagi jadi unhandledRejection (tidak bisa di-catching)", async () => {
    let unhandled = 0;
    const onUnhandled = () => unhandled++;
    process.on("unhandledRejection", onUnhandled);
    try {
      const ev = makeEventBuffer(pino({ level: "silent" }));
      ev.on("groups.update", async () => {
        throw new Error("jadibot-manager body blew up");
      });
      ev.emit("event", { "groups.update": [{ id: "x@g.us" }] });
      await tick();
      assert.equal(unhandled, 0, "contract berubah: tidak ada unhandledRejection");
    } finally {
      process.off("unhandledRejection", onUnhandled);
    }
  });

  it("rejection tidak sampai ke logger library yang silent", async () => {
    const lines = [];
    const ev = makeEventBuffer(
      pino({ level: "silent" }, { write: (s) => lines.push(s) }),
    );
    ev.on("messages.upsert", async () => {
      throw new Error("boom");
    });
    ev.emit("event", { "messages.upsert": { messages: [], type: "notify" } });
    await tick();
    assert.equal(lines.length, 0, "logger silent membuang semuanya");
  });
});

/* ------------------------------------------------------------------ */
/* A1b — fix: listener error channel reinstated                        */
/* ------------------------------------------------------------------ */

describe("attachEventErrorReporter: kegagalan handler sampai ke reporter bot", () => {
  it("rejection async dilaporkan lengkap dengan nama event", async () => {
    const { seen, report } = collector();
    const ev = makeEventBuffer(pino({ level: "silent" }));
    attachEventErrorReporter(ev, report);
    ev.on("connection.update", async () => {
      throw new Error("jadibot-manager body blew up");
    });
    ev.emit("event", { "connection.update": { connection: "open" } });
    await tick();

    assert.equal(seen.length, 1, "harus ada tepat satu laporan");
    assert.equal(seen[0].err.message, "jadibot-manager body blew up");
    assert.deepEqual(seen[0].events, ["connection.update"]);
  });

  it("throw sinkron dari destructure juga dilaporkan", async () => {
    const { seen, report } = collector();
    const ev = makeEventBuffer(pino({ level: "silent" }));
    attachEventErrorReporter(ev, report);
    // bentuk listener yang dipakai connection.js: async ([event]) => ...
    ev.on("groups.update", async ([event]) => {
      void event;
    });
    ev.emit("event", { "groups.update": undefined });
    await tick();

    assert.equal(seen.length, 1, "TypeError dari destructure harus tertangkap");
    assert.ok(seen[0].err instanceof TypeError);
    assert.deepEqual(seen[0].events, ["groups.update"]);
  });

  it("beberapa handler gagal = semua dilaporkan", async () => {
    const { seen, report } = collector();
    const ev = makeEventBuffer(pino({ level: "silent" }));
    attachEventErrorReporter(ev, report);
    ev.on("chats.upsert", async () => {
      throw new Error("a");
    });
    ev.on("groups.update", async () => {
      throw new Error("b");
    });
    ev.emit("event", {
      "chats.upsert": [],
      "groups.update": [{ id: "x@g.us" }],
    });
    await tick();
    assert.equal(seen.length, 2);
  });

  it("reporter yang melempar tidak menjatuhkan laporan lain", async () => {
    const ev = makeEventBuffer(pino({ level: "silent" }));
    const seen = [];
    attachEventErrorReporter(ev, () => {
      throw new Error("reporter bot ikut gagal");
    });
    ev.on("chats.upsert", async () => {
      throw new Error("a");
    });
    ev.on("groups.update", async () => {
      throw new Error("b");
    });
    ev.emit("event", { "chats.upsert": [], "groups.update": [{ id: "x" }] });
    await tick();
    // process harus masih hidup; Attach tidak boleh melempar keluar
    assert.ok(true, "tidak ada yang terlempar keluar ke pemanggil");
    assert.equal(seen.length, 0);
  });

  it("reporter default (tanpa report) tidak melempar", () => {
    assert.doesNotThrow(() => createEventErrorReporter()());
  });

  it("laporan tidak memuat isi event -- hanya nama eventnya", async () => {
    const { seen, report } = collector();
    const ev = makeEventBuffer(pino({ level: "silent" }));
    attachEventErrorReporter(ev, report);
    ev.on("chats.upsert", async () => {
      throw new Error("x");
    });
    ev.emit("event", {
      "chats.upsert": [{ id: "secret-group@g.us", unreadCount: 3 }],
    });
    await tick();
    const payload = JSON.stringify(seen[0].events);
    assert.ok(!payload.includes("secret-group"), "isi event tidak boleh bocor");
  });
});

/* ------------------------------------------------------------------ */
/* A1c — logger library tidak boleh membuang level error               */
/* ------------------------------------------------------------------ */

describe("createLibraryLogger: level error tidak lagi dibuang", () => {
  it("level bukan silent, dan error sampai ke reporter bot", () => {
    const seen = [];
    const log = createLibraryLogger((level, text) => seen.push({ level, text }));
    assert.notEqual(log.level, "silent", "logger silent = channel error sunyi");
    assert.equal(log.level, "error", "ambang harus error");

    log.error({ err: new Error("x") }, "library gagal");
    assert.equal(seen.length, 1, "logger.error harus sampai ke reporter");
    assert.equal(seen[0].level, "error");
    assert.match(seen[0].text, /library gagal/);
  });

  it("level lain tetap dibuang (tidak membanjiri console)", () => {
    const seen = [];
    const log = createLibraryLogger((level, text) => seen.push({ level, text }));
    for (const lvl of ["trace", "debug", "info", "warn"]) log[lvl](" chatter ");
    assert.equal(seen.length, 0, "protocol chatter harus tetap bisu");
  });

  it("error yang menyebut 'session'/'prekey' TIDAK boleh disaring", () => {
    const seen = [];
    const log = createLibraryLogger((level, text) => seen.push({ level, text }));
    // hook lama menyaring setiap pesan yang mengandung kata-kata ini, di
    // level apa pun termasuk error -- itu lubang yang bikin channel ini bisu.
    log.error({ err: new Error("e") }, "Failed to read session prekey file");
    assert.equal(seen.length, 1, "error yang menyebut session/prekey ikut hilang");
  });

  it("error dengan objek err tanpa pesan tetap dilaporkan", () => {
    const seen = [];
    const log = createLibraryLogger((level, text) => seen.push({ level, text }));
    log.error({ err: "string error" }, "tanpa objek Error");
    assert.equal(seen.length, 1);
    assert.match(seen[0].text, /tanpa objek Error/);
  });

  // `onigis/lib/Socket/socket.js:632` melaporkan kegagalan keep-alive dengan
  // logger.error({ trace: err.stack }, ...) — bukan `err`. Itu satu-satunya
  // call site error di library yang memakai `trace`, jadi kalau parse hanya
  // baca `err` penyebabnya hilang dan yang tersisa di console cuma nama
  // gejalanya.
  it("penyebab dari error yang dilacak lewat `trace` ikut sampai", () => {
    const seen = [];
    const log = createLibraryLogger((level, text) => seen.push({ level, text }));
    const cause = new Error("Connection Closed");
    log.error({ trace: cause.stack }, "error in sending keep alive");
    assert.equal(seen.length, 1);
    assert.match(seen[0].text, /error in sending keep alive/);
    assert.match(
      seen[0].text,
      /Connection Closed/,
      "penyebab keep-alive gagal harus ikut tercetak, bukan hilang",
    );
  });

  it("trace yang bukan string tidak boleh membuat logger melempar", () => {
    const seen = [];
    const log = createLibraryLogger((level, text) => seen.push({ level, text }));
    for (const trace of [{ stack: "x" }, ["a", "b"], 42, null]) {
      log.error({ trace }, "pesan tetap sampai");
    }
    assert.equal(seen.length, 4);
    assert.match(seen[0].text, /pesan tetap sampai/);
  });

  it("err menang saat library mengirim keduanya", () => {
    const seen = [];
    const log = createLibraryLogger((level, text) => seen.push({ level, text }));
    log.error(
      { err: new Error("alasan sebenarnya"), trace: "Error: lain" },
      "gagal",
    );
    assert.match(seen[0].text, /alasan sebenarnya/);
    assert.doesNotMatch(seen[0].text, /lain/);
  });

  // Sisanya call site library yang membawa error di key selain `err`/`trace`.
  // Semuanya lewat reporter yang sama, jadi kalau key itu tidak dibaca,
  // operator tetapondi lini merah tanpa sebab untuk event yang sama.
  it("penyebab ikut sampai untuk key error lain yang dipakai library", () => {
    const cases = [
      [{ error: new Error("pre-key check gagal") }, /pre-key check gagal/],
      [{ error: "upload ditolak server" }, /upload ditolak server/],
      [{ ackErr: new Error("ack receipt gagal") }, /ack receipt gagal/],
      [{ uploadError: "kuota habis", count: 3 }, /kuota habis/],
    ];
    for (const [payload, expected] of cases) {
      const seen = [];
      const log = createLibraryLogger((level, text) => seen.push({ level, text }));
      log.error(payload, "library gagal");
      assert.equal(seen.length, 1);
      assert.match(seen[0].text, expected, `payload ${JSON.stringify(Object.keys(payload))}`);
    }
  });

  it("err tetap menang atas key lain di record yang sama", () => {
    const seen = [];
    const log = createLibraryLogger((level, text) => seen.push({ level, text }));
    log.error(
      { error: new Error("kurang penting"), err: new Error("penyebab utama") },
      "gagal",
    );
    assert.match(seen[0].text, /penyebab utama/);
    assert.doesNotMatch(seen[0].text, /kurang penting/);
  });

  it("payload error yang tidak punya teks tetap tidak merusak baris log", () => {
    const seen = [];
    const log = createLibraryLogger((level, text) => seen.push({ level, text }));
    log.error({ error: { node: "x" } }, "gagal");
    assert.equal(seen.length, 1);
    assert.match(seen[0].text, /gagal/);
  });
});

/* ------------------------------------------------------------------ */
/* A1d - wiring di socket sungguhan                                    */
/* ------------------------------------------------------------------ */

describe("startConnection memasang reporter di socket sungguhan", () => {
  // Komentar harus dibuang dulu: `// attachEventErrorReporter(sock.ev)`
  // akan tetap cocok kalau matcher tidak tahu bedanya.
  const stripComments = (src) =>
    src.replace(/\/\*[\s\S]*?\*\//g, "").replace(/^[ \t]*\/\/.*$/gm, "");

  it("ada listener error yang di-attach ke ev (bukan di-comment)", async () => {
    const src = stripComments(await readFile(CONNECTION, "utf8"));
    assert.match(
      src,
      /attachEventErrorReporter\(\s*sock\.ev\s*\)/,
      "startConnection harus attach reporter ke sock.ev",
    );
    assert.doesNotMatch(
      src,
      /\/\/\s*attachEventErrorReporter/,
      "panggilan attach harus benar-benar dieksekusi",
    );
  });

  it("logger yang diteruskan ke makeWASocket bukan level silent", async () => {
    const src = stripComments(await readFile(CONNECTION, "utf8"));
    assert.doesNotMatch(src, /level:\s*"silent"/);

    const idx = src.indexOf("const sock = makeWASocket(");
    assert.ok(idx > -1, "makeWASocket tidak ditemukan");
    const call = src.slice(idx, idx + 300);
    assert.match(
      call,
      /logger:\s*libraryLogger/,
      "makeWASocket harus menerima logger level error",
    );
    assert.match(
      call,
      /makeCacheableSignalKeyStore\(\s*state\.keys,\s*libraryLogger\s*\)/,
      "key store juga harus dapat logger level error",
    );
  });

  it("reporter default menaruh pesan error + nama event di log operator", () => {
    const lines = [];
    const realLog = console.log;
    console.log = (...a) => lines.push(a.join(" "));
    try {
      createEventErrorReporter()(
        new Error("jadibot-manager body blew up"),
        ["connection.update"],
      );
    } finally {
      console.log = realLog;
    }
    assert.equal(lines.length, 1, "harus satu baris log");
    assert.match(lines[0], /connection\.update/, "nama event harus terlihat");
    assert.match(
      lines[0],
      /jadibot-manager body blew up/,
      "pesan error asli harus terlihat, bukan placeholder",
    );
  });

  it("reporter default tetap jalan tanpa nama event", () => {
    const lines = [];
    const realLog = console.log;
    console.log = (...a) => lines.push(a.join(" "));
    try {
      createEventErrorReporter()(new Error("tanpa konteks"));
      createEventErrorReporter()("string error", undefined);
    } finally {
      console.log = realLog;
    }
    assert.equal(lines.length, 2);
    assert.match(lines[1], /string error/);
  });
});
