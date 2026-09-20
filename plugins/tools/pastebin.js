import axios from "axios";
import te from "../../src/lib/ourin-error.js";
import { sendToolsPreview } from "../../src/lib/ourin-context.js";
const pluginConfig = {
  name: "pastebin",
  alias: ["paste", "pb"],
  category: "tools",
  description: "Upload teks ke Pastebin",
  usage: ".pastebin <text>",
  example: '.pastebin console.log("Hello World")',
  isOwner: false,
  isPremium: false,
  isGroup: false,
  isPrivate: false,
  cooldown: 10,
  energi: 1,
  isEnabled: true,
};

async function handler(m, { sock }) {
  let text = m.args.join(" ");

  if (m.quoted?.text) {
    text = m.quoted.text;
  }

  if (!text) {
    return m.reply(
      `📋 *ᴘᴀsᴛᴇʙɪɴ ᴜᴘʟᴏᴀᴅ*\n\n` +
        `Kirim teks untuk di-upload ke Pastebin.\n\n` +
        `*Cara pakai:*\n` +
        `• \`${m.prefix}pastebin <text>\`\n` +
        `• Reply teks dengan \`${m.prefix}pastebin\`\n\n` +
        `> Contoh: \`${m.prefix}pastebin console.log("Hello")\``,
    );
  }

  const api_dev_key = process.env.PASTEBIN_DEV_KEY || "";
  if (!api_dev_key) {
    return m.reply(
      `❌ *PASTEBIN API Key belum diset!*\n\n` +
        `> Owner harus set \`PASTEBIN_DEV_KEY\` di file \`.env\`\n` +
        `> Ambil key di: https://pastebin.com/api\n` +
        `> Lalu restart bot`
    );
  }
  const api_paste_code = text.trim();
  const api_paste_name = `Paste dari ${m.pushName || "User"} - ${new Date().toLocaleDateString("id-ID")}`;

  const data = new URLSearchParams({
    api_dev_key,
    api_option: "paste",
    api_paste_code,
    api_paste_name,
    api_paste_private: "1",
  });

  try {
    const res = await axios.post(
      "https://pastebin.com/api/api_post.php",
      data.toString(),
      {
        headers: { "Content-Type": "application/x-www-form-urlencoded" },
        timeout: 15000,
      },
    );

    const url = res.data;

    if (url.startsWith("Bad API request")) {
      return m.reply(`❌ *ɢᴀɢᴀʟ*\n\n> ${url}`);
    }

    const responseText =
      `✅ *ᴘᴀsᴛᴇʙɪɴ ʙᴇʀʜᴀsɪʟ*\n\n` +
      `╭┈┈⬡「 📋 *ᴅᴇᴛᴀɪʟ* 」\n` +
      `┃ 📝 ᴊᴜᴅᴜʟ: *${api_paste_name}*\n` +
      `┃ 📊 ᴜᴋᴜʀᴀɴ: *${text.length} chars*\n` +
      `┃ 🔗 ʟɪɴᴋ: ${url}\n` +
      `╰┈┈⬡\n\n` +
      `> Paste akan expired sesuai pengaturan Pastebin.`;
    await sendToolsPreview(
      sock,
      m.chat,
      responseText,
      "Pastebin Upload",
      api_paste_name,
      { quoted: m },
    );
  } catch (e) {
    m.reply(te(m.prefix, m.command, m.pushName));
  }
}

export { pluginConfig as config, handler };
