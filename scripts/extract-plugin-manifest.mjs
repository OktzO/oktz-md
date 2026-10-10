/**
 * extract-plugin-manifest.mjs
 *
 * Read-only static inventory of every file under `plugins/` (recursive, .js)
 * for the Rust rewrite.
 * Parses each plugin with acorn, extracts the `pluginConfig` object literal
 * (static values only), and classifies every plugin into class A-F.
 *
 * This script NEVER evaluates plugin code: `customTrigger` and
 * `games.createPlugin(...)` are recorded as unresolved markers, not run.
 *
 * Usage: node scripts/extract-plugin-manifest.mjs
 * Output: docs/rust-rewrite/plugin-manifest.json
 */

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { createRequire } from "node:module";

const require = createRequire(import.meta.url);
const acorn = require("acorn");

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const ROOT = path.resolve(__dirname, "..");
const PLUGINS_DIR = path.join(ROOT, "plugins");
const OUT_FILE = path.join(ROOT, "docs/rust-rewrite/plugin-manifest.json");

const PARSE_OPTS = {
  ecmaVersion: "latest",
  sourceType: "module",
  allowHashBang: true,
  allowAwaitOutsideFunction: true,
  allowReturnOutsideFunction: true,
};

// ---------------------------------------------------------------------------
// class definitions
// ---------------------------------------------------------------------------
const CLASS_ORDER = ["F", "E", "D", "C", "B", "A"];
const CLASS_WEIGHT = { F: 6, E: 5, D: 4, C: 3, B: 2, A: 1 };

// D-class: WA-specific protocol features. Mandated grep list.
const D_FEATURE_PATTERNS = [
  "interactiveMessage",
  "buttonsMessage",
  "listMessage",
  "nativeFlowMessage",
  "buttonParamsJson",
  "interactiveButtons",
  "prepareWAMessageMedia",
  "generateWAMessageFromContent",
  "waUploadToServer",
  "relayMessage",
  "newsletter",
  "forwardedNewsletterMessageInfo",
  "groupParticipantsUpdate",
  "viewOnce",
  "jadibot",
];

const D_FEATURE_GROUPS = {
  usesInteractive: [
    "interactiveMessage",
    "buttonsMessage",
    "listMessage",
    "nativeFlowMessage",
    "buttonParamsJson",
    "interactiveButtons",
  ],
  usesNewsletter: ["newsletter", "forwardedNewsletterMessageInfo"],
  usesGroupMeta: ["groupParticipantsUpdate"],
  usesRelay: ["relayMessage", "viewOnce"],
  usesWaUpload: [
    "prepareWAMessageMedia",
    "generateWAMessageFromContent",
    "waUploadToServer",
  ],
  usesJadibot: ["jadibot"],
};

// C-class: media / canvas / ffmpeg / OCR / image processing.
const MEDIA_IMPORTS =
  /(@napi-rs\/canvas|brat-canvas|\bcanvas\b|sharp|fluent-ffmpeg|@ffmpeg-installer\/ffmpeg|tesseract\.js|jimp|\bgm\b|file-type|adm-zip|archiver|node-upload-images|btch-downloader)/;
const MEDIA_KEYWORDS =
  /\b(createCanvas|loadImage|ffmpeg\s*\(|tesseract|recognize\s*\(|OCR|\.resize\s*\(|\.composite\s*\(|\.extend\s*\(|toBuffer\s*\(|getImageBuffer)/;

// Repo-local helper modules that perform media processing for plugins that
// never import a media package directly (e.g. `src/lib/ffmpeg.js`).
const MEDIA_HELPER_PATH = /(^|\/)(ffmpeg|canvas|ocr|image|sharp|tesseract|media)\.js$/i;

// E-class: confirmed-dead upstreams (endpoint audit).
const DEAD_HOSTS = new Map([
  ["firefly.maiku.my.id", "Cloudflare 530 / error 1033 (Argo tunnel origin down)"],
  ["ssyoutube.com", "blocked region / upstream refused"],
]);

// F-class: dangerous / illegal / platform-risky. Decision belongs to Oktz.
const F_PATH_RULES = [
  { test: (p) => p.startsWith("plugins/nsfw/"), why: "kategori nsfw/*" },
  { test: (p) => p.startsWith("plugins/pushkontak/"), why: "kategori pushkontak/* (broadcast kontak massal)" },
  { test: (p) => p.startsWith("plugins/jpm/"), why: "kategori jpm/* (kiriman massal / bom chat)" },
  { test: (p) => /^plugins\/(group|canvas)\/fakecall/i.test(p), why: "fake call (penipuan visual)" },
  // canvas/ artifacts used to deceive (fake financial statements, fake quotes)
  {
    test: (p) =>
      /^plugins\/canvas\/(fakebankjago|fakedana|fakegopay|fakeovo|fakeml|fakedev7|iqc2?|iqcpink|ttqc|fakestory\d*)\.js$/.test(
        p,
      ),
    why: "canvas/artefak palsu (rekening/saldo/chat palsu) untuk menipu",
  },
  {
    test: (p) => /^plugins\/tools\/(hack|crack|nmap|ddos|spam|phish|exploit|virus)/i.test(p),
    why: "tools/* yang bisa disalahgunakan (hack/crack/nmap/ddos/spam/phishing)",
  },
];

// Defensive look-alikes that must NOT be auto-flagged as F.
const F_ALLOWLIST = new Map([
  ["plugins/tools/phisingataubukan.js", "pemeriksa phishing (defensif), bukan alat serangan"],
]);

// Network callee detection for precise endpoint attribution.
const NET_CALLEE =
  /^(fetch|request|get|post|nodeFetch|nodefetch|undici|got|superagent|axios|https|http|ky|needle|wretch)$/;
const NET_METHOD =
  /^(get|post|put|patch|delete|head|options|request|stream|fetch)$/;

// Packages that prove an outbound HTTP client is in play. Used to decide class B
// even when the target host is not statically resolvable (dynamic/config URLs),
// without false-positiving on local helpers named get/post/request.
const HTTP_CLIENT_PKG =
  /^(axios|got|node-fetch|undici|request|needle|ky|superagent|wretch|node-fetch-native|superagent-promise|axios-retry)$/;
// Node builtins that carry raw HTTP.
const HTTP_BUILTIN = new Set(["http", "https"]);

// ---------------------------------------------------------------------------
// fs + ast helpers
// ---------------------------------------------------------------------------

function listPluginFiles(dir) {
  const out = [];
  const walk = (d) => {
    for (const entry of fs.readdirSync(d, { withFileTypes: true })) {
      const p = path.join(d, entry.name);
      if (entry.isDirectory()) walk(p);
      else if (entry.name.endsWith(".js")) out.push(p);
    }
  };
  walk(dir);
  return out.sort();
}

function parseFile(file) {
  const source = fs.readFileSync(file, "utf8");
  try {
    return { source, ast: acorn.parse(source, PARSE_OPTS) };
  } catch {
    return { source, ast: null };
  }
}

/** Minimal AST walk (no acorn-walk dependency needed). */
function walk(node, visit, parent = null) {
  if (!node || typeof node.type !== "string") return;
  visit(node, parent);
  for (const key of Object.keys(node)) {
    if (key === "type" || key === "start" || key === "end") continue;
    const child = node[key];
    if (Array.isArray(child)) {
      for (const c of child) if (c && typeof c.type === "string") walk(c, visit, node);
    } else if (child && typeof child.type === "string") {
      walk(child, visit, node);
    }
  }
}

function keyName(prop) {
  if (!prop || prop.type !== "Property") return null;
  if (prop.computed) return null;
  if (prop.key.type === "Identifier") return prop.key.name;
  if (prop.key.type === "Literal") return String(prop.key.value);
  return null;
}

// ---------------------------------------------------------------------------
// static evaluation of object-literal values (never evaluates code)
// ---------------------------------------------------------------------------

function makeEvaluator() {
  const constMap = new Map(); // name -> init node (last declaration wins)

  const collectConsts = (ast) => {
    walk(ast, (node) => {
      if (node.type === "VariableDeclarator" && node.id && node.id.type === "Identifier" && node.init) {
        constMap.set(node.id.name, node.init);
      }
    });
  };

  const evalNode = (node, depth = 0) => {
    if (!node || depth > 6) return { value: undefined, dynamic: true };
    switch (node.type) {
      case "Literal":
        return { value: node.value, dynamic: false };
      case "TemplateLiteral":
        if (node.expressions.length === 0) {
          return { value: node.quasis[0]?.value?.cooked ?? "", dynamic: false };
        }
        return { value: `<template:${node.expressions.length}expr>`, dynamic: true };
      case "ArrayExpression": {
        const arr = [];
        let dynamic = false;
        node.elements.forEach((el, i) => {
          if (!el) {
            arr.push(null);
            return;
          }
          if (el.type === "SpreadElement") {
            arr.push("<spread>");
            dynamic = true;
            return;
          }
          const r = evalNode(el, depth + 1);
          if (r.dynamic) dynamic = true;
          arr.push(r.value);
        });
        return { value: arr, dynamic };
      }
      case "ObjectExpression": {
        const obj = {};
        let dynamic = false;
        for (const p of node.properties) {
          if (p.type === "SpreadElement") {
            dynamic = true;
            continue;
          }
          const k = keyName(p);
          if (k === null) {
            dynamic = true;
            continue;
          }
          const r = evalNode(p.value, depth + 1);
          if (r.dynamic) dynamic = true;
          obj[k] = r.value;
        }
        return { value: obj, dynamic };
      }
      case "UnaryExpression":
        if (node.operator === "-" && node.argument.type === "Literal") {
          return { value: -node.argument.value, dynamic: false };
        }
        if (node.operator === "!") {
          const r = evalNode(node.argument, depth + 1);
          if (!r.dynamic) return { value: !r.value, dynamic: false };
        }
        return { value: `<${node.operator}>`, dynamic: true };
      case "ArrowFunctionExpression":
      case "FunctionExpression":
        return { value: "<function>", dynamic: true };
      case "Identifier": {
        if (constMap.has(node.name)) {
          const init = constMap.get(node.name);
          if (init && init.type === "Literal") return { value: init.value, dynamic: false };
        }
        return { value: `<ref:${node.name}>`, dynamic: true };
      }
      default:
        return { value: `<${node.type}>`, dynamic: true };
    }
  };

  return { collectConsts, evalNode, has: (n) => constMap.has(n) };
}

// ---------------------------------------------------------------------------
// locate the exported `config` binding
// ---------------------------------------------------------------------------

/** Returns the local identifier name bound to the exported `config`. */
function findConfigBinding(ast) {
  let local = null;
  let exportStyle = null;

  for (const stmt of ast.body) {
    // export { pluginConfig as config, handler }
    if (stmt.type === "ExportNamedDeclaration" && stmt.specifiers?.length) {
      for (const spec of stmt.specifiers) {
        const exported = spec.exported.name ?? spec.exported.value;
        if (exported === "config") {
          local = spec.local.name;
          exportStyle = "export { X as config }";
        }
      }
    }
    // export default { config, handler }
    if (stmt.type === "ExportDefaultDeclaration" && stmt.declaration?.type === "ObjectExpression") {
      for (const p of stmt.declaration.properties) {
        if (p.type === "Property" && keyName(p) === "config") {
          local = p.value.type === "Identifier" ? p.value.name : null;
          exportStyle = "export default { config, handler }";
        }
      }
    }
  }

  if (!local) return { local: null, exportStyle: null };
  return { local, exportStyle };
}

// ---------------------------------------------------------------------------
// endpoint extraction (AST-driven, from real network call sites)
// ---------------------------------------------------------------------------

function staticStringsOf(node, constMap, depth = 0) {
  const out = [];
  if (!node || depth > 4) return out;
  switch (node.type) {
    case "Literal":
      if (typeof node.value === "string") out.push(node.value);
      break;
    case "TemplateLiteral":
      // keep every quasi so `https://host/path?x=${y}` still yields the host
      for (const q of node.quasis) out.push(q.value.cooked ?? "");
      break;
    case "Identifier": {
      const init = constMap.get(node.name);
      if (init) out.push(...staticStringsOf(init, constMap, depth + 1));
      break;
    }
    case "MemberExpression":
      out.push(...staticStringsOf(node.object, constMap, depth + 1));
      break;
    case "ArrayExpression":
      for (const el of node.elements) out.push(...staticStringsOf(el, constMap, depth + 1));
      break;
    case "CallExpression":
      for (const a of node.arguments) out.push(...staticStringsOf(a, constMap, depth + 1));
      break;
    case "ConditionalExpression":
      out.push(...staticStringsOf(node.consequent, constMap, depth + 1));
      out.push(...staticStringsOf(node.alternate, constMap, depth + 1));
      break;
    case "BinaryExpression":
      out.push(...staticStringsOf(node.left, constMap, depth + 1));
      out.push(...staticStringsOf(node.right, constMap, depth + 1));
      break;
    default:
      break;
  }
  return out;
}

function isNetworkCall(node) {
  if (node.type !== "CallExpression") return false;
  const c = node.callee;
  if (c.type === "Identifier") return NET_CALLEE.test(c.name);
  if (c.type === "MemberExpression" && c.object.type === "Identifier") {
    return (
      NET_CALLEE.test(c.object.name) &&
      (c.property.type === "Identifier" || c.property.type === "Literal")
        ? NET_METHOD.test(c.property.name ?? c.property.value)
        : false
    );
  }
  return false;
}

function hostOf(urlish) {
  const m = /https?:\/\/([a-z0-9]([a-z0-9-]*[a-z0-9])?(\.[a-z0-9]([a-z0-9-]*[a-z0-9])?)+)(:\d+)?/i.exec(
    urlish,
  );
  return m ? m[1].toLowerCase().replace(/^www\./, "") : null;
}

const fileAnalysisCache = new Map();

/** Parse a file, returning AST + const map + the hosts its own call sites hit. */
function analyzeFile(file) {
  if (fileAnalysisCache.has(file)) return fileAnalysisCache.get(file);
  let result;
  if (!fs.existsSync(file)) {
    result = { exists: false, ast: null, source: "", constMap: new Map(), hosts: new Set(), imports: [] };
  } else {
    const { source, ast } = parseFile(file);
    const constMap = new Map();
    const imports = [];
    if (ast) {
      walk(ast, (node) => {
        if (node.type === "VariableDeclarator" && node.id?.type === "Identifier" && node.init) {
          constMap.set(node.id.name, node.init);
        }
        if (
          node.type === "ImportDeclaration" ||
          ((node.type === "ExportNamedDeclaration" || node.type === "ExportAllDeclaration") && node.source)
        ) {
          imports.push(node.source.value);
        }
      });
    }
    const hosts = new Set();
    // Does this file actually perform an outbound HTTP request? Tracked
    // separately from `hosts` because a real request can target a dynamic
    // URL (`` `${cfg.domain}/x` ``) that yields no literal host.
    const importedClients = new Set();
    for (const spec of imports) {
      if (spec.startsWith(".") || spec.startsWith("/")) continue;
      const pkg = spec.replace(/^node:/, "").split("/")[0];
      if (HTTP_CLIENT_PKG.test(pkg) || HTTP_BUILTIN.has(pkg)) importedClients.add(pkg);
    }
    let makesHttpCall = false;
    if (ast) {
      walk(ast, (node) => {
        if (!isNetworkCall(node)) return;
        makesHttpCall = true;
        for (const arg of node.arguments) {
          for (const s of staticStringsOf(arg, constMap)) {
            const h = hostOf(s);
            if (h) hosts.add(h);
          }
        }
      });
      // Bare global `fetch(...)` with no axios-style import.
      if (!makesHttpCall) {
        walk(ast, (node) => {
          if (node.type === "CallExpression" && node.callee.type === "Identifier" && node.callee.name === "fetch") {
            makesHttpCall = true;
          }
        });
      }
    }
    // An imported HTTP client is itself strong evidence of outbound HTTP,
    // even when every call site is routed through a wrapper.
    if (importedClients.size > 0) makesHttpCall = true;
    result = { exists: true, ast, source, constMap, hosts, imports, makesHttpCall };
  }
  fileAnalysisCache.set(file, result);
  return result;
}

function resolveImport(fromFile, spec) {
  if (!spec.startsWith(".") && !spec.startsWith("/")) return null;
  const base = path.resolve(path.dirname(fromFile), spec);
  if (fs.existsSync(base) && fs.statSync(base).isFile()) return base;
  return null;
}

/** Transitive closure over repo-local (relative) imports only. */
function localClosure(rootFile, maxDepth = 6) {
  const seen = new Set([rootFile]);
  const queue = [[rootFile, 0]];
  const hosts = new Set();
  const via = new Map();
  let makesHttpCall = false;

  while (queue.length) {
    const [file, depth] = queue.shift();
    const info = analyzeFile(file);
    if (info.makesHttpCall) makesHttpCall = true;
    for (const h of info.hosts) {
      if (file === rootFile) hosts.add(h);
      else if (!via.has(h)) {
        hosts.add(h);
        via.set(h, path.relative(ROOT, file));
      }
    }
    if (depth >= maxDepth) continue;
    for (const spec of info.imports) {
      const next = resolveImport(file, spec);
      if (next && !seen.has(next)) {
        seen.add(next);
        queue.push([next, depth + 1]);
      }
    }
  }
  return { files: seen, hosts, via, makesHttpCall };
}

// ---------------------------------------------------------------------------
// classification
// ---------------------------------------------------------------------------

function matchText(haystack, patterns) {
  return patterns.filter((p) => haystack.includes(p));
}

function classify(plugin) {
  const reasons = [];
  const candidates = [];

  // ---- F ----
  let fWhy = null;
  const allow = F_ALLOWLIST.get(plugin.file);
  for (const rule of F_PATH_RULES) {
    if (rule.test(plugin.file)) {
      fWhy = rule.why;
      break;
    }
  }
  if (fWhy) {
    if (allow) {
      reasons.push(`F-allowlist: ${allow}`);
    } else {
      candidates.push({ cls: "F", reason: fWhy });
    }
  }

  // ---- E ----
  const deadHits = plugin.externalEndpoints.filter((h) => DEAD_HOSTS.has(h));
  // dead host may also be reached through a local helper
  const deadVia = [];
  for (const [host, why] of DEAD_HOSTS) {
    if (plugin.externalEndpointsVia[host]) deadVia.push(`${host} (via ${plugin.externalEndpointsVia[host]}) — ${why}`);
  }
  const dead = [...new Set([...deadHits.map((h) => `${h} — ${DEAD_HOSTS.get(h)}`), ...deadVia])];
  if (dead.length) candidates.push({ cls: "E", reason: `endpoint mati: ${dead.join("; ")}` });

  // ---- D ----
  const dHits = matchText(plugin.signalText, D_FEATURE_PATTERNS);
  if (dHits.length) candidates.push({ cls: "D", reason: `fitur WA khusus: ${dHits.join(", ")}` });

  // ---- C ----
  const cHits = [];
  if (plugin.flags.usesMedia) cHits.push(plugin.mediaEvidence.join(", "));
  if (cHits.length) candidates.push({ cls: "C", reason: `media/processing: ${cHits.join("; ")}` });

  // ---- B ----
  const bHits = [];
  if (plugin.externalEndpoints.length) bHits.push(`HTTP ke ${plugin.externalEndpoints.join(", ")}`);
  else if (plugin.usesHttpViaHelper) bHits.push("HTTP lewat helper repo (tanpa host langsung)");
  else if (plugin.makesHttpCall) bHits.push("HTTP call terdeteksi; host tidak static-resolve (URL dinamis/from-config)");
  if (bHits.length) candidates.push({ cls: "B", reason: bHits.join("; ") });

  if (!candidates.length) {
    return { class: "A", classReason: "murni teks + config statis (tanpa HTTP, media, atau fitur WA khusus)" };
  }

  candidates.sort((a, b) => CLASS_WEIGHT[b.cls] - CLASS_WEIGHT[a.cls]);
  const winner = candidates[0].cls;
  const allReasons = candidates.map((c) => `${c.cls}: ${c.reason}`);
  return { class: winner, classReason: allReasons.join(" | "), classAll: candidates.map((c) => c.cls) };
}

// ---------------------------------------------------------------------------
// per-plugin extraction
// ---------------------------------------------------------------------------

function extractPlugin(file) {
  const rel = path.relative(ROOT, file);
  const info = analyzeFile(file);
  const source = info.source;
  const dir = path.relative(PLUGINS_DIR, file).split(path.sep)[0];

  const base = {
    file: rel.split(path.sep).join("/"),
    name: null,
    alias: [],
    category: null,
    description: null,
    usage: null,
    example: null,
    cooldown: null,
    energi: null,
    isEnabled: null,
    isOwner: null,
    isPremium: null,
    isGroup: null,
    isPrivate: null,
    isAdmin: null,
    isBotAdmin: null,
    skipRegistration: null,
    isBanned: null,
    noPrefix: null,
    customTrigger: null,
    exportStyle: null,
    runtimeGenerated: false,
    needsRuntimeConfig: false,
    needsRuntimeConfigReasons: [],
    portStatus: "pending",
  };

if (!info.ast) {
    base.needsRuntimeConfig = true;
    base.needsRuntimeConfigReasons.push("gagal di-parse (syntax tidak didukung acorn)");
    base.category = base.category ?? dir;
    const cls = classify({
      file: base.file,
      externalEndpoints: [],
      externalEndpointsVia: {},
      flags: {},
      signalText: source,
      mediaEvidence: [],
      usesHttpViaHelper: false,
    });
    return {
      ...base,
      ...cls,
      externalEndpoints: [],
      awaitingDecision: false,
      oktzDecision: cls.class === "F" ? "port-all" : undefined,
      flags: {
        usesInteractive: false,
        usesNewsletter: false,
        usesGroupMeta: false,
        usesRelay: false,
        usesWaUpload: false,
        usesJadibot: false,
        usesMedia: false,
        externalEndpoints: [],
      },
    };
  }

  const evaluator = makeEvaluator();
  evaluator.collectConsts(info.ast);

  // --- locate config ---
  const { local, exportStyle } = findConfigBinding(info.ast);
  base.exportStyle = exportStyle;

  let configNode = null;

  // resolve the init node of the exported binding
  if (local) {
    const init = findInitFor(info.ast, local);
    if (init && init.type === "ObjectExpression") {
      configNode = { ...evaluator.evalNode(init), node: init };
    } else if (init) {
      // games.createPlugin(...) & friends
      const callName = calleeName(init);
      base.runtimeGenerated = true;
      base.needsRuntimeConfig = true;
      base.needsRuntimeConfigReasons.push(
        `config di-generate runtime via ${callName || init.type}; tidak bisa di-resolve tanpa eksekusi`,
      );
      // partial info from games.register('id', {...})
      const reg = findGamesRegister(info.ast);
      if (reg) {
        base.name = reg.name;
        base.alias = reg.alias ?? [];
        base.description = reg.description ?? null;
        base.category = base.category ?? dir;
      } else {
        base.category = base.category ?? dir;
      }
    } else {
      base.needsRuntimeConfig = true;
      base.needsRuntimeConfigReasons.push(`binding '${local}' tidak punya initializer literal`);
      base.category = base.category ?? dir;
    }
  } else {
    base.needsRuntimeConfig = true;
    base.needsRuntimeConfigReasons.push("tidak ditemukan ekspor `config`");
    base.category = base.category ?? dir;
  }

  if (configNode && configNode.value && typeof configNode.value === "object") {
    const c = configNode.value;
    for (const f of [
      "name",
      "category",
      "description",
      "usage",
      "example",
      "cooldown",
      "energi",
      "isEnabled",
      "isOwner",
      "isPremium",
      "isGroup",
      "isPrivate",
      "isAdmin",
      "isBotAdmin",
      "skipRegistration",
      "isBanned",
    ]) {
      if (c[f] !== undefined) base[f] = c[f];
    }
    base.alias = Array.isArray(c.alias) ? c.alias : c.alias !== undefined ? [c.alias] : [];
    base.noPrefix = c.noPrefix ?? null;
    if (c.customTrigger !== undefined) {
      base.customTrigger = c.customTrigger;
      if (c.customTrigger === "<function>") {
        base.needsRuntimeConfig = true;
        base.needsRuntimeConfigReasons.push("customTrigger adalah function; butuh runtime config");
      }
    }
    if (configNode.dynamic) {
      base.needsRuntimeConfig = true;
      base.needsRuntimeConfigReasons.push("ada field config yang non-literal (dinamis)");
    }
  }

  if (!base.category) base.category = dir;
  if (!base.name && rel) base.name = path.basename(file, ".js");

  // --- flags ---
  const signalText = source;
  const flags = {};
  for (const [flag, pats] of Object.entries(D_FEATURE_GROUPS)) {
    flags[flag] = matchText(signalText, pats).length > 0;
  }

  // --- endpoints: direct + transitive local helpers ---
  const closure = localClosure(file, 6);

  const mediaEvidence = [];
  for (const imp of info.imports) {
    if (MEDIA_IMPORTS.test(imp)) mediaEvidence.push(`import ${imp}`);
  }
  const importPaths = info.imports.join(" ");
  if (MEDIA_KEYWORDS.test(signalText) && !mediaEvidence.length) mediaEvidence.push("keyword media di body");
  // Media work delegated to a repo-local helper (ffmpeg/canvas/ocr) still
  // makes this a C-class port: the Rust side must reimplement that helper.
  for (const f2 of closure.files) {
    if (f2 === file) continue;
    if (!MEDIA_HELPER_PATH.test(path.relative(ROOT, f2))) continue;
    mediaEvidence.push(`helper media: ${path.relative(ROOT, f2)}`);
  }
  flags.usesMedia = mediaEvidence.length > 0;

  const directHosts = new Set(info.hosts);
  const externalEndpointsVia = {};
  for (const h of closure.hosts) {
    if (!directHosts.has(h) && closure.via.has(h)) externalEndpointsVia[h] = closure.via.get(h);
  }
  const externalEndpoints = [...closure.hosts].sort();
  const usesHttpViaHelper =
    externalEndpointsVia && Object.keys(externalEndpointsVia).length > 0 && directHosts.size === 0;

  const classification = classify({
    file: base.file,
    externalEndpoints,
    externalEndpointsVia,
    flags,
    signalText,
    mediaEvidence,
    usesHttpViaHelper,
    makesHttpCall: closure.makesHttpCall,
  });

  const out = {
    ...base,
    externalEndpoints,
    ...classification,
    flags: {
      usesInteractive: flags.usesInteractive,
      usesNewsletter: flags.usesNewsletter,
      usesGroupMeta: flags.usesGroupMeta,
      usesRelay: flags.usesRelay,
      usesWaUpload: flags.usesWaUpload,
      usesJadibot: flags.usesJadibot,
      usesMedia: flags.usesMedia,
      externalEndpoints,
    },
  };
  if (Object.keys(externalEndpointsVia).length) {
    out.flags.externalEndpointsViaHelper = externalEndpointsVia;
  }
  if (out.awaitingDecision === undefined) {
    out.awaitingDecision = classification.class === "F";
  }
  if (classification.class === "F") {
    // Oktz (GATE 0, 2026-10-10): port semua kelas F, jangan ada yang dibuang diam-diam.
    out.oktzDecision = "port-all";
    out.awaitingDecision = false;
  }
  return out;
}

// helpers used above ---------------------------------------------------

/**
 * Resolve the initializer node bound to `name`.
 * Handles both `const X = ...` and `const { config: X } = ...`.
 * For destructuring, returns the matching property value when the init is an
 * ObjectExpression, otherwise the whole init (so the caller can flag it as
 * runtime-generated).
 */
function findInitFor(ast, name) {
  let found = null;
  walk(ast, (node) => {
    if (node.type !== "VariableDeclarator" || !node.init) return;

    if (node.id.type === "Identifier" && node.id.name === name) {
      found = node.init;
      return;
    }
    if (node.id.type === "ObjectPattern") {
      for (const prop of node.id.properties) {
        if (prop.type !== "Property") continue;
        const localName =
          prop.value.type === "Identifier"
            ? prop.value.name
            : prop.value.type === "AssignmentPattern" && prop.value.left.type === "Identifier"
              ? prop.value.left.name
              : null;
        if (localName !== name) continue;
        if (node.init.type === "ObjectExpression") {
          const k = keyName(prop);
          const match = node.init.properties.find((p) => p.type === "Property" && keyName(p) === k);
          found = match ? match.value : node.init;
        } else {
          // e.g. `const { config: pluginConfig } = games.createPlugin('x')`
          found = node.init;
        }
        return;
      }
    }
  });
  return found;
}

function calleeName(node) {
  if (!node || node.type !== "CallExpression") return null;
  const c = node.callee;
  if (c.type === "MemberExpression") {
    const obj = c.object.type === "MemberExpression"
      ? `${c.object.object.name}.${c.object.property.name}`
      : c.object.name;
    return `${obj}.${c.property.name}`;
  }
  return c.name ?? null;
}

function findGamesRegister(ast) {
  let out = null;
  walk(ast, (node) => {
    if (
      node.type === "CallExpression" &&
      calleeName(node) === "games.register" &&
      node.arguments.length >= 2
    ) {
      const idNode = node.arguments[0];
      const optNode = node.arguments[1];
      if (idNode.type === "Literal" && optNode.type === "ObjectExpression") {
        const o = {};
        for (const p of optNode.properties) {
          if (p.type !== "Property") continue;
          const k = keyName(p);
          if (k && p.value.type === "Literal") o[k] = p.value.value;
          else if (k === "alias" && p.value.type === "ArrayExpression") {
            o.alias = p.value.elements.filter((e) => e?.type === "Literal").map((e) => e.value);
          }
        }
        out = { name: idNode.value, alias: o.alias ?? [], description: o.description ?? null, title: o.title ?? null };
      }
    }
  });
  return out;
}

// ---------------------------------------------------------------------------
// main
// ---------------------------------------------------------------------------

function main() {
  const files = listPluginFiles(PLUGINS_DIR);
  const plugins = [];
  const parseFailures = [];

  for (const file of files) {
    try {
      const p = extractPlugin(file);
      if (!analyzeFile(file).ast) parseFailures.push(path.relative(ROOT, file));
      plugins.push(p);
    } catch (err) {
      parseFailures.push(`${path.relative(ROOT, file)} :: ${err.message}`);
    }
  }

  const manifest = {
    generatedAt: "2026-10-10",
    sourceGlob: "plugins/**/*.js",
    total: plugins.length,
    parseFailures,
    classCounts: {},
    categoryCounts: {},
    plugins,
  };

  for (const p of plugins) manifest.classCounts[p.class] = (manifest.classCounts[p.class] || 0) + 1;
  for (const p of plugins) manifest.categoryCounts[p.category] = (manifest.categoryCounts[p.category] || 0) + 1;

  fs.mkdirSync(path.dirname(OUT_FILE), { recursive: true });
  fs.writeFileSync(OUT_FILE, JSON.stringify(manifest, null, 2) + "\n");

  console.log(`wrote ${path.relative(ROOT, OUT_FILE)}`);
  console.log(`total=${plugins.length} parseFailures=${parseFailures.length}`);
  console.log("classes:", JSON.stringify(manifest.classCounts));
}

// silence the unused-warning free linter while keeping the helper honest
void evaluatorHasInit;
function evaluatorHasInit() {
  return true;
}

main();