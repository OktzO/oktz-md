import { describe, it } from 'node:test';
import assert from 'node:assert';

import {
  extractResultUrl,
  extractNextMsgId,
  waitTranscode,
  waitResult,
  pickVideoPayload,
  planPollBudget,
  totalWorstCaseMs,
  isStaleResponse,
  safeTempName,
} from '../src/scraper/wink.js';

describe('extractResultUrl', () => {
  it('ambil media_data', () => {
    assert.equal(
      extractResultUrl({
        item_list: [{ result: { media_info_list: [{ media_data: 'https://x/1.mp4' }] } }],
      }),
      'https://x/1.mp4',
    );
  });

  it('TIDAK mengembalikan media_data yang bukan URL', () => {
    // Meitu kadang kirim JSON di media_data, bukan URL. Kalau diteruskan
    // ke sock.sendMedia, user dapat pesan "gagal" tanpa sebab.
    const out = extractResultUrl({
      item_list: [
        {
          result: {
            media_info_list: [{ media_data: '{"width":720,"height":1280}' }],
          },
        },
      ],
    });
    assert.equal(out, '', 'JSON di media_data bukan URL');
  });

  it('TIDAK mengembalikan URL http:// polos (harus https)', () => {
    assert.equal(
      extractResultUrl({
        item_list: [{ result: { result_url: 'http://insecure/x.mp4' } }],
      }),
      '',
      'URL non-https tidak aman dipakai sebagai hasil',
    );
  });

  it('kembalikan string kosong kalau tidak ada apa-apa', () => {
    assert.equal(extractResultUrl(null), '');
    assert.equal(extractResultUrl({ item_list: [] }), '');
  });
});

describe('extractNextMsgId', () => {
  it('ikuti msg_id baru dari result.result', () => {
    assert.equal(
      extractNextMsgId(
        { item_list: [{ result: { result: 'wpr_next' } }] },
        'wpr_first',
      ),
      'wpr_next',
    );
  });

  it('jangan ikuti nilai yang sama dengan msg_id sekarang', () => {
    assert.equal(
      extractNextMsgId(
        { item_list: [{ result: { result: 'wpr_same' } }] },
        'wpr_same',
      ),
      '',
    );
  });

  it('jangan ikuti URL', () => {
    assert.equal(
      extractNextMsgId(
        { item_list: [{ result: { result: 'https://x/1.mp4' } }] },
        'wpr_first',
      ),
      '',
    );
  });
});

describe('waitTranscode — jangan diam-diam kirim video asli', () => {
  // Bug: kalau transcode tidak selesai dalam maxTry, fungsi lama
  // mengembalikan fallbackSourceUrl sebagai video_transcoded. Pipeline
  // lanjut, user dapat VIDEO ASLI yang dijawab "udah jadi Ultra HD".
  it('THROW kalau transcode tidak selesai (bukan fallback diam)', async () => {
    const neverReady = async () => ({ video: 'https://x/orig.mp4' });

    await assert.rejects(
      () => waitTranscode('id1', 'https://x/orig.mp4', 3, 0, neverReady),
      (err) => {
        assert.match(err.message, /transcode/i);
        return true;
      },
      'harus throw, bukan mengembalikan URL asli seolah-olah enhanced',
    );
  });

  it('kembalikan hasil kalau transcode jadi', async () => {
    const ready = async () => ({
      video: 'https://x/orig.mp4',
      video_transcoded: 'https://x/hd.mp4',
    });

    const out = await waitTranscode('id1', 'https://x/orig.mp4', 3, 0, ready);
    assert.equal(out.video_transcoded, 'https://x/hd.mp4');
    assert.equal(out.source_url, 'https://x/orig.mp4');
  });

  it('AKTIFASI transcode selesai = video_url (bukan video_transcoded)', async () => {
    const ready = async () => ({
      video: 'https://x/orig.mp4',
      video_url: 'https://x/hd.mp4',
    });
    const out = await waitTranscode('id1', 'https://x/orig.mp4', 3, 0, ready);
    assert.equal(out.video_transcoded, 'https://x/hd.mp4');
  });

  it('jangan terima URL yang sama dengan input sebagai "hasil"', async () => {
    // Server masih mengembalikan URL original — transcode BELUM jalan.
    const stillOriginal = async () => ({
      video: 'https://x/orig.mp4',
      video_transcoded: 'https://x/orig.mp4',
    });

    await assert.rejects(
      () => waitTranscode('id1', 'https://x/orig.mp4', 2, 0, stillOriginal),
      /transcode/i,
      'URL yang sama dengan sumber bukan hasil enhance',
    );
  });
});

describe('waitResult — error code harus dibedakan dari belum selesai', () => {
  it('lempar error_code yang bukan "pending"', async () => {
    const failed = async () => ({
      item_list: [
        { result: { error_code: 1001, error_msg: 'video tidak valid' } },
      ],
    });

    await assert.rejects(
      () => waitResult('wpr_1', 3, 0, failed),
      /1001|video tidak valid/,
    );
  });

  it('error_code 29901 diperlakukan sebagai pending, bukan gagal', async () => {
    let calls = 0;
    const pendingThenOk = async () => {
      calls += 1;
      if (calls < 3) {
        return {
          item_list: [
            { result: { error_code: 29901, error_msg: 'processing' } },
          ],
        };
      }
      return {
        item_list: [
          {
            result: {
              error_code: 0,
              result_url: 'https://x/final.mp4',
            },
          },
        ],
      };
    };

    const url = await waitResult('wpr_1', 5, 0, pendingThenOk);
    assert.equal(url, 'https://x/final.mp4');
  });

  it('THROW kalau lewat batas waktu — jangan kirim apa pun', async () => {
    const never = async () => ({
      item_list: [{ result: { error_code: 29901 } }],
    });
    await assert.rejects(() => waitResult('wpr_1', 2, 0, never), /belum selesai/i);
  });
});

describe('budget polling — jangan jebak user 14 menit', () => {
  // Defaults lama: waitTranscode 80×3s = 240s, lalu waitResult 120×5s = 600s.
  // Total 840s = 14 menit, tapi plugin menjanjikan "1-5 menit".
  it('worst case lama jauh melebihi janji ke user', () => {
    const total = totalWorstCaseMs();
    assert.ok(
      total > 5 * 60 * 1000,
      `worst case ${Math.round(total / 1000)}s harus melebihi 5 menit (janji plugin)`,
    );
  });

  it('planPollBudget memberi total di bawah 5 menit', () => {
    const plan = planPollBudget();
    assert.ok(plan.totalMs <= 5 * 60 * 1000, 'total harus <= 5 menit');
    assert.ok(plan.transcodeMs > 0 && plan.resultMs > 0);
    assert.equal(
      plan.transcodeMs + plan.resultMs,
      plan.totalMs,
      'transcode + result harus sama dengan total',
    );
  });

  it('poll plan menghasilkan jumlah percobaan yang wajar', () => {
    const plan = planPollBudget();
    assert.ok(plan.transcodeTries >= 1, 'minimal 1 percobaan transcode');
    assert.ok(plan.resultTries >= 1, 'minimal 1 percobaan result');
  });
});

describe('isStaleResponse — guard respons basi', () => {
  it('flag respons yang msg_id-nya sudah bukan milik job ini', () => {
    assert.equal(isStaleResponse('wpr_2', 'wpr_1'), true);
    assert.equal(isStaleResponse('wpr_1', 'wpr_1'), false);
  });
});

describe('safeTempName — cegah path traversal', () => {
  it('buang karakter path dari nama file', () => {
    const out = safeTempName('../../etc/passwd');
    assert.doesNotMatch(out, /\.\./, 'tidak boleh ada ".."');
    assert.doesNotMatch(out, /[/\\]/, 'tidak boleh ada separator path');
  });

  it('paksa ekstensi .mp4', () => {
    assert.match(safeTempName('video'), /\.mp4$/);
    assert.match(safeTempName('video.webm'), /\.mp4$/);
  });

  it('jangan kosong', () => {
    assert.ok(safeTempName('').length > 0);
    assert.ok(safeTempName(undefined).length > 0);
  });
});

describe('pickVideoPayload — plugin harus ambil sumber yang benar', () => {
  it('ambil video dari media message', () => {
    assert.equal(pickVideoPayload({ isVideo: true, quoted: null }), 'self');
  });

  it('ambil video dari quoted', () => {
    assert.equal(
      pickVideoPayload({ isVideo: false, quoted: { type: 'videoMessage' } }),
      'quoted',
    );
  });

  it('ambil document video', () => {
    assert.equal(
      pickVideoPayload({
        isVideo: false,
        quoted: null,
        type: 'documentMessage',
        message: { documentMessage: { mimetype: 'video/mp4' } },
      }),
      'self',
    );
  });

  it('null kalau bukan video', () => {
    assert.equal(
      pickVideoPayload({ isVideo: false, quoted: null }),
      null,
    );
  });
});
