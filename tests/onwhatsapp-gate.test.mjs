// Gate "nomor benar-benar ada di WhatsApp" harus FAIL-CLOSED.
//
//=onigis (socket.js) `onWhatsApp` tidak lagi `resolve(undefined)` saat query
// timeout — dia THROW (`Timed Out`, statusCode=408). Dua plugin membungkus
// panggilan itu dengan `catch (e) {}`:
//
//   plugins/panel/cadmin.js  → provisioning admin Pterodactyl (root_admin!)
//   plugins/fun/confess.js   → kirim pesan anonim ke nomor tujuan
//
// Kalau throw ditelan, eksekusi JATUH KE BAWAH gate: nomor yang tidak pernah
// diverifikasi tetap lolos. Test ini memakai socket stub (tidak jaringan
// WhatsApp sungguhan) + panel Pterodactyl palsu di 127.0.0.1 supaya
// "tidak provisioned" bisa dibuktikan dari hit HTTP, bukan dari regex.
import { describe, it, before, after } from "node:test";
import assert from "node:assert";
import http from "node:http";
import { readFile } from "node:fs/promises";
import path from "node:path";

const REPO = new URL("..", import.meta.url).pathname;
const TIMEOUT_ERR = Object.assign(new Error("Timed Out"), { statusCode: 408 });

function fakeM(over = {}) {
  const replies = [];
  return {
    replies,
    command: "cadminv1",
    prefix: ".",
    isOwner: true,
    pushName: "Owner",
    sender: "6285608953677@s.whatsapp.net",
    text: "adminku,6281234567890",
    mentionedJid: [],
    quoted: null,
    reply(text) {
      replies.push(String(text));
      return Promise.resolve();
    },
    ...over,
  };
}

function fakeSock(onWhatsApp) {
  const sent = [];
  return {
    sent,
    onWhatsApp,
    async sendMessage(jid, content) {
      sent.push({ jid, content });
      return { key: { id: "fake-" + sent.length } };
    },
  };
}

// sock.onWhatsApp dalam berbagai kondisi nyata:
// - throw        → query timeout / error transport (kontak TIDAK diketahui)
// - exists:null  → usync tidak menjawab baris itu (kontak TIDAK diketahui)
// - exists:false → dijawab, tapi bukan kontak
// - exists:true  → dijawab, memang kontak
const WA_STATES = {
  throwTimeout: () => {
    throw TIMEOUT_ERR;
  },
  throwGeneric: () => {
    throw new Error("socket closed");
  },
  undetermined: async () => [{ jid: "6281234567890", exists: null }],
  unknown: async () => [],
  notOnWa: async () => [{ jid: "6281234567890", exists: false }],
  onWa: async () => [{ jid: "6281234567890", exists: true }],
};

describe("cadmin — gate onWhatsApp harus fail-closed", () => {
  let panel;
  let panelUrl;
  let panelHits;
  let cadmin;
  let config;
  const originalServer1 = {};

  before(async () => {
    panelHits = [];
    panel = http.createServer((req, res) => {
      panelHits.push({ method: req.method, url: req.url });
      let body = "";
      req.on("data", (c) => (body += c));
      req.on("end", () => {
        res.writeHead(200, { "content-type": "application/json" });
        res.end(
          JSON.stringify({
            object: "user",
            attributes: { id: 42, username: "adminku" },
          }),
        );
      });
    });
    await new Promise((r) => panel.listen(0, "127.0.0.1", r));
    panelUrl = `http://127.0.0.1:${panel.address().port}`;

    cadmin = await import("../plugins/panel/cadmin.js");
    config = (await import("../config.js")).default;

    // Domain/apikey di config.js sengaja kosong di repo — isi dengan panel
    // lokal supaya gate onWhatsApp benar-benar bisa diuji. Nilai API key di
    // sini fixture test, bukan kredensial apa pun.
    originalServer1.domain = config.pterodactyl.server1.domain;
    originalServer1.apikey = config.pterodactyl.server1.apikey;
    config.pterodactyl.server1.domain = panelUrl;
    config.pterodactyl.server1.apikey = "ptla-test-fixture";
  });

  after(async () => {
    config.pterodactyl.server1.domain = originalServer1.domain;
    config.pterodactyl.server1.apikey = originalServer1.apikey;
    await new Promise((r) => panel.close(r));
  });

  async function runCadmin(onWhatsApp) {
    panelHits.length = 0;
    const m = fakeM();
    const sock = fakeSock(onWhatsApp);
    await cadmin.handler(m, { sock });
    return { m, sock };
  }

  for (const [name, state] of [
    ["timeout usync (throw 408)", WA_STATES.throwTimeout],
    ["error transport (throw)", WA_STATES.throwGeneric],
    ["usync tidak menjawab (exists:null)", WA_STATES.undetermined],
    ["jawaban kosong ([])", WA_STATES.unknown],
  ]) {
    it(`tidak provisioning admin saat nomor tidak bisa diverifikasi — ${name}`, async () => {
      const { m, sock } = await runCadmin(state);

      assert.deepStrictEqual(
        panelHits,
        [],
        "tidak boleh ada request ke panel Pterodactyl untuk nomor yang belum terverifikasi",
      );
      assert.deepStrictEqual(
        sock.sent,
        [],
        "kredensial panel tidak boleh dikirim ke nomor yang belum terverifikasi",
      );
      assert.ok(m.replies.length > 0, "user harus diberi tahu alasannya");
      assert.match(
        m.replies.join("\n"),
        /gagal|tidak dapat|validasi|tidak terdaftar|dibatalkan|pastikan/i,
        `pesan penolakan harus menjelaskan gaganya — dapat: ${m.replies.join(" | ")}`,
      );
    });
  }

  it("tetap menolak nomor yang dijawab tidak terdaftar", async () => {
    const { m, sock } = await runCadmin(WA_STATES.notOnWa);
    assert.deepStrictEqual(panelHits, [], "nomor exists:false tidak boleh diprovisioning");
    assert.deepStrictEqual(sock.sent, []);
    assert.match(m.replies.join("\n"), /tidak terdaftar/i);
  });

  it("tetap memprovisioning kalau nomor benar-benar exists (regresi)", async () => {
    const { m, sock } = await runCadmin(WA_STATES.onWa);
    assert.strictEqual(panelHits.length, 1, "panel harus dipanggil 1x");
    assert.match(panelHits[0].url, /\/api\/application\/users$/);
    assert.strictEqual(sock.sent.length, 1, "detail admin dikirim ke target");
    assert.strictEqual(sock.sent[0].jid, "6281234567890@s.whatsapp.net");
    assert.match(m.replies.join("\n"), /Data telah dikirim/i);
  });
});

describe("confess — gate onWhatsApp harus fail-closed", () => {
  let confess;

  before(async () => {
    confess = await import("../plugins/fun/confess.js");
  });

  async function runConfess(onWhatsApp) {
    const m = fakeM({
      command: "confess",
      text: "6281234567890|halo kak ini pesan rahasia",
      fullArgs: "6281234567890|halo kak ini pesan rahasia",
    });
    const sock = fakeSock(onWhatsApp);
    await confess.handler(m, { sock });
    return { m, sock };
  }

  for (const [name, state] of [
    ["timeout usync (throw 408)", WA_STATES.throwTimeout],
    ["error transport (throw)", WA_STATES.throwGeneric],
    ["usync tidak menjawab (exists:null)", WA_STATES.undetermined],
    ["jawaban kosong ([])", WA_STATES.unknown],
  ]) {
    it(`tidak mengirim pesan anonim ke nomor yang tidak bisa diverifikasi — ${name}`, async () => {
      const { m, sock } = await runConfess(state);
      assert.deepStrictEqual(
        sock.sent,
        [],
        "pesan anonim tidak boleh terkirim ke nomor yang belum terverifikasi",
      );
      assert.ok(m.replies.length > 0, "user harus diberi tahu alasannya");
      assert.doesNotMatch(
        m.replies.join("\n"),
        /BERHASIL TERKIRIM/i,
        "tidak boleh dilaporkan sukses padahal tidak terkirim",
      );
    });
  }

  it("tetap menolak nomor exists:false", async () => {
    const { m, sock } = await runConfess(WA_STATES.notOnWa);
    assert.deepStrictEqual(sock.sent, []);
    assert.match(m.replies.join("\n"), /nggak terdaftar/i);
  });

  it("tetap mengirim kalau nomor benar-benar exists (regresi)", async () => {
    const { m, sock } = await runConfess(WA_STATES.onWa);
    assert.strictEqual(sock.sent.length, 1, "pesan harus terkirim 1x");
    assert.strictEqual(sock.sent[0].jid, "6281234567890@s.whatsapp.net");
    assert.match(m.replies.join("\n"), /BERHASIL TERKIRIM/i);
  });
});

describe("tidak ada plugin yang menelan error onWhatsApp lalu jatuh ke bawah gate", () => {
  for (const file of [
    "plugins/panel/cadmin.js",
    "plugins/panel/createserver.js",
    "plugins/fun/confess.js",
  ]) {
    it(`${file}: catch pemanggil onWhatsApp wajib menolak (return), bukan kosong`, async () => {
      const src = await readFile(path.join(REPO, file), "utf8");
      const idx = src.indexOf("onWhatsApp(");
      assert.ok(idx > -1, `${file} tidak memanggil onWhatsApp — gate hilang?`);
      // buang komentar supaya yang dicek benar-benar isi catch
      const window = src
        .slice(idx, idx + 600)
        .replace(/\/\*[\s\S]*?\*\//g, "")
        .replace(/\/\/[^\n]*/g, "");
      const catchIdx = window.indexOf("catch");
      assert.ok(catchIdx > -1, `${file}: catch harus ada (fail-closed)`);
      const body = window.slice(catchIdx, catchIdx + 200);
      assert.doesNotMatch(
        body,
        /catch\s*\([^)]*\)\s*\{\s*\}/,
        `${file}: catch kosong = fall through = provisioning/pengiriman ke nomor tak terverifikasi`,
      );
      assert.match(
        body,
        /catch\s*\([^)]*\)\s*\{\s*return\b/,
        `${file}: catch harus return (deny), bukan lanjut eksekusi`,
      );
    });
  }
});
