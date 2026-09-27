import { describe, it, before } from "node:test";
import assert from "node:assert";

// uploader.js lama: 6 nama export (uploadImage/uploadToTelegraph/uploadTo0x0/
// uploadToCatbox/uploadToTmpfiles/uploadToUguu) semuanya alias dari SATU fungsi
// yang hardcode c.termai.cc + wajib TERMAI_UPLOAD_KEY. 5 alias itu tidak pernah
// di-import siapa pun dan hanya berpura-pura punya redundancy.
describe("uploadImage harus redundan, bukan satu host", () => {
  let uploadImage;

  before(async () => {
    ({ uploadImage } = await import("../src/lib/uploader.js"));
  });

  it("mencoba host berikutnya saat host pertama gagal", async () => {
    const attempted = [];
    const providers = [
      {
        name: "mati",
        run: async () => {
          attempted.push("mati");
          throw new Error("host mati");
        },
      },
      {
        name: "hidup",
        run: async () => {
          attempted.push("hidup");
          return { url: "https://ok.test/1.jpg" };
        },
      },
    ];

    const url = await uploadImage(Buffer.from("x"), "image.jpg", { providers });

    assert.equal(url, "https://ok.test/1.jpg", "harus mengembalikan URL host yang hidup");
    assert.deepEqual(attempted, ["mati", "hidup"], "harus mencoba keduanya");
  });

  it("tidak melempar hanya karena satu provider gagal", async () => {
    const providers = [
      { name: "a", run: async () => { throw new Error("down"); } },
      { name: "b", run: async () => { throw new Error("down"); } },
      { name: "c", run: async () => ({ url: "https://ok.test/c.jpg" }) },
    ];
    const url = await uploadImage(Buffer.from("x"), "image.jpg", { providers });
    assert.equal(url, "https://ok.test/c.jpg");
  });

  it("lempar error yang menyebut semua host yang dicoba", async () => {
    const providers = [
      { name: "alpha", run: async () => { throw new Error("a mati"); } },
      { name: "beta", run: async () => { throw new Error("b mati"); } },
    ];
    await assert.rejects(
      () => uploadImage(Buffer.from("x"), "image.jpg", { providers }),
      (err) => {
        assert.match(err.message, /alpha/, "error harus menyebut alpha");
        assert.match(err.message, /beta/, "error harus menyebut beta");
        return true;
      },
    );
  });

  it("menolak provider yang mengembalikan URL kosong", async () => {
    const providers = [
      { name: "kosong", run: async () => ({ url: "" }) },
      { name: "oke", run: async () => ({ url: "https://ok.test/z.jpg" }) },
    ];
    const url = await uploadImage(Buffer.from("x"), "image.jpg", { providers });
    assert.equal(url, "https://ok.test/z.jpg", "URL kosong harus dihitung gagal");
  });

  it("gagal dengan pesan jelas saat tidak ada host yang dikonfigurasi", async () => {
    await assert.rejects(
      () => uploadImage(Buffer.from("x"), "image.jpg", { providers: [] }),
      /tidak ada host upload/i,
    );
  });

  it("langsung kembali begitu satu host sukses, tidak menunggu host lain", async () => {
    let slowFinished = false;
    const providers = [
      { name: "cepat", run: async () => ({ url: "https://ok.test/fast.jpg" }) },
      {
        name: "lambat",
        run: async () => {
          await new Promise((r) => setTimeout(r, 900));
          slowFinished = true;
          return { url: "https://ok.test/slow.jpg" };
        },
      },
    ];

    const t0 = Date.now();
    const url = await uploadImage(Buffer.from("x"), "image.jpg", { providers });
    const elapsed = Date.now() - t0;

    assert.equal(url, "https://ok.test/fast.jpg");
    assert.ok(elapsed < 500, `harus selesai cepat, took ${elapsed}ms`);
    assert.equal(slowFinished, false, "host lambat tidak boleh ikut menunggu");
  });

  it("tidak memanggil host berikutnya kalau yang pertama sudah sukses", async () => {
    const called = [];
    const providers = [
      { name: "satu", run: async () => { called.push("satu"); return { url: "https://ok.test/1.jpg" }; } },
      { name: "dua", run: async () => { called.push("dua"); return { url: "https://ok.test/2.jpg" }; } },
    ];
    await uploadImage(Buffer.from("x"), "image.jpg", { providers });
    assert.deepEqual(called, ["satu"], "hanya host pertama yang boleh dipanggil");
  });

  it("memakai host berikutnya saat yang pertama lempar", async () => {
    const called = [];
    const providers = [
      { name: "satu", run: async () => { called.push("satu"); throw new Error("down"); } },
      { name: "dua", run: async () => { called.push("dua"); return { url: "https://ok.test/2.jpg" }; } },
    ];
    const url = await uploadImage(Buffer.from("x"), "image.jpg", { providers });
    assert.equal(url, "https://ok.test/2.jpg");
    assert.deepEqual(called, ["satu", "dua"]);
  });
});

describe("daftar host upload", () => {
  // Diverifikasi sungguhan 2026-09-27: upload file asli lalu URL di-fetch dan
  // dicek byte-nya == byte asli. Host yang balas HTML atau URL rusak TIDAK
  // boleh masuk daftar default.
  const VERIFIED = ["Litterbox", "Kappa", "Uguu", "Leopard", "Upload.ee", "Nekohime"];

  it("tidak butuh API key dan punya banyak host", async () => {
    const { createUploadProviders } = await import("../src/lib/upload-providers.js");
    const providers = createUploadProviders();

    assert.ok(providers.length >= 3, `butuh >=3 host, dapat ${providers.length}`);
    for (const p of providers) {
      assert.ok(p.name, "setiap provider harus punya nama");
      assert.equal(typeof p.run, "function");
    }
    const names = providers.map((p) => p.name);
    assert.equal(new Set(names).size, names.length, "nama host harus unik");
  });

  it("hanya memuat host yang sudah diverifikasi serve file asli", async () => {
    const { createUploadProviders } = await import("../src/lib/upload-providers.js");
    const names = createUploadProviders().map((p) => p.name);
    for (const n of names) {
      assert.ok(VERIFIED.includes(n), `${n} tidak ada di daftar host terverifikasi`);
    }
  });

  it("Qu.ax/TmpFiles/Top4top dibuang karena URL-nya bukan file", async () => {
    const { createUploadProviders } = await import("../src/lib/upload-providers.js");
    const names = createUploadProviders().map((p) => p.name);
    // Qu.ax  -> balas HTML 7.8KB
    // TmpFiles -> /dl/ 302 redirect ke halaman preview
    // Top4top -> pola HTML tidak ketemu
    for (const dead of ["Qu.ax", "TmpFiles", "Top4top"]) {
      assert.ok(!names.includes(dead), `${dead} harus dibuang, dia serve HTML bukan file`);
    }
  });
});

describe("alias palsu harus hilang", () => {
  it("tidak lagi mengekspor nama yang menyesatkan", async () => {
    const mod = await import("../src/lib/uploader.js");
    for (const fake of [
      "uploadToTelegraph", "uploadTo0x0", "uploadToCatbox",
      "uploadToTmpfiles", "uploadToUguu",
    ]) {
      assert.equal(
        mod[fake], undefined,
        `${fake} tidak pernah di-import dan hanya berpura-pura redundan`,
      );
    }
  });

  it("tetap mengekspor yang benar-benar dipakai", async () => {
    const mod = await import("../src/lib/uploader.js");
    assert.equal(typeof mod.uploadImage, "function");
    assert.equal(typeof mod.updateAssetUrl, "function");
  });
});
