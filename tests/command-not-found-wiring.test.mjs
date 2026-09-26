import { describe, it } from 'node:test';
import assert from 'node:assert';
import { readFile } from 'node:fs/promises';

const REPO = new URL('..', import.meta.url).pathname;
const HANDLER = `${REPO}src/handler.js`;

/**
 * Kasus "command tidak ada" ada di handler.js dan sulit diuji langsung
 * (butuh pipeline serialize + database). Jadi test ini mengunci KARAKTER
 * penting dari perilakunya lewat analisis sumber: kalau salah satu hilang,
 * test gagal.
 */
describe('handler.js — alur command tidak ditemukan', () => {
  it('tidak mengirim string Bahasa Inggris', async () => {
    const src = await readFile(HANDLER, 'utf8');
    // Hanya blok command-not-found yang diperiksa, bukan seluruh file —
    // file ini panjang dan punya banyak log dalam Bahasa Inggris.
    const idx = src.indexOf('buildNotFoundPayload(');
    assert.ok(idx > -1, 'buildNotFoundPayload tidak dipakai');
    const block = src.slice(idx - 200, idx + 2000);
    for (const en of [
      "I'm sorry",
      "I don't have that command",
      'Did you mean',
      '"Command not found"',
    ]) {
      assert.doesNotMatch(
        block,
        new RegExp(en.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')),
        `masih ada pesan Inggris di blok not-found: ${en}`,
      );
    }
  });

  it('memakai builder payload yang sudah diuji', async () => {
    const src = await readFile(HANDLER, 'utf8');
    assert.match(
      src,
      /buildNotFoundPayload\(/,
      'harus pakai buildNotFoundPayload() supaya perilakunya ter-cover test',
    );
  });

  it('mengirim tombol lewat sock.sendButton (bukan sendPreview)', async () => {
    const src = await readFile(HANDLER, 'utf8');
    const idx = src.indexOf('buildNotFoundPayload(');
    assert.ok(idx > -1, 'buildNotFoundPayload tidak dipakai');
    const block = src.slice(idx, idx + 1400);
    assert.match(block, /sendButton\(/, 'tombol harus lewat sendButton');
    assert.match(block, /buttons: payload\.buttons/, 'tombol harus diteruskan');
  });

  it('punya fallback teks kalau kartu gagal dikirim', async () => {
    const src = await readFile(HANDLER, 'utf8');
    const idx = src.indexOf('buildNotFoundPayload(');
    const block = src.slice(idx, idx + 1800);
    assert.match(
      block,
      /catch[\s\S]{0,400}?m\.reply\(payload\.text\)/,
      'kalau sendButton gagal, user harus tetap dapat teks',
    );
  });

  it('s selalu bisa dapat balasan walau tidak ada suggestion', async () => {
    const src = await readFile(HANDLER, 'utf8');
    const idx = src.indexOf('buildNotFoundPayload(');
    const block = src.slice(idx, idx + 1900);
    // Cabang tanpa tombol harus tetap m.reply, bukan diam.
    assert.match(
      block,
      /\}\s*else\s*\{[\s\S]{0,120}?m\.reply\(payload\.text\)/,
      'kalau tidak ada tombol, tetap kirim teks',
    );
  });

  it('tidak diam-diam gagal total kalau similarity dimatikan', async () => {
    const src = await readFile(HANDLER, 'utf8');
    const idx = src.indexOf('const similarityEnabled');
    assert.ok(idx > -1, 'similarityEnabled tidak ada');
    const block = src.slice(idx, idx + 300);
    assert.match(block, /db\.setting\("similarity"\)/, 'harus tetap bisa dimatikan');
  });
});
