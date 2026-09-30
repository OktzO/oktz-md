import te from "../../src/lib/error.js";
import { resolver } from "../../src/lib/resolve.js";

const pluginConfig = {
  name: "spotplay",
  alias: ["splay"],
  category: "search",
  description: "Putar musik dari Spotify",
  usage: ".spotplay <query>",
  example: ".spotplay neffex grateful",
  cooldown: 15,
  energi: 1,
  isEnabled: true,
};

async function handler(m, { sock }) {
  const query = m.text?.trim();
  if (!query)
    return m.reply(`⚠️ *ᴄᴀʀᴀ ᴘᴀᴋᴀɪ*\n\n> \`${m.prefix}spotplay <query>\``);

  await m.react("🕕");

  try {
    const { data: cari } = await resolver.resolve("spotify", { q: query });
    const tracks = cari?.tracks ?? [];

    // Backend lokal mencari lewat YT Music, dan hasilnya berupa tautan YouTube
    // yang tidak punya ID track Spotify. Lewati saja daripada meneruskan ke
    // backend unduhan yang pasti menolaknya.
    const firstTrack = tracks.find((t) => /^https?:\/\/open\.spotify\.com\/track\//i.test(t.url));

    if (!firstTrack) {
      await m.react("❌");
      return m.reply("❌ Lagu Spotify tidak ditemukan.");
    }

    const { data: result } = await resolver.resolve("spotify", { url: firstTrack.url });

    if (!result?.url) {
      await m.react("❌");
      return m.reply("❌ Gagal mengambil link download lagu Spotify.");
    }

    await sock.sendMedia(m.chat, result.url, null, m, {
      type: "audio",
      mimetype: "audio/mpeg",
      ptt: false,
      fileName: `${result.artist || "Spotify"} - ${result.title || "audio"}.mp3`,
    });

    await m.react("✅");
  } catch (e) {
    console.error("[Spotplay Error]", e);
    await m.react("☢");
    m.reply(te(m.prefix, m.command, m.pushName));
  }
}

export { pluginConfig as config, handler };
