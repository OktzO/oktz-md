import QRCode from 'qrcode'
import sharp from 'sharp'
import te from '../../src/lib/error.js'
const pluginConfig = {
    name: ['qrcustom', 'qrcode', 'qr'],
    alias: [],
    category: 'tools',
    description: 'Generate QR code custom dengan logo',
    usage: '.qrcustom <url>',
    example: '.qrcustom https://wa.me/628xxx',
    isOwner: false,
    isPremium: false,
    isGroup: false,
    isPrivate: false,
    cooldown: 10,
    energi: 1,
    isEnabled: true
}

async function handler(m, { sock }) {
    const data = m.text?.trim()

    if (!data) {
        return m.reply(
            `⚠️ *ᴄᴀʀᴀ ᴘᴀᴋᴀɪ*\n\n` +
            `> \`${m.prefix}qrcustom <url/text>\`\n\n` +
            `*Contoh:*\n` +
            `> \`${m.prefix}qrcustom https://wa.me/628xxx\`\n\n` +
            `💡 Reply gambar untuk custom logo di tengah QR`
        )
    }

    await m.reply(`🕕 *Generating QR code...*`)

    try {
        let buffer = await QRCode.toBuffer(data, { type: 'png', width: 512, margin: 2, errorCorrectionLevel: 'H' })

        let logoBuffer = null
        if (m.isImage) {
            logoBuffer = await m.download()
        } else if (m.quoted?.isImage) {
            logoBuffer = await m.quoted.download()
        }

        if (logoBuffer) {
            const logo = await sharp(logoBuffer).resize(120, 120).png().toBuffer()
            buffer = await sharp(buffer)
                .composite([{ input: logo, gravity: 'centre' }])
                .png()
                .toBuffer()
        }

        await sock.sendMessage(m.chat, {
            image: buffer,
            caption: `📱 *QR Code*\n> ${data.substring(0, 50)}${data.length > 50 ? '...' : ''}`
        }, { quoted: m })

        m.react('📱')

    } catch (err) {
        m.react('☢')
        return m.reply(te(m.prefix, m.command, m.pushName))
    }
}

export { pluginConfig as config, handler }
