import { describe, it } from "node:test";
import assert from "node:assert";

import { createMessageStore } from "../src/lib/message-store.js";

describe("createMessageStore — indeks berbasis id pesan", () => {
  it("menyimpan lebih dari 30 pesan per chat (tidak ter-evict per-chat FIFO)", () => {
    const store = createMessageStore({ maxMessages: 1000 });
    const ev = { on: (e, cb) => { if (e === "messages.upsert") cb({ messages: [] }); } };
    // inject via direct storeMessage? store has no public push; use bind with fake ev emitting upsert
    const realEv = {
      _listeners: {},
      on(e, cb) { this._listeners[e] = this._listeners[e] || []; this._listeners[e].push(cb); },
      emit(e, payload) { for (const cb of this._listeners[e] || []) cb(payload); },
    };
    store.bind(realEv);
    const msgs = [];
    for (let i = 0; i < 60; i++) {
      msgs.push({ key: { remoteJid: "g@g.us", id: `m${i}` }, message: { conversation: `hi${i}` } });
    }
    realEv.emit("messages.upsert", { messages: msgs });
    // loadMessage should find the oldest even though per-chat cap lama = 30
    assert.ok(store.loadMessage("g@g.us", "m0"), "pesan ke-0 harus masih ada");
    assert.ok(store.loadMessage("g@g.us", "m59"), "pesan ke-59 harus ada");
  });

  it("lookup by id tidak bergantung bentuk JID (LID vs PN)", () => {
    const store = createMessageStore();
    const realEv = {
      _listeners: {},
      on(e, cb) { this._listeners[e] = this._listeners[e] || []; this._listeners[e].push(cb); },
      emit(e, payload) { for (const cb of this._listeners[e] || []) cb(payload); },
    };
    store.bind(realEv);
    realEv.emit("messages.upsert", {
      messages: [{ key: { remoteJid: "123@lid", id: "ABC" }, message: { conversation: "x" } }],
    });
    // looked up with a different JID form — id is unique, must still resolve
    assert.ok(store.loadMessage("628xxx@s.whatsapp.net", "ABC"));
  });

  it("evict oldest secara global saat melebihi maxMessages", () => {
    const store = createMessageStore({ maxMessages: 3 });
    const realEv = {
      _listeners: {},
      on(e, cb) { this._listeners[e] = this._listeners[e] || []; this._listeners[e].push(cb); },
      emit(e, payload) { for (const cb of this._listeners[e] || []) cb(payload); },
    };
    store.bind(realEv);
    realEv.emit("messages.upsert", {
      messages: [
        { key: { remoteJid: "a@g.us", id: "1" }, message: { conversation: "1" } },
        { key: { remoteJid: "a@g.us", id: "2" }, message: { conversation: "2" } },
        { key: { remoteJid: "a@g.us", id: "3" }, message: { conversation: "3" } },
        { key: { remoteJid: "a@g.us", id: "4" }, message: { conversation: "4" } },
      ],
    });
    assert.equal(store.loadMessage("a@g.us", "1"), undefined, "id tertua harus ter-evict");
    assert.ok(store.loadMessage("a@g.us", "4"), "id terbaru harus ada");
    assert.equal(store.size(), 3);
  });

  it("getMessageContent mengembalikan message content", () => {
    const store = createMessageStore();
    const realEv = {
      _listeners: {},
      on(e, cb) { this._listeners[e] = this._listeners[e] || []; this._listeners[e].push(cb); },
      emit(e, payload) { for (const cb of this._listeners[e] || []) cb(payload); },
    };
    store.bind(realEv);
    realEv.emit("messages.upsert", {
      messages: [{ key: { remoteJid: "g@g.us", id: "Z" }, message: { conversation: "hello" } }],
    });
    assert.deepEqual(store.getMessageContent("Z"), { conversation: "hello" });
  });
});
