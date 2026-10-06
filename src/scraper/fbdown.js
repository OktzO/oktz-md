import { httpAxios as axios } from "../lib/http.js";

async function fbdown(url) {
  try {
    const res = await axios.get(`https://api.nexray.web.id/api/downloader/facebook`, {
      params: { url },
    });
    if (res.data?.status === true && res.data?.result) {
      const r = res.data.result;
      const medias = [];
      if (r.video_hd) medias.push({ quality: "hd", url: r.video_hd });
      if (r.video_sd) medias.push({ quality: "sd", url: r.video_sd });
      return { status: true, result: { title: r.title ?? "", medias } };
    }
    return { status: false, message: "Gagal mengambil data dari API" };
  } catch (err) {
    return { status: false, message: err.message };
  }
}

export { fbdown };
