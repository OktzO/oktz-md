import { describe, it } from "node:test";
import assert from "node:assert";
import { addExifToWebp } from "../src/lib/exif.js";

function chunkIds(buf) {
  const ids = [];
  let pos = 12;
  const riffSize = buf.readUInt32LE(4);
  const limit = buf.length;
  while (pos + 8 <= limit) {
    const id = buf.slice(pos, pos + 4).toString("ascii");
    const size = buf.readUInt32LE(pos + 4);
    ids.push({ id, size, at: pos });
    pos += 8 + size + (size % 2);
  }
  return { ids, riffSize, bytes: buf.length, walkEnd: pos, limit };
}

describe("addExifToWebp — WhatsApp-safe extended container", () => {
  it("static basic webp (sharp) diberi VP8X sebelum EXIF, RIFF size konsisten", async () => {
    const { default: sharp } = await import("sharp");
    const buf = await sharp({ create: { width: 512, height: 512, channels: 4, background: { r: 255, g: 255, b: 255, alpha: 255 } } })
      .webp({ quality: 80 })
      .toBuffer();
    assert.strictEqual(buf.indexOf(Buffer.from("VP8X")), -1, "prasyarat: sharp static tanpa VP8X");

    const out = await addExifToWebp(buf, { packname: "Oktz", author: "oktz-md" });
    const { ids, riffSize, bytes, walkEnd, limit } = chunkIds(out);

    assert.strictEqual(riffSize, bytes - 8, "RIFF size harus match file size - 8");
    assert.strictEqual(walkEnd, limit, "tidak boleh ada trailing/garbage bytes");
    assert.ok(ids[0].id === "VP8X", "chunk pertama setelah WEBP harus VP8X, dapat: " + ids[0].id);
    assert.strictEqual(ids[0].size, 10, "payload VP8X harus 10 byte");
    assert.ok(out.indexOf(Buffer.from("EXIF")) > 0, "chunk EXIF harus ada");

    const meta = await sharp(out).metadata();
    assert.strictEqual(meta.format, "webp");
    assert.strictEqual(meta.width, 512);
  });

  it("webp yang sudah extended (VP8X) tidak diduplikasi", async () => {
    const { default: sharp } = await import("sharp");
    const buf = await sharp({ create: { width: 512, height: 512, channels: 4, background: { r: 255, g: 255, b: 255, alpha: 255 } } })
      .webp({ quality: 80 })
      .toBuffer();
    const vp8x = Buffer.concat([Buffer.from("VP8X"), Buffer.from([10, 0, 0, 0]), Buffer.alloc(10)]);
    const ri = Buffer.concat([Buffer.from("RIFF"), Buffer.alloc(4), Buffer.from("WEBP"), vp8x, buf.slice(12)]);
    ri.writeUInt32LE(ri.length - 8, 4);

    const out = await addExifToWebp(ri, { packname: "Oktz", author: "oktz-md" });
    const vp8xCount = chunkIds(out).ids.filter((c) => c.id === "VP8X").length;
    assert.strictEqual(vp8xCount, 1, "tidak boleh ada VP8X dobel");
  });

  it("output masih valid & EXIF terbaca", async () => {
    const { default: sharp } = await import("sharp");
    const { readExifFromWebp } = await import("../src/lib/exif.js");
    const buf = await sharp({ create: { width: 512, height: 512, channels: 4, background: { r: 255, g: 255, b: 255, alpha: 255 } } })
      .webp({ quality: 80 })
      .toBuffer();
    const out = await addExifToWebp(buf, { packname: "Pack Test", author: "Author Test", emojis: ["x"] });
    const exif = readExifFromWebp(out);
    assert.strictEqual(exif["sticker-pack-name"], "Pack Test");
    assert.strictEqual(exif["sticker-pack-publisher"], "Author Test");
  });
});