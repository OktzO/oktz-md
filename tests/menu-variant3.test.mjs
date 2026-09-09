import { describe, it, after } from 'node:test';
import assert from 'node:assert';
import { mkdtempSync, rmSync, cpSync, existsSync, readFileSync } from 'fs';
import { tmpdir } from 'os';
import { join } from 'path';

const tmp = mkdtempSync(join(tmpdir(), 'oktz-menu3-'));
cpSync('plugins', join(tmp, 'plugins'), { recursive: true });
cpSync('src', join(tmp, 'src'), { recursive: true });
cpSync('config.js', join(tmp, 'config.js'));
cpSync('package.json', join(tmp, 'package.json'));
cpSync('node_modules', join(tmp, 'node_modules'), { recursive: true, dereference: true });
cpSync('assets', join(tmp, 'assets'), { recursive: true });
cpSync('case', join(tmp, 'case'), { recursive: true });
cpSync('data', join(tmp, 'data'), { recursive: true });

const origCwd = process.cwd();
process.chdir(tmp);

function mockSock(relays) {
  return {
    sendMessage: async (jid, c, o) => ({ key: { id: 'k' } }),
    relayMessage: async (jid, msg, o) => { relays.push(msg); },
    waUploadToServer: async () => ({}),
    user: { id: '6281@s.whatsapp.net' },
  };
}

function mockM(over = {}) {
  return {
    chat: '6281@s.whatsapp.net',
    sender: '6285608953677@s.whatsapp.net',
    pushName: 'Tester',
    prefix: '.',
    command: 'menu',
    args: [],
    isOwner: true,
    isPremium: true,
    key: { remoteJid: '6281@s.whatsapp.net', fromMe: false, id: 'x', participant: '6281@s.whatsapp.net' },
    message: { conversation: '.menu' },
    reply: async () => { },
    react: async () => { },
    ...over,
  };
}

async function runMenu(menuVariant) {
  const relays = [];
  const mod = await import(join(tmp, 'plugins/main/menu.js'));
  const db = {
    setting: (k) => k === 'menuVariant' ? menuVariant : (k === 'audioMenu' ? false : null),
    getUser: () => ({ level: 1, exp: 10, energi: 5, koin: 2, isRegistered: true }),
    getUserCount: () => 1,
    getGroup: () => ({}),
  };
  const cfg = (await import(join(tmp, 'config.js'))).default;
  await mod.default.handler(mockM(), { sock: mockSock(relays), config: cfg, db, uptime: 100 });
  const card = relays[relays.length - 1]?.viewOnceMessage?.message?.interactiveMessage;
  const raw = JSON.stringify(relays[relays.length - 1] || {});
  return { relays, card, raw };
}

describe('menu variant 3 (default config)', () => {
  it('body hanya berisi SATU "Hello Brother" (tidak dobel)', async () => {
    const { card } = await runMenu(3);
    assert.ok(card, 'kartu interactiveMessage harus terkirim');
    const count = (card.body.text.match(/Hello Brother/g) || []).length;
    assert.equal(count, 1, `Hello Brother muncul ${count}x, harus 1x`);
    const nameCount = (card.body.text.match(/BOT INFORMATION/g) || []).length;
    assert.equal(nameCount, 1, 'BOT INFORMATION harus 1x');
  });

  it('body berisi greeting dinamis (Selamat Pagi/Siang/Sore/Malam)', async () => {
    const { card } = await runMenu(3);
    assert.ok(
      /Selamat (Pagi|Siang|Sore|Malam)/.test(card.body.text),
      'harus ada greeting sesuai waktu',
    );
  });

  it('cuaca tampil sebagai kartu lokasi di quoted (pola .allmenu), bukan section body', async () => {
    const { raw, card } = await runMenu(3);
    // quoted location message berisi info cuaca
    assert.ok(raw.includes('locationMessage'), 'harus ada locationMessage quoted');
    assert.ok(
      /(°C|tidak tersedia)/.test(raw),
      'quoted location harus berisi suhu derajat atau fallback',
    );
    // cuaca TIDAK boleh jadi section di body card
    assert.ok(!/INFO LINGKUNGAN/.test(card.body.text), 'body tidak boleh ada section Info Lingkungan');
  });
});

after(() => {
  process.chdir(origCwd);
  rmSync(tmp, { recursive: true, force: true });
});
