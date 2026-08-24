import axios from "axios";
import te from "../../src/lib/ourin-error.js";

const pluginConfig = {
    name: "paptt",
    alias: ["paptt"],
    category: "nsfw",
    description: "Random paptt NSFW",
    usage: ".paptt",
    isOwner: false,
    isPremium: true,
    isGroup: false,
    isPrivate: false,
    cooldown: 5,
    energi: 2,
    isEnabled: true,
};

async function handler(m, { sock }) {
    await m.react("🕕");
    try {
        const url = `https://kyzznekoo.zone.id/api/asupan/paptt`;
        const res = await axios.get(url, {
            responseType: 'arraybuffer',
            headers: {
                "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/150.0.0.0 Safari/537.36",
                "Content-Type": "application/json"
            }
        });
        await sock.sendMessage(m.chat, { image: Buffer.from(res.data), caption: "🔞 *PAPTT*" }, { quoted: m });
        await m.react("✅");
    } catch (e) {
        console.error("[PAPTT Error]", e);
        await m.react("❌");
        m.reply(te(m.prefix, m.command, m.pushName));
    }
}

export { pluginConfig as config, handler };
