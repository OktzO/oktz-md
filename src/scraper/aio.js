import instagramDownloader from "./ig.js";
import scrapePinterest from "./pindl.js";
import ytdl, { Youtube } from "./ytdl.js";
import axios from "axios";
import * as cheerio from "cheerio";

const PLATFORM_DETECT = [
  { key: "instagram", patterns: ["instagram.com"] },
  { key: "youtube", patterns: ["youtube.com", "youtu.be"] },
  { key: "tiktok", patterns: ["tiktok.com", "vt.tiktok.com"] },
  { key: "facebook", patterns: ["facebook.com", "fb.watch", "fb.com"] },
  { key: "pinterest", patterns: ["pinterest.com", "pin.it"] },
  { key: "capcut", patterns: ["capcut.com"] },
  { key: "twitter", patterns: ["twitter.com", "x.com"] },
  { key: "threads", patterns: ["threads.net"] },
  { key: "reddit", patterns: ["reddit.com"] },
];

function detectPlatform(url) {
  const lower = (url || "").toLowerCase();
  for (const p of PLATFORM_DETECT) {
    if (p.patterns.some((pat) => lower.includes(pat))) return p.key;
  }
  return null;
}

async function handleInstagram(url) {
  const result = await instagramDownloader(url);
  const media = (result.media || []).map((item) => ({
    type: item.type === "video" || item.type === "mp4" ? "video" : "image",
    url: item.url,
  }));
  return {
    platform: "instagram",
    title: result.title || result.username || "Instagram",
    thumbnail: result.thumbnail || null,
    media,
  };
}

async function handleYoutube(url) {
  const yt = new Youtube();
  const videoResult = await yt.download(url, "mp4");
  const audioResult = await yt.download(url, "mp3");
  const media = [];
  if (videoResult?.results?.download) {
    media.push({
      type: "video",
      url: videoResult.results.download,
      quality: "mp4",
    });
  }
  if (audioResult?.results?.download) {
    media.push({
      type: "audio",
      url: audioResult.results.download,
      quality: "mp3",
    });
  }
  return {
    platform: "youtube",
    title:
      videoResult?.results?.title || audioResult?.results?.title || "YouTube",
    thumbnail: null,
    media,
  };
}

async function handleTiktok(url) {
  const tryTikWM = async (u) => {
    const domain = "https://www.tikwm.com/api/";
    const raw = (
      await axios.post(
        domain,
        {},
        {
          headers: {
            Accept: "application/json, text/javascript, */*; q=0.01",
            "Content-Type": "application/x-www-form-urlencoded; charset=UTF-8",
            Origin: "https://www.tikwm.com",
            Referer: "https://www.tikwm.com/",
            "User-Agent":
              "Mozilla/5.0 (Linux; Android 10; K) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/116.0.0.0 Mobile Safari/537.36",
            "X-Requested-With": "XMLHttpRequest",
          },
          params: { url: u, count: 12, cursor: 0, web: 1, hd: 1 },
          timeout: 15000,
        }
      )
    ).data;
    if (raw?.code !== undefined && raw.code !== 0) throw new Error(raw.msg || "TikWM error");
    const res = raw?.data || raw;
    if (!res) throw new Error("TikWM no data");
    const media = [];
    if (res?.duration === 0) {
      (res.images || []).forEach((v) => media.push({ type: "image", url: String(v) }));
    } else {
      if (res?.hdplay)
        media.push({
          type: "video",
          url: "https://www.tikwm.com" + res.hdplay,
          quality: "HD",
        });
      if (res?.play)
        media.push({
          type: "video",
          url: "https://www.tikwm.com" + res.play,
          quality: "NoWM",
        });
    }
    if (media.length === 0) throw new Error("TikWM no media");
    return {
      platform: "tiktok",
      title: res?.title || "TikTok",
      thumbnail: res?.cover ? "https://www.tikwm.com" + res.cover : null,
      author: res?.author?.nickname || null,
      media,
    };
  };

  const trySaveTT = async (u) => {
    const headers = {
      "Content-Type": "application/x-www-form-urlencoded",
      Accept: "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
      Origin: "https://savett.cc",
      Referer: "https://savett.cc/en1/download",
      "User-Agent": "Mozilla/5.0 (Linux; Android 10; K) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/139.0.0.0 Mobile Safari/537.36",
    };
    const tokenRes = await axios.get("https://savett.cc/en1/download", { timeout: 15000, headers: { "User-Agent": headers["User-Agent"] } });
    const csrf = tokenRes.data.match(/name="csrf_token" value="([^"]+)"/)?.[1];
    const cookie = (tokenRes.headers["set-cookie"] || []).map((v) => v.split(";")[0]).join("; ");
    if (!csrf || !cookie) throw new Error("SaveTT token failed");
    const htmlRes = await axios.post(
      "https://savett.cc/en1/download",
      `csrf_token=${encodeURIComponent(csrf)}&url=${encodeURIComponent(u)}`,
      { headers: { ...headers, Cookie: cookie }, timeout: 15000 }
    );
    const $ = cheerio.load(htmlRes.data);
    const media = [];
    const slides = $(".carousel-item[data-data]");
    let title = $("#video-info h3").first().text().trim() || "TikTok";
    if (slides.length) {
      slides.each((_, el) => {
        try {
          const json = JSON.parse($(el).attr("data-data").replace(/&quot;/g, '"'));
          if (Array.isArray(json.URL) && json.URL.length) {
            // URL array = beberapa resolusi, ambil yang terbaik
            const best = [...json.URL].sort((a, b) => String(b).length - String(a).length)[0];
            media.push({ type: "image", url: String(best) });
          }
        } catch {}
      });
      // MP3 dari formatselect untuk photo post
      $("#formatselect option").each((_, el) => {
        if ($(el).text().toLowerCase().includes("mp3")) {
          try {
            const json = JSON.parse($(el).attr("value").replace(/&quot;/g, '"'));
            if (json.URL) media.push({ type: "audio", url: json.URL[0] || json.URL, quality: "mp3" });
          } catch {}
        }
      });
    } else {
      $("#formatselect option").each((_, el) => {
        const raw = $(el).attr("value");
        const label = $(el).text().toLowerCase();
        if (!raw) return;
        try {
          const json = JSON.parse(raw.replace(/&quot;/g, '"'));
          if (!json.URL) return;
          if (label.includes("mp4") && !label.includes("watermark")) json.URL.forEach((v) => media.push({ type: "video", url: v, quality: "NoWM" }));
          else if (label.includes("mp3")) json.URL.forEach((v) => media.push({ type: "audio", url: v, quality: "mp3" }));
        } catch {}
      });
    }
    if (media.length === 0) throw new Error("SaveTT no media");
    return { platform: "tiktok", title, thumbnail: null, author: title, media };
  };

  try {
    return await tryTikWM(url);
  } catch (e) {
    console.warn("[aio] tikwm failed:", e.message);
    try {
      return await trySaveTT(url);
    } catch (e2) {
      console.warn("[aio] savett failed:", e2.message);
      throw e;
    }
  }
}

async function handleFacebook(url) {
  const { fbdown } = await import("btch-downloader");
  const data = await fbdown(url);
  if (!data?.status) throw new Error("Facebook API returned no data");
  const videoUrl = data.HD || data.Normal_video;
  if (!videoUrl) throw new Error("No video URL");
  return {
    platform: "facebook",
    title: data.title || "Facebook",
    thumbnail: data.thumbnail || null,
    media: [{ type: "video", url: videoUrl, quality: data.HD ? "HD" : "SD" }],
  };
}

async function handlePinterest(url) {
  const result = await scrapePinterest(url);
  if (!result?.media?.length) throw new Error("No media found");
  return {
    platform: "pinterest",
    title: result.title || "Pinterest",
    thumbnail: null,
    media: result.media.map((m) => ({
      type: m.type === "video" ? "video" : "image",
      url: m.url,
      quality: m.quality || null,
    })),
  };
}

async function handleCapcut(url) {
  const { capcut } = await import("btch-downloader");
  const data = await capcut(url);
  if (!data?.status || !data?.originalVideoUrl)
    throw new Error("No CapCut video");
  return {
    platform: "capcut",
    title: data.title || "CapCut",
    thumbnail: data.thumbnail || null,
    media: [{ type: "video", url: data.originalVideoUrl }],
  };
}

const SAVEFBS_HEADERS = {
  accept: "*/*",
  "content-type": "application/json",
  referer: "https://savefbs.com/all-in-one-video-downloader/",
  "user-agent":
    "Mozilla/5.0 (Linux; Android 10; K) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/137.0.0.0 Mobile Safari/537.36",
};

async function handleSavefbs(url) {
  const BASE_URL = "https://savefbs.com/api/v1/aio";
  const infoRes = await fetch(`${BASE_URL}/html`, {
    method: "POST",
    headers: SAVEFBS_HEADERS,
    body: JSON.stringify({
      vid: url,
      prefix: "savefbs.com",
      ex: "",
      format: "",
    }),
  });
  const html = await infoRes.text();
  const $ = cheerio.load(html);

  const title = $("h3.text-sm").text().trim();
  const thumb = $("img.aio-thumbnail").attr("src");
  const token = $(".aio-format-btn").first().attr("data-loader-id");

  const formats = [];
  $(".aio-format-btn").each((_, el) => {
    const onclick = $(el).attr("onclick");
    const match = onclick?.match(/'([^']+)'/);
    if (match) formats.push(match[1]);
  });

  if (!token || !formats.length) throw new Error("Savefbs returned no data");

  const VIDEO_PRIORITY = ["1080", "720", "480", "360"];
  const AUDIO_FORMATS = ["mp3", "wav"];
  const bestVideo = VIDEO_PRIORITY.find((f) => formats.includes(f));
  const format =
    bestVideo || formats.find((f) => !AUDIO_FORMATS.includes(f)) || formats[0];

  const dlRes = await fetch(`${BASE_URL}/download`, {
    method: "POST",
    headers: SAVEFBS_HEADERS,
    body: JSON.stringify({ token, format }),
  });
  const dlData = await dlRes.json();
  const downloadUrl = dlData?.url || dlData?.download || dlData?.data?.url;
  if (!downloadUrl) throw new Error("Savefbs no download URL");

  const isAudio = AUDIO_FORMATS.includes(format);
  return {
    platform: "savefbs",
    title: title || "Downloaded",
    thumbnail: thumb || null,
    media: [
      { type: isAudio ? "audio" : "video", url: downloadUrl, quality: format },
    ],
  };
}

const PLATFORM_HANDLERS = {
  instagram: handleInstagram,
  youtube: handleYoutube,
  tiktok: handleTiktok,
  facebook: handleFacebook,
  pinterest: handlePinterest,
  capcut: handleCapcut,
  twitter: handleSavefbs,
  threads: handleSavefbs,
  reddit: handleSavefbs,
};

async function aiodl(url) {
  const platform = detectPlatform(url);
  if (!platform) throw new Error("Platform tidak dikenali");

  const handler = PLATFORM_HANDLERS[platform];
  if (!handler) throw new Error(`Handler untuk ${platform} tidak tersedia`);

  try {
    return await handler(url);
  } catch (err) {
    if (
      platform !== "twitter" &&
      platform !== "threads" &&
      platform !== "reddit"
    ) {
      try {
        return await handleSavefbs(url);
      } catch {}
    }
    throw err;
  }
}

export { aiodl, detectPlatform };
