import { httpAxios as axios } from "../lib/http.js";
import { pathToFileURL } from "node:url";

async function downloadSpotify(spotifyUrl) {
  let response;
  try {
    response = await axios.post(
      "https://spotyloader.com/api/spotify/track",
      { url: spotifyUrl },
      {
        headers: {
          "Content-Type": "application/json",
          "User-Agent":
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
          Referer: "https://spotyloader.com/",
          Origin: "https://spotyloader.com",
        },
      },
    );
  } catch (error) {
    // `||` bukan `??`: upstream yang diblokir (Cloudflare) balas body kosong,
    // dan `??` akan memberi user pesan "spotyloader gagal: " tanpa isi. `cause`
    // + `status` disimpan supaya consumer bisa membedakan timeout, 4xx, dan DNS
    // gagal saat memilih backend fallback — tanpa respons HTTP tidak ada status
    // sama sekali, dan bukan default 500 (itu menyamarkan DNS gagal).
    const detail = error.response?.data || error.message || String(error);
    const wrapped = new Error(
      `spotyloader gagal: ${typeof detail === "string" ? detail : JSON.stringify(detail)}`,
      { cause: error },
    );
    if (error.response?.status !== undefined) wrapped.status = error.response.status;
    throw wrapped;
  }

  const data = response.data;
  if (!data?.downloadLink) {
    throw new Error(
      `spotyloader tidak mengembalikan downloadLink: ${JSON.stringify(data)}`,
    );
  }

  return {
    title: data.post.name,
    artist: data.post.artist,
    url: data.downloadLink,
    mime: data.post.mime,
  };
}

export { downloadSpotify };

// Pemanggilan CLI hanya boleh jalan kalau file ini dieksekusi langsung. Tanpa
// guard ini, satu plugin yang meng-import modul ini menembak request HTTP ke
// spotyloader setiap kali dimuat.
if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  const url = process.argv[2];
  if (url) downloadSpotify(url).then((r) => console.log(r)).catch((e) => console.error(e.message));
}
