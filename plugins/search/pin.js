import {
  generateWAMessage,
  generateWAMessageFromContent,
  jidNormalizedUser,
} from "ourin";
import axios from "axios";
import crypto from "crypto";
import te from "../../src/lib/ourin-error.js";
import { f } from "../../src/lib/ourin-http.js";

const pluginConfig = {
  name: "pin",
  alias: ["pinsearch", "pinterestsearch", "pins"],
  category: "search",
  description: "Cari gambar di Pinterest (album)",
  usage: ".pin <query>",
  example: ".pin Zhao Lusi",
  isOwner: false,
  isPremium: false,
  isGroup: false,
  isPrivate: false,
  cooldown: 10,
  energi: 1,
  isEnabled: true,
};

async function handler(m, { sock }) {
  const query = m.text?.trim();
  if (!query) {
    return m.reply(
      `🔍 *ᴘɪɴᴛᴇʀᴇsᴛ sᴇᴀʀᴄʜ*\n\n` +
        `> Contoh:\n` +
        `\`${m.prefix}pin Zhao Lusi\``,
    );
  }
  await m.react("🕕");

  try {
    // cuki.biz.id mati (401) — pindah ke azbry (sama seperti pindl.js)
    const data = await f(
      `https://api.azbry.com/api/search/pinterest?q=${encodeURIComponent(query)}`,
    );

    const results = (data?.result || [])
      .map((item) => item?.image || item?.images_url)
      .filter(Boolean)
      .slice(0, 10);
    if (results.length === 0) {
      await m.react("❌");
      return m.reply(`❌ Tidak ditemukan hasil untuk: ${query}`);
    }

    const mediaList = [];

    for (const imageUrl of results) {
      try {
        const imgRes = await axios.get(imageUrl, {
          responseType: "arraybuffer",
          timeout: 15000,
          headers: { Referer: "https://www.pinterest.com/" },
        });
        const imgBuffer = Buffer.from(imgRes.data);

        if (imgBuffer.length > 1000) {
          mediaList.push({ image: imgBuffer });
        }
      } catch {
        continue;
      }
    }

    if (mediaList.length === 0) {
      await m.react("❌");
      return m.reply("❌ Gagal memuat gambar");
    }

    try {
      const opener = generateWAMessageFromContent(
        m.chat,
        {
          messageContextInfo: { messageSecret: crypto.randomBytes(32) },
          albumMessage: {
            expectedImageCount: mediaList.length,
            expectedVideoCount: 0,
          },
        },
        {
          userJid: jidNormalizedUser(sock.user.id),
          quoted: m,
          upload: sock.waUploadToServer,
        },
      );

      await sock.relayMessage(opener.key.remoteJid, opener.message, {
        messageId: opener.key.id,
      });

      for (const content of mediaList) {
        const msg = await generateWAMessage(opener.key.remoteJid, content, {
          upload: sock.waUploadToServer,
        });

        msg.message.messageContextInfo = {
          messageSecret: crypto.randomBytes(32),
          messageAssociation: {
            associationType: 1,
            parentMessageKey: opener.key,
          },
        };

        await sock.relayMessage(msg.key.remoteJid, msg.message, {
          messageId: msg.key.id,
        });
      }
    } catch (albumErr) {
      console.log("[Pin] Album gagal, kirim satu-satu:", albumErr.message);

      // Fallback kirim satu-satu
      for (const content of mediaList) {
        await sock.sendMessage(
          m.chat,
          { image: content.image },
          { quoted: m },
        );
      }
    }
    await m.react("✅");
  } catch (err) {
    console.error("[Pin] Error:", err.message);
    await m.react("☢");
    m.reply(te(m.prefix, m.command, m.pushName));
  }
}

export { pluginConfig as config, handler };
