import te from '../../src/lib/error.js'
import { missingEnvNameOf } from '../../src/lib/aggregator.js'
import { explainFailure, MissingApiKeyError } from '../../src/lib/stalker-fallback.js'
import { resolver } from '../../src/lib/resolve.js'

const pluginConfig = {
    name: 'videy',
    alias: ['vdl', 'videydownload', 'videydl'],
    category: 'download',
    description: 'Download video dari videy.co',
    usage: '.videy <url>',
    example: '.videy https://videy.co/v?id=7ZH1ZRIF',
    isOwner: false,
    isPremium: false,
    isGroup: false,
    isPrivate: false,
    cooldown: 10,
    energi: 1,
    isEnabled: true
}

async function handler(m, { sock }) {
    const url = m.text?.trim()

    if (!url) {
        return m.reply(
            `🎬 *ᴠɪᴅᴇʏ ᴅᴏᴡɴʟᴏᴀᴅ*\n\n` +
            `> Masukkan URL videy.co\n\n` +
            `\`Contoh: ${m.prefix}videy https://videy.co/v?id=7ZH1ZRIF\``
        )
    }

    // Guard ini pemeriksaan substring, jadi `videy.co.evil.example` lolos di sini.
    // Ia sengaja dipertahankan sebagai lapis pertama — lapisan kedua, guard host
    // di kapabilitas, yang menolak itu per label host sebelum ada request CDN.
    if (!url.match(/videy\.co/i)) {
        return m.reply(`❌ URL tidak valid. Gunakan link dari videy.co`)
    }

    m.react('🕕')

    try {
        // Scraper lokal lebih dulu, aggregator jadi cadangan: key aggregator tidak
        // lagi diperiksa di plugin ini karena jalur lokal tidak butuh key sama
        // sekali, dan key kosong membuat aggregator dilewati di dalam
        // `aggregator.hit` dengan pesan yang jujur. `normalize` menolak respons
        // tanpa URL video, jadi `url` di sini tidak mungkin kosong.
        const { data } = await resolver.resolve('videy', { url })

        if (!data?.url) {
            m.react('❌')
            return m.reply(`❌ Gagal mengambil video. Link tidak valid atau sudah expired.`)
        }

        await sock.sendMedia(m.chat, data.url, null, m, {
            type: 'video',
            contextInfo: {
                forwardingScore: 99,
                isForwarded: true
            }
        })

        m.react('✅')

    } catch (error) {
        // Pemeriksaan key aggregator dipindah ke sini, bukan lagi sebelum
        // resolve. Sebelumnya plugin berhenti sebelum mencoba apa pun kalau key
        // kosong — itu benar ketika aggregator adalah satu-satunya jalan. Sekarang
        // jalur lokal tidak butuh key, jadi menolak di depan akan mematikan
        // command yang sebenarnya bisa jalan. Key baru relevan kalau jalur lokal
        // juga gagal; `missingEnvNameOf` membaca nama env dari alasan kegagalan
        // aggregator, jadi plugin ini tidak perlu tahu host mana.
        const envName = missingEnvNameOf(error)
        if (envName) {
            m.react('❌')
            return m.reply(explainFailure(new MissingApiKeyError('aggregator', envName), `${m.prefix}videy`))
        }
        m.react('☢')
        m.reply(te(m.prefix, m.command, m.pushName))
    }
}

export { pluginConfig as config, handler }
