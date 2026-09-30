#!/usr/bin/env node
// DESTRUCTIVE. Wipes the Signal session in Turso for one scope, nothing else.
//
// Scope is fixed by construction. It touches exactly two tables and only rows
// matching --scope:
//
//   DELETE FROM session_creds WHERE scope = ?
//   DELETE FROM session_keys WHERE scope = ?
//
// Every other table in the database is listed and left alone. There is no code
// path here that drops a table, truncates, or deletes without a scope filter.
//
// WHAT THIS COSTS: creds.json for the scope is gone, so the bot loses its
// registration and device identity. The next boot cannot restore a session and
// will require a fresh pair — a QR code. Every peer session and every group
// sender key for that scope is gone with no way to recover them from here.
//
// Usage:
//   node scripts/reset-turso-session.mjs --scope main --yes
//
// Run it with the bot STOPPED. A running bot holds a Turso client and will
// happily re-mirror state that has not been re-negotiated, undoing the wipe.

import { existsSync } from "node:fs";

const arg = (flag, fallback) => {
  const i = process.argv.indexOf(flag);
  return i !== -1 ? process.argv[i + 1] : fallback;
};

const scope = arg("--scope", "main");
const confirmed = process.argv.includes("--yes");

if (existsSync(".env")) { try { process.loadEnvFile(); } catch { } }

const url = process.env.TURSO_URL;
const authToken = process.env.TURSO_AUTH_TOKEN;
if (!url || !authToken) {
  console.error("TURSO_URL / TURSO_AUTH_TOKEN missing. Refusing to guess.");
  process.exit(1);
}

const { createClient } = await import("@libsql/client");
const client = createClient({ url, authToken });
const run = async (sql, args = []) => (await client.execute({ sql, args })).rows;

let reachable = true;
try { await run("SELECT 1"); } catch (e) {
  reachable = false;
  console.error(`Turso unreachable: ${e.message}`);
  if (/401/.test(e.message)) {
    console.error("  -> the token is rejected. Issue a new one in the Turso console.");
    console.error("     The token in this environment has also been pasted into a chat");
    console.error("     and must be rotated regardless.");
  }
}
if (!reachable) { client.close?.(); process.exit(1); }

const tables = (await run("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name"))
  .map((r) => r.name);

console.log("tables in this database (only the two session tables will be touched):");
for (const t of tables) {
  const isSession = t === "session_creds" || t === "session_keys";
  console.log(`  ${isSession ? "WILL DELETE" : "left alone "}  ${t}`);
}

const creds = await run("SELECT COUNT(*) AS n FROM session_creds WHERE scope = ?", [scope]);
const keys = await run(
  "SELECT category, COUNT(*) AS n FROM session_keys WHERE scope = ? GROUP BY category",
  [scope],
);
console.log(`\nscope "${scope}" before:`);
console.log(`  session_creds  ${creds[0]?.n ?? 0} row(s)`);
for (const r of keys) console.log(`  session_keys[${r.category}] ${r.n}`);
if (!keys.length) console.log("  session_keys    0 row(s)");

if (!confirmed) {
  console.log("\nDry run. Nothing was deleted.");
  console.log("Re-run with --yes to actually wipe, with the bot stopped.");
  client.close?.();
  process.exit(0);
}

await run("DELETE FROM session_keys WHERE scope = ?", [scope]);
await run("DELETE FROM session_creds WHERE scope = ?", [scope]);

const credsAfter = await run("SELECT COUNT(*) AS n FROM session_creds WHERE scope = ?", [scope]);
const keysAfter = await run("SELECT COUNT(*) AS n FROM session_keys WHERE scope = ?", [scope]);
console.log(`\nscope "${scope}" after:`);
console.log(`  session_creds  ${credsAfter[0]?.n ?? 0} row(s)`);
console.log(`  session_keys   ${keysAfter[0]?.n ?? 0} row(s)`);
console.log("\nDone. The next boot will need a fresh pair — a QR code.");
client.close?.();
