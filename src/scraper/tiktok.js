import axios from "axios";
import * as cheerio from "cheerio";

const YUULABS_API = "https://api.yuulabs.web.id/api/downloader/tiktok?url=";
const REQUEST_HEADERS = {
  "user-agent":
    "Mozilla/5.0 (Linux; Android 15; SM-F958 Build/AP3A.240905.015) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0.6723.86 Mobile Safari/537.36",
};

async function resolveTikTokUrl(url) {
  try {
    if (/v[mt]\.tiktok\.com/i.test(url)) {
      const res = await axios.get(url, {
        maxRedirects: 5,
        timeout: 10000,
        headers: { "User-Agent": REQUEST_HEADERS["user-agent"] },
        validateStatus: (s) => s >= 200 && s < 300,
      });
      const finalUrl = res.request?.res?.responseUrl || res.request?.responseURL || res.config?.url || url;
      if (finalUrl && finalUrl.includes("tiktok.com")) return finalUrl;
    }
  } catch {}
  return url;
}

async function ttdownFromTikWM(url) {
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
          "User-Agent": REQUEST_HEADERS["user-agent"],
          "X-Requested-With": "XMLHttpRequest",
        },
        params: { url, count: 12, cursor: 0, web: 1, hd: 1 },
        timeout: 15000,
      }
    )
  ).data;
  if (raw?.code !== undefined && raw.code !== 0) throw new Error(raw.msg || "TikWM error");
  const res = raw?.data || raw;
  if (!res) throw new Error("TikWM no data");
  const downloads = [];
  if (res.duration == 0 && Array.isArray(res.images)) {
    res.images.forEach((v) => downloads.push({ type: "photo", label: "Photo", url: String(v) }));
  } else {
    if (res.hdplay) downloads.push({ type: "nowatermark_hd", label: "Video HD", url: "https://www.tikwm.com" + res.hdplay });
    if (res.play) downloads.push({ type: "nowatermark", label: "Video tanpa watermark", url: "https://www.tikwm.com" + res.play });
    if (res.wmplay) downloads.push({ type: "watermark", label: "Video watermark", url: "https://www.tikwm.com" + res.wmplay });
  }
  if (res.music) downloads.push({ type: "mp3", label: "Audio MP3", url: "https://www.tikwm.com" + res.music });
  else if (res.music_info?.play) downloads.push({ type: "mp3", label: "Audio MP3", url: res.music_info.play });
  if (downloads.length === 0) throw new Error("TikWM tidak mengembalikan link download");
  return {
    title: res.title || "",
    author: { username: res.author?.nickname || res.author?.unique_id || "", avatar: res.author?.avatar ? "https://www.tikwm.com" + res.author.avatar : null },
    cover: res.cover ? "https://www.tikwm.com" + res.cover : null,
    downloads,
  };
}

const SAVETT_HEADERS = {
  "Content-Type": "application/x-www-form-urlencoded",
  Accept: "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
  Origin: "https://savett.cc",
  Referer: "https://savett.cc/en1/download",
  "User-Agent": "Mozilla/5.0 (Linux; Android 10; K) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/139.0.0.0 Mobile Safari/537.36",
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

async function ttdownFromSaveTT(url) {
  const { csrf, cookie } = await getSaveTTToken();
  const { data: html } = await axios.post(
    "https://savett.cc/en1/download",
    `csrf_token=${encodeURIComponent(csrf)}&url=${encodeURIComponent(url)}`,
    { headers: { ...SAVETT_HEADERS, Cookie: cookie }, timeout: 15000 }
  );
  const $ = cheerio.load(html);
  const downloads = [];
  const slides = $(".carousel-item[data-data]");
  const title = $("#video-info h3").first().text().trim() || "";
  const avatar = null;
  if (slides.length) {
    slides.each((_, el) => {
      try {
        const json = JSON.parse($(el).attr("data-data").replace(/&quot;/g, '"'));
        if (Array.isArray(json.URL)) json.URL.forEach((u) => downloads.push({ type: "photo", label: "Photo", url: u }));
      } catch {}
    });
    $("#formatselect option").each((_, el) => {
      if ($(el).text().toLowerCase().includes("mp3")) {
        try {
          const json = JSON.parse($(el).attr("value").replace(/&quot;/g, '"'));
          if (json.URL) downloads.push({ type: "mp3", label: "Audio MP3", url: json.URL[0] || json.URL });
        } catch {}
      }
    });
    if (downloads.length === 0) throw new Error("SaveTT photo no downloads");
    return { title, author: { username: title, avatar }, cover: null, downloads };
  }
  $("#formatselect option").each((_, el) => {
    const label = $(el).text().toLowerCase();
    const raw = $(el).attr("value");
    if (!raw) return;
    try {
      const json = JSON.parse(raw.replace(/&quot;/g, '"'));
      if (!json.URL) return;
      if (label.includes("mp4") && !label.includes("watermark")) downloads.push(...json.URL.map((u) => ({ type: "nowatermark", label: $(el).text().trim(), url: u })));
      else if (label.includes("watermark")) downloads.push(...json.URL.map((u) => ({ type: "watermark", label: $(el).text().trim(), url: u })));
      else if (label.includes("mp3")) downloads.push(...json.URL.map((u) => ({ type: "mp3", label: $(el).text().trim(), url: u })));
    } catch {}
  });
  if (downloads.length === 0) throw new Error("SaveTT tidak mengembalikan link download");
  return { title, author: { username: title, avatar }, cover: null, downloads };
}

async function ttdownFromYuuLabs(url) {
  const { data } = await axios.get(`${YUULABS_API}${encodeURIComponent(url)}`, {
    timeout: 30000,
    headers: REQUEST_HEADERS,
  });

  if (!data?.status || !data?.result) {
    throw new Error(data?.message || "YuuLabs response invalid");
  }

  const result = data.result;
  const downloads = [];

  if (result.videoUrl) {
    downloads.push({
      type: "nowatermark",
      label: "Video tanpa watermark",
      url: result.videoUrl,
    });
  }

  if (result.hdVideo) {
    downloads.push({
      type: "nowatermark_hd",
      label: "Video HD",
      url: result.hdVideo,
    });
  }

  if (result.audioUrl) {
    downloads.push({
      type: "mp3",
      label: "Audio MP3",
      url: result.audioUrl,
    });
  }

  if (downloads.length === 0) {
    throw new Error("YuuLabs tidak mengembalikan link download");
  }

  return {
    title: result.description || "",
    author: {
      username: result.author || "",
      avatar: null,
    },
    cover: null,
    downloads,
  };
}

async function ttdownFromMusicalDown(url) {
  const { data: html, headers } = await axios.get(
    "https://musicaldown.com/en",
    {
      timeout: 30000,
      headers: REQUEST_HEADERS,
    },
  );
  const $ = cheerio.load(html);

  const payload = {};
  $("#submit-form input").each((i, elem) => {
    const name = $(elem).attr("name");
    const value = $(elem).attr("value");
    if (name) payload[name] = value || "";
  });

  const urlField = Object.keys(payload).find((key) => !payload[key]);
  if (urlField) payload[urlField] = url;

  const cookieHeader = Array.isArray(headers["set-cookie"])
    ? headers["set-cookie"].join("; ")
    : "";

  const { data } = await axios.post(
    "https://musicaldown.com/download",
    new URLSearchParams(payload).toString(),
    {
      timeout: 30000,
      headers: {
        ...REQUEST_HEADERS,
        "content-type": "application/x-www-form-urlencoded; charset=UTF-8",
        cookie: cookieHeader,
        origin: "https://musicaldown.com",
        referer: "https://musicaldown.com/",
      },
    },
  );

  const $$ = cheerio.load(data);
  const videoHeader = $$(".video-header");
  const bgImage = videoHeader.attr("style");
  const coverMatch = bgImage?.match(/url\((.*?)\)/);

  const downloads = [];
  $$("a.download").each((i, elem) => {
    const $elem = $$(elem);
    const type = $elem.data("event")?.replace("_download_click", "");
    const label = $elem.text().trim();
    const downloadUrl = $elem.attr("href");
    if (!downloadUrl) return;
    downloads.push({
      type,
      label,
      url: downloadUrl,
    });
  });

  if (downloads.length === 0) {
    throw new Error("MusicalDown tidak mengembalikan link download");
  }

  return {
    title: $$(".video-desc").text().trim(),
    author: {
      username: $$(".video-author b").text().trim(),
      avatar: $$(".img-area img").attr("src"),
    },
    cover: coverMatch ? coverMatch[1] : null,
    downloads,
  };
}

async function ttdown(url) {
  if (!url.includes("tiktok.com")) throw new Error("Invalid url.");
  const resolved = await resolveTikTokUrl(url);
  const errors = [];
  const providers = [
    { name: "tikwm", fn: () => ttdownFromTikWM(resolved) },
    { name: "savett", fn: () => ttdownFromSaveTT(resolved) },
    { name: "yuulabs", fn: () => ttdownFromYuuLabs(resolved) },
    { name: "musicaldown", fn: () => ttdownFromMusicalDown(resolved) },
  ];
  for (const p of providers) {
    try {
      const res = await p.fn();
      if (res?.downloads?.length) return res;
      throw new Error("empty downloads");
    } catch (e) {
      const msg = e?.response?.status ? `${p.name} ${e.response.status}` : e.message;
      errors.push(`${p.name}: ${msg}`);
      console.warn(`[ttdown] ${p.name} failed:`, msg);
    }
  }
  throw new Error(errors.join(" | ") || "Semua provider TikTok gagal");
}

export default ttdown;
export { ttdownFromTikWM, ttdownFromSaveTT, ttdownFromYuuLabs, ttdownFromMusicalDown };
