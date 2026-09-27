import { describe, it, beforeEach, after } from "node:test";
import assert from "node:assert";
import { readFile, writeFile } from "node:fs/promises";
import { existsSync } from "node:fs";
import path from "node:path";
import { extendSocket } from "../src/lib/socket.js";
import { forgetCachedJid } from "../src/lib/lid.js";

// cacheLidJid() yang dipakai resolveFromSock menulis database/lid-cache.json
// lewat timer persist DAN Map in-memory. Kalau cache-nya hangat, getName
// menyelesaikan mapping di baris pertamanya (getCachedJid) dan sama sekali
// tidak menyentuh fallback yang diperbaiki -- test jadi lulus even though
// fix-nya dibalik. Sudah diuji: dengan cache hangat, 3 test di bawah tetap
// hijau walau fix-nya dimatikan. Jadi tiap test Starts dari cache dingin lewat
// forgetCachedJid(), dan file runtime-nya juga dibersihkan supaya tidak
// meninggalkan jejak untuk run berikutnya.

const SOCKET = new URL("../src/lib/socket.js", import.meta.url).pathname;
const LID_CACHE_FILE = path.join(
  new URL("..", import.meta.url).pathname,
  "database",
  "lid-cache.json",
);

const LID = "165648885899430@lid";
const LID_BACKREF = "165648885899431@lid";
const LID_NOTIFY = "165648885899432@lid";
const PN = "6281234567890@s.whatsapp.net";
const PN_BACKREF = "6281234567891@s.whatsapp.net";
const PN_NOTIFY = "6281234567892@s.whatsapp.net";

const TEST_LIDS = [LID, LID_BACKREF, LID_NOTIFY];

async function scrubLidCacheFile() {
  if (!existsSync(LID_CACHE_FILE)) return;
  try {
    const data = JSON.parse(await readFile(LID_CACHE_FILE, "utf8"));
    let changed = false;
    for (const lid of TEST_LIDS) {
      const bare = lid.replace("@lid", "");
      for (const k of [lid, `${bare}@s.whatsapp.net`]) {
        if (k in data) {
          delete data[k];
          changed = true;
        }
      }
    }
    if (changed) await writeFile(LID_CACHE_FILE, JSON.stringify(data));
  } catch {}
}

// Dijalankan SEBELUM tiap test: Map in-memory dibersihkan dulu (itu yang
// dipakai resolveFromSock), lalu file-nya.
async function coldLidCache() {
  for (const lid of TEST_LIDS) forgetCachedJid(lid);
  await scrubLidCacheFile();
}

// Fake socket: onWhatsApp guarantees result.jid === the jid passed in, and
// returns only { jid, exists } -- no name, no resolved PN. That is exactly the
// new contract that killed the old fallback.
function makeSock(contacts, extra = {}) {
  return extendSocket({
    sendMessage: async () => ({}),
    profilePictureUrl: async () => null,
    user: { id: "628999:1@s.whatsapp.net" },
    store: { contacts: { ...contacts } },
    onWhatsApp: async (jid) => [{ jid, exists: true }],
    ...extra,
  });
}

beforeEach(coldLidCache);
after(scrubLidCacheFile);

/* ------------------------------------------------------------------ */
/* A5 - fallback onWhatsApp kehilangan pembeda terakhir                 */
/* ------------------------------------------------------------------ */

describe("getName: fallback onWhatsApp tidak lagi bisa diandalkan", () => {
  it("source: tidak boleh memakai result.jid sebagai kunci lookup nama", async () => {
    const src = await readFile(SOCKET, "utf8");
    const idx = src.indexOf("if (sock.onWhatsApp) {");
    assert.ok(idx > -1, "blok onWhatsApp tidak ditemukan");
    const block = src.slice(idx, idx + 900);
    assert.doesNotMatch(
      block,
      /sock\.store\.contacts\[result\.jid\]/,
      "contacts[result.jid] dijamin mengulang miss -- result.jid === id",
    );
  });
});

/* ------------------------------------------------------------------ */
/* A5 — LID harus tetap ketemu namanya lewat mapping PN                */
/* ------------------------------------------------------------------ */

describe("getName: nama LID ditemukan lewat mapping PN", () => {
  it("LID dengan mapping di signalRepository -> nama dari store PN", async () => {
    const sock = await makeSock({ [PN]: { name: "Budi Santoso" } }, {
      signalRepository: {
        lidMapping: { getPNForLID: async (lid) => (lid === LID ? PN : null) },
      },
    });

    const name = await sock.getName(LID);
    assert.equal(name, "Budi Santoso", "nama LID harus ter-resolve via mapping PN");
  });

  it("LID dengan back-reference contact.lid di store -> nama dari store PN", async () => {
    const sock = await makeSock({ [PN_BACKREF]: { name: "Budi Santoso", lid: LID_BACKREF } });
    const name = await sock.getName(LID_BACKREF);
    assert.equal(name, "Budi Santoso");
  });

  it("notify dipakai kalau name tidak ada di entry PN", async () => {
    const sock = await makeSock({ [PN_NOTIFY]: { notify: "Budhi" } }, {
      signalRepository: {
        lidMapping: { getPNForLID: async () => PN_NOTIFY },
      },
    });
    assert.equal(await sock.getName(LID_NOTIFY), "Budhi");
  });

  it("kunci LID sendiri tetap dicoba lebih dulu (store boleh keyed by LID)", async () => {
    const sock = await makeSock({ [LID]: { name: "Budi (by lid)" } }, {
      signalRepository: {
        lidMapping: { getPNForLID: async () => PN },
      },
    });
    assert.equal(await sock.getName(LID), "Budi (by lid)");
  });
});

/* ------------------------------------------------------------------ */
/* A5 — tanpa mapping, tetap Unknown (dilarang fabrikasi nomor)        */
/* ------------------------------------------------------------------ */

describe("getName: LID tanpa mapping tetap Unknown, bukan nomor palsu", () => {
  it("tidak fabricating nomor dari digit LID", async () => {
    const sock = await makeSock({});
    const name = await sock.getName(LID);
    assert.equal(name, "Unknown", `dapat "${name}" -- nomor LID dipalsukan`);
  });

  it("PN biasa tetap berfungsi normal (tidak regresi)", async () => {
    const sock = await makeSock({ [PN]: { name: "Budi Santoso" } });
    assert.equal(await sock.getName(PN), "Budi Santoso");
  });

  it("onWhatsApp yang melempar tidak memunculkan error", async () => {
    const sock = await makeSock({}, {
      onWhatsApp: async () => {
        throw new Error("iq timeout");
      },
    });
    assert.equal(await sock.getName(PN), "+6281234567890");
  });
});
