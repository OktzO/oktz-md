import { describe, it, after } from 'node:test';
import assert from 'node:assert';
import { mkdtempSync, rmSync, cpSync, writeFileSync, existsSync, readFileSync } from 'fs';
import { tmpdir } from 'os';
import { join } from 'path';

const tmp = mkdtempSync(join(tmpdir(), 'oktz-plugin-cmds-'));
cpSync('plugins', join(tmp, 'plugins'), { recursive: true });
cpSync('src', join(tmp, 'src'), { recursive: true });
cpSync('config.js', join(tmp, 'config.js'));
cpSync('package.json', join(tmp, 'package.json'));
cpSync('node_modules', join(tmp, 'node_modules'), { recursive: true, dereference: true });

const origCwd = process.cwd();
process.chdir(tmp);

function mockSock(captured) {
  return {
    sendMessage: async (jid, content, opts) => {
      captured.push({ kind: 'send', jid, content, opts });
      return { key: { id: 'k1' } };
    },
    relayMessage: async (jid, message, opts) => {
      captured.push({ kind: 'relay', jid, message, opts });
    },
    waUploadToServer: async () => { throw new Error('no upload in test'); },
    user: { id: '6281@s.whatsapp.net' },
  };
}

function mockM(over = {}) {
  return {
    chat: '6281@s.whatsapp.net',
    sender: '6285608953677@s.whatsapp.net',
    pushName: 'Tester',
    prefix: '.',
    command: 'sc',
    args: [],
    fullArgs: '',
    key: {
      remoteJid: '6281@s.whatsapp.net',
      fromMe: false,
      id: 'msg1',
      participant: '6285608953677@s.whatsapp.net',
    },
    message: { conversation: 'pesan asli' },
    reply: async (t) => { },
    react: async () => { },
    ...over,
  };
}

describe('sc command', () => {
  it('mengirim kartu interactiveMessage (bukan interactiveButtons) dengan tombol ke github.com/OktzO/oktz-md', async () => {
    const captured = [];
    const mod = await import('../plugins/main/sc.js');
    await mod.handler(mockM(), { sock: mockSock(captured) });

    const relay = captured.find((c) => c.kind === 'relay');
    assert.ok(relay, 'relayMessage harus dipanggil');
    const json = JSON.stringify(relay.message);
    assert.ok(json.includes('nativeFlowMessage'), 'harus pakai nativeFlowMessage');
    assert.ok(json.includes('cta_url'), 'harus ada tombol cta_url');
    assert.ok(json.includes('github.com/OktzO/oktz-md'), 'URL harus ke repo github OktzO/oktz-md');
    // kartu harus dibungkus viewOnceMessage agar dirender WhatsApp
    const wrapped = relay.message?.viewOnceMessage?.message?.interactiveMessage;
    assert.ok(wrapped, 'interactiveMessage harus dalam wrapper viewOnceMessage');
    assert.ok(!json.includes('interactiveButtons'), 'tidak boleh pakai interactiveButtons (dead key)');
  });
});

describe('getplugin command', () => {
  it('mengirim source code sebagai document .js yang bisa didownload', async () => {
    // pastikan ada file target yang stabil untuk dibaca
    writeFileSync(
      join(tmp, 'plugins', 'fun', 'oktzprobe.js'),
      `const pluginConfig = { name: "oktzprobe", alias: ["op"], category: "fun", description: "probe", usage: ".oktzprobe", example: ".oktzprobe", isOwner: false, isPremium: false, isGroup: false, isPrivate: false, isAdmin: false, isBotAdmin: false, cooldown: 3, limit: 1, isEnabled: true };\nasync function handler(m) { await m.reply("probe"); }\nexport { pluginConfig as config, handler };\n`,
    );

    const captured = [];
    const mod = await import('../plugins/owner/getplugin.js');
    await mod.handler(mockM({ command: 'getplugin', args: ['oktzprobe'] }), { sock: mockSock(captured) });

    const send = captured.find((c) => c.kind === 'send');
    assert.ok(send, 'sendMessage harus dipanggil');
    assert.ok(send.content?.document, 'harus kirim document');
    assert.ok(String(send.content?.fileName).endsWith('.js'), 'fileName harus .js');
    assert.ok(!JSON.stringify(send.content).includes('interactiveButtons'), 'tidak boleh pakai interactiveButtons');
  });
});

describe('addplugin command', () => {
  it('reply file .js: mengunduh isi file (bukan caption) lalu menulis plugin', async () => {
    const fileBuf = Buffer.from(
      `const pluginConfig = { name: "oktzadd", alias: ["oa"], category: "fun", description: "x", usage: ".oktzadd", example: ".oktzadd", isOwner: false, isPremium: false, isGroup: false, isPrivate: false, isAdmin: false, isBotAdmin: false, cooldown: 3, limit: 1, isEnabled: true };\nasync function handler(m) { await m.reply("ok"); }\nexport { pluginConfig as config, handler };\n`,
    );
    // quoted document serialized: TIDAK punya mimetype/filename — hanya isDocument + body(caption)
    const quoted = {
      isDocument: true,
      isMedia: true,
      type: 'documentMessage',
      body: 'nih pluginnya bang',
      download: async () => fileBuf,
    };

    const captured = [];
    const replies = [];
    const mod = await import('../plugins/owner/addplugin.js');
    await mod.handler(
      mockM({
        command: 'addplugin',
        args: ['oktzadd', 'fun'],
        reply: async (t) => replies.push(t),
        quoted,
      }),
      { sock: mockSock(captured) },
    );

    assert.ok(existsSync(join(tmp, 'plugins/fun/oktzadd.js')), 'file plugin harus tertulis');
    const written = JSON.stringify(readFileSync(join(tmp, 'plugins/fun/oktzadd.js'), 'utf8'));
    assert.ok(written.includes('oktzadd'), 'isi file harus KODE dari download, bukan caption');
    assert.ok(!written.includes('nih pluginnya'), 'caption tidak boleh jadi isi plugin');
  });
});

describe('delplugin command', () => {
  it('menghapus file plugin berdasarkan nama', async () => {
    writeFileSync(
      join(tmp, 'plugins', 'fun', 'oktzdel.js'),
      `const pluginConfig = { name: "oktzdel" };\nexport { pluginConfig as config };\n`,
    );

    const captured = [];
    const replies = [];
    const mod = await import('../plugins/owner/delplugin.js');
    await mod.handler(
      mockM({ command: 'delplugin', fullArgs: 'oktzdel', args: ['oktzdel'], reply: async (t) => replies.push(t) }),
      { sock: mockSock(captured) },
    );

    assert.ok(!existsSync(join(tmp, 'plugins/fun/oktzdel.js')), 'file plugin harus terhapus');
    assert.ok(replies.some((t) => t.includes('berhasil')), 'reply harus menyatakan berhasil');
  });
});

after(() => {
  process.chdir(origCwd);
  rmSync(tmp, { recursive: true, force: true });
});
