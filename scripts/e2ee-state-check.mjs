#!/usr/bin/env node
// Read-only E2EE state diagnostic. Safe to paste into a panel console.
//
// Prints counts and shapes, never a value: no key material, no tokens, no JIDs
// beyond the account's own. Everything it reports is already visible in the
// boot log or the directory listing.
//
// The question it answers: are the two decrypt failures
//   "No session found to decrypt message"  (group sender key)
//   "no session"                          (1:1 session)
// caused by state that is ABSENT, or by state that is present but unreadable?
// The fix is completely different in each case, and the boot log cannot tell
// them apart: quarantine prints a line you would have seen, its absence only
// rules quarantine out, it does not prove the files exist.

import { readdirSync, readFileSync, statSync, existsSync } from "node:fs";
import { join } from "node:path";

const SESSION_DIRS = ["storage/session", "storage/sessions", "session", "sessions"];
const findSessionDir = () => SESSION_DIRS.find((d) => existsSync(d) && statSync(d).isDirectory());

const line = (s = "") => process.stdout.write(s + "\n");

line("=== installed versions ===");
for (const p of ["onigis", "oktz-signal", "oktz-curve25519"]) {
  try {
    const v = JSON.parse(readFileSync(`node_modules/${p}/package.json`, "utf8")).version;
    line(`  ${p.padEnd(18)} ${v}`);
  } catch {
    line(`  ${p.padEnd(18)} NOT INSTALLED`);
  }
}

const dir = findSessionDir();
line();
line("=== local auth state ===");
if (!dir) {
  line(`  NO session directory. Looked in: ${SESSION_DIRS.join(", ")}`);
} else {
  line(`  directory: ${dir}`);
  let files = [];
  try {
    files = readdirSync(dir);
  } catch (e) {
    line(`  UNREADABLE: ${e.message}`);
  }
  // Prefixes as useMultiFileAuthState writes them.
  const groups = {
    "pre-key (ours)": /^pre-key-/,
    "session (peers)": /^session-/,
    "sender-key (groups)": /^sender-key-/,
    "app-state-sync": /^app-state-sync-/,
  };
  for (const [label, re] of Object.entries(groups)) {
    const hits = files.filter((f) => re.test(f) && !f.includes(".broken-"));
    line(`  ${label.padEnd(22)} ${String(hits.length).padStart(4)} file(s)`);
  }
  const broken = files.filter((f) => f.includes(".broken-"));
  line(`  quarantined (.broken-)   ${String(broken.length).padStart(4)} file(s)`);
  if (broken.length) {
    line("    quarantine is the smoking gun — these were unreadable and were moved aside.");
    for (const f of broken.slice(0, 5)) line(`      ${f}`);
  }

  // Why this matters most: the rc.8 sender-key fix reads a record left under the
  // PRE-rename store name ("<group>::undefined::<device>"). That only helps if
  // such a record still exists. With zero sender-key files there is nothing to
  // fall back to and the fix cannot engage.
  const sk = files.filter((f) => /^sender-key-/.test(f) && !f.includes(".broken-"));
  if (sk.length) {
    let legacy = 0, current = 0, unreadable = 0;
    for (const f of sk) {
      try {
        const body = JSON.parse(readFileSync(join(dir, f), "utf8"));
        const id = Object.keys(body)[0] ?? "";
        if (id.includes("::undefined::")) legacy++;
        else current++;
      } catch {
        unreadable++;
      }
    }
    line();
    line("  sender-key store names:");
    line(`    under the legacy (pre-rc.8) name   ${legacy}`);
    line(`    under the current name             ${current}`);
    line(`    unreadable                         ${unreadable}`);
    if (legacy) {
      line("    -> legacy records exist, so the rc.8 fallback CAN recover these.");
    } else if (current) {
      line("    -> only current-name records. The fallback has nothing to find;");
      line("       these were written after the rename and a plain miss is expected.");
    }
  }

  const credsPath = join(dir, "creds.json");
  line();
  if (!existsSync(credsPath)) {
    line("  creds.json ABSENT — the boot log said the session was pulled from Turso.");
    line("  That is expected after a fresh container; it is not itself a fault.");
  } else {
    try {
      const c = JSON.parse(readFileSync(credsPath, "utf8"));
      line(`  creds.registered = ${c.registered}`);
      line(`  own JID           = ${c.me?.id ?? "—"}`);
      line(`  registrationId    = ${c.me?.id ? "(stored under me.id)" : "—"}`);
      line(`  has signedPreKey  = ${Boolean(c.signedPreKey)}`);
      // A device that re-paired has a new registrationId; every peer session
      // and group sender key on file belongs to the previous identity.
      line();
      line("  If registered is false, this container restored creds from Turso and");
      line("  any peer session / sender key still on disk describes the OLD identity.");
    } catch (e) {
      line(`  creds.json UNREADABLE: ${e.message}`);
    }
  }
}

line();
line("=== turso mirror (scope 'main') ===");
line("  This is the decisive check. The bot falls back to Turso for any key it");
line("  cannot find on disk, so if Turso holds the rows the state is only");
line("  misplaced, not lost, and the existing fallback will find it.");
try {
  process.loadEnvFile();
} catch { }
const cfg = (await import("../src/lib/turso.js")).default ?? null;
const tursoCfg = (await import("../config.js")).default?.turso ?? null;
if (tursoCfg?.enabled && tursoCfg?.url) {
  const { createClient } = await import("@libsql/client");
  const client = createClient({ url: tursoCfg.url, authToken: tursoCfg.authToken });
  try {
    const creds = await client.execute({
      sql: "SELECT creds FROM session_creds WHERE scope = ?", args: ["main"]
    });
    line(`  session_creds rows        ${creds.rows.length ? "1" : "0"}`);
    const byCat = await client.execute({
      sql: "SELECT category, COUNT(*) AS n FROM session_keys WHERE scope = ? GROUP BY category",
      args: ["main"]
    });
    if (!byCat.rows.length) {
      line("  session_keys rows         0  <- NOTHING mirrored");
      line("    This is a fault, not a fresh container. Every session and sender key");
      line("    existed locally and was never written to Turso, so the fallback that");
      line("    is supposed to cover this has nothing to fall back to.");
    } else {
      for (const r of byCat.rows) {
        line(`  session_keys[${String(r.category).padEnd(14)}] ${r.n}`);
      }
    }
  } catch (e) {
    line(`  turso UNREACHABLE: ${e.message}`);
  } finally {
    client.close?.();
  }
} else {
  line("  turso disabled or not configured in config.js");
}

line();
line("=== what this means ===");
line("  session-*.json = 0   -> peers must re-negotiate. Normal after a container");
line("                           replacement; resolves within a few messages.");
line("  sender-key-* = 0     -> every group is undecryptable until each sender");
line("                           redistributes. WhatsApp does that when a member");
line("                           sends again, so it can recover on its own — but");
line("                           only if the group keeps receiving traffic.");
line("  .broken- present    -> files existed and were unreadable. Tell me the");
line("                           count; that is a real fault, not a fresh container.");
line();
line("  The gap worth knowing about: nothing in this stack asks a sender to");
line("  resend its sender key when decryption fails. Recovery depends on that");
line("  sender happening to send again.");
