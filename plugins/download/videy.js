import te from '../../src/lib/error.js'
import { missingEnvNameOf } from '../../src/lib/aggregator.js'
import { explainFailure, MissingApiKeyError } from '../../src/lib/stalker-fallback.js'
import { resolver, adaNormalisasiGagal } from '../../src/lib/resolve.js'

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

/**
 * Host Videy dicek per label, sama seperti yang dilakukan kapabilitas dan
 * seperti guard plugin `sfiledl`/`douyindl` dari Task 5 dan Task 6:
 * `videy.co.evil.example` dan `notvidey.co` lolos `includes`/regex longgar
 * tapi bukan Videy. Guard plugin ada supaya user dapat jawabannya sebelum
 * kapabilitas sempat mencatat kegagalan host.
 *
 * Sub-domain ikut diterima karena halaman videy dilayani dari `www.videy.co`.
 */
function hostVidey(url) {
    let parsed
    try {
        parsed = new URL(String(url).trim())
    } catch {
        return false
    }
    if (parsed.protocol !== 'http:' && parsed.protocol !== 'https:') return false
    const host = parsed.hostname.toLowerCase()
    return host === 'videy.co' || host.endsWith('.videy.co')
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

    if (!hostVidey(url)) {
        return m.reply(`❌ URL tidak valid. Gunakan link dari videy.co`)
    }

    m.react('🕕')

    try {
        // Scraper lokal lebih dulu, aggregator jadi cadangan: key aggregator tidak
        // lagi diperiksa di plugin ini karena jalur lokal tidak butuh key sama
        // sekali, dan key kosong membuat aggregator dilewati di dalam
        // `aggregator.hit` dengan pesan yang jujur.
        const { data } = await resolver.resolve('videy', { url })

        // Pemeriksaan `!data?.url` yang dulu ada di sini sudah dihapus: `normalize`
        // menolak respons tanpa URL video, jadi setelah resolve sukses `data.url`
        // tidak mungkin kosong dan cabang itu tidak pernah bisa bernilai true. Yang
        // dicakupnya kini ditangani di `catch` lewat `adaNormalisasiGagal`.

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
        // Semua backend menjawab tapi tidak ada yang bisa dikirim: tautan berbagi videy
        // sudah dihapus, atau videy tidak punya video untuk id itu. CDN videy tidak
        // memberi tanda kedaluwarsa, jadi user perlu diberi tahu itu tautannya —
        // kesalahan yang bisa diperbaiki dengan link lain, bukan masalah proses.
        if (adaNormalisasiGagal(error)) {
            m.react('❌')
            return m.reply(`❌ Gagal mengambil video. Link tidak valid atau sudah expired.`)
        }
        m.react('☢')
        m.reply(te(m.prefix, m.command, m.pushName))
    }
}

export { pluginConfig as config, handler }
