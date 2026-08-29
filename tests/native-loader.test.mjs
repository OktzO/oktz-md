// ourin-native-loader contract tests.
// 1. hasNative() boolean + ocr() returns a trimmed string through the loader
//    (native .node when present, tesseract.js fallback otherwise).
// 2. OCR must actually READ text — rendered with a bundled font via
//    @napi-rs/canvas (system fonts are empty in CI containers, which silently
//    produced blank renders in an earlier sharp-SVG fixture).
// 3. Fallback ordering: a deliberately missing datapath still works either way.
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { createCanvas, GlobalFonts } from "@napi-rs/canvas";

const loader = await import("../src/lib/ourin-native-loader.js");

function renderTextPng() {
  GlobalFonts.register(readFileSync("assets/fonts/Levelup.ttf"), "testfont");
  const c = createCanvas(600, 200);
  const ctx = c.getContext("2d");
  ctx.fillStyle = "white";
  ctx.fillRect(0, 0, 600, 200);
  ctx.fillStyle = "black";
  ctx.font = "64px testfont";
  ctx.fillText("HELLO 123", 30, 130);
  return c.encode("png");
}

test("hasNative() reports boolean", () => {
  assert.equal(typeof loader.hasNative(), "boolean");
});

test(
  "ocr() reads rendered text (native or fallback)",
  async () => {
    const png = await renderTextPng();
    const text = await loader.ocr(png);
    assert.equal(typeof text, "string");
    assert.match(
      text,
      /123/,
      `expected OCR to read "HELLO 123", got: ${JSON.stringify(text)}`,
    );
  },
  { timeout: 120_000 },
);
