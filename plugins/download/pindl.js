import fs from "fs";
import axios from "axios";
import path from "path";
import { queueFFmpeg } from "./../../src/lib/ffmpeg.js";
import { httpAxios } from "../../src/lib/http.js";
import te from "../../src/lib/error.js";
import { resolver } from "../../src/lib/resolve.js";

// Batas body video yang ditarik lewat axios biasa, dan juga batas file yang
// boleh dibaca utuh untuk dikirim (sock.sendMedia menerima Buffer). Di kotak 1GB
// dengan batas RSS 550MB, menarik video tanpa batas bisa membuat proses dibunuh
// memory-monitor — gagal dengan pesan jauh lebih murah daripada bot ikut mati.
//
// Jalur GIF sudah ikut dibatasi, tapi bukan dengan angka sendiri: ia menarik lewat
// `httpAxios` (src/lib/http.js:20) yang `maxContentLength`-nya 25MB, sehingga GIF
// ikut memakai angka yang sama dengan setiap pembacaan HTTP lain di repo ini dan
// tidak ada konstanta kedua yang bisa ikut melenceng. 25MB, bukan 64MB video,
// karena yang dipegang setelahnya adalah Buffer GIF penuh: file GIF besar sudah
// gawat sebelum ffmpeg sempat mengubahnya jadi mp4.
const BATAS_BODY_VIDEO = 64 * 1024 * 1024;

/** Batas bersifat inklusif: file tepat sebesar batas masih boleh dikirim. */
export function terlaluBesar(ukuran) {
  return ukuran > BATAS_BODY_VIDEO;
}

/**
 * Terjemahkan URL video menjadi rencana unduhan.
 *
 * URL aggregator berakhiran .mp4 dan sebenarnya sebuah master HLS, jadi ditulis
 * ulang ke .m3u8 supaya audio dan video bisa diambil terpisah. URL dari scraper
 * lokal berakhiran /720p dan isinya file langsung: membacanya sebagai manifest
 * berarti menarik seluruh video ke memori hanya supaya bisa di-`split("\n")`.
 */
function rencanaVideo(url) {
  const asal = String(url ?? "");
  const masterUrl = asal.includes(".mp4")
    ? asal.replace(/720p|480p|360p|240p/g, "hls").replace(".mp4", ".m3u8")
    : asal;
  return { masterUrl, hls: masterUrl.includes(".m3u8") };
}

/**
 * Host Pinterest dicek per label, sama seperti yang dilakukan kapabilitas:
 * `notpinterest.com` dan `pin.it.evil.example` bukan Pinterest dan tetap lolos
 * `includes`. Guard plugin ada supaya user dapat jawabannya sebelum kapabilitas
 * sempat mencatat kegagalan host — tiga link palsu dari satu user sudah cukup
 * membuka breaker scraper yang sehat selama 30 detik untuk semua orang.
 */
function hostPinterest(url) {
  let parsed;
  try {
    parsed = new URL(String(url).trim());
  } catch {
    return false;
  }
  if (parsed.protocol !== "http:" && parsed.protocol !== "https:") return false;
  const host = parsed.hostname.toLowerCase();
  if (host === "pin.it") return true;
  if (!host.split(".").includes("pinterest")) return false;
  return /^\/pin\//i.test(parsed.pathname);
}

const pluginConfig = {
  name: "pindl",
  alias: ["pinterestdl", "pindownload", "pintdl"],
  category: "download",
  description: "Download gambar/video dari Pinterest",
  usage: ".pindl <url>",
  example: ".pindl https://pin.it/xxx",
  isOwner: false,
  isPremium: false,
  isGroup: false,
  isPrivate: false,
  cooldown: 10,
  energi: 1,
  isEnabled: true,
};
async function handler(m, { sock }) {
  const url = m.text?.trim();
  if (!url) {
    return m.reply(
      `📌 *ᴘɪɴᴛᴇʀᴇsᴛ ᴅᴏᴡɴʟᴏᴀᴅ*\n\n` +
        `> Download gambar/video dari Pinterest\n\n` +
        `*ᴄᴏɴᴛᴏʜ:*\n` +
        `> \`${m.prefix}pindl https://pin.it/xxx\`\n` +
        `> \`${m.prefix}pindl https://pinterest.com/pin/xxx\``,
    );
  }
  // Yang menentukan benar atau tidak adalah `hostPinterest`. Syarat `includes`
  // di bawah tidak mengubah hasil — `hostPinterest` menyiratkan keduanya — dan
  // hanya menyaring teks tanpa struktur URL lebih dulu sebelum `new URL()` dipanggil.
  if ((!url.includes("pinterest") && !url.includes("pin.it")) || !hostPinterest(url)) {
    return m.reply("❌ URL tidak valid. Gunakan link Pinterest.");
  }
  m.react("🕕");
  try {
    // Scraper lokal lebih dulu, aggregator jadi cadangan: `data.media` sudah
    // dinormalisasi ke bentuk yang langsung dikonsumsi loop di bawah, jadi parser
    // aggregator lama tidak lagi perlu hidup di plugin ini.
    const { data } = await resolver.resolve("pinterest", { url });
    const mediaList = Array.isArray(data?.media) ? data.media : [];

    // `normalize` sudah menolak respons tanpa media, jadi daftar di sini tidak
    // mungkin kosong setelah resolve berhasil. Pemeriksaan tetap dijaga karena
    // ✅ di bawah dihitung dari file yang benar-benar terkirim, dan daftar kosong
    // berarti tidak ada satu pun file untuk dikirim.
    if (mediaList.length === 0) {
      throw new Error("Tidak ada media ditemukan");
    }

    // Reaksi ✅ hanya sah kalau ada file yang benar-benar terkirim.
    // Sebelumnya m.react("✅") berada di luar loop tanpa syarat: primary
    // gagal, fallback gagal, user dapat 0 file tapi tetap lihat centang hijau.
    let sentCount = 0;
    let lastError = "";

    for (const media of mediaList) {
      if (media.type === "video") {
        const { masterUrl, hls } = rencanaVideo(media.url);

        const tempDir = path.join(process.cwd(), "temp");
        if (!fs.existsSync(tempDir)) fs.mkdirSync(tempDir, { recursive: true });
        
        const timestamp = Date.now();
        const videoTemp = path.join(tempDir, `${timestamp}_v.mp4`);
        const audioTemp = path.join(tempDir, `${timestamp}_a.mp4`);
        const outputFile = path.join(tempDir, `${timestamp}_final.mp4`);

        try {
            let videoStreamUrl = null;
            let audioStreamUrl = null;

            // Hanya manifest HLS yang perlu diurai. Untuk file langsung, ffmpeg
            // menarik URL-nya sendiri secara streaming: kalau axios ikut menarik
            // lebih dulu, video penuh masuk proses sebelum ffmpeg sempat jalan.
            if (hls) {
                const resHls = await axios.get(masterUrl, {
                    headers: {
                        'User-Agent': 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36',
                        'Referer': 'https://www.pinterest.com/'
                    },
                    maxContentLength: BATAS_BODY_VIDEO
                });
                const text = resHls.data;
                const lines = text.split("\n").map(l => l.trim()).filter(Boolean);

                const baseUrl = masterUrl.substring(0, masterUrl.lastIndexOf("/") + 1);
                let lastBandwidth = 0;

                for (let i = 0; i < lines.length; i++) {
                  if (lines[i].startsWith("#EXT-X-STREAM-INF")) {
                    const bwMatch = lines[i].match(/BANDWIDTH=(\d+)/);
                    const bw = bwMatch ? parseInt(bwMatch[1]) : 0;
                    if (bw > lastBandwidth) {
                      lastBandwidth = bw;
                      const u = lines[i + 1];
                      videoStreamUrl = u?.startsWith("http") ? u : baseUrl + u;
                    }
                  }
                  if (lines[i].startsWith("#EXT-X-MEDIA") && lines[i].includes("TYPE=AUDIO")) {
                    const mUrl = lines[i].match(/URI="([^"]+)"/);
                    if (mUrl) audioStreamUrl = mUrl[1].startsWith("http") ? mUrl[1] : baseUrl + mUrl[1];
                  }
                }
            }

            const ffmpegHdr = `-user_agent "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36" -headers "Referer: https://www.pinterest.com/"`;

            // Tanpa manifest (atau tanpa varian video di dalamnya), ffmpeg
            // memakai URL itu apa adanya; video dan audio terpisah hanya perlu
            // digabung kalau keduanya benar-benar ada di manifest.
            if (!hls || !videoStreamUrl) {
                await queueFFmpeg(`ffmpeg -y ${ffmpegHdr} -i "${masterUrl}" -c copy "${outputFile}"`);
            } else {
                await queueFFmpeg(`ffmpeg -y ${ffmpegHdr} -i "${videoStreamUrl}" -c copy "${videoTemp}"`);
                if (audioStreamUrl) {
                    await queueFFmpeg(`ffmpeg -y ${ffmpegHdr} -i "${audioStreamUrl}" -c copy "${audioTemp}"`);
                    await queueFFmpeg(`ffmpeg -y -i "${videoTemp}" -i "${audioTemp}" -c copy "${outputFile}"`);
                } else {
                    fs.renameSync(videoTemp, outputFile);
                }
            }

            // File hasil ffmpeg dibaca utuh untuk dikirim, jadi ukurannya diperiksa
            // lebih dulu: video raksasa akan mendorong proses melewati batas RSS
            // 550MB, dan gagal dengan pesan lebih baik daripada bot ikut mati.
            const ukuran = fs.statSync(outputFile).size;
            if (terlaluBesar(ukuran)) {
              // Ditandai supaya catch di bawah tahu ini bukan masalah jaringan:
              // unduhan manual akan berhenti di batas yang sama, jadi mengulangnya
              // hanya membuat user menunggu pull 64MB yang pasti gagal.
              const gagal = new Error(
                `video ${Math.round(ukuran / 1024 / 1024)} MB melebihi batas ${BATAS_BODY_VIDEO / 1024 / 1024} MB`,
              );
              gagal.terlaluBesar = true;
              throw gagal;
            }

            await sock.sendMedia(m.chat, fs.readFileSync(outputFile), null, m, {
                type: "video",
                contextInfo: { forwardingScore: 99, isForwarded: true }
            });
            sentCount += 1;

        } catch (err) {
            console.error("[PinDL HLS Error]:", err.message);
            if (err?.terlaluBesar) {
                // Bukan jaringan yang rusak, jadi tidak ada yang perlu dicoba lagi:
                // unduhan manual memakai batas yang sama dan pasti gagal. Alasan
                // sebenarnya diteruskan apa adanya supaya user tidak membaca pesan
                // axios yang tidak menjelaskan apa pun.
                lastError = err.message;
            } else {
              try {
                // Fallback: Pinterest memblokir generic axios (403), jadi download manual pakai User-Agent.
                // Batas ukuran wajib: instance axios ini maxContentLength-nya -1,
                // dan satu video besar bisa membuat proses melewati batas 550MB.
                const fallbackBuffer = await axios.get(media.url, { 
                    responseType: 'arraybuffer',
                    headers: {
                        'User-Agent': 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36',
                        'Referer': 'https://www.pinterest.com/'
                    },
                    maxContentLength: BATAS_BODY_VIDEO
                });
                await sock.sendMedia(m.chat, Buffer.from(fallbackBuffer.data), null, m, {
                  type: "video",
                  contextInfo: { forwardingScore: 99, isForwarded: true },
                });
                sentCount += 1;
              } catch (fallbackErr) {
                console.error("[PinDL Fallback Error]:", fallbackErr.message);
                lastError = fallbackErr.message;
              }
            }
        } finally {
            if (fs.existsSync(videoTemp)) fs.unlinkSync(videoTemp);
            if (fs.existsSync(audioTemp)) fs.unlinkSync(audioTemp);
            if (fs.existsSync(outputFile)) fs.unlinkSync(outputFile);
        }
      } else if (media.type === "image") {
        if (media.url.includes("gif")) {
          const tempPath = path.join(process.cwd(), "temp");
          if (!fs.existsSync(tempPath))
            fs.mkdirSync(tempPath, { recursive: true });
          const id = Date.now();
          const gifPath = path.join(tempPath, `pin-${id}.gif`);
          const mp4Path = path.join(tempPath, `pin-${id}.mp4`);
          try {
            // `f()` sengaja tidak dipakai di sini: ia memakai undici dan tidak punya
            // batas ukuran, jadi satu GIF besar bisa masuk penuh ke memori di kotak
            // 1GB yang plafon RSS-nya 550MB. `httpAxios` membawa `maxContentLength`
            // 25MB, dan `responseType: "arraybuffer"` mengembalikan Buffer yang bisa
            // langsung ditulis ke disk tanpa disalin jadi string lebih dulu.
            const res = await httpAxios.get(media.url, {
              responseType: "arraybuffer",
            });
            const raw = Buffer.isBuffer(res?.data)
              ? res.data
              : Buffer.from(res?.data ?? "");
            if (!raw.length) throw new Error("Gagal download GIF");
            fs.writeFileSync(gifPath, raw);
            await queueFFmpeg(
              `ffmpeg -y -ignore_loop 0 -i "${gifPath}" -t 30 -vf "scale=trunc(iw/2)*2:trunc(ih/2)*2" -c:v libx264 -pix_fmt yuv420p -movflags faststart -preset ultrafast -an "${mp4Path}"`,
            );
            if (!fs.existsSync(mp4Path)) throw new Error("Gagal convert GIF");
            await sock.sendMedia(m.chat, fs.readFileSync(mp4Path), null, m, {
              type: "video",
              gifPlayback: true,
              contextInfo: {
                forwardingScore: 99,
                isForwarded: true,
              },
            });
            sentCount += 1;
          } catch (gifErr) {
            console.error("[PinDL] GIF convert error:", gifErr.message);
            await sock.sendMedia(m.chat, media.url, null, m, {
              type: "image",
              contextInfo: { forwardingScore: 99, isForwarded: true },
            });
            sentCount += 1;
          } finally {
            if (fs.existsSync(gifPath)) fs.unlinkSync(gifPath);
            if (fs.existsSync(mp4Path)) fs.unlinkSync(mp4Path);
          }
        } else {
          await sock.sendMedia(m.chat, media.url, null, m, {
            type: "image",
            contextInfo: {
              forwardingScore: 99,
              isForwarded: true,
            },
          });
            sentCount += 1;
        }
      }
    }
    if (sentCount > 0) {
      m.react("✅");
    } else {
      m.react("❌");
      m.reply(
        `❌ *ɢᴀɢᴀʟ*\n\n> Videonya gagal diunduh.\n` +
          (lastError ? `> Penyebab: \`${String(lastError).slice(0, 120)}\`\n` : "") +
          `> Pinterest biasanya memblokir unduhan dari luar — coba lagi nanti ya.`,
      );
    }
  } catch (error) {
    console.error("[PinDL] Error:", error);
    m.react("☢");
    m.reply(te(m.prefix, m.command, m.pushName));
  }
}
export { rencanaVideo, pluginConfig as config, handler };
