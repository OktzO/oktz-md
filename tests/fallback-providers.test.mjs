import { test, mock, beforeEach, describe } from 'node:test';
import assert from 'node:assert';
import { readFile } from 'node:fs/promises';

const realAxios = await import('axios');
const { default: _ignoredAxiosDefault, ...axiosNamed } = realAxios;
const axiosStub = {
  ...realAxios.default,
  get: async () => {
    throw new Error('axios tidak di-stub untuk test ini');
  },
};
mock.module('axios', { defaultExport: axiosStub, namedExports: axiosNamed });

/**
 * Stub axios.get berdasarkan host. Setiap entri = [regex, handler].
 * Handler mengembalikan { status, body } — throw berarti network error.
 */
let routes = [];
let calls = [];

function installRoutes(list) {
  routes = list;
  calls = [];
  const handle = async (url, opts) => {
    calls.push({ url: String(url), host: hostOf(url) });
    for (const [pattern, handler] of routes) {
      if (pattern.test(String(url))) {
        const out = await handler(String(url), opts);
        if (out instanceof Error) throw out;
        return { status: out.status ?? 200, data: out.body, headers: {} };
      }
    }
    return { status: 404, data: {}, headers: {} };
  };
  axiosStub.get = handle;
  axiosStub.post = handle;
}

function hostOf(url) {
  const m = String(url).match(/^https?:\/\/([^/]+)/);
  return m ? m[1] : '?';
}

const isFirefly = /firefly\.maiku\.my\.id/;
const isNexray = /api\.nexray\.(eu\.cc|web\.id)/;

// Bentuk balasan nexray yang sudah diverifikasi live (lihat tests/tourl.test.mjs
// untuk pola mock yang sama).
const NEXRAY_GITHUB = {
  status: true,
  author: 'NexRay',
  result: {
    username: 'torvalds',
    nickname: 'Linus Torvalds',
    bio: 'creator of Linux',
    profile_pic: 'https://avatars.githubusercontent.com/u/1024025',
    url: 'https://github.com/torvalds',
    company: 'Linux Foundation',
    location: 'Portland, OR',
    email: null,
    public_repo: 12,
    public_gists: 1,
    followers: 325220,
    following: 0,
  },
};

const NEXRAY_TIKTOK = {
  status: true,
  author: 'NexRay',
  result: {
    username: 'mrbeast',
    name: 'Jimmy Donaldson',
    avatar: 'https://p16.example/avatar.jpeg',
    bio: 'youtuber',
    link: 'https://www.tiktok.com/@mrbeast',
    verified: 'Unverified',
    private: 'No',
    stats: {
      followers: '1000',
      following: '10',
      likes: '500',
      videos: '20',
      raw_followers: 1000,
      raw_following: 10,
      raw_likes: 500,
      raw_videos: 20,
    },
  },
};

const NEXRAY_NPMJS = {
  status: true,
  author: 'NexRay',
  result: {
    name: 'antfu',
    version_latest: '0.0.0',
    description: 'Hi, I am Anthony Fu',
    author: { name: 'antfu', email: 'a@b.c' },
    license: 'MIT',
    homepage: 'https://github.com/antfu/antfu#readme',
  },
};

// Bentuk balasan firefly (sekarang) — dipakai untuk membuktikan primary menang.
const FIREFLY_GITHUB = {
  status: true,
  data: {
    username: 'torvalds',
    name: 'Linus Torvalds',
    bio: 'creator of Linux',
    avatar: 'https://avatars.githubusercontent.com/u/1024025',
    url: 'https://github.com/torvalds',
    company: 'Linux Foundation',
    location: 'Portland, OR',
    public_repos: 12,
    followers: 325220,
    following: 0,
  },
};

function mockSock(captured) {
  return {
    user: { id: '62811@s.whatsapp.net' },
    async sendMessage(jid, content) {
      captured.push(content);
      return { key: { id: 's1' } };
    },
    async sendMedia(jid, source, caption) {
      captured.push({ image: source, caption });
      return { key: { id: 's1' } };
    },
    async reply() {
      // m.reply ditangani mockM
    },
  };
}

function mockM(over = {}) {
  return {
    chat: '62811@s.whatsapp.net',
    sender: '62822@s.whatsapp.net',
    pushName: 'Tester',
    prefix: '.',
    command: 'githubstalk',
    args: ['torvalds'],
    key: { remoteJid: '62811@s.whatsapp.net', id: 'm1' },
    message: { conversation: '.githubstalk torvalds' },
    reply: async () => {},
    react: async () => {},
    ...over,
  };
}

async function loadPlugin(path) {
  return import(path);
}

beforeEach(() => {
  installRoutes([]);
});

describe('fallback provider — githubstalk', () => {
  test('primary (firefly) hidup ⇒ nexray TIDAK dipanggil', async () => {
    installRoutes([
      [isFirefly, async () => ({ body: FIREFLY_GITHUB })],
      [isNexray, async () => ({ body: NEXRAY_GITHUB })],
    ]);

    const { handler } = await loadPlugin('../plugins/stalker/githubstalk.js');
    const captured = [];
    await handler(mockM(), { sock: mockSock(captured) });

    assert.ok(calls.length > 0, 'primary harus dipanggil');
    assert.ok(
      calls.every((c) => isFirefly.test(c.url)),
      `nexray tidak boleh dipanggil kalau primary hidup. Actual: ${JSON.stringify(calls.map((c) => c.host))}`,
    );
    assert.equal(captured.length, 1, 'harus tetap terkirim dari primary');
  });

  test('primary 500 ⇒ nexray dipakai dan caption terisi dari shape nexray', async () => {
    installRoutes([
      [isFirefly, async () => ({ status: 500, body: { status: false } })],
      [isNexray, async () => ({ body: NEXRAY_GITHUB })],
    ]);

    const { handler } = await loadPlugin('../plugins/stalker/githubstalk.js');
    const captured = [];
    await handler(mockM(), { sock: mockSock(captured) });

    const nexrayCalls = calls.filter((c) => isNexray.test(c.url));
    assert.ok(nexrayCalls.length > 0, 'nexray harus dicoba saat primary gagal');
    assert.ok(
      nexrayCalls[0].url.includes('torvalds'),
      'username harus diteruskan ke nexray',
    );

    assert.equal(captured.length, 1, 'harus terkirim hasil nexray');
    const out = captured[0];
    const caption = out.caption || '';
    // Field nexray yang namanya beda dari firefly harus tetap termapping.
    assert.match(caption, /torvalds/, 'username harus muncul');
    assert.match(caption, /Linus Torvalds/, 'nickname nexray -> Nama');
    assert.match(caption, /325\.2K|325220/, 'followers nexray harus diformat');
    assert.match(caption, /12/, 'public_repo nexray harus jadi Public Repos');
    assert.ok(
      out.image?.url === NEXRAY_GITHUB.result.profile_pic ||
        out.image === NEXRAY_GITHUB.result.profile_pic,
      `avatar harus dari profile_pic nexray, dapat: ${JSON.stringify(out.image)}`,
    );
  });

  test('primary DAN fallback mati ⇒ user dapat pesan, bukan diam', async () => {
    installRoutes([
      [isFirefly, async () => ({ status: 530, body: {} })],
      [isNexray, async () => ({ status: 404, body: { status: false, error: 'Endpoint not found' } })],
    ]);

    const { handler } = await loadPlugin('../plugins/stalker/githubstalk.js');
    const replies = [];
    const captured = [];
    await handler(
      mockM({ reply: async (t) => replies.push(t) }),
      { sock: mockSock(captured) },
    );

    assert.equal(captured.length, 0, 'tidak boleh mengirim card kosong');
    assert.ok(replies.length > 0, 'user harus diberi tahu');
  });
});

describe('fallback provider — tiktokstalk', () => {
  test('primary 500 ⇒ nexray dipakai, stats.likes dipetakan ke caption', async () => {
    installRoutes([
      [isFirefly, async () => ({ status: 530, body: {} })],
      [isNexray, async () => ({ body: NEXRAY_TIKTOK })],
    ]);

    const { handler } = await loadPlugin('../plugins/stalker/tiktokstalk.js');
    const captured = [];
    await handler(
      mockM({ command: 'tiktokstalk', args: ['mrbeast'] }),
      { sock: mockSock(captured) },
    );

    assert.ok(calls.some((c) => isNexray.test(c.url)), 'nexray harus dicoba');
    assert.equal(captured.length, 1);
    const caption = captured[0].caption || '';
    assert.match(caption, /@mrbeast/);
    assert.match(caption, /Jimmy Donaldson/, 'name nexray -> Nama');
    assert.match(caption, /500/, 'likes nexray harus muncul (firefly pakai s.hearts)');
    assert.match(caption, /20/, 'videos nexray harus muncul');
  });
});

describe('fallback provider — npmstalk', () => {
  test('primary 500 ⇒ nexray /stalker/npmjs dipakai dengan param `name`', async () => {
    installRoutes([
      [isFirefly, async () => ({ status: 530, body: {} })],
      [isNexray, async () => ({ body: NEXRAY_NPMJS })],
    ]);

    const { handler } = await loadPlugin('../plugins/stalker/npmstalk.js');
    const captured = [];
    const replies = [];
    await handler(
      mockM({
        command: 'npmstalk',
        args: ['antfu'],
        reply: async (t) => replies.push(t),
      }),
      { sock: mockSock(captured) },
    );

    const nexrayCalls = calls.filter((c) => isNexray.test(c.url));
    assert.ok(nexrayCalls.length > 0, 'nexray harus dicoba');
    assert.match(
      nexrayCalls[0].url,
      /\/stalker\/npmjs\?/,
      'path harus /stalker/npmjs',
    );
    assert.match(
      nexrayCalls[0].url,
      /name=antfu/,
      'param harus `name` bukan `username` (nexray 400 kalau username)',
    );
    // Nexray npmjs tidak mengembalikan avatar, jadi plugin harus fallback ke
    // teks — tidak boleh mencoba kirim image kosong.
    assert.equal(captured.length, 0, 'tidak boleh kirim image tanpa avatar');
    assert.equal(replies.length, 1, 'harus kirim caption sebagai teks');
    const caption = replies[0];
    assert.match(caption, /antfu/);
    assert.match(caption, /Anthony Fu/, 'description nexray harus tampil');
  });
});

describe('fallback provider — ai (cuki 401 → nexray tanpa key)', () => {
  const CUKI = /api\.cuki\.biz\.id/;

  test('wormgpt: cuki 401 ⇒ nexray /ai/chatgpt dipakai', async () => {
    installRoutes([
      [CUKI, async () => ({ status: 401, body: { success: false, error: 'API key required' } })],
      [isNexray, async () => ({ body: { status: true, author: 'NexRay', result: 'Jawaban dari nexray' } })],
    ]);

    const { handler } = await loadPlugin('../plugins/ai/wormgpt.js');
    const replies = [];
    await handler(
      mockM({ command: 'wormgpt', args: ['perkenalkan dirimu'], reply: async (t) => replies.push(t) }),
      { sock: mockSock([]) },
    );

    const nexrayCalls = calls.filter((c) => isNexray.test(c.url));
    assert.ok(nexrayCalls.length > 0, 'nexray harus dicoba saat cuki 401');
    assert.match(nexrayCalls[0].url, /\/ai\/chatgpt\?/, 'path harus /ai/chatgpt');
    assert.ok(
      !/apikey/i.test(nexrayCalls[0].url),
      'nexray tidak boleh menerima apikey',
    );
    assert.ok(
      replies.some((r) => r.includes('Jawaban dari nexray')),
      `user harus dapat jawaban nexray, dapat: ${JSON.stringify(replies)}`,
    );
  });

  test('wormgpt: cuki hidup ⇒ nexray TIDAK dipanggil', async () => {
    installRoutes([
      [CUKI, async () => ({ body: { status: true, data: { response: 'dari cuki' } } })],
      [isNexray, async () => ({ body: { status: true, result: 'dari nexray' } })],
    ]);

    const { handler } = await loadPlugin('../plugins/ai/wormgpt.js');
    const replies = [];
    await handler(
      mockM({ command: 'wormgpt', args: ['halo'], reply: async (t) => replies.push(t) }),
      { sock: mockSock([]) },
    );

    assert.ok(
      calls.every((c) => !isNexray.test(c.url)),
      `nexray tidak boleh dipanggil. Actual: ${JSON.stringify(calls.map((c) => c.host))}`,
    );
    assert.ok(replies.some((r) => r.includes('dari cuki')));
  });

  test('gita: cuki 401 ⇒ nexray /ai/gitagpt dipakai', async () => {
    installRoutes([
      [CUKI, async () => ({ status: 401, body: { success: false } })],
      [isNexray, async () => ({ body: { status: true, author: 'NexRay', result: 'Jawaran Bhagavad Gita' } })],
    ]);

    const { handler } = await loadPlugin('../plugins/ai/gita.js');
    const replies = [];
    await handler(
      mockM({ command: 'gita', args: ['What is dharma?'], reply: async (t) => replies.push(t) }),
      { sock: mockSock([]) },
    );

    const nexrayCalls = calls.filter((c) => isNexray.test(c.url));
    assert.ok(nexrayCalls.length > 0, 'nexray harus dicoba saat cuki 401');
    assert.match(nexrayCalls[0].url, /\/ai\/gitagpt\?/, 'path harus /ai/gitagpt');
    assert.ok(
      replies.some((r) => r.includes('Jawaran Bhagavad Gita')),
      `user harus dapat jawaban nexray, dapat: ${JSON.stringify(replies)}`,
    );
  });
});

describe('fallback provider — sendngl (cuki 401 → nexray /tools/spamngl)', () => {
  const CUKI = /api\.cuki\.biz\.id/;

  test('cuki 401 ⇒ nexray dipanggil dengan url + jumlah + pesan', async () => {
    const cfg = (await import('../config.js')).default;
    const saved = cfg.APIkey.cuki;
    cfg.APIkey.cuki = 'dummy';
    installRoutes([
      [CUKI, async () => ({ status: 401, body: { success: false, error: 'API key required' } })],
      [isNexray, async () => ({ body: { status: true, author: 'NexRay', result: 'Finished sending messages to abc' } })],
    ]);

    const { handler } = await loadPlugin('../plugins/tools/sendngl.js');
    const replies = [];
    const sent = [];
    try {
      await handler(
        mockM({
          command: 'sendngl',
          text: 'https://ngl.link/abc | hai',
          reply: async (t) => replies.push(t),
        }),
        { sock: mockSock(sent) },
      );
    } finally {
      cfg.APIkey.cuki = saved;
    }

    const nexrayCalls = calls.filter((c) => isNexray.test(c.url));
    assert.ok(nexrayCalls.length > 0, 'nexray harus dicoba saat cuki 401');
    // m.text pada serialize.js = bagian SETELAH nama command, jadi
    // "https://ngl.link/abc | hai" — bukan termasuk "sendngl".
    const u = nexrayCalls[0].url;
    const q = new URL(u).searchParams;
    assert.equal(q.get('url'), 'https://ngl.link/abc', `url salah: ${u}`);
    assert.equal(q.get('jumlah'), '1', `jumlah salah: ${u}`);
    assert.equal(q.get('pesan'), 'hai', `pesan salah: ${u}`);
    // Plugin kirim konfirmasi lewat sock.sendMessage, bukan m.reply.
    const all = [...replies, ...sent.map((s) => s.text || '')];
    assert.ok(
      all.some((r) => /DONE|berhasil/i.test(r)),
      `user harus dapat konfirmasi. Dapat: ${JSON.stringify(all)}`,
    );
  });

  test('cuki hidup ⇒ nexray TIDAK dipanggil', async () => {
    const cfg = (await import('../config.js')).default;
    const saved = cfg.APIkey.cuki;
    cfg.APIkey.cuki = 'dummy';
    installRoutes([
      [CUKI, async () => ({ body: { success: true, status: true } })],
      [isNexray, async () => ({ body: { status: true, result: 'x' } })],
    ]);

    const { handler } = await loadPlugin('../plugins/tools/sendngl.js');
    const sent = [];
    try {
      await handler(
        mockM({
          command: 'sendngl',
          text: 'https://ngl.link/abc | hai',
          reply: async () => {},
        }),
        { sock: mockSock(sent) },
      );
    } finally {
      cfg.APIkey.cuki = saved;
    }

    assert.ok(
      calls.every((c) => !isNexray.test(c.url)),
      `nexray tidak boleh dipanggil. Actual: ${JSON.stringify(calls.map((c) => c.host))}`,
    );
    assert.equal(sent.length, 1, 'konfirmasi tetap terkirim dari primary');
  });
});

describe('fallback provider — nikparser (obscura 404 → nexray /tools/nikparse)', () => {
  const OBSCURA = /api\.obscuraworks\.org/;

  test('obscura 404 ⇒ nexray dipakai, field dinormalisasi ke bentuk obscura', async () => {
    installRoutes([
      [OBSCURA, async () => ({ status: 404, body: {} })],
      [
        isNexray,
        async () => ({
          body: {
            status: true,
            author: 'NexRay',
            result: {
              nik: '3201234567890123',
              kelamin: 'PEREMPUAN',
              lahir: '05/07/1989',
              lahir_lengkap: 'Rabu, 05 Juli 1989',
              provinsi: { kode: '32', nama: 'JAWA BARAT' },
              kotakab: { kode: '3201', nama: 'KABUPATEN BOGOR', jenis: 'Kabupaten' },
              kecamatan: { kode: '320123', nama: 'RANCA BUNGUR' },
              kode_wilayah: '32.01.23',
              nomor_urut: '0123',
            },
          },
        }),
      ],
    ]);

    const { handler } = await loadPlugin('../plugins/tools/nikparser.js');
    const replies = [];
    await handler(
      mockM({
        command: 'nikparser',
        text: '3201234567890123',
        reply: async (t) => replies.push(t),
      }),
      { sock: mockSock([]) },
    );

    const nexrayCalls = calls.filter((c) => isNexray.test(c.url));
    assert.ok(nexrayCalls.length > 0, 'nexray harus dicoba saat obscura 404');
    assert.match(nexrayCalls[0].url, /\/tools\/nikparse\?nik=3201234567890123/);

    assert.equal(replies.length, 1, 'harus satu balasan hasil parse');
    const out = replies[0];
    assert.match(out, /3201234567890123/, 'NIK harus tampil');
    assert.match(out, /♀️/, 'PEREMPUAN harus jadi ikon perempuan');
    assert.match(out, /Wanita|Pria/i, 'gender harus diterjemahkan');
    assert.match(out, /Jawa Barat/i, 'provinsi dari nexray harus tampil');
    assert.doesNotMatch(out, /undefined|NaN/, 'tidak boleh ada undefined/NaN');
  });

  test('obscura hidup ⇒ nexray TIDAK dipanggil', async () => {
    installRoutes([
      [
        OBSCURA,
        async () => ({
          body: {
            valid: true,
            raw: '3201234567890123',
            birthISO: '1989-07-05T00:00:00.000Z',
            gender: 'wanita',
            provinceId: 32,
            province: 'Jawa Barat',
            kabupatenKotaId: '3201',
            kecamatanId: '320123',
            uniqcode: '0123',
          },
        }),
      ],
      [isNexray, async () => ({ body: { status: true, result: {} } })],
    ]);

    const { handler } = await loadPlugin('../plugins/tools/nikparser.js');
    const replies = [];
    await handler(
      mockM({
        command: 'nikparser',
        text: '3201234567890123',
        reply: async (t) => replies.push(t),
      }),
      { sock: mockSock([]) },
    );

    assert.ok(
      calls.every((c) => !isNexray.test(c.url)),
      `nexray tidak boleh dipanggil. Actual: ${JSON.stringify(calls.map((c) => c.host))}`,
    );
  });
});

describe('plugin tanpa padanan nexray — pesan jujur, bukan diam', () => {
  // Dicek live: nexray tidak punya padanan untuk watercolortext (semua
  // /ephoto/* 500), caribug (/ai/llamacoder 500, /ai/hammer 400), atau
  // am-data. Jadi tidak bisa diberi fallback — tapi harus jujur soal key.
  for (const [path, command, envName, input] of [
    ['../plugins/canvas/watercolortext.js', 'watercolortext', 'APIKEY_CUKI', { args: ['halo'] }],
    [
      '../plugins/tools/caribug.js',
      'caribug',
      'APIKEY_CUKI',
      { args: ['function', 't(){}'], quoted: { message: { conversation: 'function t(){}' } } },
    ],
  ]) {
    test(`${command}: key kosong ⇒ pesan menyebut ${envName}`, async () => {
      const cfg = (await import('../config.js')).default;
      const saved = cfg.APIkey.cuki;
      cfg.APIkey.cuki = '';

      installRoutes([[/./, async () => ({ status: 401, body: {} })]]);

      const { handler } = await loadPlugin(path);
      const replies = [];
      // caribug menerima (m, { args }) — destructure kedua argumen wajib ada.
      const ctx = { args: input.args, sock: mockSock([]) };
      try {
        await handler(
          mockM({
            command,
            text: `${command} halo dunia`,
            reply: async (t) => replies.push(t),
            ...input,
          }),
          ctx,
        );
      } catch {
        /* sebagian plugin melempar; yang penting ada balasan */
      }

      cfg.APIkey.cuki = saved;
      assert.ok(replies.length > 0, `${command} harus membalas sesuatu`);
      assert.ok(
        replies.some((r) => r.includes(envName)),
        `${command} harus menyebut ${envName}. Dapat: ${JSON.stringify(replies)}`,
      );
    });
  }
});

describe('tanpa padanan nexray — key kosong harus jujur (neoxr / covenant)', () => {
  // neoxr: /api/chord & /api/sfile menjawab "apikey is required" walau key
  // dikirim (dicek live). covenant: api.covenant.sbs DNS gagal.
  // Nexray tidak punya endpoint chord / sfile / melolo, jadi tidak ada
  // fallback — tapi plugin tidak boleh diam atau pesan generik.
  const cases = [
    ['../plugins/search/chords.js', 'chords', 'APIKEY_NEOXR', { args: ['komang'] }],
    ['../plugins/download/sfiledl.js', 'sfiledl', 'APIKEY_NEOXR', { args: ['https://sfile.mobi/x'] }],
    ['../plugins/search/melolo.js', 'melolo', 'APIKEY_COVENANT', { args: ['fantasy'], text: 'fantasy' }],
  ];

  for (const [path, command, envName, input] of cases) {
    test(`${command}: key kosong ⇒ pesan menyebut ${envName}`, async () => {
      const cfg = (await import('../config.js')).default;
      const savedNexr = cfg.APIkey.neoxr;
      const savedCov = cfg.APIkey.covenant;
      cfg.APIkey.neoxr = '';
      cfg.APIkey.covenant = '';

      installRoutes([[/./, async () => ({ status: 500, body: {} })]]);

      const { handler } = await loadPlugin(path);
      const replies = [];
      try {
        await handler(
          mockM({
            command,
            text: `${command} ${(input.args || []).join(' ')}`,
            reply: async (t) => replies.push(t),
            ...input,
          }),
          { sock: mockSock([]), args: input.args },
        );
      } catch {
        /* sebagian plugin melempar; yang penting ada balasan */
      }

      cfg.APIkey.neoxr = savedNexr;
      cfg.APIkey.covenant = savedCov;

      assert.ok(replies.length > 0, `${command} harus membalas sesuatu`);
      assert.ok(
        replies.some((r) => r.includes(envName)),
        `${command} harus menyebut ${envName}. Dapat: ${JSON.stringify(replies)}`,
      );
    });
  }
});

describe('plugin yang butuh API key — pesan harus jujur', () => {
  // Skenario tersering di deployment: .env masih template, jadi key kosong.
  // Pin ini yang paling sering terjadi di deployment: .env masih template.
  for (const [path, command, envName] of [
    ['../plugins/stalker/igstalk.js', 'igstalk', 'APIKEY_FIREFLY'],
    ['../plugins/stalker/ytstalk.js', 'ytstalk', 'APIKEY_FIREFLY'],
    ['../plugins/ai/text2img.js', 'text2img', 'APIKEY_FIREFLY'],
    ['../plugins/ai/txt2img.js', 'txt2img', 'APIKEY_NEOXR'],
  ]) {
    test(`${command}: API key kosong ⇒ pesan menyebut ${envName}`, async () => {
      const prev = process.env.APIKEY_FIREFLY;
      const prevN = process.env.APIKEY_NEOXR;
      delete process.env.APIKEY_FIREFLY;
      delete process.env.APIKEY_NEOXR;
      const cfg = (await import('../config.js')).default;
      const savedFirefly = cfg.APIkey.firefly;
      const savedNexr = cfg.APIkey.neoxr;
      cfg.APIkey.firefly = '';
      cfg.APIkey.neoxr = '';

      installRoutes([[/./, async () => ({ status: 530, body: {} })]]);

      const { handler } = await loadPlugin(path);
      const replies = [];
      try {
        await handler(
          mockM({
            command,
            args: ['kucing lucu'],
            reply: async (t) => replies.push(t),
          }),
          { sock: mockSock([]) },
        );
      } catch {
        /* beberapa plugin melempar; yang penting dia balas */
      }

      cfg.APIkey.firefly = savedFirefly;
      cfg.APIkey.neoxr = savedNexr;
      if (prev !== undefined) process.env.APIKEY_FIREFLY = prev;
      if (prevN !== undefined) process.env.APIKEY_NEOXR = prevN;

      assert.ok(replies.length > 0, `${command} harus membalas sesuatu`);
      assert.ok(
        replies.some((r) => r.includes(envName)),
        `${command} harus menyebut ${envName}. Dapat: ${JSON.stringify(replies)}`,
      );
    });
  }

  test('host mati 530 dengan key terisi ⇒ tetap pesan error, bukan diam', async () => {
    const cfg = (await import('../config.js')).default;
    const saved = cfg.APIkey.firefly;
    cfg.APIkey.firefly = 'dummy-key';

    installRoutes([[/./, async () => ({ status: 530, body: {} })]]);

    const { handler } = await loadPlugin('../plugins/ai/text2img.js');
    const replies = [];
    await handler(
      mockM({ command: 'text2img', args: ['kucing'], reply: async (t) => replies.push(t) }),
      { sock: mockSock([]) },
    ).catch(() => {});

    cfg.APIkey.firefly = saved;
    assert.ok(replies.length > 0, 'harus tetap ada balasan');
  });
});

describe('regression — tidak ada hardcode host ganda', () => {
  test('helper fallback ada di lib, bukan inline di tiap plugin', async () => {
    const lib = await readFile(
      new URL('../src/lib/stalker-fallback.js', import.meta.url),
      'utf8',
    );
    assert.match(lib, /export async function withFallback/);
    for (const f of [
      'fetchGithubProfile',
      'fetchTiktokProfile',
      'fetchNpmProfile',
      'fetchAiText',
      'sendNgl',
      'parseNik',
    ]) {
      assert.match(
        lib,
        new RegExp(`export async function ${f}\\b`),
        `helper ${f} harus ada di lib`,
      );
    }
  });

  test('tidak ada plugin yang memanggil host mati secara langsung', async () => {
    // firefly 530 dan cuki 401 — keduanya harus lewat helper.
    for (const f of [
      '../plugins/stalker/githubstalk.js',
      '../plugins/stalker/tiktokstalk.js',
      '../plugins/stalker/npmstalk.js',
      '../plugins/ai/wormgpt.js',
      '../plugins/ai/gita.js',
    ]) {
      const src = await readFile(new URL(f, import.meta.url), 'utf8');
      assert.doesNotMatch(
        src,
        /firefly\.maiku\.my\.id/,
        `${f} masih hardcode firefly (host mati 530)`,
      );
      assert.doesNotMatch(
        src,
        /api\.cuki\.biz\.id/,
        `${f} masih hardcode cuki (401 tanpa key)`,
      );
    }
  });

  test('semua stalker memakai helper fallback yang sama', async () => {
    for (const f of ['githubstalk', 'tiktokstalk', 'npmstalk']) {
      const src = await readFile(
        new URL(`../plugins/stalker/${f}.js`, import.meta.url),
        'utf8',
      );
      assert.doesNotMatch(
        src,
        /firefly\.maiku\.my\.id\/api\/stalk-[a-z]+\?apikey=/,
        `${f} tidak boleh lagi memanggil firefly langsung — harus lewat helper`,
      );
    }
  });
});
