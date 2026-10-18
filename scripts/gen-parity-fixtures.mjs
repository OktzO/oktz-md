// Fixture parity dari JS produksi — nilai diambil dari `config.js` yang
// benar-benar jalan, bukan dari diligent transkripsi.
//
//   node scripts/gen-parity-fixtures.mjs
//
// Jalankan ulang setiap kali `config.js` berubah, lalu jalankan test parity.

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import config from "../config.js";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const OUT = path.join(ROOT, "rust/lib/oktz-core/tests/fixtures/parity");
fs.mkdirSync(OUT, { recursive: true });

const out = {
  messages: config.messages,
  info: {
    website: config.info.website,
    botName: config.bot.name,
    version: config.bot.version,
    developer: config.bot.developer,
  },
  saluran: { id: config.saluran.id, name: config.saluran.name },
};

const file = path.join(OUT, "config_text.json");
fs.writeFileSync(file, JSON.stringify(out, null, 2));
console.log(`${file} (${Object.keys(config.messages).length} pesan)`);