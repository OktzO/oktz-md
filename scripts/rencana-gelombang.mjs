// Susun rencana gelombang port: panggil `bandingkan-nama-plugin.mjs`, baca
// daftar plugin yang belum di-port dari stdout-nya, kelompokkan menurut
// kapabilitas yang masih hilang, lalu bagi jadi N berkas brief agent.
//
// Pemakaian:
//   node scripts/rencana-gelombang.mjs [jumlah-agent] [--prefiks=gelombang]
//
// Keluaran: `/tmp/opencode/<prefiks>-<n>.md` per agent, `/tmp/opencode/bebas.txt`,
// dan ringkasan jumlah per kapabilitas di stdout.
import { readFileSync, writeFileSync, mkdirSync, existsSync } from "node:fs";
import { execFileSync } from "node:child_process";

const JUMLAH = Number(process.argv[2] || 8);
const PREFIKS = (process.argv[3] || "--prefiks=gelombang").replace("--prefiks=", "");
const OUT = "/tmp/opencode";
mkdirSync(OUT, { recursive: true });

const keluaran = execFileSync("node", ["scripts/bandingkan-nama-plugin.mjs"], {
  encoding: "utf8",
  maxBuffer: 64 * 1024 * 1024,
});
const belum = [...keluaran.matchAll(/^ {2}- \S+ \| (.+\.js)$/gm)].map((m) => m[1]);
writeFileSync(`${OUT}/belum.txt`, belum.join("\n") + "\n");

// `sock` sengaja dipisah: `sock.sendMessage` tidak dihitung, karena semua
// plugin memakainya dan di Rust sudah ada `Reply`.
//
// fs, ffmpeg, sharp, sock, cheerio, dan answerHandler sudah punya padanan di
// Rust (lihat `docs/rust-rewrite/PORT-BRIEF.md`), jadi yang tersisa sebagai
// penghalang hanya AIRich dan pustaka `src/lib/games.js`.
function kapabilitas(t) {
  const perlu = [];
  if (/\bAIRich\b|sendTable|addTable/.test(t)) perlu.push("AIRich");
  if (/import .*lib\/games|\bgames\.[a-zA-Z]+\(/.test(t)) perlu.push("games");
  return [...new Set(perlu)];
}

const bebas = [];
const per = new Map();
const baris = [];
for (const f of belum) {
  let t;
  try {
    t = readFileSync(f, "utf8");
  } catch {
    continue;
  }
  const perlu = kapabilitas(t);
  if (perlu.length === 0) bebas.push(f);
  else {
    baris.push(`${perlu.join("+")}\t${f}`);
    const kunci = perlu.join("+");
    per.set(kunci, (per.get(kunci) || 0) + 1);
  }
}

writeFileSync(`${OUT}/bebas.txt`, bebas.join("\n") + "\n");
writeFileSync(`${OUT}/terblokir.txt`, baris.join("\n") + "\n");

console.log(`belum di-port: ${belum.length} | bebas kapabilitas: ${bebas.length} | terblokir: ${baris.length}`);
for (const [k, v] of [...per.entries()].sort((a, b) => b[1] - a[1])) console.log(`  ${k}: ${v}`);

if (bebas.length === 0) process.exit(0);

// Bagi rata; tiap berkas JS hanya muncul di satu kelompok.
const kelompok = Array.from({ length: JUMLAH }, () => []);
bebas.forEach((f, i) => kelompok[i % JUMLAH].push(f));
let tertulis = 0;
kelompok.forEach((daftar, i) => {
  if (daftar.length === 0) return;
  tertulis++;
  const isi = `# ${PREFIKS} ${i + 1}

Baca \`docs/rust-rewrite/PORT-BRIEF.md\` DULU; aturan di sana mengikat.

## Plugin milikmu (${daftar.length} berkas)

${daftar.map((f) => `- \`${f}\``).join("\n")}

## Keluaran

\`rust/plugin/src/<kategori>/<nama>.rs\` untuk setiap
\`plugins/<kategori>/<nama>.js\`, plus daftar singkat: berkas yang dibuat,
jumlah test per berkas, dan berkas yang tidak jadi di-port beserta alasannya.
JANGAN menyentuh \`mod.rs\` mana pun.
`;
  writeFileSync(`${OUT}/${PREFIKS}-${tertulis}.md`, isi);
});
console.log(`brief tertulis: ${tertulis} berkas di ${OUT}/${PREFIKS}-*.md`);
