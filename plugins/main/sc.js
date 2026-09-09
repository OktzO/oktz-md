import { getAssetBuffer } from "../../src/lib/ourin-asset-manager.js";
import config from "../../config.js"
import {
  generateWAMessageFromContent,
} from "ourin";
import { wrapInteractive } from "../../src/lib/ourin-rich-messages.js";

const pluginConfig = {
    name: "sc",
    alias: ["script"],
    category: "main",
    description: "Link script bot wa terbaru",
    usage: ".sc",
    example: ".sc",
    isPremium: false,
    isOwner: false,
    isBanned: false,
    isAdmin: false,
    cooldown: 10,
    energi: 0,
    isBotAdmin: false,
    isEnabled: true
}

const REPO_URL = "https://github.com/OktzO/oktz-md";

async function handler(m, { sock }) {
    // Kartu tombol harus dikirim sebagai interactiveMessage + nativeFlowMessage
    // via relayMessage (pola yang terbukti dirender WhatsApp — key
    // `interactiveButtons` pada sendMessage TIDAK didukung lib ourin/onigis
    // dan di-drop/throw diam-diam).
    const msg = generateWAMessageFromContent(
        m.chat,
        wrapInteractive({
            messageContextInfo: {},
            interactiveMessage: {
                body: {
                    text: `🌾 Halo kak *${m.pushName}*

Script bot ini open source, kamu bisa dapatkan langsung di repo *Oktz-MD*

> 🤖 *Name*: ${config.bot?.name}
> ⚙️ *Version*: ${config.bot?.version}
> 👨‍💻 *Developer*: ${config.bot?.developer}
> 🧩 *Repo*: OktzO/oktz-md

Klik tombol di bawah untuk mendapatkan scriptnya ya!`
                },
                footer: { text: `${config.bot?.name} — Oktz-MD` },
                nativeFlowMessage: {
                    buttons: [
                        {
                            name: "cta_url",
                            buttonParamsJson: JSON.stringify({
                                display_text: "🥐 Kunjungi GitHub OktzO",
                                url: REPO_URL,
                                merchant_url: REPO_URL
                            })
                        }
                    ]
                }
            }
        }),
        { quoted: m, userJid: sock.user?.id },
    )
    return await sock.relayMessage(m.chat, msg.message, { messageId: msg.key.id })
}

export { pluginConfig as config, handler }
