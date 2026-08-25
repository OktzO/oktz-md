// data/husbu/index.js
// Loader pool husbu: merge file franchise, validasi entry, dedupe nama, hitung bobot tier.
import { logger } from "../../src/lib/ourin-logger.js";
import { husbus as naruto } from "./naruto.js";
import { husbus as onePiece } from "./one-piece.js";
import { husbus as bleach } from "./bleach.js";
import { husbus as jujutsu } from "./jujutsu-kaisen.js";
import { husbus as kimetsu } from "./kimetsu.js";
import { husbus as mha } from "./my-hero-academia.js";
import { husbus as aot } from "./attack-on-titan.js";
import { husbus as dragonBall } from "./dragon-ball.js";
import { husbus as hunterXHunter } from "./hunter-x-hunter.js";
import { husbus as blackClover } from "./black-clover.js";
import { husbus as haikyuu } from "./haikyuu.js";
import { husbus as kuroko } from "./kuroko.js";
import { husbus as blueLock } from "./blue-lock.js";
import { husbus as fairyTail } from "./fairy-tail.js";
import { husbus as sao } from "./sword-art-online.js";
import { husbus as rezero } from "./rezero.js";
import { husbus as fate } from "./fate.js";
import { husbus as genshin } from "./genshin-impact.js";
import { husbus as honkaiStarRail } from "./honkai-star-rail.js";
import { husbus as bsd } from "./bungo-stray-dogs.js";
import { husbus as spyFamily } from "./spy-x-family.js";
import { husbus as chainsaw } from "./chainsaw-man.js";
import { husbus as tokyoRevengers } from "./tokyo-revengers.js";
import { husbus as opm } from "./one-punch-man.js";
import { husbus as deathNote } from "./death-note.js";
import { husbus as codeGeass } from "./code-geass.js";
import { husbus as fma } from "./fullmetal-alchemist.js";
import { husbus as tokyoGhoul } from "./tokyo-ghoul.js";
import { husbus as sds } from "./seven-deadly-sins.js";
import { husbus as kaguya } from "./kaguya.js";
import { husbus as cote } from "./cote.js";
import { husbus as assassClass } from "./assassination-classroom.js";
import { husbus as oregairu } from "./oregairu.js";
import { husbus as bakaTest } from "./baka-to-test.js";

export const TIER_WEIGHTS = { Common: 55, Rare: 25, Epic: 13, Legendary: 5.5, Mythic: 1.5 };
export const VALID_HUSB_PERSONALITIES = [
  "tsundere", "kuudere", "genki", "yandere", "dandere", "oji-san",
  "playboy", "prince", "badboy", "sunao", "femboy",
];

const FRANCHISES = [
  naruto, onePiece, bleach, jujutsu, kimetsu, mha, aot, dragonBall,
  hunterXHunter, blackClover, haikyuu, kuroko, blueLock, fairyTail, sao, rezero, fate, genshin,
  honkaiStarRail, bsd, spyFamily, chainsaw, tokyoRevengers, opm, deathNote, codeGeass,
  fma, tokyoGhoul, sds, kaguya, cote, assassClass, oregairu, bakaTest,
];

let cache = null;

function buildPool() {
  const seen = new Set();
  const all = [];
  for (const list of FRANCHISES) {
    for (const h of list || []) {
      const missing = !h || !h.name || !h.series || !h.age || !h.height || !h.weight || !h.tier || !h.personality;
      if (missing) { logger.warn("husbu", `skip invalid entry (${h?.name || "no-name"})`); continue; }
      if (!TIER_WEIGHTS[h.tier]) { logger.warn("husbu", `skip invalid tier ${h.tier} (${h.name})`); continue; }
      if (!VALID_HUSB_PERSONALITIES.includes(h.personality)) { logger.warn("husbu", `skip invalid personality ${h.personality} (${h.name})`); continue; }
      const key = h.name.trim().toLowerCase();
      if (seen.has(key)) { logger.warn("husbu", `duplicate name: ${h.name}`); continue; }
      seen.add(key);
      all.push({ ...h, name: h.name.trim(), series: h.series.trim(), keyword: h.keyword || `${h.name} ${h.series} anime` });
    }
  }
  const tierCount = {};
  for (const h of all) tierCount[h.tier] = (tierCount[h.tier] || 0) + 1;
  for (const h of all) h.rollWeight = TIER_WEIGHTS[h.tier] / tierCount[h.tier];
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
  return pool.filter(h =>
    h.name.toLowerCase().includes(q) ||
    h.series.toLowerCase().includes(q) ||
    h.tier.toLowerCase().includes(q) ||
    h.personality.toLowerCase().includes(q)
  );
}
