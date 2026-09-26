import { test, mock, before, describe } from 'node:test';
import assert from 'node:assert';
import { readFile } from 'node:fs/promises';

const realOnigis = await import('onigis');

const PNG = Buffer.concat([
  Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
  Buffer.alloc(256, 0x41),
]);
// Buffer yang dikembalikan downloadMediaMessage. Ditukar di test batas ukuran.
let PNG_SIZE = null;

const HOST_URLS = {
  litterbox: 'https://litter.catbox.moe/abc.png',
  kappa: 'https://kappa.lol/abc',
  uguu: 'https://d.uguu.se/abc.png',
  tmpfiles: 'https://tmpfiles.org/123/abc.png',
  uploadee: 'https://www.upload.ee/image/1/abc.png',
  top4top: 'https://g.top4top.io/p_abc.png',
  leopard: 'https://leopard.hosting.pecon.us/dl/abc/abc.png',
  quax: 'https://qu.ax/abc',
  nekohime: 'https://cdn.nekohime.site/file/abc.png',
  pasters: 'https://paste.rs/abc',
  cnets: 'https://paste.c-net.org/abc',
};

let fetchCalls = [];
let downloadShouldFail = false;
let downloadError = new Error('media 410gone');

const downloadMediaMessage = async () => {
  if (downloadShouldFail) throw downloadError;
  return PNG_SIZE || PNG;
};

function fakeResponse(url) {
  const u = String(url);
  if (u.includes('litterbox')) {
    return { ok: true, text: async () => HOST_URLS.litterbox };
  }
  if (u.includes('kappa.lol')) {
    return {
      ok: true,
      text: async () => JSON.stringify({ link: HOST_URLS.kappa }),
    };
  }
  if (u.includes('uguu.se')) {
    return {
      ok: true,
      json: async () => ({
        success: true,
        files: [{ url: HOST_URLS.uguu }],
      }),
    };
  }
  if (u.includes('tmpfiles.org')) {
    return {
      ok: true,
      json: async () => ({
        status: 'success',
        data: { url: HOST_URLS.tmpfiles },
      }),
    };
  }
  if (u.includes('upload.ee')) {
    if (u.includes('ubr_link_upload')) {
      return { text: async () => '<script>startUpload("PID123")</script>' };
    }
    if (u.includes('page=finished')) {
      return {
        text: async () =>
          `<input id="file_src" value="${HOST_URLS.uploadee}">`,
      };
    }
    return { text: async () => '' };
  }
  if (u.includes('top4top.io')) {
    if (u.endsWith('/')) {
      return { text: async () => '<input name="sid" value="S1">' };
    }
    return {
      text: async () => `<input value="${HOST_URLS.top4top}">`,
    };
  }
  if (u.includes('leopard')) {
    return {
      text: async () =>
        `Download link: <a href=${HOST_URLS.leopard}>x</a>`,
    };
  }
  if (u.includes('qu.ax')) {
    return {
      ok: true,
      json: async () => ({ success: true, files: [{ url: HOST_URLS.quax }] }),
    };
  }
  if (u.includes('nekohime')) {
    return {
      ok: true,
      json: async () => ({ files: [{ url: HOST_URLS.nekohime }] }),
    };
  }
  if (u.includes('paste.rs')) {
    return { ok: true, text: async () => HOST_URLS.pasters };
  }
  if (u.includes('c-net.org')) {
    return { ok: true, text: async () => HOST_URLS.cnets + '\n' };
  }
  return { ok: false, status: 500, text: async () => '', json: async () => ({}) };
}

let pasteHostsDown = false;
// Kalau hostDelayMs > 0, setiap host file SLEEP selama itu (ms) sebelum
// merespons — dipakai untuk menguji fan-out benar-benar paralel.
let hostDelayMs = 0;
// Host tertentu bisa dibuat lambat tanpa memperlambat yang lain.
let slowHostMs = 0;
const slowHosts = new Set();
let inflightPeak = 0;
let inflightNow = 0;

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

function delayFor(u) {
  if (u.includes('paste.')) return 0;
  for (const key of slowHosts) {
    if (u.includes(key)) return slowHostMs;
  }
  return hostDelayMs;
}

const fakeFetch = async (url, opts = {}) => {
  const u = String(url);
  fetchCalls.push({ url: u, method: opts.method || 'GET' });

  if (u.includes('paste.rs') || u.includes('c-net.org')) {
    fetchCalls.push({ pasteBody: String(opts.body) });
    if (pasteHostsDown) {
      return { ok: false, status: 503, text: async () => '' };
    }
  }

  const delay = delayFor(u);
  if (delay > 0) {
    inflightNow += 1;
    if (inflightNow > inflightPeak) inflightPeak = inflightNow;
    try {
      await sleep(delay);
    } finally {
      inflightNow -= 1;
    }
  }

  return fakeResponse(url);
};

mock.module('node-fetch', { defaultExport: fakeFetch });
const { default: _ignored, ...onigisNamed } = realOnigis;

mock.module('onigis', {
  namedExports: {
    ...onigisNamed,
    downloadMediaMessage,
  },
});

const mod = await import('../plugins/tools/tourl.js');
const { handler } = mod;

function makeSock() {
  const calls = { sendButton: [], relay: [], send: [] };
  return {
    calls,
    user: { id: '62811@s.whatsapp.net' },
    async sendButton(...args) {
      calls.sendButton.push(args);
      return { key: { id: 'btn1' } };
    },
    async relayMessage(jid, message, options) {
      calls.relay.push({ jid, message, options });
    },
    async sendMessage(jid, content) {
      calls.send.push({ jid, content });
      return { key: { id: 's1' } };
    },
  };
}

function makeM(over = {}) {
  return {
    prefix: '.',
    command: 'tourl',
    chat: '62811@s.whatsapp.net',
    sender: '62822@s.whatsapp.net',
    pushName: 'Tester',
    type: 'imageMessage',
    message: { imageMessage: { mimetype: 'image/png' } },
    mimetype: 'image/png',
    isMedia: true,
    quoted: null,
    args: [],
    fullArgs: '',
    key: { remoteJid: '62811@s.whatsapp.net', id: 'm1' },
    reply: async () => {},
    react: async () => {},
    ...over,
  };
}

before(() => {
  fetchCalls = [];
});

/** Reset counter timing sebelum tiap test Concurrency. */
function resetConcurrency() {
  fetchCalls = [];
  hostDelayMs = 0;
  inflightPeak = 0;
  inflightNow = 0;
}

describe('tourl — text path (paste)', () => {
  test('teks setelah command di-paste, bukan dikirim ke host file', async () => {
    const sock = makeSock();
    const m = makeM({
      isMedia: false,
      type: 'conversation',
      message: { conversation: '.tourl halo dunia ini isi pasteku' },
      fullArgs: 'halo dunia ini isi pasteku',
    });

    await handler(m, { sock });

    assert.equal(sock.calls.sendButton.length, 1, 'harus kirim card hasil paste');
    const [, source, text, , options] = sock.calls.sendButton[0];
    assert.strictEqual(source, null, 'tidak ada header media untuk teks');
    assert.match(text, /paste\.rs/);
    assert.match(text, /c-net\.org/);

    const posted = fetchCalls.filter((c) => c.pasteBody !== undefined);
    assert.ok(posted.length >= 1, 'harus POST body teks ke paste host');
    assert.ok(
      posted.some((c) => c.pasteBody.includes('halo dunia ini isi pasteku')),
      `body paste harus berisi teks aslinya, dapat: ${JSON.stringify(posted)}`,
    );
    assert.ok(
      !fetchCalls.some((c) => /litterbox|kappa\.lol|uguu\.se|tmpfiles|upload\.ee|top4top|leopard|qu\.ax|nekohime/.test(c.url || '')),
      'jalur teks tidak boleh menyentuh host file',
    );
  });

  test('teks yang di-reply ikut di-paste', async () => {
    const sock = makeSock();
    const m = makeM({
      isMedia: false,
      type: 'conversation',
      message: { conversation: '.tourl' },
      fullArgs: '',
      quoted: {
        isMedia: false,
        type: 'conversation',
        message: { conversation: 'ini isi chat yang mau dipaste' },
        body: 'ini isi chat yang mau dipaste',
        key: { remoteJid: '62811@s.whatsapp.net', id: 'q1' },
      },
    });

    await handler(m, { sock });

    assert.equal(sock.calls.sendButton.length, 1);
    const posted = fetchCalls.filter((c) => c.pasteBody !== undefined);
    assert.ok(
      posted.some((c) => c.pasteBody.includes('ini isi chat yang mau dipaste')),
      'harus paste teks yang di-reply',
    );
  });

  test('args menang atas teks yang di-reply', async () => {
    const sock = makeSock();
    const m = makeM({
      isMedia: false,
      type: 'conversation',
      message: { conversation: '.tourl isi dari args' },
      fullArgs: 'isi dari args',
      quoted: {
        isMedia: false,
        type: 'conversation',
        message: { conversation: 'isi dari reply' },
        body: 'isi dari reply',
        key: { remoteJid: '62811@s.whatsapp.net', id: 'q1' },
      },
    });

    await handler(m, { sock });

    const posted = fetchCalls.filter((c) => c.pasteBody !== undefined);
    assert.ok(
      posted.some((c) => c.pasteBody.includes('isi dari args')),
      'args harus dipakai',
    );
    assert.ok(
      !posted.some((c) => c.pasteBody.includes('isi dari reply')),
      'teks reply tidak boleh dipakai kalau ada args',
    );
  });

  test('command kosong tanpa media tetap nampilin usage', async () => {
    const sock = makeSock();
    const replies = [];
    const m = makeM({
      isMedia: false,
      type: 'conversation',
      message: { conversation: '.tourl' },
      fullArgs: '',
      reply: async (t) => replies.push(t),
    });

    await handler(m, { sock });

    assert.equal(sock.calls.sendButton.length, 0, 'tidak boleh kirim card');
    assert.equal(replies.length, 1);
    assert.match(replies[0], /MEDIA UPLOADER/);
  });

  test('kirim card lewat sock.sendButton dengan tombol cta_copy, bukan relayMessage mentah', async () => {
    const sock = makeSock();
    await handler(makeM(), { sock });

    assert.equal(
      sock.calls.sendButton.length,
      1,
      'harus lewat sock.sendButton supaya node biz/native_flow ikut',
    );
    assert.equal(
      sock.calls.relay.length,
      0,
      'tidak boleh relayMessage manual — tombol cta_copy hilang diam-diam',
    );

    const [, source, text, quoted, options] = sock.calls.sendButton[0];
    assert.ok(Buffer.isBuffer(source), 'header media harus buffer');
    assert.match(text, /UPLOAD BERHASIL/);
    assert.strictEqual(quoted.command, 'tourl');
    assert.ok(Array.isArray(options.buttons), 'harus ada buttons');
    assert.ok(options.buttons.length > 0, 'buttons tidak boleh kosong');
    for (const btn of options.buttons) {
      assert.equal(btn.name, 'cta_copy');
      const params = JSON.parse(btn.buttonParamsJson);
      assert.match(params.copy_code, /^https?:\/\//, 'copy_code harus URL');
    }
  });

  test('semua host yang sukses|url-nya muncul di body, tombol dibatasi 5', async () => {
    const sock = makeSock();
    await handler(makeM(), { sock });

    const [, , text, , options] = sock.calls.sendButton[0];

    for (const url of [
      HOST_URLS.litterbox,
      HOST_URLS.kappa,
      HOST_URLS.uguu,
      HOST_URLS.tmpfiles,
      HOST_URLS.uploadee,
      HOST_URLS.top4top,
      HOST_URLS.leopard,
      HOST_URLS.quax,
      HOST_URLS.nekohime,
    ]) {
      assert.ok(text.includes(url), `body harus memuat ${url}`);
    }

    assert.equal(options.buttons.length, 5, 'tombol dibatasi 5');
    for (const btn of options.buttons) {
      assert.equal(btn.name, 'cta_copy');
      const params = JSON.parse(btn.buttonParamsJson);
      assert.ok(text.includes(params.copy_code), 'copy_code harus ada di body');
      assert.doesNotMatch(params.copy_code, /paste\./, 'jalur media tidak pakai paste');
    }
  });

  test('error download di-log, bukan ditelan diam-diam', async () => {
    const sock = makeSock();
    const replies = [];
    const logged = [];
    const realError = console.error;
    downloadShouldFail = true;
    console.error = (...a) => logged.push(a.join(' '));
    try {
      await handler(makeM({ reply: async (t) => replies.push(t) }), { sock });
    } finally {
      console.error = realError;
      downloadShouldFail = false;
    }
    assert.equal(sock.calls.sendButton.length, 0, 'tidak boleh kirim card');
    assert.ok(replies.length > 0, 'user tetap dapat pesan error');
    assert.ok(
      logged.some((l) => l.includes('410gone')),
      `log harus memuat pesan error asli, dapat: ${JSON.stringify(logged)}`,
    );
  });

  test('error download di-log pada jalur media-yang-direply juga', async () => {
    const sock = makeSock();
    const replies = [];
    const logged = [];
    const realError = console.error;
    downloadShouldFail = true;
    console.error = (...a) => logged.push(a.join(' '));
    try {
      const m = makeM({
        isMedia: false,
        type: 'conversation',
        message: { conversation: '.tourl' },
        reply: async (t) => replies.push(t),
        quoted: {
          isMedia: true,
          type: 'imageMessage',
          message: { imageMessage: { mimetype: 'image/png' } },
          key: { remoteJid: '62811@s.whatsapp.net', id: 'q1' },
        },
      });
      await handler(m, { sock });
    } finally {
      console.error = realError;
      downloadShouldFail = false;
    }
    assert.equal(sock.calls.sendButton.length, 0, 'tidak boleh kirim card');
    assert.ok(replies.length > 0, 'user tetap dapat pesan error');
    assert.ok(
      logged.some((l) => l.includes('410gone')),
      `log harus memuat pesan error asli, dapat: ${JSON.stringify(logged)}`,
    );
  });

  test('tidak ada host mati yang diklaim sukses', async () => {
    const sock = makeSock();
    await handler(makeM(), { sock });
    const text = sock.calls.sendButton[0][2];

    assert.doesNotMatch(text, /catbox\.moe\/files/);
    assert.doesNotMatch(text, /0x0\.st/);
    assert.doesNotMatch(text, /faddlaninco/);
    assert.doesNotMatch(text, /imgdrop/);
    assert.doesNotMatch(text, /8upload/);
    assert.doesNotMatch(text, /unggah\.web\.id/);
  });
});

// test/utils ini sengaja dipakai dua suite di atas; pola mock dicopy dari
// tests/plugin-cmds.test.mjs (mockSock + mockM).
describe('tourl — fan-out upload', () => {
  // 9 host × timeout 30-120s kalau sequential = menit. Harusnya paralel.
  test('9 host dijalankan paralel, bukan satu-satu', async () => {
    resetConcurrency();
    hostDelayMs = 120;

    const sock = makeSock();
    const t0 = Date.now();
    await handler(makeM(), { sock });
    const elapsed = Date.now() - t0;

    assert.ok(
      inflightPeak >= 4,
      `host harus jalan bareng. Peak concurrency cuma ${inflightPeak} — masih sequential`,
    );
    // 9 host × 120ms sequential = 1080ms+; paralel jauh di bawah itu.
    assert.ok(
      elapsed < 900,
      `terlalu lama (${elapsed}ms) untuk 9 host yang jalan bareng`,
    );
    assert.equal(sock.calls.sendButton.length, 1, 'tetap kirim satu card');
  });

  test('semua host tetap dicoba walau sebagian gagal', async () => {
    resetConcurrency();
    const sock = makeSock();
    await handler(makeM(), { sock });

    const text = sock.calls.sendButton[0][2];
    // Setiap uploader punya beberapa request (upload.ee = 3, top4top = 2),
    // jadi cek host-nya muncul di call log — bukan URL persisnya.
    const triedHosts = [
      'litterbox',
      'kappa.lol',
      'uguu.se',
      'tmpfiles.org',
      'upload.ee',
      'top4top.io',
      'leopard',
      'qu.ax',
      'nekohime',
    ];
    for (const h of triedHosts) {
      assert.ok(
        fetchCalls.some((c) => c.url.includes(h)),
        `${h} tidak pernah dicoba — fan-out tidak lengkap. Actual: ${JSON.stringify([...new Set(fetchCalls.map((c) => c.url.slice(0, 45)))].slice(0, 12))}`,
      );
    }
    assert.ok(text.includes('UPLOAD BERHASIL'));
  });

  test('satu host lambat tidak menghilangkan hasil host lain', async () => {
    resetConcurrency();
    // Nekohime lambat 4 detik. perHostMs di plugin 30s, jadi host ini
    // memang selesai (bukan timeout) — yang diuji di sini: hasilnya tetap
    // terkirim bersama 8 host lain, tidak ada yang hilang.
    slowHosts.add('nekohime');
    slowHostMs = 4000;
    try {
      const sock = makeSock();
      await handler(makeM(), { sock });

      assert.equal(sock.calls.sendButton.length, 1);
      const text = sock.calls.sendButton[0][2];
      assert.ok(text.includes('kappa.lol'), 'host cepat harus tetap masuk');
      assert.ok(text.includes('qu.ax'), 'host cepat harus tetap masuk');
      assert.ok(text.includes('nekohime.site'), 'host lambat juga harus masuk');
    } finally {
      slowHosts.clear();
      slowHostMs = 0;
    }
  });

  test('host yang tidak merespons dalam deadline ditandai timeout', async () => {
    resetConcurrency();
    // Semua host sleep 30 detik — melewati perHostMs (30s) dan deadline
    // (45s) kalau keduanya bekerja. Tapi ini akan bikin test 30+ detik,
    // jadi yang diuji: user tetap dapat balapan (tidak menggantung).
    // Deadline-nya sendiri diuji langsung di tests/upload-fanout.test.mjs.
    hostDelayMs = 30000;

    const sock = makeSock();
    const replies = [];
    const t0 = Date.now();
    await handler(
      makeM({ reply: async (t) => replies.push(t), react: async () => {} }),
      { sock },
    );
    const elapsed = Date.now() - t0;

    assert.ok(
      elapsed < 60000,
      `harus terpotong oleh deadline, bukan menunggu 30s+ per host. Elapsed: ${elapsed}ms`,
    );
    const delivered = sock.calls.sendButton.length > 0 || replies.length > 0;
    assert.ok(delivered, 'user harus tetap dapat balasan');
  });
});

describe('tourl — batas ukuran file', () => {
  test('file melebihi batas ditolak sebelum contacting host mana pun', async () => {
    resetConcurrency();
    const prev = PNG_SIZE;
    PNG_SIZE = Buffer.alloc(120 * 1024 * 1024, 0x41);
    try {
      const sock = makeSock();
      const replies = [];
      await handler(
        makeM({ reply: async (t) => replies.push(t), react: async () => {} }),
        { sock },
      );

      assert.equal(
        fetchCalls.filter(
          (c) =>
            /litter|kappa|uguu|tmpfiles|upload\.ee|top4top|leopard|qu\.ax|nekohime/.test(
              c.url,
            ),
        ).length,
        0,
        'tidak boleh mulai upload kalau file kelewat besar',
      );
      assert.ok(replies.length > 0, 'user harus diberi tahu file terlalu besar');
      assert.match(replies[0], /terlalu besar|MB|GB/i);
    } finally {
      PNG_SIZE = prev;
    }
  });

  test('file di bawah batas tetap di-upload normal', async () => {
    resetConcurrency();
    PNG_SIZE = Buffer.alloc(2 * 1024 * 1024, 0x41);
    try {
      const sock = makeSock();
      await handler(makeM(), { sock });
      assert.equal(
        sock.calls.sendButton.length,
        1,
        'file 2MB harus tetap diproses',
      );
    } finally {
      PNG_SIZE = null;
    }
  });
});

describe('tourl — paste gagal semua', () => {
  test('kalau semua paste host mati, lapor ke user dan jangan diam', async () => {
    const sock = makeSock();
    const replies = [];
    const logged = [];
    const realError = console.error;
    pasteHostsDown = true;
    console.error = (...a) => logged.push(a.join(' '));
    try {
      await handler(
        makeM({
          isMedia: false,
          type: 'conversation',
          message: { conversation: '.tourl teks yang pasti gagal' },
          fullArgs: 'teks yang pasti gagal',
          reply: async (t) => replies.push(t),
        }),
        { sock },
      );
    } finally {
      console.error = realError;
      pasteHostsDown = false;
    }
    assert.equal(sock.calls.sendButton.length, 0, 'tidak boleh klaim sukses');
    assert.equal(replies.length, 1, 'user harus diberi tahu');
    assert.match(replies[0], /gagal semua/);
    assert.ok(
      logged.some((l) => l.includes('[tourl]')),
      'error paste harus di-log',
    );
  });
});

describe('tourl — impor & regression', () => {
  test('tidak ada sisa referensi host yang sudah dibuang', async () => {
    const src = await readFile(
      new URL('../plugins/tools/tourl.js', import.meta.url),
      'utf8',
    );
    for (const gone of [
      'uploadToCatbox',
      'uploadToPone',
      'uploadTo8upload',
      'uploadTo0x0_alt',
      'uploadToTermai',
      'uploadToFaddlaninco',
      'uploadToUnggah',
      'uploadToImgDrop',
      'imgdrop',
      'file-type',
    ]) {
      assert.doesNotMatch(src, new RegExp(gone), `${gone} harus hilang`);
    }
    assert.match(src, /sendButton/, 'harus pakai sock.sendButton');
  });

  test('tidak ada import mati di tourl', async () => {
    const src = await readFile(
      new URL('../plugins/tools/tourl.js', import.meta.url),
      'utf8',
    );
    for (const [imp, uses] of [
      ['FormData', /new FormData\(\)/],
      ['fetch', /await fetch\(/],
      ['mime', /mime\.lookup/],
      ['downloadMediaMessage', /downloadMediaMessage\(/],
      ['getContentType', /getContentType\(/],
      ['te', /te\(/],
      ['config', /config\.bot/],
    ]) {
      assert.match(src, uses, `${imp} di-import tapi tidak dipakai — hapus import-nya`);
    }
  });

  test('tidak ada import mati di plugin yang di-swap', async () => {
    const files = [
      '../plugins/stalker/githubstalk.js',
      '../plugins/stalker/tiktokstalk.js',
      '../plugins/stalker/npmstalk.js',
      '../plugins/ai/wormgpt.js',
      '../plugins/ai/gita.js',
    ];
    for (const f of files) {
      const src = await readFile(new URL(f, import.meta.url), 'utf8');
      // Kalau axios/config masih di-import tapi tidak ada yang memakainya,
      // itu sisa refactor — buang supaya tidak membingungkan.
      if (/^import axios/m.test(src)) {
        assert.match(
          src,
          /axios\.(get|post)\(/,
          `${f} import axios tapi tidak memakainya`,
        );
      }
      if (/^import config/m.test(src)) {
        assert.match(
          src,
          /config\./,
          `${f} import config tapi tidak memakainya`,
        );
      }
    }
  });
});

describe('tourl — regression', () => {
  test('no-op split/map/join dibuang', async () => {
    const src = await readFile(
      new URL('../plugins/tools/tourl.js', import.meta.url),
      'utf8',
    );
    assert.doesNotMatch(
      src,
      /contentTxt\.split\("\\n"\)\.map/,
      'fungsi identitas harus dihapus',
    );
  });
});
