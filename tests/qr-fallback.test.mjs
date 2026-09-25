import { describe, it, before, beforeEach, afterEach } from "node:test";
import assert from "node:assert";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";

describe("qr fallback", () => {
  let m;

  before(async () => {
    m = await import("../src/lib/qr-fallback.js");
  });

  describe("shouldUseQrFallback", () => {
    it("keeps pairing code when nothing went wrong", () => {
      assert.equal(
        m.shouldUseQrFallback({
          usePairingCode: true,
          fallbackEnabled: true,
          rateLimited: false,
        }),
        false,
      );
    });

    it("takes over once pairing code is rate limited", () => {
      assert.equal(
        m.shouldUseQrFallback({
          usePairingCode: true,
          fallbackEnabled: true,
          rateLimited: true,
        }),
        true,
      );
    });

    it("respects an operator who disabled the fallback", () => {
      assert.equal(
        m.shouldUseQrFallback({
          usePairingCode: true,
          fallbackEnabled: false,
          rateLimited: true,
        }),
        false,
      );
    });

    it("is already in qr mode when pairing code is off entirely", () => {
      assert.equal(
        m.shouldUseQrFallback({
          usePairingCode: false,
          fallbackEnabled: true,
          rateLimited: false,
        }),
        true,
      );
    });
  });

  describe("shouldRenderQr", () => {
    it("renders the first qr", () => {
      assert.equal(m.shouldRenderQr({ qr: "abc", lastRendered: null }), true);
    });

    it("skips an identical qr so the terminal is not spammed", () => {
      assert.equal(
        m.shouldRenderQr({ qr: "abc", lastRendered: "abc" }),
        false,
      );
    });

    it("renders again when the library rotates the ref", () => {
      assert.equal(
        m.shouldRenderQr({ qr: "def", lastRendered: "abc" }),
        true,
      );
    });

    it("never renders an empty qr", () => {
      assert.equal(m.shouldRenderQr({ qr: "", lastRendered: null }), false);
      assert.equal(m.shouldRenderQr({ qr: null, lastRendered: null }), false);
      assert.equal(m.shouldRenderQr({ qr: undefined, lastRendered: "x" }), false);
    });

    it("stops rendering once the repeat cap is reached", () => {
      let last = null;
      let renders = 0;
      for (let i = 0; i < 12; i++) {
        const qr = `ref-${i}`;
        if (
          m.canRenderMore({ renderedCount: renders, maxPrints: 3 }) &&
          m.shouldRenderQr({ qr, lastRendered: last })
        ) {
          renders++;
          last = qr;
        }
      }
      assert.equal(renders, 3);
    });

    it("always allows a render when no cap is set", () => {
      for (let i = 0; i < 50; i++) {
        assert.equal(
          m.canRenderMore({ renderedCount: i, maxPrints: Infinity }),
          true,
        );
      }
    });
  });

  describe("buildQrFilePath", () => {
    it("puts the file under the given directory", () => {
      const p = m.buildQrFilePath("/tmp/botqr", 1700000000000);
      assert.equal(path.dirname(p), "/tmp/botqr");
      assert.match(path.basename(p), /^qr-1700000000000\.png$/);
    });

    it("keeps two stamps from colliding", () => {
      assert.notEqual(
        m.buildQrFilePath("/tmp/botqr", 1),
        m.buildQrFilePath("/tmp/botqr", 2),
      );
    });
  });

  describe("pruneOldQrFiles", () => {
    let dir;

    beforeEach(() => {
      dir = fs.mkdtempSync(path.join(os.tmpdir(), "qrprune-"));
    });

    afterEach(() => {
      fs.rmSync(dir, { recursive: true, force: true });
    });

    const touch = (name) => fs.writeFileSync(path.join(dir, name), "x");

    it("keeps the newest files by mtime", () => {
      const now = Date.now();
      for (const [name, age] of [
        ["qr-1.png", 3000],
        ["qr-2.png", 2000],
        ["qr-3.png", 1000],
        ["qr-4.png", 0],
      ]) {
        touch(name);
        const t = new Date(now - age);
        fs.utimesSync(path.join(dir, name), t, t);
      }

      m.pruneOldQrFiles(dir, 2);

      const left = fs.readdirSync(dir).sort();
      assert.deepEqual(left, ["qr-3.png", "qr-4.png"]);
    });

    it("never deletes unrelated files in the directory", () => {
      touch("keep-me.txt");
      touch("qr-1.png");
      m.pruneOldQrFiles(dir, 0);
      assert.deepEqual(fs.readdirSync(dir), ["keep-me.txt"]);
    });

    it("does not throw when the directory is missing", () => {
      assert.doesNotThrow(() =>
        m.pruneOldQrFiles(path.join(dir, "nope"), 2),
      );
    });

    it("leaves the directory empty when there is nothing to prune", () => {
      assert.doesNotThrow(() => m.pruneOldQrFiles(dir, 5));
    });
  });
});
