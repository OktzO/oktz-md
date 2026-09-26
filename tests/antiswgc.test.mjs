import { describe, it, beforeEach } from 'node:test';
import assert from 'node:assert';

import {
  detectSwGcType,
  isSwGcCandidate,
  handleAntiSwGc,
  handleAntiTagSW,
} from '../src/lib/group-protection.js';

const CHAT = '120363000000000000@g.us';
const SENDER = '62811@s.whatsapp.net';
const BOT = '62899@s.whatsapp.net';

function makeDb(setting) {
  return {
    getGroup: () => ({ antiswgc: 'on', ...setting }),
    setGroup: () => {},
  };
}

function makeSock(opts = {}) {
  const calls = [];
  return {
    calls,
    user: { id: BOT },
    async groupMetadata() {
      return {
        participants: opts.participants || [
          { id: SENDER, admin: null },
          { id: BOT, admin: 'admin' },
        ],
      };
    },
    async sendMessage(jid, content) {
      calls.push({ jid, content });
      if (content?.delete) return { key: { id: 'd' } };
      return { key: { id: 's' } };
    },
  };
}

function makeRawMsg(message, key = {}) {
  return {
    key: {
      remoteJid: CHAT,
      fromMe: false,
      id: 'msg1',
      participant: SENDER,
      ...key,
    },
    message,
  };
}

describe('detectSwGcType', () => {
  it('mendeteksi groupMentionedMessage', () => {
    assert.equal(
      detectSwGcType({ message: { groupMentionedMessage: { text: 'hi' } } }),
      'message.groupMentionedMessage',
    );
  });

  it('mendeteksi statusMentionMessage', () => {
    assert.equal(
      detectSwGcType({ message: { statusMentionMessage: { text: 'hi' } } }),
      'message.statusMentionMessage',
    );
  });

  it('mendeteksi groupStatusMentionMessage', () => {
    assert.equal(
      detectSwGcType({
        message: { groupStatusMentionMessage: { text: 'hi' } },
      }),
      'message.groupStatusMentionMessage',
    );
  });

  it('mendeteksi groupStatusMessage', () => {
    assert.equal(
      detectSwGcType({ message: { groupStatusMessage: { text: 'hi' } } }),
      'message.groupStatusMessage',
    );
  });

  it('mendeteksi contextInfo.groupMentions yang tidak kosong', () => {
    const type = detectSwGcType({
      message: {
        extendedTextMessage: {
          text: 'hi',
          contextInfo: { groupMentions: [{ groupJid: CHAT }] },
        },
      },
    });
    assert.equal(type, 'message.extendedTextMessage.contextInfo.groupMentions');
  });

  it('TIDAK menandai groupMentions kosong sebagai SW', () => {
    // groupMentions: [] adalah hal normal — bot lo sendiri mengirim ini
    // (src/lib/serialize.js:1201). Kalau ikut kena, bot delete pesan sendiri.
    assert.equal(
      detectSwGcType({
        message: {
          extendedTextMessage: { text: 'hi', contextInfo: { groupMentions: [] } },
        },
      }),
      null,
    );
  });

  it('TIDAK menandai pesan teks biasa', () => {
    assert.equal(
      detectSwGcType({ message: { conversation: 'halo dunia' } }),
      null,
    );
  });

  it('aman dari cyclic object', () => {
    const msg = { conversation: 'x' };
    msg.contextInfo = { message: msg };
    assert.doesNotThrow(() => detectSwGcType({ message: msg }));
  });
});

describe('handleAntiSwGc', () => {
  let sock;

  beforeEach(() => {
    sock = makeSock();
  });

  it('tidak aktif kalau setting bukan "on"', async () => {
    const handled = await handleAntiSwGc(
      makeRawMsg({ groupMentionedMessage: { text: 'x' } }),
      sock,
      makeDb({ antiswgc: 'off' }),
    );
    assert.equal(handled, false);
    assert.equal(sock.calls.length, 0, 'tidak boleh kirim apa pun');
  });

  it('hapus pesan + laporkan saatgroup mention terdeteksi', async () => {
    const handled = await handleAntiSwGc(
      makeRawMsg({ groupMentionedMessage: { text: 'x' } }),
      sock,
      makeDb(),
    );
    assert.equal(handled, true);

    const deletes = sock.calls.filter((c) => c.content?.delete);
    assert.equal(deletes.length, 1, 'harus ada 1 revoke');
    assert.equal(deletes[0].content.delete.id, 'msg1');

    const notice = sock.calls.find((c) => c.content?.text);
    assert.ok(notice, 'harus ada pesan pemberitahuan');
    assert.match(notice.content.text, /AntiSWGC/);
    assert.match(notice.content.text, /groupMentionedMessage/);
  });

  it('TIDAK menyentuh pesan tanpa SWGC sama sekali', async () => {
    const handled = await handleAntiSwGc(
      makeRawMsg({ conversation: 'halo dunia' }),
      sock,
      makeDb(),
    );
    assert.equal(handled, false);
    assert.equal(sock.calls.length, 0, 'pesan biasa tidak boleh dihapus');
  });

  it('TIDAK menyentuh pesan bot sendiri', async () => {
    const handled = await handleAntiSwGc(
      makeRawMsg(
        { groupMentionedMessage: { text: 'x' } },
        { participant: BOT },
      ),
      sock,
      makeDb(),
    );
    assert.equal(handled, false);
    assert.equal(sock.calls.length, 0);
  });

  it('TIDAK menyentuh pesan dari admin', async () => {
    const adminSock = makeSock({
      participants: [
        { id: SENDER, admin: 'admin' },
        { id: BOT, admin: 'admin' },
      ],
    });
    const handled = await handleAntiSwGc(
      makeRawMsg({ groupMentionedMessage: { text: 'x' } }),
      adminSock,
      makeDb(),
    );
    assert.equal(handled, false);
    assert.equal(adminSock.calls.length, 0);
  });

  it('memberi tahu kalau bot bukan admin, tanpa revoke', async () => {
    const notAdminSock = makeSock({
      participants: [
        { id: SENDER, admin: null },
        { id: BOT, admin: null },
      ],
    });
    const handled = await handleAntiSwGc(
      makeRawMsg({ groupMentionedMessage: { text: 'x' } }),
      notAdminSock,
      makeDb(),
    );
    assert.equal(handled, true);

    const deletes = notAdminSock.calls.filter((c) => c.content?.delete);
    assert.equal(deletes.length, 0, 'tidak boleh coba revoke tanpa admin');

    const notice = notAdminSock.calls.find((c) => c.content?.text);
    assert.match(notice.content.text, /bukan admin/i);
  });

  it('tidak crash kalau groupMetadata gagal', async () => {
    const brokenSock = {
      user: { id: BOT },
      calls: [],
      async groupMetadata() {
        throw new Error('group metadata unavailable');
      },
      async sendMessage(jid, content) {
        this.calls.push({ jid, content });
      },
    };
    const handled = await handleAntiSwGc(
      makeRawMsg({ groupMentionedMessage: { text: 'x' } }),
      brokenSock,
      makeDb(),
    );
    assert.equal(handled, false, 'gagal metadata = jangan proses');
  });

  it('menangani LID participant (grup addressing mode lid)', async () => {
    const lid = '123456789012345@lid';
    const lidSock = makeSock({
      participants: [
        { id: lid, admin: null },
        { id: BOT, admin: 'admin' },
      ],
    });
    const handled = await handleAntiSwGc(
      makeRawMsg({ groupMentionedMessage: { text: 'x' } }, { participant: lid }),
      lidSock,
      makeDb(),
    );

    assert.equal(handled, true);
    // Revoke harus pakai addressing ASLI (LID), bukan PN hasil resolusi —
    // kalau pakai PN, server tidak cocok dan pesan tidak terhapus.
    const del = lidSock.calls.find((c) => c.content?.delete);
    assert.ok(del, 'pesan harus di-revoke');
    assert.equal(
      del.content.delete.participant,
      lid,
      'revoke harus memakai participant LID asli dari key',
    );
    assert.ok(
      lidSock.calls.some((c) => c.content?.text),
      'harus ada pemberitahuan',
    );
  });

  it('tidak revoke kalau participant LID Woodyrupakan sebagai bot', async () => {
    const botLid = '999888777666555@lid';
    const botLidSock = makeSock({
      participants: [
        { id: '111222333@lid', admin: null },
        { id: BOT, admin: 'admin' },
      ],
    });
    botLidSock.user = { id: BOT, lid: botLid };

    const handled = await handleAntiSwGc(
      makeRawMsg(
        { groupMentionedMessage: { text: 'x' } },
        { participant: botLid },
      ),
      botLidSock,
      makeDb(),
    );
    assert.equal(handled, false, 'LID bot sendiri tidak boleh diproses');
  });
});

describe('false positive: pesan biasa yang contain SWGC di dalam quote', () => {
  it('TIDAK menandai pesan yang mengutip pesan SWGC', () => {
    // User balas pesan yang isinya status-mention. Balasannya sendiri
    // BUKAN SWGC — hanya ada di quotedMessage. Kalau detector rekursif
    // tanpa batas, bot akan hapus balasan biasa.
    const type = detectSwGcType({
      message: {
        extendedTextMessage: {
          text: 'mantap',
          contextInfo: {
            quotedMessage: {
              groupMentionedMessage: { text: 'cek ini' },
            },
          },
        },
      },
    });
    assert.equal(
      type,
      null,
      `pesan balasan biasa harus aman, bukan "${type}"`,
    );
  });

  it('TIDAK menandai statusMentionMessage yang ada di dalam quote', () => {
    const type = detectSwGcType({
      message: {
        imageMessage: {
          caption: 'wih',
          contextInfo: {
            quotedMessage: { statusMentionMessage: { text: 'sw' } },
          },
        },
      },
    });
    assert.equal(type, null);
  });
});

describe('false negative: bentuk yang belum terdeteksi', () => {
  it('mendeteksi groupStatusMessageV2Extension', () => {
    // onigis TIDAK meng-unwrap key ini, dan gate lama di connection.js
    // juga tidak mengeceknya — ini penyebab "kadang lolos".
    const type = detectSwGcType({
      message: {
        groupStatusMessageV2Extension: {
          message: { conversation: 'hai' },
        },
      },
    });
    assert.equal(
      type,
      'message.groupStatusMessageV2Extension',
      'groupStatusMessageV2Extension harus terdeteksi',
    );
  });

  it('mendeteksi groupMentions di dalam documentWithCaptionMessage', () => {
    const type = detectSwGcType({
      message: {
        documentWithCaptionMessage: {
          message: {
            documentMessage: {
              contextInfo: { groupMentions: [{ groupJid: CHAT }] },
            },
          },
        },
      },
    });
    assert.ok(
      type && type.includes('groupMentions'),
      `documentWithCaption harus ikut dicek, dapat: ${type}`,
    );
  });

  it('mendeteksi groupMentions di dalam ephemeralMessage', () => {
    const type = detectSwGcType({
      message: {
        ephemeralMessage: {
          message: {
            extendedTextMessage: {
              text: 'hai',
              contextInfo: { groupMentions: [{ groupJid: CHAT }] },
            },
          },
        },
      },
    });
    assert.ok(type && type.includes('groupMentions'), `dapat: ${type}`);
  });
});

describe('isSwGcCandidate — satu detector untuk semua pemanggil', () => {
  it('true untuk SWGC langsung', () => {
    assert.equal(
      isSwGcCandidate({ message: { groupMentionedMessage: { text: 'x' } } }),
      true,
    );
  });

  it('false untuk pesan biasa', () => {
    assert.equal(
      isSwGcCandidate({ message: { conversation: 'halo' } }),
      false,
    );
  });

  it('menyamakan dengan detectSwGcType untuk semua bentuk', () => {
    const shapes = [
      { message: { groupStatusMessage: { text: 'x' } } },
      { message: { groupStatusMessageV2: { text: 'x' } } },
      { message: { groupStatusMessageV2Extension: { message: { conversation: 'x' } } } },
      { message: { statusMentionMessage: { text: 'x' } } },
      {
        message: {
          extendedTextMessage: {
            text: 'x',
            contextInfo: { groupMentions: [{ groupJid: CHAT }] },
          },
        },
      },
      {
        message: {
          ephemeralMessage: {
            message: { groupMentionedMessage: { text: 'x' } },
          },
        },
      },
      { message: { conversation: 'biasa' } },
      { message: {} },
    ];
    for (const s of shapes) {
      assert.equal(
        isSwGcCandidate(s),
        Boolean(detectSwGcType(s)),
        `ketidakcocokan untuk ${JSON.stringify(s).slice(0, 70)}`,
      );
    }
  });

  it('aman untuk input rusak', () => {
    assert.equal(isSwGcCandidate(null), false);
    assert.equal(isSwGcCandidate({}), false);
    assert.equal(isSwGcCandidate({ message: null }), false);
  });
});

describe('handleAntiTagSW — bug yang sama dengan antiswgc', () => {
  // handleAntiTagSW punya pola botNumber yang identik dengan bug yang
  // sudah ditemukan di handleAntiSwGc: split(":")[0] + "@s.whatsapp.net"
  // menghasilkan domain dobel kalau sock.user.id tidak punya device suffix.
  function tagSock(opts = {}) {
    const calls = [];
    return {
      calls,
      user: { id: opts.userId ?? BOT, ...(opts.lid ? { lid: opts.lid } : {}) },
      async groupMetadata() {
        return {
          // WA mengirim participants[].id sebagai PN polos; device suffix
          // hanya ada di sock.user.id.
          participants: opts.participants || [
            { id: SENDER, admin: null },
            { id: String(opts.userId ?? BOT).split(':')[0], admin: 'admin' },
          ],
        };
      },
      async sendMessage(jid, content) {
        calls.push({ jid, content });
      },
    };
  }

  const SW_MSG = { groupMentionedMessage: { text: 'sw' } };

  it('TIDAK revoke pesan bot sendiri (userId tanpa device suffix)', async () => {
    const sock = tagSock({ userId: '62899@s.whatsapp.net' });
    const handled = await handleAntiTagSW(
      { key: { remoteJid: CHAT, fromMe: false, id: 'm1', participant: BOT }, message: SW_MSG },
      sock,
      makeDb({ antitagsw: 'on', antiswgc: 'off' }),
    );
    assert.equal(handled, false, 'pesan bot sendiri tidak boleh diproses');
    assert.equal(sock.calls.length, 0, `tidak boleh kirim apa pun. Actual: ${JSON.stringify(sock.calls)}`);
  });

  it('TIDAK revoke pesan bot sendiri (userId dengan device suffix)', async () => {
    const sock = tagSock({ userId: '62899:12@s.whatsapp.net' });
    const handled = await handleAntiTagSW(
      { key: { remoteJid: CHAT, fromMe: false, id: 'm1', participant: '62899@s.whatsapp.net' }, message: SW_MSG },
      sock,
      makeDb({ antitagsw: 'on', antiswgc: 'off' }),
    );
    assert.equal(handled, false);
    assert.equal(sock.calls.length, 0);
  });

  it('TIDAK revoke pesan bot sendiri di grup addressing_mode=lid', async () => {
    const botLid = '999888777666555@lid';
    const sock = tagSock({ userId: BOT, lid: botLid });
    const handled = await handleAntiTagSW(
      { key: { remoteJid: CHAT, fromMe: false, id: 'm1', participant: botLid }, message: SW_MSG },
      sock,
      makeDb({ antitagsw: 'on', antiswgc: 'off' }),
    );
    assert.equal(handled, false, 'LID bot sendiri tidak boleh diproses');
    assert.equal(sock.calls.length, 0);
  });

  it('TETAP revoke pesan user biasa saat bot punya device suffix', async () => {
    const sock = tagSock({ userId: '62899:12@s.whatsapp.net' });
    const handled = await handleAntiTagSW(
      { key: { remoteJid: CHAT, fromMe: false, id: 'm1', participant: SENDER }, message: SW_MSG },
      sock,
      makeDb({ antitagsw: 'on', antiswgc: 'off' }),
    );
    assert.equal(handled, true, 'fitur harus tetap berfungsi');
    assert.ok(sock.calls.some((c) => c.content?.delete), 'harus ada revoke');
  });

  it('TETAP revoke pesan user biasa saat bot tanpa device suffix', async () => {
    const sock = tagSock({ userId: '62899@s.whatsapp.net' });
    const handled = await handleAntiTagSW(
      { key: { remoteJid: CHAT, fromMe: false, id: 'm1', participant: SENDER }, message: SW_MSG },
      sock,
      makeDb({ antitagsw: 'on', antiswgc: 'off' }),
    );
    assert.equal(handled, true, 'fix tidak boleh mematikan fitur');
    assert.ok(sock.calls.some((c) => c.content?.delete), 'harus ada revoke');
  });

  it('TIDAK aktif kalau setting bukan on', async () => {
    const sock = tagSock({});
    const handled = await handleAntiTagSW(
      { key: { remoteJid: CHAT, fromMe: false, id: 'm1', participant: SENDER }, message: SW_MSG },
      sock,
      makeDb({ antitagsw: 'off' }),
    );
    assert.equal(handled, false);
    assert.equal(sock.calls.length, 0);
  });
});
