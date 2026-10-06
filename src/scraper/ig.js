import { igdl } from "btch-downloader";

function extFromToken(mediaUrl) {
  try {
    const token = new URL(mediaUrl).searchParams.get("token");
    if (!token) return "";
    const payload = JSON.parse(
      Buffer.from(token.split(".")[1], "base64").toString(),
    );
    return String(payload.url || "").match(/\.([a-z0-9]+)(?:\?|$)/i)?.[1]?.toLowerCase() || "";
  } catch {
    return "";
  }
}

async function instagramDownloader(url) {
  const data = await igdl(url);

  if (!data || !data.status || !Array.isArray(data.result) || data.result.length === 0) {
    throw new Error("Gagal mengambil media dari Instagram");
  }

  const media = data.result
    .filter((item) => String(item?.url || "").trim() !== "")
    .map((item) => {
    const ext = extFromToken(item.url);
    const isVideo = ext === "mp4" || ext === "mov" || ext === "m3u8" || String(item.url).includes(".mp4");
    return {
      type: isVideo ? "video" : "image",
      url: item.url,
      thumbnail: item.thumbnail || "",
    };
    });

  if (media.length === 0) {
    throw new Error("Gagal mengambil media dari Instagram");
  }

  const thumbUrl = media[0]?.thumbnail || "";

  return {
    status: true,
    username: "-",
    title: "Instagram",
    caption: "",
    thumbnail: thumbUrl,
    avatar: "",
    media: media,
  };
}

export default instagramDownloader;
