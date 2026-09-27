import fetch from "node-fetch";
import { downloadMediaMessage, getContentType } from "onigis";
import te from "../../src/lib/error.js";
import config from "../../config.js";
import { runUploadFanout } from "../../src/lib/upload-fanout.js";
import { UPLOAD_PROVIDERS } from "../../src/lib/upload-providers.js";

const pluginConfig = {
  name: "tourl",
  alias: ["upload", "url"],
  category: "tools",
  description: "Upload media ke multiple host, atau paste teks dan dapet link",
  usage: ".tourl (reply/kirim media) | .tourl <teks>",
  example: ".tourl",
  isOwner: false,
  isPremium: false,
  isGroup: false,
  isPrivate: false,
  cooldown: 10,
  energi: 1,
  isEnabled: true,
};

// Batas keras: file di atas ini ditolak sebelum contacting host mana pun.
// 9 host gratisan umumnya menolak di 10-100MB; kirim 200MB ke semuanya cuma
// membakar kuota IP dan memastikan semua host menolak.
const MAX_BYTES = 25 * 1024 * 1024;

// Batas waktu per host dan untuk seluruh fan-out. Tanpa ini satu host yang
// menggantung menahan semua hasil — user nunggu tanpa dapat apa-apa.
const FANOUT_PER_HOST_MS = 30000;
const FANOUT_DEADLINE_MS = 45000;

function formatBytes(bytes) {
  if (bytes >= 1024 * 1024 * 1024) {
    return `${(bytes / (1024 * 1024 * 1024)).toFixed(1)} GB`;
  }
  return `${Math.round(bytes / (1024 * 1024))} MB`;
}

// Daftar host upload tinggal satu sumber: src/lib/upload-providers.js, sama
// dengan yang dipakai uploader internal. Dulu tourl punya 9 salinan sendiri
// dan 4 di antaranya (TmpFiles, Qu.ax, Top4top) URL-nya serve HTML, bukan
// file — user dapat link rusak. Daftar itu sekarang 6 host yang sudah
// diverifikasi: upload 165KB lalu URL di-fetch, byte == byte asli.
const UPLOADERS = UPLOAD_PROVIDERS.map((p) => ({ name: p.name, fn: p.run }));

// Paste host buat upload teks. Dua-duanya raw-body POST tanpa API key, sudah
// dicek live 2026-09-26. Yang lain (dpaste, pastes.io, hastebin, ix.io) mati
// atau diblokir jadi tidak masuk daftar.
const PASTERS = [
  { name: "Paste.rs", host: "Paste.rs", url: "https://paste.rs" },
  { name: "C-Net", host: "C-Net", url: "https://paste.c-net.org" },
];

async function uploadToPaster(paster, text) {
  const res = await fetch(paster.url, {
    method: "POST",
    body: text,
    headers: { "content-type": "text/plain; charset=utf-8" },
    timeout: 30000,
  });

  if (!res.ok) throw new Error(`${paster.name} gagal`);

  const raw = (await res.text()).trim();
  const url = raw.split("\n")[0].trim();
  if (!/^https?:\/\//.test(url)) {
    throw new Error(`${paster.name} balasan bukan URL`);
  }
  return { host: paster.host, url, expires: "Permanent" };
}

// Host mati (dicek 2026-09-26) sudah dihapus: ImgDrop, Catbox (412),
// Pone (403), 8upload (body kosong), 0x0.st (DNS mati), Termai (butuh
// TERMAI_UPLOAD_KEY), Faddlaninco (DNS mati), Unggah (API 404).

function resolveText(m) {
  const args = (m.fullArgs || m.text || "").trim();
  if (args) return args;

  const quotedBody = (m.quoted?.body || "").trim();
  if (quotedBody) return quotedBody;

  return "";
}

async function handleText(m, text, sock) {
  await m.react("🕕");

  const results = [];
  const failed = [];

  for (const paster of PASTERS) {
    try {
      results.push(await uploadToPaster(paster, text));
    } catch (e) {
      failed.push(paster.name);
      console.error(`[tourl] paste ${paster.name} gagal: ${e?.message || e}`);
    }
  }

  if (results.length === 0) {
    await m.react("❌");
    return m.reply(
      `❌ Aduh kak, paste-nya gagal semua!\n\n> Gagal di server: ${failed.join(", ")}`,
    );
  }

  let body = `📝 *TEXT UPLOAD BERHASIL!* 📝\n\n`;
  body += `Teks kamu udah di-paste, yuk ambil linknya! ✨\n\n`;

  let contentTxt = "";
  results.forEach((r, i) => {
    contentTxt += `📋 *Server :* ${r.host}\n`;
    contentTxt += `⏳ *Expired :* ∞ Permanen\n`;
    contentTxt += `🔗 *Link :*\n`;
    contentTxt += `${r.url}`;
    if (i < results.length - 1) contentTxt += `\n\n`;
  });
  body += contentTxt;

  if (failed.length > 0) {
    body += `\n\n⚠️ _Fyi kak, ada yang gagal di server: ${failed.join(", ")}_`;
  }

  const buttons = results.slice(0, 3).map((r, i) => ({
    name: "cta_copy",
    buttonParamsJson: JSON.stringify({
      display_text: `📋 Salin Link ${r.host}`,
      id: `copy_${i}`,
      copy_code: r.url,
    }),
  }));

  try {
    await sock.sendButton(m.chat, null, body, m, {
      header: { title: "T O U R L" },
      footer: config.bot.name,
      buttons,
    });
  } catch (err) {
    console.error(`[tourl] gagal kirim card paste: ${err?.message || err}`);
    await m.reply(body);
  }

  await m.react("✅");
}

function getFileExtension(mimetype) {
  const mimeMap = {
    "image/jpeg": "jpg",
    "image/png": "png",
    "image/gif": "gif",
    "image/webp": "webp",
    "video/mp4": "mp4",
    "video/3gpp": "3gp",
    "video/quicktime": "mov",
    "audio/mpeg": "mp3",
    "audio/ogg": "ogg",
    "audio/wav": "wav",
    "audio/mp4": "m4a",
    "application/pdf": "pdf",
    "application/zip": "zip",
  };
  return mimeMap[mimetype] || "bin";
}

async function handler(m, { sock }) {
  let media = null;
  let mimetype = null;
  let filename = "file";

  if (m.quoted?.message) {
    const type = getContentType(m.quoted.message);
    if (!type || type === "conversation" || type === "extendedTextMessage") {
      // Quoted bukan media. Kalau ada teks untuk dipaste (args menang, lalu
      // isi chat yang di-reply), tanganin sebagai jalur paste.
      const pasteText = resolveText(m);
      if (pasteText) {
        return await handleText(m, pasteText, sock);
      }
      return m.reply("⚠️ Kak, tolong reply ke file (gambar/video/audio/berkas) ya!");
    }

    try {
      media = await downloadMediaMessage(
        { key: m.quoted.key, message: m.quoted.message },
        "buffer",
        {},
      );
      const content = m.quoted.message[type];
      mimetype = content?.mimetype || "application/octet-stream";
      filename = content?.fileName || `file.${getFileExtension(mimetype)}`;
    } catch (e) {
      console.error(`[tourl] gagal download media yang di-reply: ${e?.message || e}`);
      return m.reply(te(m.prefix, m.command, m.pushName));
    }
  } else if (m.message) {
    const type = getContentType(m.message);
    if (!type || type === "conversation" || type === "extendedTextMessage") {
      // Tidak ada media. Kalau ada teks, perlakukan sebagai isi paste.
      const pasteText = resolveText(m);

      if (pasteText) {
        return await handleText(m, pasteText, sock);
      }

      let txt = `📤 *MEDIA UPLOADER* 📤\n\n`;
      txt += `Halo kak! Butuh link untuk media kamu? Aku bisa bantu uploadin ke berbagai server gratisan loh!\n\n`;
      txt += `*Cara Pakai:*\n`;
      txt += `👉 Kirim media dengan caption \`${m.prefix}tourl\`\n`;
      txt += `👉 Atau reply media yang udah ada dengan \`${m.prefix}tourl\`\n`;
      txt += `👉 Kirim teks setelah command (\`${m.prefix}tourl <teks>\`) buat dapet link paste`;
      return m.reply(txt);
    }

    try {
      media = await downloadMediaMessage(
        { key: m.key, message: m.message },
        "buffer",
        {},
      );
      const content = m.message[type];
      mimetype = content?.mimetype || "application/octet-stream";
      filename = content?.fileName || `file.${getFileExtension(mimetype)}`;
    } catch (e) {
      console.error(`[tourl] gagal download media: ${e?.message || e}`);
      return m.reply(te(m.prefix, m.command, m.pushName));
    }
  }

  if (!media || media.length === 0) {
    return m.reply("❌ Waduh kak, medianya nggak kebaca. Coba kirim ulang deh!");
  }

  if (media.length > MAX_BYTES) {
    return m.reply(
      `❌ *Kak, filenya kelewat besar!*\n\n` +
        `> Ukuran: *${formatBytes(media.length)}*\n` +
        `> Batas: *${formatBytes(MAX_BYTES)}*\n\n` +
        `> Kebanyakan host gratisan nggak kuat file segede itu,` +
        ` jadi percuma dicoba. Compres dulu ya, kak 🙏`,
    );
  }

  await m.react("🕕");

  // Semua host jalan bareng dengan batas waktu per-host dan batas total.
  // Sequential bikin user nunggu 9× timeout; tanpa batas per-host, satu host
  // yang menggantung menahan semua hasil.
  const { done: results, failed, timedOut } = await runUploadFanout(
    UPLOADERS.map((u) => ({ name: u.name, run: () => u.fn(media, filename) })),
    { deadlineMs: FANOUT_DEADLINE_MS, perHostMs: FANOUT_PER_HOST_MS },
  );

  const notSucceeded = [...failed, ...timedOut];

  if (results.length === 0) {
    await m.react("❌");
    return m.reply(
      `❌ Aduh kak, semuanya pada error pas upload!\n\n> Gagal di server: ${notSucceeded.join(", ")}`,
    );
  }

  let text = `🚀 *UPLOAD BERHASIL!* 🚀\n\n`;
  text += `Yeay! Media kamu udah berhasil di-upload ke server awan. Silakan pilih linknya dan salin pakai tombol di bawah ya kak! ✨\n\n`;

  let contentTxt = "";
  results.forEach((r, i) => {
    const status = r.expires === "Permanent" ? "∞ Permanen" : r.expires;
    contentTxt += `☁️ *Server :* ${r.host}\n`;
    contentTxt += `⏳ *Expired :* ${status}\n`;
    contentTxt += `🔗 *Link :*\n`;
    contentTxt += `${r.url}`;
    if (i < results.length - 1) contentTxt += `\n\n`;
  });

  text += contentTxt;

  if (failed.length > 0) {
    text += `\n\n⚠️ _Fyi kak, ada yang gagal di server: ${notSucceeded.join(", ")}_`;
  }

  // Root cause tombol tidak pernah muncul: blok ini dulu memanggil
  // generateWAMessage tanpa `upload` ( selalu throw "options.upload is not a
  // function" -> jatuh ke catch -> kartu tidak pernah terkirim) lalu relay
  // tanpa additionalNodes biz/native_flow yang WA butuh agar tombol hidup.
  // sock.sendButton sudah benar-benar tested di repo (src/lib/socket.js) dan
  // handle upload header media + node biz sekaligus.
  const isImage = mimetype.startsWith("image");
  const isVideo = mimetype.startsWith("video");
  const buttons = results.slice(0, 5).map((r, i) => ({
    name: "cta_copy",
    buttonParamsJson: JSON.stringify({
      display_text: `📋 Salin Link ${r.host}`,
      id: `copy_${i}`,
      copy_code: r.url,
    }),
  }));

  try {
    await sock.sendButton(m.chat, isImage || isVideo ? media : null, text, m, {
      type: isImage ? "image" : "video",
      header: { title: "T O U R L" },
      footer: config.bot.name,
      buttons,
    });
  } catch (err) {
    console.error(`[tourl] gagal kirim card: ${err?.message || err}`);
    await m.reply(text);
  }

  await m.react("✅");
}

export { pluginConfig as config, handler };
