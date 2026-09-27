import { describe, it } from "node:test";
import assert from "node:assert";
import fs from "node:fs";
import path from "node:path";

const root = path.join(process.cwd());

function read(rel) {
  return fs.readFileSync(path.join(root, rel), "utf-8");
}

// komentar sering menyebut host yang sudah dibuang; yang dicari adalah host
// yang benar-benar dipakai di kode.
function codeOnly(src) {
  return src
    .replace(/\/\*[\s\S]*?\*\//g, "")
    .split("\n")
    .map((l) => l.replace(/^\s*\/\/.*$/, "").replace(/\s\/\/.*$/, ""))
    .join("\n");
}

// Semua file ini punya upload sendiri yang hardcode satu host. dippedrotekan
// 2026-09-27: catbox.moe membalas HTTP 412 "Invalid uploader" dan
// c.termai.cc butuh TERMAI_UPLOAD_KEY. Keduanya sekarang lewat
// src/lib/uploader.js yang fan-out ke 6 host terverifikasi.
const SINGLE_HOST_UPLOADERS = [
  "src/lib/tmpfiles.js",
  "plugins/store/addlist.js",
  "plugins/store/addproduk.js",
  "plugins/store/editlist.js",
  "plugins/store/editproduk.js",
  "plugins/canvas/gura.js",
];

describe("tidak ada upload single-host yang tersisa", () => {
  for (const file of SINGLE_HOST_UPLOADERS) {
    it(`${file} tidak hardcode host upload lagi`, () => {
      const src = codeOnly(read(file));
      assert.ok(
        !/catbox\.moe\/user\/api\.php/.test(src),
        `${file} masih upload ke catbox.moe (HTTP 412, sudah mati)`,
      );
      assert.ok(
        !/c\.termai\.cc/.test(src),
        `${file} masih upload ke c.termai.cc (butuh TERMAI_UPLOAD_KEY)`,
      );
    });
  }

  it("semua file itu mengimpor uploader resilient", () => {
    for (const file of SINGLE_HOST_UPLOADERS) {
      const src = read(file);
      assert.match(
        src,
        /(\.\.\/)+src\/lib\/uploader\.js|['"]\.\/uploader\.js['"]/,
        `${file} harus pakai src/lib/uploader.js`,
      );
    }
  });
});

describe("adapter tmpfiles.js", () => {
  it("uploadTo0x0 mengembalikan {url, directUrl} seperti pemanggil fakeml.js/", async () => {
    const { uploadTo0x0 } = await import("../src/lib/tmpfiles.js");
    const out = await uploadTo0x0(Buffer.from("x"), {
      providers: [{ name: "t", run: async () => ({ url: "https://ok.test/a.jpg" }) }],
    });
    assert.equal(out.url, "https://ok.test/a.jpg");
    assert.equal(out.directUrl, "https://ok.test/a.jpg", "fakeml.js pakai .directUrl");
  });

  it("menolak buffer non-Buffer", async () => {
    const { uploadTo0x0 } = await import("../src/lib/tmpfiles.js");
    await assert.rejects(() => uploadTo0x0("bukan buffer", {}), /buffer/i);
  });
});

describe("tourl memakai daftar host terverifikasi", () => {
  it("tidak lagi punya daftar 9 host lama yang 4-nya rusak", () => {
    const src = read("plugins/tools/tourl.js");
    for (const dead of ["tmpfiles.org", "qu.ax", "top4top.io"]) {
      assert.ok(
        !new RegExp(`https://${dead.replace(/\./g, "\\.")}`).test(codeOnly(src)),
        `tourl.js masih offered ${dead} yang serve HTML bukan file`,
      );
    }
  });

  it("mengimpor daftar provider dari lib", () => {
    assert.match(read("plugins/tools/tourl.js"), /upload-providers\.js/);
  });
});
