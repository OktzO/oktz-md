import { describe, it, before, beforeEach, afterEach } from "node:test";
import assert from "node:assert";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";

describe("pairing state", () => {
  let m;

  before(async () => {
    m = await import("../src/lib/pairing-state.js");
  });

  describe("pairing pending marker", () => {
    it("is false on a fresh creds object", () => {
      assert.equal(m.isPairingPending({}), false);
    });

    it("survives a JSON round trip once marked", () => {
      const creds = { registered: false, me: { id: "628@s.whatsapp.net" } };
      m.markPairingPending(creds);

      const reloaded = JSON.parse(JSON.stringify(creds));
      assert.equal(reloaded.additionalData.pairingPending, true);
      assert.equal(m.isPairingPending(reloaded), true);
    });

    it("clears the marker while keeping unrelated additionalData keys", () => {
      const creds = { additionalData: { keepMe: 7 } };
      m.markPairingPending(creds);
      m.clearPairingPending(creds);

      assert.equal(creds.additionalData.pairingPending, undefined);
      assert.equal(creds.additionalData.keepMe, 7);
    });

    it("clearing a marker that was never set does not throw", () => {
      const creds = {};
      assert.doesNotThrow(() => m.clearPairingPending(creds));
    });
  });

  describe("resetPairingCreds", () => {
    it("clears the provisional fields a failed pairing leaves behind", () => {
      const creds = {
        registered: true,
        me: { id: "628@s.whatsapp.net", name: "~" },
        pairingCode: "OKTZZLAH",
        account: { accountSignatureKey: "x" },
        signalIdentities: [{ identifier: "y" }],
        noiseKey: { public: "keep-noise" },
      };

      m.resetPairingCreds(creds);

      assert.equal(creds.me, undefined);
      assert.equal(creds.pairingCode, undefined);
      assert.equal(creds.account, undefined);
      assert.deepEqual(creds.signalIdentities, []);
      assert.equal(creds.registered, false);
      assert.equal(m.isPairingPending(creds), false);
    });

    it("keeps device identity keys so the next pairing reuses them", () => {
      const creds = { noiseKey: { public: "n" }, signedIdentityKey: { public: "s" } };
      m.resetPairingCreds(creds);
      assert.deepEqual(creds.noiseKey, { public: "n" });
      assert.deepEqual(creds.signedIdentityKey, { public: "s" });
    });

    it("is idempotent", () => {
      const creds = { me: { id: "628@s.whatsapp.net" } };
      m.resetPairingCreds(creds);
      assert.doesNotThrow(() => m.resetPairingCreds(creds));
      assert.equal(creds.me, undefined);
    });
  });

  describe("classifyClose", () => {
    it("purges the session on a genuine logout of a paired session", () => {
      const creds = { account: { accountSignatureKey: "x" } };
      assert.equal(m.classifyClose(401, creds), "purge-session");
    });

    it("purges a pairing-code session that reached companion finish", () => {
      const creds = { registered: true, account: { accountSignatureKey: "x" } };
      assert.equal(m.classifyClose(401, creds), "purge-session");
    });

    it("purges a QR-paired session whose registered flag is still false", () => {
      const creds = { registered: false, account: { accountSignatureKey: "x" } };
      assert.equal(m.classifyClose(401, creds), "purge-session");
    });

    it("resets pairing on 401 while a pairing is pending", () => {
      assert.equal(
        m.classifyClose(401, { additionalData: { pairingPending: true } }),
        "reset-pairing",
      );
    });

    it("resets instead of purging a brand new session with no auth evidence", () => {
      assert.equal(m.classifyClose(401, {}), "reset-pairing");
    });

    it("resets instead of purging provisional creds with no auth evidence", () => {
      const creds = { registered: false, me: { id: "628@s.whatsapp.net" } };
      assert.equal(m.classifyClose(401, creds), "reset-pairing");
    });

    it("treats missing creds as unauthenticated rather than purging", () => {
      assert.equal(m.classifyClose(401, undefined), "reset-pairing");
    });

    it("reconnects on a session conflict without purging", () => {
      const creds = { account: { accountSignatureKey: "x" } };
      assert.equal(m.classifyClose(440, creds), "reconnect");
    });

    it("reconnects on forbidden rather than nuking auth", () => {
      assert.equal(m.classifyClose(403, { additionalData: {} }), "reconnect");
    });

    it("reconnects on a plain transport close", () => {
      assert.equal(m.classifyClose(428, { account: {} }), "reconnect");
    });

    it("reconnects when no status code is available", () => {
      assert.equal(m.classifyClose(undefined, { account: {} }), "reconnect");
    });

    it("reconnects on a pending pairing transport close", () => {
      assert.equal(
        m.classifyClose(428, { additionalData: { pairingPending: true } }),
        "reconnect",
      );
    });
  });

  describe("isRateLimitError", () => {
    it("detects the rate-overlimit message", () => {
      assert.equal(m.isRateLimitError(new Error("rate-overlimit")), true);
    });

    it("detects statusCode 429 on the Boom output", () => {
      assert.equal(m.isRateLimitError({ output: { statusCode: 429 } }), true);
    });

    it("detects the IQ error payload shape carrying 429 in data", () => {
      assert.equal(m.isRateLimitError({ data: 429 }), true);
    });

    it("detects a numeric 429 in the message", () => {
      assert.equal(m.isRateLimitError(new Error("request failed 429")), true);
    });

    it("rejects an unrelated 401", () => {
      assert.equal(m.isRateLimitError({ output: { statusCode: 401 } }), false);
    });

    it("rejects a transport close", () => {
      assert.equal(m.isRateLimitError(new Error("Connection Closed")), false);
    });

    it("rejects a null error", () => {
      assert.equal(m.isRateLimitError(null), false);
    });

    it("rejects a non-error object without any rate signal", () => {
      assert.equal(m.isRateLimitError({}), false);
    });
  });

  describe("cooldown store", () => {
    let dir;

    beforeEach(() => {
      dir = fs.mkdtempSync(path.join(os.tmpdir(), "pairing-cd-"));
    });

    afterEach(() => {
      fs.rmSync(dir, { recursive: true, force: true });
    });

    it("starts with no cooldown", () => {
      assert.equal(m.readCooldown(dir), 0);
    });

    it("persists a deadline and reads it back", () => {
      const until = Date.now() + 60_000;
      m.writeCooldown(dir, until);
      assert.equal(m.readCooldown(dir), until);
    });

    it("survives a cooldown outliving the current process", () => {
      const until = Date.now() + 60_000;
      m.writeCooldown(dir, until);
      const reloaded = m.readCooldown(dir);
      assert.equal(reloaded, until);
    });

    it("clears a stored cooldown", () => {
      m.writeCooldown(dir, Date.now() + 60_000);
      m.clearCooldown(dir);
      assert.equal(m.readCooldown(dir), 0);
    });

    it("treats a corrupt file as no cooldown instead of throwing", () => {
      fs.writeFileSync(path.join(dir, ".pairing-cooldown"), "not json");
      assert.equal(m.readCooldown(dir), 0);
    });

    it("reports an expired deadline as no active cooldown", () => {
      const past = Date.now() - 1_000;
      m.writeCooldown(dir, past);
      assert.equal(m.getCooldownRemainingMs(dir, Date.now()), 0);
    });

    it("reports remaining milliseconds for a live deadline", () => {
      const now = Date.now();
      m.writeCooldown(dir, now + 30_000);
      const remaining = m.getCooldownRemainingMs(dir, now);
      assert.ok(remaining > 29_000 && remaining <= 30_000, `got ${remaining}`);
    });

    it("prunes the marker once the deadline passes", () => {
      const now = Date.now();
      m.writeCooldown(dir, now - 1_000);
      m.getCooldownRemainingMs(dir, now);
      assert.equal(fs.existsSync(path.join(dir, ".pairing-cooldown")), false);
    });
  });

  describe("pairing number validation", () => {
    it("strips formatting characters from a valid international number", () => {
      assert.equal(m.normalizePairingNumber("+62 (851) 438-85645"), "6285143885645");
    });

    it("rejects a number that sanitizes to nothing", () => {
      assert.equal(m.normalizePairingNumber(""), null);
      assert.equal(m.normalizePairingNumber("   "), null);
      assert.equal(m.normalizePairingNumber("not-a-number"), null);
    });

    it("rejects a zero-length digit string", () => {
      assert.equal(m.normalizePairingNumber("+"), null);
    });
  });
});
