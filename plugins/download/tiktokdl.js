import axios from "axios";
import * as cheerio from "cheerio";

async function resolveTikTokUrl(url) {
  try {
    if (/v[mt]\.tiktok\.com/i.test(url)) {
      const res = await axios.get(url, {
        maxRedirects: 5,
        timeout: 10000,
        headers: {
          "User-Agent":
            "Mozilla/5.0 (Linux; Android 10; K) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/116.0.0.0 Mobile Safari/537.36",
        },
        validateStatus: (s) => s >= 200 && s < 300,
      });
      const finalUrl =
        res.request?.res?.responseUrl ||
        res.request?.responseURL ||
        res.config?.url ||
        url;
      if (finalUrl && finalUrl.includes("tiktok.com")) return finalUrl;
    }
  } catch {}
  return url;
}

async function tiktokDlViaTikWM(url) {
  function formatNumber(integer) {
    const numb = parseInt(integer);
    if (isNaN(numb)) return "0";
    return Number(numb).toLocaleString().replace(/,/g, ".");
  }

  function formatDate(n, locale = "en") {
    const d = new Date(n * 1000);
    if (isNaN(d.getTime())) return "-";
    return d.toLocaleDateString(locale, {
      weekday: "long",
      day: "numeric",
      month: "long",
      year: "numeric",
      hour: "numeric",
      minute: "numeric",
      second: "numeric",
    });
  }

  const domain = "https://www.tikwm.com/api/";
  const raw = (
    await axios.post(
      domain,
      {},
      {
        headers: {
          Accept: "application/json, text/javascript, */*; q=0.01",
          "Accept-Language": "id-ID,id;q=0.9,en-US;q=0.8,en;q=0.7",
          "Content-Type": "application/x-www-form-urlencoded; charset=UTF-8",
          Origin: "https://www.tikwm.com",
          Referer: "https://www.tikwm.com/",
          "Sec-Ch-Ua": '"Not)A;Brand" ;v="24" , "Chromium" ;v="116"',
          "Sec-Ch-Ua-Mobile": "?1",
          "Sec-Ch-Ua-Platform": "Android",
          "Sec-Fetch-Dest": "empty",
          "Sec-Fetch-Mode": "cors",
          "Sec-Fetch-Site": "same-origin",
          "User-Agent":
            "Mozilla/5.0 (Linux; Android 10; K) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/116.0.0.0 Mobile Safari/537.36",
          "X-Requested-With": "XMLHttpRequest",
        },
        params: { url, count: 12, cursor: 0, web: 1, hd: 1 },
        timeout: 15000,
        validateStatus: (s) => s >= 200 && s < 300,
      }
    )
  ).data;

  // tikwm returns { code: 0, msg: 'success', data: {...} }  or { code: -1, msg: '...' }
  if (raw?.code !== undefined && raw.code !== 0) {
    throw new Error(raw.msg || "TikWM error: " + JSON.stringify(raw).slice(0, 200));
  }
  const res = raw?.data || raw;

  if (!res || (!res.duration && res.duration !== 0 && !res.images)) {
    throw new Error("TikWM tidak mengembalikan data valid");
  }

  const data = [];
  if (res?.duration == 0) {
    (res.images || []).forEach((v) => data.push({ type: "photo", url: String(v) }));
  } else {
    data.push(
      {
        type: "watermark",
        url: res?.wmplay ? "https://www.tikwm.com" + res.wmplay : null,
      },
      {
        type: "nowatermark",
        url: res?.play ? "https://www.tikwm.com" + res.play : null,
      },
      {
        type: "nowatermark_hd",
        url: res?.hdplay ? "https://www.tikwm.com" + res.hdplay : null,
      }
    );
    // filter null
  }

  const filteredData = data.filter((d) => d.url);

  return {
    status: true,
    _provider: "tikwm",
    title: res.title || "",
    taken_at: res.create_time ? formatDate(res.create_time).replace("1970", "").trim() : "-",
    region: res.region || "-",
    id: res.id || "",
    durations: res.duration,
    duration: (res.duration || 0) + " Seconds",
    cover: res.cover ? "https://www.tikwm.com" + res.cover : null,
    size_wm: res.wm_size || null,
    size_nowm: res.size || null,
    size_nowm_hd: res.hd_size || null,
    data: filteredData,
    music_info: {
      id: res.music_info?.id || "",
      title: res.music_info?.title || "-",
      author: res.music_info?.author || "-",
      album: res.music_info?.album || null,
      url: res.music ? "https://www.tikwm.com" + res.music : res.music_info?.play || null,
    },
    stats: {
      views: formatNumber(res.play_count),
      likes: formatNumber(res.digg_count),
      comment: formatNumber(res.comment_count),
      share: formatNumber(res.share_count),
      download: formatNumber(res.download_count),
    },
    author: {
      id: res.author?.id || "",
      fullname: res.author?.unique_id || "-",
      nickname: res.author?.nickname || "-",
      avatar: res.author?.avatar ? "https://www.tikwm.com" + res.author.avatar : null,
    },
  };
}

// Fallback: savett.cc (same logic as tiktokdl2.js) but normalized to tikwm-like shape for handler compatibility
const SAVETT_HEADERS = {
  "Content-Type": "application/x-www-form-urlencoded",
  Accept: "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
  Origin: "https://savett.cc",
  Referer: "https://savett.cc/en1/download",
  "User-Agent":
    "Mozilla/5.0 (Linux; Android 10; K) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/139.0.0.0 Mobile Safari/537.36",
};

async function getSaveTTToken() {
  const res = await axios.get("https://savett.cc/en1/download", {
    timeout: 15000,
    headers: { "User-Agent": SAVETT_HEADERS["User-Agent"] },
  });
  const csrf = res.data.match(/name="csrf_token" value="([^"]+)"/)?.[1];
  const cookie = (res.headers["set-cookie"] || []).map((v) => v.split(";")[0]).join("; ");
  if (!csrf || !cookie) throw new Error("Gagal mendapatkan token saveTT");
  return { csrf, cookie };
}

async function fetchSaveTT(url, csrf, cookie) {
  const res = await axios.post(
    "https://savett.cc/en1/download",
    `csrf_token=${encodeURIComponent(csrf)}&url=${encodeURIComponent(url)}`,
    { headers: { ...SAVETT_HEADERS, Cookie: cookie }, timeout: 15000, validateStatus: (s) => s >= 200 && s < 300 }
  );
  return res.data;
}

function parseSaveTTResponse(html) {
  const $ = cheerio.load(html);
  const stats = [];
  $("#video-info .my-1 span").each((_, el) => {
    stats.push($(el).text().trim());
  });
  const data = {
    username: $("#video-info h3").first().text().trim(),
    views: stats[0] || null,
    likes: stats[1] || null,
    bookmarks: stats[2] || null,
    comments: stats[3] || null,
    shares: stats[4] || null,
    duration: $("#video-info p.text-muted").first().text().replace(/Duration:/i, "").trim() || null,
    type: null,
    downloads: { nowm: [], wm: [] },
    mp3: [],
    slides: [],
  };
  const slides = $(".carousel-item[data-data]");
  if (slides.length) {
    data.type = "photo";
    slides.each((_, el) => {
      try {
        const json = JSON.parse($(el).attr("data-data").replace(/&quot;/g, '"'));
        if (Array.isArray(json.URL)) {
          json.URL.forEach((u) => {
            data.slides.push({ index: data.slides.length + 1, url: u });
          });
        }
      } catch {}
    });
    return data;
  }
  data.type = "video";
  $("#formatselect option").each((_, el) => {
    const label = $(el).text().toLowerCase();
    const raw = $(el).attr("value");
    if (!raw) return;
    try {
      const json = JSON.parse(raw.replace(/&quot;/g, '"'));
      if (!json.URL) return;
      if (label.includes("mp4") && !label.includes("watermark")) {
        data.downloads.nowm.push(...json.URL);
      }
      if (label.includes("watermark")) {
        data.downloads.wm.push(...json.URL);
      }
      if (label.includes("mp3")) {
        data.mp3.push(...json.URL);
      }
    } catch {}
  });
  return data;
}

async function tiktokDlViaSaveTT(url) {
  const { csrf, cookie } = await getSaveTTToken();
  const html = await fetchSaveTT(url, csrf, cookie);
  const parsed = parseSaveTTResponse(html);

  if (parsed.type === "photo" && parsed.slides.length > 0) {
    return {
      status: true,
      _provider: "savett",
      title: parsed.username || "",
      taken_at: "-",
      region: "-",
      durations: 0,
      duration: "0 Seconds",
      cover: null,
      data: parsed.slides.map((s) => ({ type: "photo", url: s.url })),
      music_info: { title: "-", author: "-", url: parsed.mp3[0] || null },
      stats: {
        views: parsed.views || "-",
        likes: parsed.likes || "-",
        comment: parsed.comments || "-",
        share: parsed.shares || "-",
        download: "-",
      },
      author: { fullname: parsed.username || "-", nickname: parsed.username || "-", avatar: null },
      _rawSaveTT: parsed,
    };
  }

  if (parsed.type === "video" && parsed.downloads.nowm.length > 0) {
    const data = [];
    if (parsed.downloads.nowm[0]) data.push({ type: "nowatermark", url: parsed.downloads.nowm[0] });
    if (parsed.downloads.wm[0]) data.push({ type: "watermark", url: parsed.downloads.wm[0] });
    if (parsed.mp3[0]) data.push({ type: "mp3", url: parsed.mp3[0] });
    // hd fallback: if multiple nowm URLs, treat first as hd
    return {
      status: true,
      _provider: "savett",
      title: parsed.username || "",
      taken_at: "-",
      region: "-",
      durations: 1,
      duration: parsed.duration || "1 Seconds",
      cover: null,
      data,
      music_info: { title: "-", author: "-", url: parsed.mp3[0] || null },
      stats: {
        views: parsed.views || "-",
        likes: parsed.likes || "-",
        comment: parsed.comments || "-",
        share: parsed.shares || "-",
        download: "-",
      },
      author: { fullname: parsed.username || "-", nickname: parsed.username || "-", avatar: null },
      _rawSaveTT: parsed,
    };
  }

  if (parsed.mp3.length > 0) {
    return {
      status: true,
      _provider: "savett",
      title: parsed.username || "",
      taken_at: "-",
      region: "-",
      durations: 0,
      duration: "0 Seconds",
      cover: null,
      data: [{ type: "mp3", url: parsed.mp3[0] }],
      music_info: { title: "-", author: "-", url: parsed.mp3[0] },
      stats: {
        views: parsed.views || "-",
        likes: parsed.likes || "-",
        comment: parsed.comments || "-",
        share: parsed.shares || "-",
        download: "-",
      },
      author: { fullname: parsed.username || "-", nickname: parsed.username || "-", avatar: null },
      _rawSaveTT: parsed,
    };
  }

  throw new Error("SaveTT tidak mengembalikan data video/slide");
}

async function tiktokDl(url) {
  const resolved = await resolveTikTokUrl(url);
  const errors = [];
  // try tikwm first
  try {
    return await tiktokDlViaTikWM(resolved);
  } catch (e) {
    const msg = e?.response?.status ? `TikWM ${e.response.status}` : e.message;
    errors.push(`tikwm: ${msg}`);
    console.warn("[tiktokDl] tikwm failed:", msg);
  }
  // fallback to savett
  try {
    return await tiktokDlViaSaveTT(resolved);
  } catch (e) {
    const msg = e?.response?.status ? `saveTT ${e.response.status}` : e.message;
    errors.push(`savett: ${msg}`);
    console.warn("[tiktokDl] savett failed:", msg);
  }
  throw new Error(errors.join(" | ") || "Semua provider TikTok gagal");
}

const pluginConfig = {
  name: ["tiktok", "tt", "ttmp4"],
  alias: ["tiktokdl", "ttdown"],
  category: "download",
  description: "Download video/slide TikTok tanpa watermark",
  usage: ".tiktok <url>",
  example: ".tiktok https://vt.tiktok.com/xxx",
  isOwner: false,
  isPremium: false,
  isGroup: false,
  isPrivate: false,
  cooldown: 10,
  energi: 1,
  isEnabled: true,
};

async function handler(m, { sock }) {
  const text = m.text?.trim();
  const prefix = m.prefix;
  const command = m?.command;
  if (!text) {
    m.react("❌");
    return m.reply(
      `📌 Contoh: *${prefix + command} https://vt.tiktok.com/...*`
    );
  }
  if (!/tiktok\.com|vt\.tiktok|vm\.tiktok/i.test(text)) {
    m.react("❌");
    return m.reply("❌ URL tidak valid. Gunakan link TikTok yang valid.\n\nContoh: `.tt https://vt.tiktok.com/ZSxxxx/`");
  }
  m.react("🕕");
  try {
    const result = await tiktokDl(text);

    const musicButton = {
      name: "quick_reply",
      buttonParamsJson: JSON.stringify({
        display_text: "🎵 Ambil Music",
        id: `${prefix}ttmp3 ${text}`,
      }),
    };

    const isVideo = result.durations > 0 && result.duration !== "0 Seconds" && result.data.some((d) => d.type.includes("nowatermark") || d.type === "video" || d.type === "watermark");
    // Also handle savett shape where data contains nowatermark
    if (isVideo) {
      const videoItem = result.data.find(
        (e) => e.type === "nowatermark_hd" || e.type === "nowatermark" || e.type === "video"
      ) || result.data.find((e) => e.type === "watermark") || result.data[0];

      if (!videoItem?.url) throw new Error("URL video tidak ditemukan");

      const caption =
        `🎵 *𝗧 𝗜 𝗞 𝗧 𝗢 𝗞  -  𝗗 𝗢 𝗪 𝗡 𝗟 𝗢 𝗔 𝗗 𝗘 𝗥* [${result._provider || "tikwm"}]\n\n` +
        `- Author: *${result.author.nickname}* (${result.author.fullname})\n` +
        `- Caption: ${result.title || "-"}\n` +
        `- Music: ${result.music_info.title} - ${result.music_info.author}\n` +
        `- Duration: ${result.duration}\n` +
        `- Uploaded: ${result.taken_at}\n` +
        `- Region: ${result.region}\n\n` +
        `*Statistik Video:*\n` +
        `- Views: *${result.stats.views}*\n` +
        `- Likes: *${result.stats.likes}*\n` +
        `- Comments: *${result.stats.comment}*\n` +
        `- Shares: *${result.stats.share}*\n` +
        `- Downloads: *${result.stats.download}*`;

      const videoRes = await axios.get(videoItem.url, {
        responseType: "arraybuffer",
        timeout: 60000,
        headers: {
          "User-Agent":
            "Mozilla/5.0 (Linux; Android 10; K) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Mobile Safari/537.36",
          Referer: "https://www.tiktok.com/",
        },
        maxRedirects: 5,
      });

      const ct = videoRes.headers["content-type"] || "";
      if (ct.includes("text/html")) throw new Error("Gagal mendapatkan file video (response HTML)");

      await sock.sendButton(m.chat, Buffer.from(videoRes.data), caption, m, {
        type: "video",
        mimetype: videoRes.headers["content-type"]?.includes("video") ? videoRes.headers["content-type"] : "video/mp4",
        buttons: [musicButton],
      });
    } else {
      const caption =
        `📸 *𝗧 𝗜 𝗞 𝗧 𝗢 𝗞  -  𝗗 𝗢 𝗪 𝗡 𝗟 𝗢 𝗔 𝗗 𝗘 𝗥* [${result._provider || "tikwm"}]\n\n` +
        `- Author: *${result.author.nickname}* (${result.author.fullname})\n` +
        `- Caption: ${result.title || "-"}\n` +
        `- Music: ${result.music_info.title} - ${result.music_info.author}\n` +
        `- Uploaded: ${result.taken_at}\n` +
        `- Region: ${result.region}\n\n` +
        `*Statistik Konten:*\n` +
        `- Views: *${result.stats.views}*\n` +
        `- Likes: *${result.stats.likes}*\n` +
        `- Comments: *${result.stats.comment}*\n` +
        `- Shares: *${result.stats.share}*\n` +
        `- Downloads: *${result.stats.download}*`;

      const slides = result.data
        ?.filter((d) => d.type === "photo" || d.url?.match(/\.(jpg|jpeg|png|webp)/i) || result.durations === 0)
        .map((zan, idx) => ({
          image: { url: zan.url },
          caption: idx === 0 ? caption : "",
        })) || [];

      // Fallback if slides empty but data has photo-like URLs
      const finalSlides = slides.length ? slides : result.data.map((zan, idx) => ({
        image: { url: zan.url },
        caption: idx === 0 ? caption : "",
      }));

      if (finalSlides.length === 0) throw new Error("Tidak ada slide foto ditemukan");

      await sock.sendMessage(
        m.chat,
        {
          albumMessage: finalSlides,
        },
        { quoted: m }
      );

      await sock.sendButton(m.chat, null, `📸 Slide berhasil dikirim!\nTekan tombol di bawah untuk ambil music.`, m, {
        buttons: [musicButton],
      });
    }
    m.react("✅");
  } catch (e) {
    console.error("[tiktokDl] handler error:", e);
    m.react("❌");
    const detail = e.message?.slice(0, 300) || "Unknown error";
    m.reply(`❌ *Gagal mengunduh TikTok*\n\n> ${detail}\n\nCoba lagi nanti, atau bisa coba *${prefix}tt2* sebagai alternatif.\nJika tetap gagal, pastikan link valid dan video tidak private.`);
  }
}

export { pluginConfig as config, handler, tiktokDl, tiktokDlViaTikWM, tiktokDlViaSaveTT };
