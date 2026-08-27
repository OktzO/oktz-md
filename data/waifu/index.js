// data/waifu/index.js
// Loader pool waifu: merge file franchise, validasi entry, dedupe nama, hitung bobot tier.
import { logger } from "../../src/lib/ourin-logger.js";
import { waifus as naruto } from "./naruto.js";
import { waifus as onePiece } from "./one-piece.js";
import { waifus as bleach } from "./bleach.js";
import { waifus as kimetsu } from "./kimetsu.js";
import { waifus as jujutsu } from "./jujutsu-kaisen.js";
import { waifus as mha } from "./my-hero-academia.js";
import { waifus as dragonBall } from "./dragon-ball.js";
import { waifus as sao } from "./sword-art-online.js";
import { waifus as rezero } from "./rezero.js";
import { waifus as fate } from "./fate.js";
import { waifus as genshin } from "./genshin-impact.js";
import { waifus as hsr } from "./honkai-star-rail.js";
import { waifus as kaguya } from "./kaguya-sama.js";
import { waifus as konosuba } from "./konosuba.js";
import { waifus as chainsaw } from "./chainsaw-man.js";
import { waifus as spyFamily } from "./spy-x-family.js";
import { waifus as aot } from "./attack-on-titan.js";
import { waifus as violet } from "./violet-evergarden.js";
import { waifus as franxx } from "./darling-in-franxx.js";
import { waifus as steins } from "./steins-gate.js";
import { waifus as kOn } from "./k-on.js";
import { waifus as oshi } from "./oshi-no-ko.js";
import { waifus as dateAlive } from "./date-a-live.js";
import { waifus as dxd } from "./highschool-dxd.js";
import { waifus as nier } from "./nier.js";
import { waifus as cote } from "./classroom-of-the-elite.js";
import { waifus as frieren } from "./frieren.js";
import { waifus as loveLive } from "./love-live.js";
import { waifus as fairyTail } from "./fairy-tail.js";
import { waifus as komi } from "./komi.js";
import { waifus as dressUp } from "./my-dress-up-darling.js";
import { waifus as lycoris } from "./lycoris-recoil.js";
import { waifus as bocchi } from "./bocchi-the-rock.js";
import { waifus as mushoku } from "./mushoku-tensei.js";
import { waifus as dandadan } from "./dandadan.js";
import { waifus as toradora } from "./toradora.js";
import { waifus as monogatari } from "./monogatari.js";
import { waifus as nikke } from "./nikke.js";
import { waifus as blueArchive } from "./blue-archive.js";
import { waifus as matoSeihei } from "./mato-seihei.js";

export const TIER_WEIGHTS = { Common: 55, Rare: 25, Epic: 13, Legendary: 5.5, Mythic: 1.5 };
export const VALID_PERSONALITIES = [
  "tsundere", "deredere", "kuudere", "dandere", "yandere",
  "onee-san", "genki", "himedere", "ojou-sama", "amayadori",
];

const FRANCHISES = [
  naruto, onePiece, bleach, kimetsu, jujutsu, mha, dragonBall, sao, rezero,
  fate, genshin, hsr, kaguya, konosuba, chainsaw, spyFamily, aot, violet,
  franxx, steins, kOn, oshi, dateAlive, dxd, nier, cote, frieren, loveLive,
  fairyTail, komi, dressUp, lycoris, bocchi, mushoku, dandadan, toradora,
  monogatari, nikke, blueArchive, matoSeihei,
];

let cache = null;

function buildPool() {
  const seen = new Set();
  const all = [];
  for (const list of FRANCHISES) {
    for (const w of list || []) {
      const missing = !w || !w.name || !w.series || !w.age || !w.height || !w.weight || !w.tier || !w.personality;
      if (missing) { logger.warn("waifu", `skip invalid entry (${w?.name || "no-name"})`); continue; }
      if (!TIER_WEIGHTS[w.tier]) { logger.warn("waifu", `skip invalid tier ${w.tier} (${w.name})`); continue; }
      if (!VALID_PERSONALITIES.includes(w.personality)) { logger.warn("waifu", `skip invalid personality ${w.personality} (${w.name})`); continue; }
      const key = w.name.trim().toLowerCase();
      if (seen.has(key)) { logger.warn("waifu", `duplicate name: ${w.name}`); continue; }
      seen.add(key);
      all.push({ ...w, name: w.name.trim(), series: w.series.trim(), keyword: w.keyword || `${w.name} ${w.series} anime` });
    }
  }
  const tierCount = {};
  for (const w of all) tierCount[w.tier] = (tierCount[w.tier] || 0) + 1;
  for (const w of all) w.rollWeight = TIER_WEIGHTS[w.tier] / tierCount[w.tier];
  return all;
}

export function getPool() {
  if (!cache) cache = buildPool();
  return cache;
}

export function searchPool(query = "") {
  const q = String(query).toLowerCase();
  const pool = getPool();
  if (!q) return pool;
  return pool.filter(w =>
    w.name.toLowerCase().includes(q) ||
    w.series.toLowerCase().includes(q) ||
    w.tier.toLowerCase().includes(q) ||
    w.personality.toLowerCase().includes(q)
  );
}