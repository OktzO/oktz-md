import axios from 'axios';
import te from '../../src/lib/ourin-error.js';

const pluginConfig = {
    name: "iqc2",
    alias: ["qc3"],
    category: "canvas",
    description: "Membuat Fake Quote iOS style dengan informasi baterai dan provider.",
    usage: ".iqc2 [text/reply]",
    isOwner: false,
    isPremium: false,
    isGroup: false,
    isPrivate: false,
    cooldown: 5,
    energi: 2,
    isEnabled: true,
};

async function handler(m, { sock, text }) {
    try {
        const targetText = text || (m.quoted && m.quoted.text ? m.quoted.text : "");
        
        if (!targetText) {
            return m.reply(
                `Halo *${m.pushName}*, sepertinya kamu belum memasukkan teksnya.\n\n` +
                `Silakan gunakan perintah dengan format:\n` +
                `- .iqc2 <teks kamu>\n` +
                `- Atau balas (reply) pesan orang lain dengan .iqc2`
            );
        }

        await m.react('🕕');

        const providers = ["INDOSAT", "TELKOMSEL", "XL", "TRI", "SMARTFREN", "WIFI"];
        const randomProvider = providers[Math.floor(Math.random() * providers.length)];
        
        const now = new Date();
        const jam = now.toLocaleTimeString("id-ID", { timeZone: "Asia/Jakarta", hour: "2-digit", minute: "2-digit" }).replace('.', ':');
        
        const randomBaterai = Math.floor(Math.random() * (100 - 10 + 1)) + 10; // 10-100

        const apiUrl = `https://api.nexray.eu.cc/maker/v1/iqc?text=${encodeURIComponent(targetText)}&provider=${encodeURIComponent(randomProvider)}&jam=${encodeURIComponent(jam)}&baterai=${randomBaterai}`;
        
        await sock.sendMessage(m.chat, { image: { url: apiUrl } }, { quoted: m });
        
        await m.react('✅');

    } catch (error) {
        console.error("[IQC2 Plugin Error]", error);
        await m.react('❌');
        m.reply(`Maaf *${m.pushName}*, terjadi kesalahan saat mencoba membuat gambar quote. Silakan coba lagi beberapa saat.`);
    }
}

export { pluginConfig as config, handler };
