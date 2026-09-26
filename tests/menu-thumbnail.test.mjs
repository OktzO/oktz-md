import { describe, it } from 'node:test';
import assert from 'node:assert';
import { readFile } from 'node:fs/promises';
import path from 'node:path';

const REPO = new URL('..', import.meta.url).pathname;

/**
 * Bug: `getAssetBuffer(key)` mengembalikan `null` kalau file tidak ada /
 * gagal dibaca. Tapi `sharp(null)` THROW SYNCHRONUS — jadi pola
 *
 *   await sharp(getAssetBuffer("foto")).resize(300,300).toBuffer()
 *                 .catch(() => null)
 *
 * tidak pernah menangkap error: `.catch()` menempel ke promise hasil
 * `toBuffer()`, sementara `sharp()` sudah lempar SEBELUM promise itu
 * ada. Hasilnya: satu variant menu lempar, seluruh `.menu` jadi hilang.
 *
 * Test ini mengunci dua hal:
 * 1. Helper thumbnail aman (test perilakunya langsung, bukan analisis regex)
 * 2. Tidak ada `sharp(getAssetBuffer(...))` telanjang di file menu
 */
describe('asset thumbnail helper', () => {
  it('safeThumbnail mengembalikan null, bukan throw, saat input null', async () => {
    const { safeThumbnail } = await import('../src/lib/asset-manager.js');
    await assert.doesNotReject(
      () => safeThumbnail(null, 300),
      'harus tahan null — ini penyebab variant menu hilang',
    );
    assert.equal(await safeThumbnail(null, 300), null);
  });

  it('safeThumbnail menghasilkan buffer valid untuk buffer asli', async () => {
    const { safeThumbnail } = await import('../src/lib/asset-manager.js');
    const buf = await import('node:fs/promises').then((fs) =>
      fs.readFile(path.join(REPO, 'assets/image/foto.jpeg')),
    );
    const out = await safeThumbnail(buf, 300);
    assert.ok(Buffer.isBuffer(out), 'harus mengembalikan Buffer');
    assert.ok(out.length > 0, 'buffer tidak boleh kosong');
  });

  it('safeThumbnail menelan input rusak tanpa melempar', async () => {
    const { safeThumbnail } = await import('../src/lib/asset-manager.js');
    for (const bad of [Buffer.from('bukan gambar'), '', undefined, 42, {}]) {
      await assert.doesNotReject(
        () => safeThumbnail(bad, 300),
        `input ${typeof bad} tidak boleh bikin throw`,
      );
    }
  });

  it('safeThumbnail menolak input yang bukan gambar tanpa melempar', async () => {
    const { safeThumbnail } = await import('../src/lib/asset-manager.js');
    assert.equal(await safeThumbnail(Buffer.from('bukan gambar'), 300), null);
  });
});

describe('file menu tidak punya sharp(getAssetBuffer) telanjang', () => {
  const MENU_FILES = [
    'plugins/main/menu.js',
    'plugins/main/menucat.js',
    'plugins/main/allmenu.js',
    // serialize.js punya pola yang sama — jpegThumbnail dari asset yang
    // bisa null, dipanggil saat bikin kartu menu/allmenu.
    'src/lib/serialize.js',
  ];

  for (const f of MENU_FILES) {
    it(`${f} memakai helper aman, bukan sharp() langsung`, async () => {
      const src = await readFile(path.join(REPO, f), 'utf8');
      // Komen boleh menyebut polanya; yang dicek adalah kode nyata.
      const code = src
        .replace(/\/\/[^\n]*/g, '')
        .replace(/\/\*[\s\S]*?\*\//g, '');
      const hits = [...code.matchAll(/sharp\(\s*getAssetBuffer\(/g)];
      assert.equal(
        hits.length,
        0,
        `${f} punya ${hits.length} pemanggilan sharp(getAssetBuffer(...)) yang bisa null → throw synchronus`,
      );
    });
  }
});

describe('getAssetBuffer boleh return null — itu kontraknya', () => {
  it('kode yang memakainya harus tahu bisa null', async () => {
    const src = await readFile(
      path.join(REPO, 'src/lib/asset-manager.js'),
      'utf8',
    );
    assert.match(
      src,
      /export function getAssetBuffer[\s\S]{0,900}?return null;/,
      'getAssetBuffer memang kontraknya bisa null — caller wajib cope',
    );
  });
});
