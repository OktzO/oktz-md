import axios from "axios";

const YOUTUBE_ID_REGEX =
  /(?:youtube\.com\/(?:[^\/]+\/.+\/|(?:v|e(?:mbed)?)\/|.*[?&]v=)|youtu\.be\/)([^"&?\/\s]{11})/;

function extractVideoId(url) {
  return String(url || "").match(YOUTUBE_ID_REGEX)?.[1] || null;
}

const CONVERT_FAILURE_MESSAGE = "Gagal mendapatkan data konversi.";

export function describeConvertFailure(convert) {
  const code = convert?.error;
  if (code === undefined || code === null || code === 0 || code === "0") {
    return CONVERT_FAILURE_MESSAGE;
  }
  return `${CONVERT_FAILURE_MESSAGE} (upstream error ${code})`;
}

async function ytdl(url, format = "mp3") {
  try {
    const videoId = extractVideoId(url);

    if (!videoId) {
      return {
        status: false,
        mess: "Format URL tidak dikenali atau bukan link YouTube yang valid.",
      };
    }

    const normalizedFormat =
      String(format || "mp3").toLowerCase() === "mp4" ? "mp4" : "mp3";

    const client = axios.create({
      timeout: 60000,
      headers: {
        "User-Agent":
          "Mozilla/5.0 (Linux; Android 16; NX729J) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/141.0.7271.123 Mobile Safari/537.36",
        Referer: "https://id.ytmp3.mobi/",
      },
    });

    const { data: init } = await client.get("https://d.ymcdn.org/api/v1/init", {
      params: {
        p: "y",
        23: "1llum1n471",
        _: Math.random(),
      },
    });

    if (!init?.convertURL) {
      return {
        status: false,
        mess: "Gagal menginisialisasi server (Init failed).",
      };
    }

    const { data: convert } = await client.get(init.convertURL, {
      params: {
        v: videoId,
        f: normalizedFormat,
        _: Math.random(),
      },
    });

    if (!convert?.progressURL || !convert?.downloadURL) {
      return {
        status: false,
        mess: describeConvertFailure(convert),
      };
    }

    let progress = 0;
    let title = convert.title || "";
    let attempts = 0;
    const maxAttempts = 20;

    while (progress < 3 && attempts < maxAttempts) {
      const { data } = await client.get(convert.progressURL);

      if ((data?.error || 0) > 0) {
        return {
          status: false,
          mess: `Error dari server: ${data.error}`,
        };
      }

      progress = Number(data?.progress || 0);
      title = data?.title || title;

      if (progress < 3) {
        attempts += 1;
        await new Promise((resolve) => setTimeout(resolve, 250));
      }
    }

    if (attempts >= maxAttempts && progress < 3) {
      return {
        status: false,
        mess: "Request timeout (proses terlalu lama).",
      };
    }

    return { status: true, title, dl: convert.downloadURL };
  } catch (e) {
    return { status: false, mess: `System Error: ${e.message}` };
  }
}

class Youtube {
  constructor() {
    this.CREATED_BY = "Ditzzy";
    this.NOTE = "Thank you for using this scrape";
  }

  wrapResponse(data) {
    return {
      created_by: this.CREATED_BY,
      note: this.NOTE,
      results: data,
    };
  }

  async download(url, format = "audio") {
    const outputFormat = ["video", "mp4"].includes(
      String(format || "audio").toLowerCase(),
    )
      ? "mp4"
      : "mp3";
    const result = await ytdl(url, outputFormat);

    if (!result?.status || !result?.dl) {
      throw new Error(result?.mess || "Gagal mengunduh konten YouTube");
    }

    return this.wrapResponse({
      title: result.title,
      download: result.dl,
      url: result.dl,
      format: outputFormat,
    });
  }
}

export { ytdl, Youtube };
export default ytdl;
