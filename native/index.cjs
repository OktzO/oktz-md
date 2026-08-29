"use strict";

// Platform resolver for prebuilt ourin-native binaries (napi-rs conventions).
// The JS-side wrapper src/lib/ourin-native-loader.js handles fallback ordering
// and the tesseract.js path when no binary matches.

const { existsSync } = require("node:fs");
const { join } = require("node:path");

const dir = join(__dirname, "platforms");

function candidates() {
  if (process.platform !== "linux") return [];
  if (process.arch !== "x64") return [];
  const gnu = join(dir, "x86_64-unknown-linux-gnu", "ourin_native.linux-x64-gnu.node");
  const musl = join(dir, "x86_64-unknown-linux-musl", "ourin_native.linux-x64-musl.node");
  // gnu first (most panels), musl as the Alpine fallback; loader JS catches the rest.
  return [gnu, musl];
}

for (const p of candidates()) {
  if (existsSync(p)) {
    module.exports = require(p);
    return;
  }
}

throw new Error(
  `ourin-native: no prebuilt binary for ${process.platform}-${process.arch} ` +
  `(looked in native/platforms/)`,
);
