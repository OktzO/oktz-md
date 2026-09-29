#!/usr/bin/env node
// Secret-leak guard for CI.
//
// Two checks:
//   1. `.env` must be ignored by git and must not be tracked.
//   2. No value from `.env` may appear verbatim in a tracked file.
//
// The detector NEVER prints a secret value. On failure it prints the variable
// NAME and the file path (plus line number) only, so the log is safe to paste
// into an issue. Exits non-zero so the workflow fails loudly.
//
// Why value-exact matching: a key name alone (`APIKEY_CUKI`) is not a leak —
// the code legitimately references variable names. Only the literal value in a
// tracked file is the finding.
import { readFileSync, existsSync } from "node:fs";
import { execFileSync } from "node:child_process";
import path from "node:path";

const ROOT = process.cwd();
const ENV_FILE = path.join(ROOT, ".env");

// Values shorter than this are ignored. A 1-6 char "secret" produces false
// positives on any prose that happens to contain those characters (the CUKI
// key is 6 chars and was itself a real leak, so the floor is deliberately low,
// but 0-length and single-char values are not secrets).
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

// ── Check 1: .env must be ignored and untracked ──────────────────────────────
if (existsSync(ENV_FILE)) {
  let ignored = false;
  try {
    git(["check-ignore", "-q", ".env"]);
    ignored = true;
  } catch {
    ignored = false;
  }
  if (!ignored) {
    findings.push("guard: .env exists but is NOT covered by .gitignore");
  }
  let tracked = false;
  try {
    git(["ls-files", "--error-unmatch", ".env"]);
    tracked = true;
  } catch {
    tracked = false;
  }
  if (tracked) {
    findings.push("guard: .env is TRACKED by git — it must be untracked");
  }
} else {
  findings.push(
    "guard: .env not found — cannot verify secrets. " +
      "Copy .env.example to .env before running this guard.",
  );
}

// ── Check 2: no .env value may appear in a tracked file ─────────────────────
if (existsSync(ENV_FILE)) {
  const pairs = readFileSync(ENV_FILE, "utf8")
    .split("\n")
    .map((line) => line.match(/^\s*([A-Za-z_][A-Za-z0-9_]*)\s*=\s*(.*)$/))
    .filter(Boolean)
    .map((m) => ({
      name: m[1],
      // strip surrounding quotes and trailing comments are NOT stripped:
      // a quoted value must still be matched in full, and stripping a trailing
      // `# comment` could hide a leak by shortening the needle.
      value: m[2].trim().replace(/^["']|["']$/g, ""),
    }))
    .filter((p) => p.value.length >= MIN_VALUE_LENGTH);

  const files = git(["ls-files", "-z"]).split("\0").filter(Boolean);

  for (const p of pairs) {
    for (const rel of files) {
      if (rel === ".env" || rel === ".env.example") continue;
      let text;
      try {
        text = readFileSync(path.join(ROOT, rel), "utf8");
      } catch {
        continue;
      }
      if (!text.includes(p.value)) continue;
      const lineNo = text.slice(0, text.indexOf(p.value)).split("\n").length;
      // var name + path + line only. The value is never interpolated here.
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
  "✓ secret leak guard: .env is gitignored and untracked; " +
    "no .env value appears in a tracked file\n",
);
