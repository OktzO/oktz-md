// Kelompokkan plugin yang belum ter-port berdasarkan kapabilitas yang masih
// hilang. Plugin "bebas" bisa langsung di-port tanpa menambah API baru; sisanya
// terblokir sampai kapabilitasnya ada.
//
// Keluaran: dua daftar di `/tmp/opencode/bebas.txt` dan `/tmp/opencode/terblokir.txt`.
import { readFileSync, writeFileSync } from "node:fs";

const files = readFileSync("/tmp/opencode/belum.txt", "utf8").trim().split("\n");

function kapabilitas(t) {
  const perlu = [];
  if (/\bsock\./.test(t)) perlu.push("sock");
  if (/require\(["']fs|fs\.(readFile|writeFile|existsSync|mkdir|readdir|unlink|createWriteStream|createReadStream|readFileSync)/.test(t))
    perlu.push("fs");
  if (/sharp/.test(t)) perlu.push("sharp");
  if (/ffmpeg|ffprobe/.test(t)) perlu.push("ffmpeg");
  if (/cheerio/.test(t)) perlu.push("cheerio");
  if (/saluranCtx|config\.saluran/.test(t)) perlu.push("saluran");
  if (/answerHandler|replyHandler/.test(t)) perlu.push("entry2");
  if (/\bAIRich\b|addTable|addCode\(/.test(t)) perlu.push("AIRich");
  if (/getDatabase\(\)|db\.(update|set|get)[A-Z]/.test(t)) perlu.push("db-mutation");
  return [...new Set(perlu)];
}

const bebas = [];
const terblokir = [];
for (const f of files) {
  let t;
  try {
    t = readFileSync(f, "utf8");
  } catch {
    continue;
  }
  const perlu = kapabilitas(t);
  if (perlu.length === 0) bebas.push(f);
  else terblokir.push([f, perlu.join("+")]);
}

writeFileSync("/tmp/opencode/bebas.txt", bebas.join("\n") + "\n");
writeFileSync(
  "/tmp/opencode/terblokir.txt",
  terblokir.map(([f, k]) => `${k}\t${f}`).join("\n") + "\n",
);
console.log("bebas:", bebas.length, "| terblokir:", terblokir.length);