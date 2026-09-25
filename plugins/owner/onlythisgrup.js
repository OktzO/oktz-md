import { getDatabase } from '../../src/lib/database.js'
import { findParticipantByNumber } from '../../src/lib/lid.js'
import te from '../../src/lib/error.js'

const pluginConfig = {
    name: 'onlythisgrup',
    alias: ['onlythisgroup', 'lockgrup', 'lockgroup'],
    category: 'owner',
    description: 'Bot hanya aktif di grup ini saja',
    usage: '.onlythisgrup',
    example: '.onlythisgrup',
    isOwner: true,
    isPremium: false,
    isGroup: true,
    isPrivate: false,
    cooldown: 5,
    energi: 0,
    isEnabled: true
}

async function handler(m, { sock }) {
    try {
        const db = getDatabase()
        const current = db.setting('onlyThisGroup') || null

        if (current && (current === m.chat || current.jid === m.chat)) {
            db.setting('onlyThisGroup', null)
            db.save()
            return m.reply(`🔓 *UNLOCKED*\n\nBot kembali aktif di semua grup secara publik.`)
        }

        const botNumber = sock.user.id.split(':')[0] + '@s.whatsapp.net'
        const groupMetadata = await sock.groupMetadata(m.chat).catch(() => null)
        
        if (!groupMetadata) {
            return m.reply(`❌ Gagal mendapatkan metadata grup.`)
        }

        const participants = groupMetadata.participants
        // p.id berisi LID di grup addressing_mode=lid, dan admin non-admin datang
        // sebagai null/undefined -> cek manual "?.admin !== null" selalu salah arah.
        const botParticipant = findParticipantByNumber(participants, botNumber) ||
            findParticipantByNumber(participants, sock.user?.lid)
        const isBotAdmin = !!botParticipant?.admin

        if (!isBotAdmin) {
            return m.reply(`❌ *AKSES DITOLAK*\n\nBot harus menjadi admin di grup ini terlebih dahulu agar bisa mengambil tautan undangan (link grup).`)
        }

        const inviteCode = await sock.groupInviteCode(m.chat).catch(() => null)
        
        if (!inviteCode) {
            return m.reply(`❌ Gagal mengambil tautan undangan grup. Pastikan bot adalah admin yang sah.`)
        }

        const inviteLink = `https://chat.whatsapp.com/${inviteCode}`
        const groupName = groupMetadata.subject

        db.setting('onlyThisGroup', {
            jid: m.chat,
            name: groupName,
            link: inviteLink
        })
        db.save()

        await m.reply(
            `🔒 *LOCKED BERHASIL*\n\n` +
            `Mulai sekarang, bot hanya bisa digunakan secara eksklusif di grup:\n` +
            `*${groupName}*\n\n` +
            `Pengguna di grup lain akan diarahkan untuk bergabung melalui tautan:\n` +
            `${inviteLink}\n\n` +
            `Ketik \`.onlythisgrup\` lagi untuk membuka kunci.`
        )
    } catch (error) {
        console.error(error)
        m.reply(te(m.prefix, m.command, m.pushName))
    }
}

export { pluginConfig as config, handler }