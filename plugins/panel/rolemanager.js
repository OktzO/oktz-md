import { isLid, lidToJid } from '../../src/lib/lid.js'
import { addRole, removeRole, listByRole, canManageRole, getUserRole, VALID_SERVERS } from '../../src/lib/roles-cpanel.js'
const ROLES = ['owner', 'ceo', 'reseller']
const allCommands = []

ROLES.forEach(role => {
    VALID_SERVERS.forEach(ver => {
        allCommands.push(`add${role}${ver}`)
        allCommands.push(`del${role}${ver}`)
        allCommands.push(`list${role}${ver}`)
    })
})

const pluginConfig = {
    name: allCommands,
    alias: [],
    category: 'panel',
    description: 'Kelola owner/ceo/reseller per server',
    usage: '.addownerv1 @user atau .listceov2',
    example: '.addresellerv1 @user',
    isOwner: false,
    isPremium: false,
    isGroup: false,
    isPrivate: false,
    cooldown: 5,
    energi: 0,
    isEnabled: true
}

function cleanJid(jid) {
    if (!jid) return null
    if (isLid(jid)) jid = lidToJid(jid)
    return jid.includes('@') ? jid : jid + '@s.whatsapp.net'
}

function getNumber(jid) {
    const clean = cleanJid(jid)
    return clean ? clean.split('@')[0] : null
}

function parseCommand(cmd) {
    const match = cmd.match(/^(add|del|list)(owner|ceo|reseller)(v[1-5])$/i)
    if (!match) return null
    return {
        action: match[1].toLowerCase(),
        role: match[2].toLowerCase(),
        server: match[3].toLowerCase()
    }
}

function capitalize(str) {
    return str.charAt(0).toUpperCase() + str.slice(1)
}

function handler(m, { sock }) {
    const parsed = parseCommand(m.command)
    if (!parsed) {
        return m.reply(`❌ Command tidak valid.`)
    }
    
    const { action, role, server } = parsed
    const serverLabel = server.toUpperCase()
    const roleLabel = capitalize(role)
    
    if (action === 'list') {
        const list = listByRole(server, role)
        if (list.length === 0) {
            return m.reply(`📋 *ᴅᴀꜰᴛᴀʀ ${roleLabel.toUpperCase()} ${serverLabel}*\n\n> Belum ada ${role} terdaftar.`)
        }
        
        let txt = `📋 *ᴅᴀꜰᴛᴀʀ ${roleLabel.toUpperCase()} ${serverLabel}*\n\n`
        txt += `> Total: *${list.length}* ${role}\n\n`
        list.forEach((num, i) => {
            txt += `${i + 1}. \`${num}\`\n`
        })
        txt += `\n> _Role: ${roleLabel} | Server: ${serverLabel}_`
        return m.reply(txt)
    }
    
    if (!canManageRole(m.sender, server, role, m.isOwner)) {
        const userRole = getUserRole(m.sender, server)
        return m.reply(
            `❌ *ᴀᴋsᴇs ᴅɪᴛᴏʟᴀᴋ*\n\n` +
            `> Kamu tidak bisa mengelola *${roleLabel}* di *${serverLabel}*\n` +
            `> Role kamu: *${userRole ? capitalize(userRole) : 'Tidak ada'}*\n\n` +
            `> Hirarki: Owner > CEO > Reseller`
        )
    }
    
    let targetUser = null
    if (m.quoted?.sender) {
        targetUser = getNumber(m.quoted.sender)
    } else if (m.mentionedJid?.length > 0) {
        targetUser = getNumber(m.mentionedJid[0])
    } else if (m.text?.trim()) {
        targetUser = m.text.trim().replace(/[^0-9]/g, '')
    }
    
    if (!targetUser) {
        return m.reply(
            `⚠️ *ᴄᴀʀᴀ ᴘᴀᴋᴀɪ*\n\n` +
            `> \`${m.prefix}${m.command} @user\`\n` +
            `> \`${m.prefix}${m.command} 628xxx\`\n` +
            `> Reply pesan user`
        )
    }
    
    if (action === 'add') {
        const result = addRole(targetUser, server, role)
        if (!result.success) {
            return m.reply(`❌ *ɢᴀɢᴀʟ*\n\n> ${result.error}`)
        }
        
        m.react('✅')
        return m.reply(
            `✅ *${roleLabel.toUpperCase()} ᴅɪᴛᴀᴍʙᴀʜᴋᴀɴ*\n\n` +
            `╭┈┈⬡「 📋 *ᴅᴇᴛᴀɪʟ* 」\n` +
            `┃ 📱 ɴᴏᴍᴏʀ: \`${targetUser}\`\n` +
            `┃ 🏷️ ʀᴏʟᴇ: \`${roleLabel}\`\n` +
            `┃ 🖥️ sᴇʀᴠᴇʀ: \`${serverLabel}\`\n` +
            `┃ 📊 ᴛᴏᴛᴀʟ: \`${listByRole(server, role).length}\` ${role}\n` +
            `╰┈┈⬡`
        )
    }
    
    if (action === 'del') {
        const result = removeRole(targetUser, server, role)
        if (!result.success) {
            return m.reply(`❌ *ɢᴀɢᴀʟ*\n\n> ${result.error}`)
        }
        
        m.react('✅')
        return m.reply(
            `✅ *${roleLabel.toUpperCase()} ᴅɪʜᴀᴘᴜs*\n\n` +
            `> Nomor: \`${targetUser}\`\n` +
            `> Server: *${serverLabel}*\n` +
            `> Total: *${listByRole(server, role).length}* ${role}`
        )
    }
}

export { pluginConfig as config, handler }