import { describe, it } from "node:test";
import assert from "node:assert";

import {
  classifyLibraryError,
  createLibraryLogRouter,
} from "../src/lib/library-log.js";

describe("library-log: klasifikasi kegagalan dekripsi", () => {
  it("rollback dengan detail 'No session found' diklasifikasi decrypt-rollback", () => {
    assert.equal(
      classifyLibraryError("transaction failed, rolling back", "No session found to decrypt message"),
      "decrypt-rollback",
    );
  });
  it("'failed to decrypt message' dengan 'No session found' -> decrypt-miss", () => {
    assert.equal(
      classifyLibraryError("failed to decrypt message", "No session found to decrypt message"),
      "decrypt-miss",
    );
  });
  it("'failed to decrypt message' dengan 'Bad MAC' -> decrypt-hard", () => {
    assert.equal(classifyLibraryError("failed to decrypt message", "Bad MAC"), "decrypt-hard");
  });
  it("'sent retry receipt' -> retry-sent", () => {
    assert.equal(classifyLibraryError("sent retry receipt", ""), "retry-sent");
  });
  it("'reached retry limit...' -> retry-exhausted", () => {
    assert.equal(
      classifyLibraryError("reached retry limit, no retry request sent: x", ""),
      "retry-exhausted",
    );
  });
});

describe("library-log: agregasi per jendela waktu", () => {
  it("baris rollback duplikat di-drop; baris utama pertama dicetak; ringkasan di-flush", () => {
    const seen = [];
    let now = 1000;
    const router = createLibraryLogRouter({
      windowMs: 60000,
      report: (level, text) => seen.push({ level, text }),
      now: () => now,
    });

    router.report("error", "transaction failed, rolling back", "No session found to decrypt message");
    router.report("error", "failed to decrypt message", "No session found to decrypt message");
    router.report("error", "failed to decrypt message", "No session found to decrypt message");
    router.report("error", "sent retry receipt", "");

    // rollback dropped; first decrypt-miss printed; second suppressed; retry-sent counted only
    const decryptLines = seen.filter((s) => /failed to decrypt message/.test(s.text));
    assert.equal(decryptLines.length, 1, "hanya baris pertama yang dicetak");

    // flush manual -> ringkasan
    router.flush();
    const summary = seen.find((s) => /E2EE ringkasan/.test(s.text));
    assert.ok(summary, "ringkasan harus dicetak saat flush");
    assert.match(summary.text, /2 dekripsi gagal/);
    assert.match(summary.text, /1 retry receipt dikirim/);
    assert.match(summary.text, /1 transaksi di-rollback/);
  });

  it("decrypt-hard dicetak maksimal 3 kali lalu diagregasi", () => {
    const seen = [];
    let now = 1000;
    const router = createLibraryLogRouter({
      windowMs: 60000,
      report: (level, text) => seen.push({ level, text }),
      now: () => now,
    });
    for (let i = 0; i < 7; i++) {
      router.report("error", "failed to decrypt message", "Bad MAC");
    }
    const hardLines = seen.filter((s) => /failed to decrypt message/.test(s.text));
    assert.equal(hardLines.length, 3, "cap 3 baris hard");
    router.flush();
    const summary = seen.find((s) => /E2EE ringkasan/.test(s.text));
    assert.ok(summary);
    assert.match(summary.text, /7 dekripsi gagal \(MAC/);
  });
});
