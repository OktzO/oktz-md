#!/usr/bin/env node
// Secret-leak guard for CI.
//
// Two checks:
//   1. The env file must be ignored by git and must not be tracked.
//   2. No value from the env file may appear verbatim in a tracked file.
//
// The detector NEVER prints a secret value. On failure it prints the variable
// NAME and the file path (plus line number) only, so the log is safe to paste
// into an issue. Exits non-zero so the workflow fails loudly.
//
// Why value-exact matching: a key name alone (`APIKEY_CUKI`) is not a leak —
// the code legitimately references variable names. Only the literal value in a
// tracked file is the finding.
//
// Usage: node scripts/secret-guard.mjs [--env-file <path>]
//   Default env file is .env. CI passes a synthetic file instead, because a
//   real .env must never be present on a runner. Passing .env.example is
//   deliberately NOT supported: the template holds placeholder defaults
//   (BRAT_FONT_URL, BRAT_EMOJI_URL) that legitimately appear in source, so
//   seeding from it produces false positives, not detections.
import { readFileSync, existsSync } from "node:fs";
import { execFileSync } from "node:child_process";
import path from "node:path";

const ROOT = process.cwd();

const envFlagIndex = process.argv.indexOf("--env-file");
const ENV_FILE = path.resolve(
  ROOT,
  envFlagIndex !== -1 ? process.argv[envFlagIndex + 1] : ".env",
);
// Shown in messages. An absolute path outside the repo (CI's synthetic file)
// reads better by name than as a pile of ../ segments.
const ENV_LABEL = path.relative(ROOT, ENV_FILE).startsWith("..")
  ? path.basename(ENV_FILE)
  : path.relative(ROOT, ENV_FILE);

// Values shorter than this are ignored. A 1-5 char "secret" produces false
// positives on any prose that happens to contain those characters. The floor
// is 6 because the shortest real key in this project is exactly 6 characters
// — and that one was a genuine leak, so lowering the floor further would buy
// nothing but noise.
const MIN_VALUE_LENGTH = 6;

function git(args) {
  // stderr is suppressed on purpose: `ls-files --error-unmatch` and
  // `check-ignore -q` exit non-zero as a normal answer, and their message
  // would otherwise be printed as noise next to our own report.
  return execFileSync("git", args, {
    cwd: ROOT,
    encoding: "utf8",
    stdio: ["ignore", "pipe", "ignore"],
  });
}

function fail(lines) {
  process.stderr.write(lines.join("\n") + "\n");
  process.exit(1);
}

const findings = [];

// The ignore/tracked check is about the real credential file, so it always
// looks at `.env` itself even when --env-file points somewhere else. Checking
// a synthetic file's git status would answer a question nobody asked.
const CREDENTIAL_FILE = ".env";

// ── Check 1: the credential file must be ignored and untracked ───────────────
if (existsSync(ENV_FILE)) {
  let ignored = false;
  try {
    git(["check-ignore", "-q", CREDENTIAL_FILE]);
    ignored = true;
  } catch {
    ignored = false;
  }
  if (!ignored) {
    findings.push(
      `guard: ${CREDENTIAL_FILE} exists but is NOT covered by .gitignore`,
    );
  }
  let tracked = false;
  try {
    git(["ls-files", "--error-unmatch", CREDENTIAL_FILE]);
    tracked = true;
  } catch {
    tracked = false;
  }
  if (tracked) {
    findings.push(
      `guard: ${CREDENTIAL_FILE} is TRACKED by git — it must be untracked`,
    );
  }
} else {
  findings.push(
    `guard: ${ENV_LABEL} not found — cannot verify secrets. ` +
      `Copy .env.example to .env before running this guard locally.`,
  );
}

// ── Check 2: no env value may appear in a tracked file ──────────────────────
if (existsSync(ENV_FILE)) {
  const envRelPath = path.relative(ROOT, ENV_FILE);
  const pairs = readFileSync(ENV_FILE, "utf8")
    .split("\n")
    .map((line) => line.match(/^\s*([A-Za-z_][A-Za-z0-9_]*)\s*=\s*(.*)$/))
    .filter(Boolean)
    .map((m) => ({
      name: m[1],
      // Surrounding quotes are stripped so a quoted value still matches.
      // A trailing `# comment` is deliberately NOT stripped: shortening the
      // needle could hide a leak that the operator would have to see.
      value: m[2].trim().replace(/^["']|["']$/g, ""),
    }))
    .filter((p) => p.value.length >= MIN_VALUE_LENGTH);

  const files = git(["ls-files", "-z"]).split("\0").filter(Boolean);

  for (const p of pairs) {
    for (const rel of files) {
      // Never compare the env file against itself, and never against the
      // template: the template documents the same variable NAMES with
      // placeholder values, so matching it would flag every empty assignment.
      if (rel === envRelPath || rel === ".env" || rel === ".env.example") {
        continue;
      }
      let text;
      try {
        text = readFileSync(path.join(ROOT, rel), "utf8");
      } catch {
        continue;
      }
      if (!text.includes(p.value)) continue;
      const lineNo = text.slice(0, text.indexOf(p.value)).split("\n").length;
      // Variable name + path + line only. The value is never interpolated,
      // not even into the failure message, so this log is safe to share.
      findings.push(
        `LEAK: value of ${p.name} appears verbatim in tracked file ${rel}:${lineNo}`,
      );
    }
  }
}

if (findings.length > 0) {
  fail([
    "",
    "✗ SECRET LEAK GUARD FAILED",
    "",
    ...findings,
    "",
    "Rotate every affected variable, then rewrite the offending lines.",
    "Do NOT paste the values into a commit message, issue, or CI log.",
    "This detector prints variable names and paths only, by design.",
    "",
  ]);
}

process.stdout.write(
  `✓ secret leak guard: ${CREDENTIAL_FILE} is gitignored and untracked; ` +
    `no value from ${ENV_LABEL} appears in a tracked file\n`,
);
