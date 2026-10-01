// `httpAxios`, bukan axios lepas: instance ini yang membawa timeout 15 detik,
// batas body 25MB, dan keep-alive. axios default punya `maxContentLength: -1`
// (tanpa batas), jadi satu respons host rusak bisa menarik isi penuh ke memori
// di kotak 1GB. Pola yang sama dipakai src/scraper/spotify.js.
import { httpAxios as axios } from "../lib/http.js";

// `signal` opsional supaya pemanggil lama yang hanya mengirim URL tetap jalan,
// tapi backend resolver boleh teruskannya: AbortController di resolve.js hanya
// menghentikan resolver menunggu, dan tanpa signal di sini POST-nya tetap hidup
// sampai timeout 15 detik milik httpAxios sambil memegang socket.
async function DouyinDL(douyinUrl, { signal } = {}) {
  const response = await axios.post(
    "https://snapvideotools.com/api/snap",
    { text: douyinUrl },
    {
      headers: {
        "Content-Type": "application/json",
        Accept: "application/json, text/javascript, */*; q=0.01",
        "X-Requested-With": "XMLHttpRequest",
        "User-Agent":
          "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
        Referer: "https://snapvideotools.com/",
        Origin: "https://snapvideotools.com",
      },
      signal,
    },
  );

  const data = response.data.data;

  if (!data || !data.mediaUrls) {
    return {
      status: false,
      error: "Data tidak ditemukan",
    };
  }

  const video = data.mediaUrls.find((m) => m.type === "video");
  const audio = data.mediaUrls.find((m) => m.type === "audio");

  return {
    status: true,
    title: data.title,
    platform: data.platformName,
    video: video ? video.url : null,
    audio: audio ? audio.url : null,
  };
}

export { DouyinDL };
