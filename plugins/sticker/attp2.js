import { createCanvas } from '@napi-rs/canvas'
import { addExifToWebp } from '../../src/lib/ourin-exif.js'
import config from '../../config.js'
import te from '../../src/lib/ourin-error.js'

const pluginConfig = {
  name: 'attp2',
  alias: ['attp2'],
  category: 'sticker',
  description: 'Buat sticker animasi teks',
  usage: '.attp2 <teks>',
  example: '.attp2 Halo dunia',
  cooldown: 5,
  energi: 2,
}

async function localAnimatedText(text) {
    const frames = 8
    const colors = ['#FF0000', '#00FF00', '#0000FF', '#FFFF00', '#00FFFF', '#FF00FF']
    const canvas = createCanvas(512, 512)
    const ctx = canvas.getContext('2d')
    const buffers = []

    for (let i = 0; i < frames; i++) {
        ctx.clearRect(0, 0, 512, 512)
        ctx.fillStyle = '#000000'
        ctx.fillRect(0, 0, 512, 512)

        let fontSize = 50
        ctx.font = `bold ${fontSize}px sans-serif`
        let words = text.split(' ')
        let lines = []
        let currentLine = ''

        for (const word of words) {
            const test = currentLine ? currentLine + ' ' + word : word
            if (ctx.measureText(test).width > 460) {
                if (currentLine) lines.push(currentLine)
                currentLine = word
            } else {
                currentLine = test
            }
        }
        if (currentLine) lines.push(currentLine)

        const offsetY = Math.sin(i / frames * Math.PI * 2) * 10
        const lineHeight = fontSize * 1.3
        const totalH = lines.length * lineHeight
        let startY = (512 - totalH) / 2 + fontSize + offsetY

        ctx.textAlign = 'center'
        ctx.textBaseline = 'alphabetic'

        for (const line of lines) {
            ctx.font = `bold ${fontSize}px sans-serif`
            ctx.fillStyle = colors[i % colors.length]
            ctx.fillText(line, 256, startY)
            startY += lineHeight
        }

        buffers.push(await canvas.encode('png'))
    }

    return buffers
}

async function handler(m, { sock }) {
  const text = m.text?.trim() || m.quoted?.text?.trim()

  if (!text) {
    return m.reply(`⚠️ Harap masukkan teks!\nContoh: \`${m.prefix}${m.command} Halo semua\``)
  }

  await m.react('🕕')

  try {
    const colors = encodeURIComponent(JSON.stringify(["#FF0000", "#00FF00", "#0000FF", "#FFFF00", "#00FFFF", "#FF00FF"]))
    const apiUrl = `https://api.neoxr.eu/api/attp2?text=${encodeURIComponent(text)}&color=${colors}&apikey=${config.APIkey.neoxr}`

    let sent = false
    try {
        const response = await axios.get(apiUrl)
        if (response.data.status && response.data.data?.url) {
            const stickerUrl = response.data.data.url
            const stickerBuffer = await axios.get(stickerUrl, { responseType: 'arraybuffer' }).then(res => res.data)
            await sock.sendVideoAsSticker(m.chat, stickerBuffer, m, {
                packname: config.sticker.packname,
                author: config.sticker.author
            })
            sent = true
        }
    } catch (e) { }

    if (!sent) {
        const frames = await localAnimatedText(text)
        for (const buf of frames) {
            try {
                const sticker = await addExifToWebp(buf, {
                    packname: config.sticker.packname,
                    author: config.sticker.author
                })
                await sock.sendMessage(m.chat, { sticker }, { quoted: m })
                await new Promise(r => setTimeout(r, 300))
            } catch (e) { }
        }
    }

    await m.react('✅')
  } catch (error) {
    console.log(error)
    await m.react('☢')
    m.reply(te(m.prefix, m.command, m.pushName))
  }
}

export { pluginConfig as config, handler }
