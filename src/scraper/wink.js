import axios from "axios";
import { httpAxios } from "../lib/http.js";
import FormData from "form-data";
import crypto from "node:crypto";
import fs from "node:fs";
import fsp from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { CookieJar } from "tough-cookie";
import { wrapper } from "axios-cookiejar-support";

const BASE_URL = "https://wink.ai";
const STRATEGY_URL = "https://strategy.app.meitudata.com";

const CLIENT_ID = "1189857605";
const VERSION = "5.1.2";
const COUNTRY_CODE = "ID";
const CLIENT_LANGUAGE = "en_US";
const CLIENT_TIMEZONE = "Asia/Jakarta";

const TASK_TYPE = "11";
const CONTENT_TYPE = "2";

const UA =
  "Mozilla/5.0 (Linux; Android 10; K) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/147.0.0.0 Mobile Safari/537.36";

let _api = null;

async function getApi() {
  if (_api) return _api;

  const gnum = crypto.randomUUID();
  const jar = new CookieJar();

  await jar.setCookie(`_sm=${gnum}; Path=/; Domain=wink.ai`, BASE_URL);
  await jar.setCookie(
    `meitustat=${encodeURIComponent(JSON.stringify({ wgid: gnum }))}; Path=/; Domain=wink.ai`,
    BASE_URL,
  );

  _api = {
    client: wrapper(
      axios.create({
        baseURL: BASE_URL,
        timeout: 30000,
        jar,
        withCredentials: true,
        validateStatus: () => true,
        headers: {
          accept: "*/*",
          origin: BASE_URL,
          referer: `${BASE_URL}/video-enhancer/upload`,
          "user-agent": UA,
          "sec-ch-ua":
            '"Google Chrome";v="147", "Not.A/Brand";v="8", "Chromium";v="147"',
          "sec-ch-ua-mobile": "?1",
          "sec-ch-ua-platform": '"Android"',
          ab_info: JSON.stringify({
            ab_codes: [],
            version: "1.4.4",
          }),
        },
      }),
    ),
    gnum,
  };

  return _api;
}

function extToMime(file) {
  const ext = path.extname(file).toLowerCase();

  if (ext === ".mp4") return "video/mp4";
  if (ext === ".mov") return "video/quicktime";
  if (ext === ".webm") return "video/webm";
  if (ext === ".mkv") return "video/x-matroska";

  return "application/octet-stream";
}

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function makeTrace() {
  return `${crypto.randomBytes(16).toString("hex")}-${crypto.randomBytes(8).toString("hex")}-1`;
}

function traceHeaders() {
  const trace = makeTrace();

  return {
    "sentry-trace": trace,
    baggage: [
      "sentry-environment=release",
      "sentry-release=5.1.2%20(b60d25c477f43c6dfac4107810f26d442320f4f1)",
      "sentry-public_key=e1bf914f3448d9bc8a10c7e499d17d54",
      `sentry-trace_id=${trace.split("-")[0]}`,
      "sentry-sampled=true",
      "sentry-sample_rate=0.75",
    ].join(","),
  };
}

async function baseParams(extra = {}) {
  const { gnum } = await getApi();
  return new URLSearchParams({
    client_id: CLIENT_ID,
    version: VERSION,
    country_code: COUNTRY_CODE,
    gnum,
    client_language: CLIENT_LANGUAGE,
    client_channel_id: "",
    client_timezone: CLIENT_TIMEZONE,
    ...extra,
  });
}

async function getMaatSign() {
  const { client: api } = await getApi();
  const params = await baseParams({
    suffix: ".mp4",
    type: "temp",
    count: "1",
  });

  const res = await api.get(
    `/api/file/get_maat_sign.json?${params.toString()}`,
    {
      headers: traceHeaders(),
    },
  );

  if (res.status >= 400 || res.data?.code !== 0) {
    throw new Error(`get_maat_sign gagal: ${JSON.stringify(res.data)}`);
  }

  return res.data.data;
}

async function getUploadPolicy(sign) {
  const params = new URLSearchParams({
    app: sign.app,
    count: String(sign.count),
    sig: sign.sig,
    sigTime: sign.sig_time,
    sigVersion: sign.sig_version,
    suffix: sign.suffix,
    type: sign.type,
  });

  const res = await httpAxios.get(
    `${STRATEGY_URL}/upload/policy?${params.toString()}`,
    {
      headers: {
        accept: "*/*",
        origin: BASE_URL,
        referer: `${BASE_URL}/`,
        "user-agent": UA,
        "sec-ch-ua":
          '"Google Chrome";v="147", "Not.A/Brand";v="8", "Chromium";v="147"',
        "sec-ch-ua-mobile": "?1",
        "sec-ch-ua-platform": '"Android"',
      },
      validateStatus: () => true,
    },
  );

  if (res.status >= 400 || !Array.isArray(res.data) || !res.data[0]?.qiniu) {
    throw new Error(`upload policy gagal: ${JSON.stringify(res.data)}`);
  }

  return res.data[0].qiniu;
}

async function uploadToQiniu(policy, filePath) {
  const form = new FormData();

  form.append("file", fs.createReadStream(filePath), {
    filename: path.basename(filePath),
    contentType: extToMime(filePath),
  });

  form.append("token", policy.token);
  form.append("key", policy.key);
  form.append("fname", path.basename(filePath));

  const res = await httpAxios.post(policy.url, form, {
    headers: form.getHeaders({
      origin: BASE_URL,
      referer: `${BASE_URL}/`,
      "user-agent": UA,
      accept: "*/*",
    }),
    maxBodyLength: Infinity,
    maxContentLength: Infinity,
    validateStatus: () => true,
  });

  if (res.status >= 400) {
    throw new Error(
      `upload qiniu gagal HTTP ${res.status}: ${typeof res.data === "string" ? res.data : JSON.stringify(res.data)}`,
    );
  }

  if (!res.data?.url && !res.data?.data) {
    throw new Error(
      `upload qiniu response tidak valid: ${JSON.stringify(res.data)}`,
    );
  }

  return {
    file_key: policy.key,
    source_url: res.data.url || res.data.data || policy.data,
    raw_url: res.data.data || policy.data,
    raw: res.data,
  };
}

async function getVideoInfo(fileKey) {
  const { client: api } = await getApi();
  const body = await baseParams({
    file_key: fileKey,
  });

  const res = await api.post(
    "/api/file/video_cover_and_display_info_ext.json",
    body.toString(),
    {
      headers: {
        ...traceHeaders(),
        "content-type": "application/x-www-form-urlencoded;charset=UTF-8",
      },
    },
  );

  if (res.status >= 400 || res.data?.code !== 0) {
    throw new Error(`video info gagal: ${JSON.stringify(res.data)}`);
  }

  return res.data.data;
}

async function startTranscode(fileKey) {
  const { client: api } = await getApi();
  const body = await baseParams({
    file_key: fileKey,
  });

  const res = await api.post(
    "/api/file/video_trans_start.json",
    body.toString(),
    {
      headers: {
        ...traceHeaders(),
        "content-type": "application/x-www-form-urlencoded;charset=UTF-8",
      },
    },
  );

  if (res.status >= 400 || res.data?.code !== 0 || !res.data?.data?.id) {
    throw new Error(`transcode start gagal: ${JSON.stringify(res.data)}`);
  }

  return res.data.data.id;
}

async function queryTranscode(id) {
  const { client: api } = await getApi();
  const params = await baseParams({
    id,
  });

  const res = await api.get(
    `/api/file/video_trans_query.json?${params.toString()}`,
    {
      headers: traceHeaders(),
    },
  );

  if (res.status >= 400 || res.data?.code !== 0) {
    throw new Error(`transcode query gagal: ${JSON.stringify(res.data)}`);
  }

  return res.data.data;
}

/**
 * Tunggu transcode selesai. Kalau lewat batas waktu, THROW — jangan
 * kembalikan URL asli seolah-olah itu hasil enhance. Versi lama diam-diam
 * mengembalikan fallbackSourceUrl, jadi user bayar energi + dapat video
 * mentah yang dijawab "udah jadi Ultra HD".
 *
 * @param queryFn injeksi untuk test; default ke queryTranscode.
 */
async function waitTranscode(
  id,
  fallbackSourceUrl,
  maxTry = 80,
  delayMs = 3000,
  queryFn = queryTranscode,
) {
  let last = null;

  for (let i = 1; i <= maxTry; i++) {
    const data = await queryFn(id);
    last = data;

    const video = data?.video || data?.url || data?.source_url || "";
    const videoTranscoded =
      data?.video_transcoded ||
      data?.transcoded_video ||
      data?.transcoded_url ||
      data?.video_url ||
      "";

    // Hasil yang sah harus URL https DAN berbeda dari sumber — kalau sama,
    // transcode belum jalan dan server cuma mengulang URL original.
    if (isUsableUrl(videoTranscoded) && videoTranscoded !== fallbackSourceUrl) {
      return {
        source_url: video || fallbackSourceUrl,
        video_transcoded: videoTranscoded,
        raw: data,
      };
    }

    if (i < maxTry) await sleep(delayMs);
  }

  throw new Error(
    `Transcode tidak selesai dalam ${Math.round((maxTry * delayMs) / 1000)} detik. ` +
      `Respons terakhir: ${JSON.stringify(last).slice(0, 200)}`,
  );
}

async function delivery(sourceUrl, videoTranscoded, taskName) {
  const { client: api } = await getApi();
  const body = await baseParams({
    type: TASK_TYPE,
    content_type: CONTENT_TYPE,
    source_url: sourceUrl,
    type_params: JSON.stringify({
      is_mirror: 0,
      orientation_tag: 1,
      j_420_trans: "1",
      return_ext: "2",
    }),
    right_detail: JSON.stringify({
      source: "1",
      touch_type: "4",
      function_id: "630",
      material_id: "63011",
      url: "https://wink.ai/video-enhancer/upload",
    }),
    ext_params: JSON.stringify({
      task_name: taskName,
      records: TASK_TYPE,
      video_transcoded: videoTranscoded,
    }),
    with_prepare: "1",
  });

  const res = await api.post("/api/meitu_ai/delivery.json", body.toString(), {
    headers: {
      ...traceHeaders(),
      "content-type": "application/x-www-form-urlencoded;charset=UTF-8",
    },
  });

  if (res.status >= 400 || res.data?.code !== 0) {
    throw new Error(`delivery gagal: ${JSON.stringify(res.data)}`);
  }

  const data = res.data.data || {};

  return {
    msg_id: data.msg_id || "",
    prepare_msg_id: data.prepare_msg_id || "",
    raw: data,
  };
}

async function queryBatch(msgId) {
  const { client: api } = await getApi();
  const params = await baseParams({
    msg_ids: msgId,
  });

  const res = await api.get(
    `/api/meitu_ai/query_batch.json?${params.toString()}`,
    {
      headers: {
        ...traceHeaders(),
        referer: `${BASE_URL}/video-enhancer/upload`,
      },
    },
  );

  if (res.status >= 400 || res.data?.code !== 0) {
    throw new Error(`query batch gagal: ${JSON.stringify(res.data)}`);
  }

  return res.data.data;
}

function isUsableUrl(value) {
  return typeof value === "string" && value.startsWith("https://");
}

function extractResultUrl(data) {
  const item = data?.item_list?.[0];
  const media = item?.result?.media_info_list?.[0];

  // Meitu kadang mengisi media_data dengan JSON dimensi, bukan URL. Teruskan
  // mentah ke sendMedia = user cuma lihat "gagal" tanpa sebab.
  for (const candidate of [
    media?.media_data,
    item?.result?.result_url,
    item?.result?.url,
    item?.client_ext_params?.video_transcoded,
  ]) {
    if (isUsableUrl(candidate)) return candidate;
  }
  return "";
}

function extractNextMsgId(data, currentMsgId) {
  const item = data?.item_list?.[0];
  const resultValue = item?.result?.result || "";
  const realMsgId = item?.result?.msg_id || item?.msg_id || "";

  if (
    resultValue &&
    resultValue !== currentMsgId &&
    !resultValue.startsWith("http") &&
    !resultValue.startsWith("https")
  ) {
    return resultValue;
  }

  if (
    realMsgId &&
    realMsgId !== currentMsgId &&
    !realMsgId.startsWith("wpr_")
  ) {
    return realMsgId;
  }

  return "";
}

/**
 * Tunggu hasil akhir. Bedakan "gagal" (error_code selain pending) dari
 * "belum selesai" — yang kedua bukan error, hanya perlu waktu lagi.
 *
 * @param queryFn injeksi untuk test; default ke queryBatch.
 */
async function waitResult(firstMsgId, maxTry = 120, delayMs = 5000, queryFn = queryBatch) {
  let msgId = firstMsgId;
  let last = null;

  for (let i = 1; i <= maxTry; i++) {
    const data = await queryFn(msgId);
    last = data;

    const nextMsgId = extractNextMsgId(data, msgId);

    if (nextMsgId) {
      msgId = nextMsgId;
      if (i < maxTry) await sleep(1000);
      continue;
    }

    const errorCode = data?.item_list?.[0]?.result?.error_code;
    const errorMsg = data?.item_list?.[0]?.result?.error_msg;

    // Gagal betulan → berhenti sekarang, jangan habis 10 menit buat await.
    if (errorCode && errorCode !== 29901 && errorCode !== 0) {
      throw new Error(`task gagal: ${errorCode} ${errorMsg || ""}`);
    }

    const url = extractResultUrl(data);
    if (isUsableUrl(url) && errorCode === 0) {
      return url;
    }

    if (i < maxTry) await sleep(delayMs);
  }

  throw new Error(
    `Result belum selesai dalam ${Math.round((maxTry * delayMs) / 1000)} detik. ` +
      `Respons terakhir: ${JSON.stringify(last).slice(0, 200)}`,
  );
}

async function winkEnhance(video, { filename } = {}) {
  if (!video) throw new Error("video is required");

  const safeName = safeTempName(filename || `wink-${crypto.randomUUID()}`);
  const filePath = Buffer.isBuffer(video)
    ? path.join(os.tmpdir(), safeName)
    : video;
  const shouldCleanup = Buffer.isBuffer(video);

  if (shouldCleanup) {
    await fsp.writeFile(filePath, video);
  }

  if (!fs.existsSync(filePath)) {
    throw new Error(`File tidak ditemukan: ${filePath}`);
  }

  try {
    const taskName = `Enhancer-Ultra HD-${path.parse(filePath).name}`;
    const plan = planPollBudget();

    const sign = await getMaatSign();
    const policy = await getUploadPolicy(sign);
    const uploaded = await uploadToQiniu(policy, filePath);

    await getVideoInfo(uploaded.file_key);

    const transcodeId = await startTranscode(uploaded.file_key);
    const transcode = await waitTranscode(
      transcodeId,
      uploaded.source_url,
      plan.transcodeTries,
      plan.transcodeDelay,
    );

    const task = await delivery(
      transcode.source_url,
      transcode.video_transcoded,
      taskName,
    );
    const firstMsgId = task.msg_id || task.prepare_msg_id;

    if (!firstMsgId) {
      throw new Error(
        `delivery tidak mengembalikan msg_id: ${JSON.stringify(task.raw)}`,
      );
    }

    const resultUrl = await waitResult(
      firstMsgId,
      plan.resultTries,
      plan.resultDelay,
    );

    return {
      resultUrl,
    };
  } finally {
    if (shouldCleanup) {
      try {
        await fsp.unlink(filePath);
      } catch {}
    }
  }
}

/**
 * Nama file temp yang aman. Dipakai untuk os.tmpdir() — tanpa sanitasi,
 * filename dari luar bisa keluar dari direktori temp (path traversal).
 */
function safeTempName(input) {
  const base = String(input || "")
    .replace(/[/\\]+/g, "-")
    .replace(/\.\./g, "")
    .replace(/[^a-zA-Z0-9._-]/g, "-")
    .replace(/^-+/, "")
    .slice(0, 80);
  const stem = base.replace(/\.mp4$/i, "") || "wink-video";
  return `${stem}.mp4`;
}

/**
 * Anggaran polling total. Default lama 80×3s + 120×5s = 840 detik (14
 * menit) padahal plugin menjanjikan "1-5 menit" — userebak nunggu dan
 * tidak tahu prosesnya jalan atau mati. Sekarang dibatasi total 4 menit
 * 30 detik, dibagi: transcode 150s, result 120s.
 */
const POLL = { transcodeMs: 150000, resultMs: 120000 };

function planPollBudget() {
  const transcodeDelay = 3000;
  const resultDelay = 4000;
  return {
    transcodeMs: POLL.transcodeMs,
    resultMs: POLL.resultMs,
    totalMs: POLL.transcodeMs + POLL.resultMs,
    transcodeTries: Math.ceil(POLL.transcodeMs / transcodeDelay),
    resultTries: Math.ceil(POLL.resultMs / resultDelay),
    transcodeDelay,
    resultDelay,
  };
}

function totalWorstCaseMs() {
  // Angka default lama, dipakai test untuk mengunci janji "1-5 menit".
  return 80 * 3000 + 120 * 5000;
}

/** Deteksi respons basi: msg_id yang diminta sudah bukan milik job ini. */
function isStaleResponse(responseMsgId, currentMsgId) {
  return Boolean(responseMsgId) && responseMsgId !== currentMsgId;
}

/**
 * Sumber video: pesan sendiri atau yang di-reply. Plugin pernah memanggil
 * `m.quoted.download()` lebih dulu; kalau tidak ada quoted, pemanggil
 m.download tetap jalan karena `?.()`. Dipisah supaya jelas.
 */
function pickVideoPayload(m) {
  if (m?.isVideo) return "self";
  if (m?.quoted?.type === "videoMessage") return "quoted";
  if (
    m?.type === "documentMessage" &&
    m?.message?.documentMessage?.mimetype?.startsWith("video")
  ) {
    return "self";
  }
  if (
    m?.quoted?.type === "documentMessage" &&
    m?.quoted?.message?.documentMessage?.mimetype?.startsWith("video")
  ) {
    return "quoted";
  }
  return null;
}

export default winkEnhance;
export {
  extractResultUrl,
  extractNextMsgId,
  waitTranscode,
  waitResult,
  pickVideoPayload,
  planPollBudget,
  totalWorstCaseMs,
  isStaleResponse,
  safeTempName,
};
