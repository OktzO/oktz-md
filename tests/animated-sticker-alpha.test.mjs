import { describe, it, before } from "node:test";
import assert from "node:assert";
import { execFileSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import ffmpegInstaller from "@ffmpeg-installer/ffmpeg";

const FF = ffmpegInstaller.path;
const tmp = fs.mkdtempSync(path.join(os.tmpdir(), "stk-alpha-"));

function run(cmd) {
  execFileSync(cmd[0], cmd.slice(1), { stdio: "pipe" });
}

// vf chain yang dipakai .s untuk video/gif -> sticker webp animasi.
// Bug: pad color 0x00000000 pada input non-alpha (mp4/yuv420p) jadi HITAM
// padat, bukan transparan — video mengecil dengan background hitam.
// Fix: format=rgba sebelum pad agar alpha ikut dipertahankan.
import { STICKER_WEBP_VF } from "../src/lib/ourin-ffmpeg.js";

let webpPath;

before(() => {
  const src = path.join(tmp, "in.mp4");
  run([FF, "-y", "-f", "lavfi", "-i", "color=c=red:s=640x360:d=2:r=12", "-pix_fmt", "yuv420p", src]);
  webpPath = path.join(tmp, "out.webp");
  run([FF, "-y", "-i", src, "-vf", STICKER_WEBP_VF, "-vcodec", "libwebp", "-loop", "0", "-preset", "default", "-an", "-vsync", "0", "-q:v", "50", webpPath]);
});

describe("sticker webp alpha (bug: background hitam)", () => {
  it("korner output transparan (alpha=0), bukan hitam solid", async () => {
    const { default: sharp } = await import("sharp");
    const px = await sharp(webpPath).extract({ left: 0, top: 0, width: 1, height: 1 }).raw().toBuffer();
    assert.strictEqual(px[3], 0, `corner alpha harus 0 (transparan), dapat ${px[3]}: ${Array.from(px)}`);
  });

  it("isi video tetap render (center alpha=255, warna merah)", async () => {
    const { default: sharp } = await import("sharp");
    const px = await sharp(webpPath).extract({ left: 256, top: 256, width: 1, height: 1 }).raw().toBuffer();
    assert.strictEqual(px[3], 255, "center harus opaque");
    assert.ok(px[0] > 200, `center harus merah, dapat r=${px[0]}`);
  });
});