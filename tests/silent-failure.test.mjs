import { describe, it, beforeEach } from 'node:test';
import assert from 'node:assert';
import { readFile, writeFile, rm, mkdir } from 'node:fs/promises';
import { existsSync } from 'node:fs';
import path from 'node:path';

const REPO = new URL('..', import.meta.url).pathname;

/* ------------------------------------------------------------------ */
/* sticker.js — jangan kirim buffer tak terproses seolah hasil filter  */
/* ------------------------------------------------------------------ */

describe('sticker.js — gagal proses ≠ sukses', () => {
  const STICKER = path.join(REPO, 'plugins/sticker/sticker.js');

  it('tidak mengabaikan error FFmpeg lalu tetap kirim buffer asli', async () => {
    const src = await readFile(STICKER, 'utf8');
    // Kalau catch FFmpeg hanya console.error tanpa menandai kegagalan,
    // buffer asli akan terkirim seolah-olah efeknya berlaku.
    const catchIdx = src.indexOf('[Sticker] FFmpeg processing failed:');
    assert.ok(catchIdx > -1, 'catch FFmpeg tidak ditemukan');
    const catchBlock = src.slice(catchIdx, catchIdx + 320);
    assert.match(
      catchBlock,
      /processed\s*=\s*false|notifyUser|reportFailure|gagal/,
      'catch FFmpeg harus menandai bahwa efek gagal, bukan cuma log',
    );
    // Setelah catch, flag itu harus dipakai saat mengirim
    const after = src.slice(catchIdx + 320, catchIdx + 1400);
    assert.match(
      after,
      /hasProcessing\s*&&\s*!processed|if\s*\(!processed\)|processed\)/,
      'flag kegagalan harus dipakai sebelum/ketika mengirim sticker',
    );
  });

  it('memberi tahu user saat efek tidak bisa diterapkan', async () => {
    const src = await readFile(STICKER, 'utf8');
    assert.match(
      src,
      /gagal|effect|efek|diabaikan|tidak bisa/i,
      'harus ada cabang yang memberi tahu user efek gagal',
    );
  });
});

/* ------------------------------------------------------------------ */
/* pindl.js — jangan ✅ kalau 0 file terkirim                           */
/* ------------------------------------------------------------------ */

describe('pindl.js — tidak boleh ✅ tanpa file terkirim', () => {
  const PINDL = path.join(REPO, 'plugins/download/pindl.js');

  it('✅ hanya kalau ada file yang benar-benar terkirim', async () => {
    const src = await readFile(PINDL, 'utf8');
    const okIdx = src.lastIndexOf('m.react("✅")');
    assert.ok(okIdx > -1, 'reaksi ✅ harus ada di file');
    const before = src.slice(0, okIdx);
    assert.match(
      before,
      /if \(sentCount\s*>\s*0\)/,
      'reaksi ✅ harus dijaga oleh penghitung file terkirim',
    );
    // Setiap jalur pengiriman harus menaikkan penghitung.
    const sends = before.split('await sock.sendMedia').length - 1;
    const increments = before.split('sentCount += 1').length - 1;
    assert.ok(
      increments >= sends,
      `ada ${sends} sendMedia tapi hanya ${increments} yang menambah penghitung — ada jalur yang tidak dihitung`,
    );
    // Dan harus ada jalur ❌ + pesan user.
    const after = src.slice(okIdx);
    assert.match(after, /m\.react\("❌"\)/, 'harus ada reaksi gagal');
  });

  it('laporkan ke user saat tidak ada media yang berhasil diunduh', async () => {
    const src = await readFile(PINDL, 'utf8');
    assert.match(
      src,
      /m\.reply\(/,
      'harus ada pesan ke user di jalur gagal',
    );
  });
});

/* ------------------------------------------------------------------ */
/* autoreply.js — mtype tidak pernah di-set                            */
/* ------------------------------------------------------------------ */

describe('autoreply.js — deteksi gambar harus pakai field yang benar', () => {
  const AUTOREPLY = path.join(REPO, 'plugins/group/autoreply.js');

  it('tidak lagi relies on m.quoted.mtype (tidak pernah di-set di src/)', async () => {
    const src = await readFile(AUTOREPLY, 'utf8');
    // `m.mtype` / `m.quoted.mtype` adalah nama field yang TIDAK PERNAH di-set
    // serializer — sisa penamaan Baileys lama. Kalau muncul dalam kondisi,
    // itu bug. Komentar tidak ikut dihitung.
    const code = src.replace(/\/\/[^\n]*/g, '').replace(/\/\*[\s\S]*?\*\//g, '');
    assert.doesNotMatch(
      code,
      /m\.quoted\.mtype|\bm\.mtype\b/,
      'mtype tidak pernah di-set serializer — sisa field Baileys lama',
    );
  });

  it('hapus file gambar lama hanya kalau ada gambar baru', async () => {
    const src = await readFile(AUTOREPLY, 'utf8');
    // Pola data-loss: unlink dijalankan saat imagePath null (tidak ada
    // gambar baru) tapi entri lama punya gambar.
    assert.doesNotMatch(
      src,
      /customReplies\[i\]\.image\s*&&\s*customReplies\[i\]\.image\s*!==\s*imagePath[\s\S]{0,200}?unlinkSync/,
      'hapus file lama harus dijaga agar tidak jalan saat gambar baru null',
    );
  });
});

/* ------------------------------------------------------------------ */
/* m.quoted tidak punya mimetype / filename                            */
/* ------------------------------------------------------------------ */

describe('quoted message tidak punya mimetype/filename di level quoted', () => {
  it('serialize.js tidak menyetel mimetype/filename di objek quoted', async () => {
    const src = await readFile(
      path.join(REPO, 'src/lib/serialize.js'),
      'utf8',
    );
    // Ambil blok object literal `const quoted = { ... }`.
    const start = src.indexOf('const quoted = {');
    assert.ok(start > -1, 'blok quoted tidak ditemukan');
    const end = src.indexOf('quoted.download = async', start);
    const block = src.slice(start, end);
    assert.doesNotMatch(
      block,
      /^\s{4}mimetype:/m,
      'quoted TIDAK punya field mimetype — plugin yang memakainya salah',
    );
    assert.doesNotMatch(
      block,
      /^\s{4}filename:/m,
      'quoted TIDAK punya field filename — plugin yang memakainya salah',
    );
  });

  it('quoted punya fileName di dalam message[quotedType]', async () => {
    const src = await readFile(
      path.join(REPO, 'src/lib/serialize.js'),
      'utf8',
    );
    const start = src.indexOf('const quoted = {');
    const end = src.indexOf('quoted.download = async', start);
    const block = src.slice(start, end);
    assert.match(
      block,
      /message: quotedMessage/,
      'isi pesan ada di quoted.message, bukan di field top-level',
    );
  });

  it('deploy.js membaca filename dari tempat yang benar', async () => {
    const src = await readFile(path.join(REPO, 'plugins/tools/deploy.js'), 'utf8');
    assert.doesNotMatch(
      src,
      /m\.quoted\.mimetype|m\.quoted\.filename/,
      'berhenti membaca field yang tidak pernah ada di quoted',
    );
    assert.match(
      src,
      /documentMessage\?\.fileName|message\?\.documentMessage/,
      'harus baca fileName dari quoted.message.documentMessage',
    );
  });

  it('swgc/swgcv2 tidak lagi relies on m.quoted.mimetype atau m.quoted.msg', async () => {
    for (const f of [
      'plugins/owner/swgc.js',
      'plugins/owner/swgcv2.js',
    ]) {
      const src = await readFile(path.join(REPO, f), 'utf8');
      // Buang komentar: yang dicek adalah pemakaian di KODE.
      const code = src.replace(/\/\/[^\n]*/g, '').replace(/\/\*[\s\S]*?\*\//g, '');
      assert.doesNotMatch(code, /m\.quoted\.mimetype/, `${f} salah field`);
      assert.doesNotMatch(code, /m\.quoted\.msg\b/, `${f} salah field`);
      assert.match(
        code,
        /quoted\?\.message\?\.\[m\.quoted\?\.type\]|quoted\.message\?\.\[/,
        `${f} harus baca dari quoted.message[type]`,
      );
    }
  });
});

/* ------------------------------------------------------------------ */
/* download dari quoted harus cek jenis media                          */
/* ------------------------------------------------------------------ */

describe('download dari quoted harus cek jenis media', () => {
  const FILES = [
    'plugins/canvas/igstory.js',
    'plugins/canvas/igstoryimg.js',
    'plugins/canvas/iqcpink.js',
    'plugins/sticker/smeme.js',
    'plugins/sticker/smemevid.js',
  ];

  for (const f of FILES) {
    it(`${f} tidak asal unduh quoted tanpa cek media`, async () => {
      const src = await readFile(path.join(REPO, f), 'utf8');
      if (!/m\.quoted.*download/.test(src)) {
        return; // tidak menyentuh quoted.download
      }
      assert.match(
        src,
        /quoted\?\.isMedia\s*===\s*true|quoted\.isMedia|quoted\?\.type\s*===|isMedia|isImage|isVideo/,
        `${f} perlu cek quoted itu media apa sebelum download`,
      );
    });
  }
});

/* ------------------------------------------------------------------ */
/* addproduk / addlist — upload gagal harus dilaporkan                 */
/* ------------------------------------------------------------------ */

describe('store: upload gagal harus terlihat', () => {
    for (const f of [
      'plugins/store/addproduk.js',
      'plugins/store/addlist.js',
    ]) {
      it(`${f} memberi tahu kalau upload media gagal`, async () => {
        const src = await readFile(path.join(REPO, f), 'utf8');
        const idx = src.indexOf('Upload error:');
        assert.ok(idx > -1, 'catch upload tidak ditemukan');
        const block = src.slice(idx, idx + 320);
        assert.match(
          block,
          /uploadWarning\s*=/,
          `${f}: catch upload harus menyiapkan peringatan, bukan cuma log`,
        );

        // Peringatan itu harus benar-benar sampai ke user.
        const after = src.slice(idx + 320);
        assert.match(
          after,
          /if \(uploadWarning\)/,
          `${f}: uploadWarning harus dicetak ke pesan balasan`,
        );

        // Dan harus di-deklarasikan SEBELUM try/catch — kalau setelahnya,
        // assignment di dalam catch kena TDZ ReferenceError.
        const declIdx = src.indexOf('let uploadWarning');
        assert.ok(
          declIdx > -1 && declIdx < idx,
          `${f}: deklarasi uploadWarning harus sebelum catch`,
        );
      });
    }
});
