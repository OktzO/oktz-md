import axios from 'axios'
import config from '../../config.js'
import * as timeHelper from '../../src/lib/time.js'
import te from '../../src/lib/error.js'
const pluginConfig = {
  name: ["cekvps", "cekdroplet", "vpsstatus", "infovps"],
  alias: [],
  category: "vps",
  description: "Cek detail VPS DigitalOcean",
  usage: ".cekvps <id>",
  example: ".cekvps 123456789",
  isOwner: false,
  isPremium: false,
  isGroup: false,
  isPrivate: false,
  cooldown: 5,
  energi: 0,
  isEnabled: true,
};

function hasAccess(sender, isOwner) {
  if (isOwner) return true;
  const cleanSender = sender?.split("@")[0];
  if (!cleanSender) return false;
  const doConfig = config.digitalocean || {};
  return (
    (doConfig.sellers || []).includes(cleanSender) ||
    (doConfig.ownerPanels || []).includes(cleanSender)
  );
}

async function handler(m, { sock }) {
  const token = config.digitalocean?.token;

  if (!token) {
    return m.reply(`⚠️ *ᴅɪɢɪᴛᴀʟᴏᴄᴇᴀɴ ʙᴇʟᴜᴍ ᴅɪsᴇᴛᴜᴘ*`);
  }

  if (!hasAccess(m.sender, m.isOwner)) {
    return m.reply(`❌ *ᴀᴋsᴇs ᴅɪᴛᴏʟᴀᴋ*`);
  }

  const dropletId = m.text?.trim();
  if (!dropletId) {
    return m.reply(`⚠️ *ᴄᴀʀᴀ ᴘᴀᴋᴀɪ*\n\n> \`${m.prefix}cekvps <droplet_id>\``);
  }

  try {
    const response = await axios.get(
      `https://api.digitalocean.com/v2/droplets/${dropletId}`,
      {
        headers: { Authorization: `Bearer ${token}` },
      },
    );

    const droplet = response.data.droplet;
    const ip =
      droplet.networks?.v4?.find((n) => n.type === "public")?.ip_address || "-";
    const ipv6 = droplet.networks?.v6?.[0]?.ip_address || "-";
    const status =
      droplet.status === "active" ? "🟢 Active" : "🔴 " + droplet.status;

    let txt = `📋 *ᴅᴇᴛᴀɪʟ ᴠᴘs*\n\n`;
    txt += `╭─「 🖥️ *ɪɴꜰᴏ* 」\n`;
    txt += `┃ 🆔 \`ɪᴅ\`: *${droplet.id}*\n`;
    txt += `┃ 🏷️ \`ɴᴀᴍᴇ\`: *${droplet.name}*\n`;
    txt += `┃ 📊 \`sᴛᴀᴛᴜs\`: *${status}*\n`;
    txt += `┃ 🌐 \`ɪᴘᴠ4\`: *${ip}*\n`;
    txt += `┃ 🌍 \`ɪᴘᴠ6\`: *${ipv6}*\n`;
    txt += `╰───────────────\n\n`;
    txt += `╭─「 🧠 *sᴘᴇᴄ* 」\n`;
    txt += `┃ 💾 \`ʀᴀᴍ\`: *${droplet.memory} MB*\n`;
    txt += `┃ ⚡ \`ᴄᴘᴜ\`: *${droplet.vcpus} vCPU*\n`;
    txt += `┃ 💿 \`ᴅɪsᴋ\`: *${droplet.disk} GB*\n`;
    txt += `┃ 🌏 \`ʀᴇɢɪᴏɴ\`: *${droplet.region?.name || droplet.region?.slug}*\n`;
    txt += `┃ 💻 \`ᴏs\`: *${droplet.image?.distribution} ${droplet.image?.name}*\n`;
    txt += `╰───────────────\n\n`;
    txt += `> 📅 Created: ${timeHelper.fromTimestamp(droplet.created_at, "DD MMMM YYYY HH:mm:ss")}`;

    await m.reply(txt);
  } catch (err) {
    return m.reply(te(m.prefix, m.command, m.pushName))
  }
}

export { pluginConfig as config, handler }