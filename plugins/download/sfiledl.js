import te from '../../src/lib/error.js'
import { missingEnvNameOf } from '../../src/lib/aggregator.js'
import { explainFailure, MissingApiKeyError } from '../../src/lib/stalker-fallback.js'
import { resolver, adaNormalisasiGagal } from '../../src/lib/resolve.js'

const pluginConfig = {
    name: 'sfiledl',
    alias: ['sfile', 'sfiledownload'],
    category: 'download',
    description: 'Download file dari Sfile.mobi',
    usage: '.sfiledl <url>',
    example: '.sfiledl https://sfile.mobi/xxx',
    isOwner: false,
    isPremium: false,
    isGroup: false,
    isPrivate: false,
    cooldown: 15,
    energi: 1,
    isEnabled: true
}

/**
 * Host Sfile dicek per label, sama seperti yang dilakukan kapabilitas:
 * `sfile.mobi.evil.example` dan `notsfile.mobi` bukan Sfile dan tetap lolos
 * `includes`. Guard plugin ada supaya user dapat jawabannya sebelum kapabilitas
 * sempat mencatat kegagalan host — tiga link palsu dari satu user sudah cukup
 * membuka breaker sfile yang sehat selama 30 detik untuk semua orang.
 *
 * `sfile.co` ikut diterima: domain itu masih dipakai halaman unduhan sfile dan
 * plugin ini sudah menerimanya dari sebelum Phase 1.
 */
function hostSfile(url) {
    let parsed
    try {
        parsed = new URL(String(url).trim())
    } catch {
        return false
    }
    if (parsed.protocol !== 'http:' && parsed.protocol !== 'https:') return false
    const host = parsed.hostname.toLowerCase()
    return host === 'sfile.mobi' || host.endsWith('.sfile.mobi') || host === 'sfile.co' || host.endsWith('.sfile.co')
}

async function handler(m, { sock }) {
    const url = m.text?.trim()

    if (!url) {
        return m.reply(
            `⚠️ *ᴄᴀʀʀᴀ ᴘᴀᴋᴀɪ*\n\n` +
            `> \`${m.prefix}sfiledl <url_sfile>\`\n\n` +
            `> Contoh: \`${m.prefix}sfiledl https://sfile.mobi/xxxxx\``
        )
    }

    if (!hostSfile(url)) {
        return m.reply(`❌ URL harus dari sfile.mobi atau sfile.co!`)
    }

    m.react('🕕')

    try {
        // Scraper lokal lebih dulu, aggregator jadi cadangan: key aggregator tidak
        // lagi diperiksa di plugin ini karena scraper lokal tidak butuh key
        // sama sekali, dan key kosong membuat aggregator dilewati di dalam
        // `aggregator.hit` dengan pesan yang jujur.
        const { data } = await resolver.resolve('sfile', { url })

        // Pemeriksaan `!data?.url` yang dulu ada di sini sudah dihapus: `normalize`
        // menolak respons tanpa URL unduhan, jadi setelah resolve sukses `data.url`
        // tidak mungkin kosong dan cabang itu tidak pernah bisa bernilai true. Yang
        // dicakupnya kini ditangani di `catch` lewat `adaNormalisasiGagal`.

        await sock.sendMedia(m.chat, data.url, null, m, {
            type: 'document',
            fileName: data.filename,
            mimetype: data.mime,
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
        // scraper lokal tidak butuh key, jadi menolak di depan akan mematikan
        // command yang sebenarnya bisa jalan. Key baru relevan kalau scraper
        // lokal juga gagal; `missingEnvNameOf` membaca nama env dari alasan
        // kegagalan aggregator, jadi plugin ini tidak perlu tahu host mana.
        const envName = missingEnvNameOf(error);
        if (envName) {
            m.react('❌')
            return m.reply(explainFailure(new MissingApiKeyError('aggregator', envName), `${m.prefix}sfiledl`))
        }
        // Semua backend menjawab tapi tidak ada yang bisa dikirim: file-nya memang
        // sudah hilang dari sfile, atau tautannya bukan file sfile yang bisa diunduh.
        // Itu kesalahan link dari user dan ada jalan keluarnya — kirim link lain —
        // jadi jawabannya pesan link, bukan template error operator. Pemisahan
        // dilakukan dari `tried`, karena `normalize` menutup jalan "data.url kosong".
        if (adaNormalisasiGagal(error)) {
            m.react('❌')
            return m.reply(`❌ Gagal mendapatkan link download. File mungkin tidak tersedia.`)
        }
        m.react('☢')
        m.reply(te(m.prefix, m.command, m.pushName))
    }
}

export { pluginConfig as config, handler }