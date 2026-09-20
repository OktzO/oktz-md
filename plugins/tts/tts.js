import te from "../../src/lib/ourin-error.js";
import ourinApi from "../../src/lib/ourin-apimanager.js";
const pluginConfig = {
  name: "tts",
  alias: ["say"],
  category: "tts",
  description: "Google Text To Speech",
  usage: ".tts <text>",
  example: ".tts halo semua",
  cooldown: 10,
  energi: 1,
  isEnabled: true,
};

async function handler(m, { sock }) {
  const text = m.text?.trim();

  if (!text) {
    return m.reply(`🎤 *Google TTS*\n\nGunakan:\n${m.prefix}tts halo dunia`);
  }

  m.react("🎤");

  async function textToSpeech2(text) {
    try {
      const response = await ourinApi.nexray.geminiTts(text);
      return response;
    } catch (error) {
      throw error;
    }
  }

  try {
    const t = await textToSpeech2(text);
    await sock.sendMessage(
      m.chat,
      {
        audio: { url: t.result },
        mimetype: "audio/mpeg",
      },
      { quoted: m },
    );
    m.react("✅");
  } catch (err) {
    m.react("☢");
    m.reply(te(m.prefix, m.command, m.pushName));
  }
}

export { pluginConfig as config, handler };
