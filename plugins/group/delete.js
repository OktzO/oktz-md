import { isSameGroupMember } from '../../src/lib/ourin-lid.js'
const pluginConfig = {
    name: 'delete',
    alias: ['del', 'hapus', 'd'],
    category: 'group',
    description: 'Hapus pesan dengan reply',
    usage: '.delete (reply pesan)',
    example: '.delete',
    isOwner: false,
    isPremium: false,
    isGroup: true,
    isPrivate: false,
    isAdmin: true,
    isBotAdmin: false,
    cooldown: 3,
    energi: 0,
    isEnabled: true
}

async function handler(m, { sock }) {
    if (!m.quoted) {
        return m.reply('⚠️ *Reply pesan yang ingin dihapus!*')
    }

    const quotedSender = m.quoted.sender || m.quoted.key?.participant
    const botJids = [sock.user?.id?.split(':')[0] + '@s.whatsapp.net', sock.user?.lid].filter(Boolean)
    // peserta yang sama bisa tertulis LID di satu sisi, PN di sisi lain
    const isOwnMessage = m.quoted.key?.fromMe || isSameGroupMember(m.groupMembers, quotedSender, m.sender)
    const isBotMessage = m.quoted.key?.fromMe || botJids.includes(quotedSender) || botJids.includes(m.quoted.key?.rawParticipant)

    if (!isOwnMessage && !isBotMessage) {
        if (!m.isBotAdmin) {
            return m.reply('⚠️ *Bot harus jadi admin untuk hapus pesan orang lain!*')
        }
        if (!m.isAdmin && !m.isOwner) {
            return m.reply('⚠️ *Hanya admin yang bisa hapus pesan orang lain!*')
        }
    }

    try {
        const key = {
            remoteJid: m.chat,
            id: m.quoted.key.id,
            fromMe: m.quoted.key.fromMe,
            // rawParticipant = addressing asli dari server. Di grup
            // addressing_mode=lid, pesan disimpan dengan participant LID;
            // pakai nomor PN hasil resolve bikin revoke tidak dicocokkan
            // server (pesan tampak "tidak terhapus" tanpa error).
            participant: m.quoted.key.rawParticipant || quotedSender
        }

        await sock.sendMessage(m.chat, { delete: key })
        await m.react('✅')

    } catch (err) {
        console.error('[Delete Error]', err)
        if (err.message?.includes('not found') || err.message?.includes('forbidden')) {
            await m.reply('❌ *Gagal menghapus!*\n> Pesan mungkin sudah dihapus atau terlalu lama.')
        } else {
            await m.react('❌')
        }
    }
}

export { pluginConfig as config, handler }