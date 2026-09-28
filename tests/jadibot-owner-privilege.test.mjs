// B1 — jadibot owner = full bot owner.
//
// Rantai yang dipakai:
//   1. .jadibot isPremium (plugins/main/jadibot.js) -> user BERBAYAR bisa
//      menyalakan sub-bot.
//   2. src/lib/jadibot-manager.js:addJadibotOwner(id, sender) -> pencipta
//      dicatat sebagai OWNER jadibot itu.
//   3. Pesan jadibot masuk ke router yang sama: messageHandler(msg, childSock,
//      { isJadibot: true, jadibotId: id }).
//   4. src/handler.js:738 -> m.isOwner = isJadibotOwner(...) = true.
//   5. src/handler.js:1132 -> `>>` masuk new AsyncFunction dengan
//      `await import('child_process')` di scope.
//
// Jadi `>> require('child_process').execSync('id').toString()` dari akun
// PREMIUM = RCE penuh di host operator, dengan .env terjangkau. Tidak ada
// exploit, tidak ada race, empat baris.
//
// Test ini mengunci batasnya: di dalam jadibot, premium tetap dapat, owner tidak.
import { describe, it, before } from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";

import { loadPlugins, getPlugin, getAllPlugins } from "../src/lib/plugins.js";
import {
  blockedJadibotCommand,
  isJadibotOwnerOnlyCommand,
  JADIBOT_BLOCKED_BODY_PREFIXES,
} from "../src/lib/jadibot-manager.js";

const REPO = new URL("..", import.meta.url).pathname;
const OWNER_JID = "628500000001@s.whatsapp.net";

const RCE_PAYLOAD = ">> require('child_process').execSync('id').toString()";

function chatMsg(text) {
  return {
    key: { remoteJid: OWNER_JID, id: "MSG1", fromMe: false },
    message: { conversation: text },
    messageTimestamp: Math.floor(Date.now() / 1000),
  };
}

before(async () => {
  // Test ini memuat SEMUA 829 plugin (satu-satunya cara membuktikan seluruh
  // direktori owner ikut terblokir, bukan cuma plugin yang disebut). Salah
  // satu plugin game menarik `src/lib/game-data.js`, yang di top level membuat
  // setInterval 5 menit TANPA .unref(): begitu test selesai, child process
  // runner tidak pernah keluar dan `npm test` menggantung selamanya.
  //
  // Test ini tidak butuh interval itu hidup, jadi timer yang dibuat selama
  // loadPlugins di-unref di sini. Perbaikan yang benar ada di game-data.js
  // (`.unref()` pada interval itu) — di luar cakupan tugas ini.
  const created = [];
  const origInterval = globalThis.setInterval;
  const origTimeout = globalThis.setTimeout;
  globalThis.setInterval = (...args) => {
    const t = origInterval(...args);
    created.push(t);
    return t;
  };
  globalThis.setTimeout = (...args) => {
    const t = origTimeout(...args);
    created.push(t);
    return t;
  };
  try {
    await loadPlugins(`${REPO}plugins`);
  } finally {
    globalThis.setInterval = origInterval;
    globalThis.setTimeout = origTimeout;
    for (const timer of created) timer.unref?.();
  }
});

describe("B1 — batas privileges jadibot owner", () => {
  it("plugin owner memang terdaftar sebagai owner-only (fixture test)", () => {
    assert.ok(getPlugin("exec"), "exec harus terdaftar");
    assert.equal(getPlugin("exec").config.isOwner, true);
    assert.equal(getPlugin("exec").config.category, "owner");
  });

  it("RCE: `>>` dengan child_process diblokir di dalam jadibot", () => {
    assert.equal(
      blockedJadibotCommand(chatMsg(RCE_PAYLOAD), { isJadibot: true }),
      "eval",
      "jalur eval owner tidak boleh bisa dijangkau dari jadibot",
    );
  });

  it("inspect `!!` diblokir juga (bocor data, bukan cuma RCE)", () => {
    assert.equal(
      blockedJadibotCommand(chatMsg("!! Object.keys(process.env)"), {
        isJadibot: true,
      }),
      "eval",
    );
  });

  it("daftar prefix yang diblokir memang yang dipakai handler.js", async () => {
    const handlerSrc = await readFile(`${REPO}src/handler.js`, "utf8");
    for (const prefix of JADIBOT_BLOCKED_BODY_PREFIXES) {
      assert.ok(
        handlerSrc.includes(`m.body?.startsWith("${prefix}")`),
        `handler.js tidak lagi memakai prefix ${prefix} — gate harus ikut`,
      );
    }
  });

  it("plugin owner (exec / ganticode) diblokir di dalam jadibot", () => {
    for (const text of [".exec rm -rf /", ".ganticode ping main", ".> "]) {
      assert.equal(
        blockedJadibotCommand(chatMsg(text), { isJadibot: true }),
        "owner",
        `${text} harus diblokir`,
      );
    }
  });

  it("alias plugin owner ikut diblokir (killalljadibots, addown, stopalljadibot)", () => {
    for (const text of [".killalljadibots", ".addown 628999", ".stopalljadibot"]) {
      assert.equal(
        blockedJadibotCommand(chatMsg(text), { isJadibot: true }),
        "owner",
        `${text} harus diblokir`,
      );
    }
  });

  it("seluruh direktori `owner` diblokir, bukan cuma plugin yang disebut", () => {
    const owners = getAllPlugins().filter((p) => p.config?.category === "owner");
    assert.ok(
      owners.length > 100,
      `harusnya ada banyak plugin owner, ada ${owners.length}`,
    );
    for (const plugin of owners) {
      assert.equal(
        isJadibotOwnerOnlyCommand(plugin.config.name),
        true,
        `${plugin.config.name} (kategori owner) harus masuk daftar blokir`,
      );
    }
  });

  it("pesan biasa, plugin non-owner, dan commands jadibot sendiri TIDAK diblokir", () => {
    const allowed = [
      "halo bot",
      ".menu",
      ".ping",
      ".jadibot",
      ".stopjadibot",
      ".help exec",
      "anjing|http://x.co",
      ".toImage",
      "promo diskon 50%",
    ];
    for (const text of allowed) {
      assert.equal(
        blockedJadibotCommand(chatMsg(text), { isJadibot: true }),
        null,
        `${text} tidak boleh diblokir`,
      );
    }
  });

  it("isi pesan yang DIBALUT (quote) tidak memicu blokir", () => {
    const quoted = {
      key: { remoteJid: OWNER_JID, id: "MSG3", fromMe: false },
      message: {
        extendedTextMessage: {
          text: "cek ini",
          contextInfo: { quotedMessage: { conversation: RCE_PAYLOAD } },
        },
      },
      messageTimestamp: Math.floor(Date.now() / 1000),
    };
    assert.equal(blockedJadibotCommand(quoted, { isJadibot: true }), null);
  });

  it("wrapper viewOnce tidak bisa menyelundupkan eval", () => {
    const wrapped = {
      key: { remoteJid: OWNER_JID, id: "MSG4", fromMe: false },
      message: {
        extendedTextMessage: {
          viewOnceMessageV2: { message: { conversation: ">> 1+1" } },
        },
      },
      messageTimestamp: Math.floor(Date.now() / 1000),
    };
    assert.equal(
      blockedJadibotCommand(wrapped, { isJadibot: true }),
      "eval",
    );
  });

  it("JALUR UTAMA TIDAK terpengaruh: owner asli di bot utama tetap bisa eval", () => {
    // Gate hanya berlaku di dispatch jadibot. Owner asli tidak pernah lewat
    // sana, jadi handler.js:1132 tetap utuh.
    for (const text of [RCE_PAYLOAD, "!! Object.keys(process.env)", ".exec ls"]) {
      assert.equal(
        blockedJadibotCommand(chatMsg(text), { isJadibot: false }),
        null,
        `di luar jadibot gate harus no-op: ${text}`,
      );
    }
  });

  it("gate benar-benar terpasang sebelum messageHandler (bukan fungsi mati)", async () => {
    const src = await readFile(`${REPO}src/lib/jadibot-manager.js`, "utf8");
    const gateAt = src.indexOf("blockedJadibotCommand(");
    const dispatchAt = src.indexOf("messageHandler(msg, childSock");
    assert.ok(gateAt !== -1, "blockedJadibotCommand harus dipakai di jadibot-manager");
    assert.ok(dispatchAt !== -1, "dispatch messageHandler harus ada");
    assert.ok(
      gateAt < dispatchAt,
      "gate harus diperiksa SEBELUM pesan masuk ke router",
    );
  });

  // Gate pemanggil TIDAK cukup: handler.js menulis ulang m.body/m.command
  // SESUDAH gate itu (perintah VN dari voice note + sticker command), jadi
  // `.exec ...` masih bisa dibentuk dari voice note. Dua tes di bawah
  // mengunci lapis kedua.
  it("lapis kedua: `>>`/`!!` juga ditolak di router saat isJadibot", async () => {
    const src = await readFile(`${REPO}src/handler.js`, "utf8");
    for (const prefix of JADIBOT_BLOCKED_BODY_PREFIXES) {
      const at = src.indexOf(`m.body?.startsWith("${prefix}")`);
      assert.ok(at !== -1, `cabang ${prefix} tidak ada di handler.js`);
      assert.match(
        src.slice(at, at + 220),
        /isJadibot/,
        `cabang ${prefix} harus menolak jadibot, bukan hanya owner host`,
      );
    }
  });

  it("lapis kedua: blokir kategori/command jadibot bukan hanya di grup", async () => {
    const src = await readFile(`${REPO}src/handler.js`, "utf8");
    const dispatchAt = src.indexOf("let plugin = getPlugin(m.command);");
    const blockAt = src.indexOf(
      "jadibotBlockedCategories = [",
      dispatchAt,
    );
    const groupAt = src.indexOf("if (m.isGroup) {", dispatchAt);
    assert.ok(dispatchAt !== -1, "titik dispatch plugin tidak ditemukan");
    assert.ok(blockAt !== -1, "daftar blokir jadibot hilang dari handler.js");
    assert.ok(groupAt !== -1, "cabang if (m.isGroup) tidak ditemukan");
    assert.ok(
      blockAt < groupAt,
      "blokir jadibot harus di luar if (m.isGroup); kalau di dalam, " +
        "private chat lolos dan user premium dapat owner plugins dari DM",
    );
  });
});
