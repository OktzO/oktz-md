import { describe, it } from 'node:test';
import assert from 'node:assert';

import {
  buildNotFoundPayload,
  NOT_FOUND_BUTTON_LIMIT,
} from '../src/lib/command-suggestion.js';

const COMMANDS = [
  'menu',
  'help',
  'ping',
  'sticker',
  'profile',
  'level',
  'owner',
  'speedtest',
];

describe('buildNotFoundPayload', () => {
  it('pesan memakai Bahasa Indonesia, bukan Inggris', () => {
    const out = buildNotFoundPayload('meny', COMMANDS, {
      prefix: '.',
      pushName: 'Budi',
    });

    assert.ok(out.text.length > 0);
    for (const en of [
      "I'm sorry",
      "I don't have that command",
      'Did you mean',
      'Command not found',
    ]) {
      assert.doesNotMatch(
        out.text,
        new RegExp(en.replace(/[.*+?^${}()|[\]\\]/g, '\\$&'), 'i'),
        `pesan masih ada teks Inggris: "${en}"`,
      );
    }
    assert.match(out.text, /tidak ditemukan|belum tersedia|tidak ada/i);
  });

  it('menyebut command yang diketik user', () => {
    const out = buildNotFoundPayload('meny', COMMANDS, { prefix: '.' });
    assert.match(out.text, /meny/);
  });

  it('memakai nama user saat tersedia', () => {
    const out = buildNotFoundPayload('meny', COMMANDS, {
      prefix: '.',
      pushName: 'Budi',
    });
    assert.match(out.text, /Budi/);
  });

  it('tidak crash kalau pushName kosong', () => {
    const out = buildNotFoundPayload('meny', COMMANDS, { prefix: '.' });
    assert.ok(out.text.length > 0);
    assert.doesNotMatch(out.text, /\s{3,}\*undefined/);
  });

  it('beri tombol quick_reply untuk command yang mirip', () => {
    const out = buildNotFoundPayload('meny', COMMANDS, { prefix: '.' });
    assert.ok(out.buttons.length > 0, 'harus ada tombol');
    for (const b of out.buttons) {
      assert.equal(b.name, 'quick_reply');
      const params = JSON.parse(b.buttonParamsJson);
      assert.ok(params.display_text, 'tombol harus punya label');
      assert.ok(
        params.id.startsWith('.'),
        `id tombol harus jadi command yang bisa diketuk: ${params.id}`,
      );
    }
  });

  it('id tombol = command lengkap dengan prefix', () => {
    const out = buildNotFoundPayload('meny', COMMANDS, { prefix: '.' });
    const ids = out.buttons.map((b) => JSON.parse(b.buttonParamsJson).id);
    assert.ok(
      ids.includes('.menu'),
      `tombol harus mengirim ".menu" supaya langsung jalan. Got: ${JSON.stringify(ids)}`,
    );
  });

  it('batas jumlah tombol (WA max 3 baris × 2 kolom)', () => {
    const many = Array.from({ length: 30 }, (_, i) => `cmd${i}`);
    const out = buildNotFoundPayload('cmd1', many, { prefix: '.' });
    assert.ok(
      out.buttons.length <= NOT_FOUND_BUTTON_LIMIT,
      `terlalu banyak tombol: ${out.buttons.length}`,
    );
    assert.equal(out.buttons.length, NOT_FOUND_BUTTON_LIMIT);
  });

  it('tombol selalu genap agar rapi di grid WA', () => {
    const out = buildNotFoundPayload('meny', COMMANDS, { prefix: '.' });
    assert.equal(
      out.buttons.length % 2,
      0,
      'jumlah tombol harus genap supaya grid 2 kolom tidak bolong',
    );
  });

  it('hanya memberi command yang benar-benar ada', () => {
    const out = buildNotFoundPayload('zzzzzzz', COMMANDS, { prefix: '.' });
    const ids = out.buttons.map((b) => JSON.parse(b.buttonParamsJson).id);
    for (const id of ids) {
      assert.ok(
        COMMANDS.includes(id.slice(1)),
        `tombol menawarkan command yang tidak ada: ${id}`,
      );
    }
  });

  it('label tombol memuat command lengkap dengan prefix', () => {
    const out = buildNotFoundPayload('meny', COMMANDS, { prefix: '!' });
    const first = JSON.parse(out.buttons[0].buttonParamsJson);
    assert.match(first.display_text, /!\w+/);
  });

  it('tidak error kalau daftar command kosong', () => {
    const out = buildNotFoundPayload('apapun', [], { prefix: '.' });
    assert.equal(out.buttons.length, 0);
    assert.match(out.text, /tidak ditemukan|belum tersedia/i);
    // Tetap harus menawarkan jalan keluar.
    assert.match(out.text, /menu|help/i);
  });

  it('tidak error kalau input bukan string', () => {
    const out = buildNotFoundPayload(undefined, COMMANDS, { prefix: '.' });
    assert.ok(out.text.length > 0);
  });

  it('prefix multi-karakter tetap jalan', () => {
    const out = buildNotFoundPayload('meny', COMMANDS, { prefix: '!!' });
    const first = JSON.parse(out.buttons[0].buttonParamsJson);
    assert.ok(first.id.startsWith('!!'), `id: ${first.id}`);
  });

  it('menyarankan command teratas sebagai suggestions[0]', () => {
    const out = buildNotFoundPayload('meny', COMMANDS, { prefix: '.' });
    const top = out.buttons[0];
    assert.ok(top, 'harus ada tombol pertama');
    const firstCmd = JSON.parse(top.buttonParamsJson).id.slice(1);
    assert.equal(firstCmd, 'menu', 'saran teratas harus yang paling mirip');
  });
});
