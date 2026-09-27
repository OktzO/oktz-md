import { describe, it, mock, after } from "node:test";
import assert from "node:assert/strict";

// Database asli (lowdb + Turso produksi) tidak boleh disentuh tes ini, jadi
// modulnya di-stub sebelum plugin diimpor.
const dbStub = {
  group: { anticustom: "on", anticustomRules: [] },
  writes: [],
  getGroup() {
    return this.group;
  },
  setGroup(chat, patch) {
    this.writes.push({ chat, patch });
    Object.assign(this.group, patch);
  },
  setting: () => false,
};

mock.module("../src/lib/database.js", {
  namedExports: { getDatabase: () => dbStub },
});

const { handler, replyHandler, config } = await import(
  "../plugins/group/anticustom.js"
);

const CHAT = "120363000000000000@g.us";


// Sesi wizard disimpan di global dan sengaja tidak dibuang kalau pola ditolak
// (admin boleh lanjut), jadi tiap tes pakai pengirim sendiri. Timeout 10 menit
// di plugin juga harus dibersihkan supaya proses tes tidak menggantung.
after(() => {
  const sessions = globalThis.anticustomSessions;
  if (!sessions) return;
  for (const session of sessions.values()) {
    if (session?.timeout) clearTimeout(session.timeout);
  }
  sessions.clear();
});

let wigCount = 0;

function makeWig() {
  wigCount += 1;
  const prompts = [];
  const replies = [];
  const state = { prompts, replies };

  const sock = {
    async sendMessage(chat, content) {
      prompts.push(content.text);
      return { key: { id: `P${prompts.length}` } };
    },
  };

  const m = {
    chat: CHAT,
    sender: `628555${String(wigCount).padStart(6, "0")}@s.whatsapp.net`,
    isCommand: false,
    prefix: ".",
    body: "",
    quoted: null,
    args: ["add"],
    async reply(text) {
      replies.push(text);
    },
  };

  // Balas pertanyaan terakhir bot, persis seperti yang dilakukan user.
  const answer = async (text) => {
    m.quoted = prompts.length ? { id: `P${prompts.length}` } : null;
    m.body = text;
    return replyHandler(m, { sock });
  };

  return { m, sock, prompts, replies, answer };
}

const containsRule = (name) => ({
  name,
  groupName: name,
  pattern: name,
  type: "contains",
  action: "remove",
  flags: "i",
});

describe("AntiCustom — wizard instalasi (B4 ReDoS)", () => {
  it("pola berbahaya ditolak di wizard dan tidak pernah disimpan", async () => {
    dbStub.group = { anticustom: "on", anticustomRules: [] };
    dbStub.writes = [];
    const wig = makeWig();

    await handler(wig.m, { sock: wig.sock });
    assert.equal(wig.prompts.length, 1, "wizard harus mulai dengan pertanyaan judul");

    await wig.answer("Anti Kotor");
    assert.equal(wig.prompts.length, 2);

    const handled = await wig.answer("regex: (a+)+$");
    assert.equal(handled, true);
    assert.match(wig.replies.at(-1), /ditolak/i);
    assert.match(wig.replies.at(-1), /kuantifier/i);
    assert.equal(
      wig.prompts.length,
      2,
      "pola ditolak tidak boleh lanjut ke pertanyaan berikutnya",
    );
    assert.equal(dbStub.writes.length, 0, "tidak boleh ada penulisan ke database");
  });

  it("rule contains biasa tetap bisa dipasang lewat wizard", async () => {
    dbStub.group = { anticustom: "on", anticustomRules: [] };
    dbStub.writes = [];
    const wig = makeWig();

    await handler(wig.m, { sock: wig.sock });
    await wig.answer("Anti Kotor");
    await wig.answer("anjing, goblok, tolol");
    await wig.answer("hapus");
    await wig.answer("ya");

    assert.equal(dbStub.writes.length, 1);
    const saved = dbStub.writes[0].patch.anticustomRules;
    assert.equal(saved.length, 3);
    assert.deepEqual(
      saved.map((rule) => rule.pattern),
      ["anjing", "goblok", "tolol"],
    );
    assert.equal(dbStub.writes[0].patch.anticustom, "on");
  });

  it("regex aman tetap bisa dipasang lewat wizard", async () => {
    dbStub.group = { anticustom: "on", anticustomRules: [] };
    dbStub.writes = [];
    const wig = makeWig();

    await handler(wig.m, { sock: wig.sock });
    await wig.answer("Anti Kata");
    await wig.answer("regex: (anj|anjing|a+n+j+)");
    await wig.answer("kick");
    await wig.answer("ya");

    assert.equal(dbStub.writes.length, 1);
    const saved = dbStub.writes[0].patch.anticustomRules;
    assert.equal(saved[0].type, "regex");
    assert.equal(saved[0].pattern, "(anj|anjing|a+n+j+)");
    assert.equal(saved[0].action, "kick");
  });

  it("admin diberi tahu saat rule melewati batas per grup, bukan diam-diam diabaikan", async () => {
    // Grup sudah penuh: setiap rule di luar cap tidak akan pernah dievaluasi
    // per pesan, jadi lebih baik bilang sekarang daripada diam-diam.
    const { CUSTOM_MAX_RULES } = await import("../src/lib/group-protection.js");
    dbStub.group = {
      anticustom: "on",
      anticustomRules: Array.from({ length: CUSTOM_MAX_RULES }, (_, i) =>
        containsRule(`lama-${i}`),
      ),
    };
    dbStub.writes = [];
    const wig = makeWig();

    await handler(wig.m, { sock: wig.sock });
    await wig.answer("Anti Baru");
    await wig.answer("anjing");
    await wig.answer("hapus");
    const handled = await wig.answer("ya");

    assert.equal(handled, true);
    assert.match(wig.replies.at(-1), /batas|terlalu banyak/i);
    assert.equal(dbStub.writes.length, 0, "rule baru tidak boleh disimpan");
    assert.equal(dbStub.group.anticustomRules.length, CUSTOM_MAX_RULES);
  });

  it("admin boleh tetap menambah selama belum melewati batas", async () => {
    const { CUSTOM_MAX_RULES } = await import("../src/lib/group-protection.js");
    dbStub.group = {
      anticustom: "on",
      anticustomRules: Array.from({ length: CUSTOM_MAX_RULES - 1 }, (_, i) =>
        containsRule(`lama-${i}`),
      ),
    };
    dbStub.writes = [];
    const wig = makeWig();

    await handler(wig.m, { sock: wig.sock });
    await wig.answer("Anti Baru");
    await wig.answer("anjing");
    await wig.answer("hapus");
    await wig.answer("ya");

    assert.equal(dbStub.writes.length, 1);
    assert.equal(dbStub.writes[0].patch.anticustomRules.length, CUSTOM_MAX_RULES);
  });

  it("kategori group + isAdmin tetap seperti sebelumnya (tidak terkait patch ini)", () => {
    assert.equal(config.category, "group");
    assert.equal(config.isAdmin, true);
  });
});
