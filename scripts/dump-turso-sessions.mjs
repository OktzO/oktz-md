// Dump data sesi Turso ke JSON untuk spike importer S1 (offline, tanpa jaringan
// saat test). Baca kredensial dari .env — JANGAN hardcode token di sini.
//
//   node scripts/dump-turso-sessions.mjs
//
// Output: rust-spike/fixtures/turso_session_keys.json
//
// Format data disimpan apa adanya (BufferJSON gaya Baileys: {type:"Buffer",
// data:"<base64>"} atau data sebagai array angka) — importer Rust yang harus
// handle keduanya, bukan script ini.

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { createRequire } from "node:module";

const require = createRequire(import.meta.url);
const { createClient } = require("@libsql/client");

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const ROOT = path.resolve(__dirname, "..");

// .env minimal: baca tanpa dependensi tambahan.
function loadEnv(file) {
  try {
    for (const line of fs.readFileSync(file, "utf8").split("\n")) {
      const m = line.match(/^\s*([A-Z0-9_]+)\s*=\s*(.*)\s*$/);
      if (m && !process.env[m[1]]) process.env[m[1]] = m[2].trim();
    }
  } catch {}
}
loadEnv(path.join(ROOT, ".env"));

const url = process.env.TURSO_URL;
const authToken = process.env.TURSO_AUTH_TOKEN;
if (!url || !authToken) {
  console.error("TURSO_URL / TURSO_AUTH_TOKEN tidak ada. Isi .env dulu.");
  process.exit(1);
}

const client = createClient({ url, authToken });

// Pre-key 11rb lebih — cuma ambil sebagian untuk menjaga ukuran fixture.
// Sesi & sender-key yang penting diambil semua.
const CATEGORY_CAP = { "pre-key": 50, "tctoken": 5 };

async function main() {
  const cats = await client.execute(
    "SELECT category, COUNT(*) n FROM session_keys GROUP BY category ORDER BY n DESC"
  );
  console.error("kategori di Turso:");
  for (const r of cats.rows) console.error(`  ${r.category.padEnd(22)} ${r.n}`);

  const out = {};
  let totalRows = 0;
  for (const { category } of cats.rows) {
    const cap = CATEGORY_CAP[category];
    const sql = cap
      ? `SELECT scope, category, id, data FROM session_keys WHERE category = ? LIMIT ${cap}`
      : `SELECT scope, category, id, data FROM session_keys WHERE category = ?`;
    const res = await client.execute(sql, [category]);
    out[category] = res.rows.map((r) => ({
      scope: r.scope,
      category: r.category,
      id: r.id,
      data: r.data,
    }));
    totalRows += res.rows.length;
    console.error(`  dump ${category}: ${res.rows.length} baris`);
  }

  const outPath = path.join(ROOT, "rust-spike", "fixtures", "turso_session_keys.json");
  fs.mkdirSync(path.dirname(outPath), { recursive: true });
  fs.writeFileSync(outPath, JSON.stringify(out, null, 0));
  const kb = (fs.statSync(outPath).size / 1024).toFixed(1);
  console.error(`\ntotal ${totalRows} baris -> ${outPath} (${kb} KB)`);
}

main().catch((e) => {
  console.error("ERR", e.message);
  process.exit(1);
});