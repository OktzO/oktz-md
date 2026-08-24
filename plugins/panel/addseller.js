import config from '../../config.js'
import fs from 'fs'
import path from 'path'
import { isLid, lidToJid } from '../../src/lib/ourin-lid.js'
import { getDatabase } from '../../src/lib/ourin-database.js'
import { getGroupMode } from '../group/botmode.js'
const pluginConfig = {
    name: 'addseller',
    alias: ['addreseller', 'delseller', 'delreseller', 'listseller', 'listreseller'],
    category: 'panel',
    description: 'Kelola seller/reseller panel',
    usage: '.addseller @user atau .delseller @user',
    example: '.addseller @user',
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

function getLegacyData() {
    const p = path.join(process.cwd(), 'database', 'cpanel', 'legacy_sellers.json')
    if (fs.existsSync(p)) {
        try { return JSON.parse(fs.readFileSync(p, 'utf8')) } catch { return { sellers: [], ownerPanels: [] } }
    }
    return { sellers: [], ownerPanels: [] }
}

function saveLegacyData(data) {
    try {
        const dir = path.join(process.cwd(), 'database', 'cpanel')
        if (!fs.existsSync(dir)) fs.mkdirSync(dir, { recursive: true })
        const p = path.join(dir, 'legacy_sellers.json')
        fs.writeFileSync(p, JSON.stringify(data, null, 2), 'utf8')
        return true
    } catch (e) {
        console.error('[Panel] Failed to save legacy data:', e.message)
        return false
    }
}

function hasAccess(senderJid, isOwner, legacyData) {
    if (isOwner) return true
    const cleanSender = cleanJid(senderJid)?.split('@')[0]
    if (!cleanSender) return false
    const ownerPanels = legacyData?.ownerPanels || []
    return ownerPanels.includes(cleanSender)
}

function handler(m, { sock }) {
    const db = getDatabase()
    const cmd = m.command.toLowerCase()
    
    let legacyData = getLegacyData()
    if (config.pterodactyl) {
        config.pterodactyl.sellers = legacyData.sellers
        config.pterodactyl.ownerPanels = legacyData.ownerPanels
    }
    
    if (!hasAccess(m.sender, m.isOwner, legacyData)) {
        return m.reply(`❌ *ᴀᴋsᴇs ᴅɪᴛᴏʟᴀᴋ*\n\n> Fitur ini hanya untuk Owner atau Owner Panel.`)
    }
    
    if (!legacyData.sellers) {
        legacyData.sellers = []
    }
    
    const isAdd = ['addseller', 'addreseller'].includes(cmd)
    const isDel = ['delseller', 'delreseller'].includes(cmd)
    const isList = ['listseller', 'listreseller'].includes(cmd)
    
    if (isList) {
        if (legacyData.sellers.length === 0) {
            return m.reply(`📋 *ᴅᴀꜰᴛᴀʀ sᴇʟʟᴇʀ/ʀᴇsᴇʟʟᴇʀ*\n\n> Belum ada seller terdaftar.`)
        }
        
        let txt = `📋 *ᴅᴀꜰᴛᴀʀ sᴇʟʟᴇʀ/ʀᴇsᴇʟʟᴇʀ*\n\n`
        txt += `> Total: *${legacyData.sellers.length}* seller\n\n`
        legacyData.sellers.forEach((s, i) => {
            txt += `${i + 1}. \`${s}\`\n`
        })
        txt += `\n> _Seller bisa create server (1gb-10gb v1/v2/v3)_`
        return m.reply(txt)
    }
    
    let targetUser = null
    if (m.quoted?.sender) {
        targetUser = getNumber(m.quoted.sender)
    } else if (m.mentionedJid?.length > 0) {
        targetUser = getNumber(m.mentionedJid[0])
    } else if (m.text?.trim()) {
        targetUser = m.text.trim().replace(/[^0-9]/g, '')
    } else {
        targetUser = getNumber(m.sender)
    }
    
    if (!targetUser) {
        return m.reply(
            `⚠️ *ᴄᴀʀᴀ ᴘᴀᴋᴀɪ*\n\n` +
            `> \`${m.prefix}${cmd} @user\`\n` +
            `> \`${m.prefix}${cmd} 628xxx\`\n` +
            `> Reply pesan user`
        )
    }
    
    if (isAdd) {
        if (legacyData.sellers.includes(targetUser)) {
            return m.reply(`❌ \`${targetUser}\` sudah menjadi seller.`)
        }
        
        let roleChanged = ''
        const ownerIdx = (legacyData.ownerPanels || []).indexOf(targetUser)
        if (ownerIdx !== -1) {
            legacyData.ownerPanels.splice(ownerIdx, 1)
            roleChanged = `\n> ⚡ Auto-downgrade dari Owner Panel ke Seller`
        }
        
        legacyData.sellers.push(targetUser)
        
        if (saveLegacyData(legacyData)) {
            m.react('✅')
            return m.reply(
                `✅ *sᴇʟʟᴇʀ ᴅɪᴛᴀᴍʙᴀʜᴋᴀɴ*\n\n` +
                `╭┈┈⬡「 📋 *ᴅᴇᴛᴀɪʟ* 」\n` +
                `┃ 📱 ɴᴏᴍᴏʀ: \`${targetUser}\`\n` +
                `┃ 🏷️ sᴛᴀᴛᴜs: \`Seller/Reseller\`\n` +
                `┃ 🔓 ᴀᴋsᴇs: \`Create Server (1gb-10gb v1-v3)\`\n` +
                `┃ 📊 ᴛᴏᴛᴀʟ: \`${legacyData.sellers.length}\` seller\n` +
                `╰┈┈⬡${roleChanged}`
            )
        } else {
            return m.reply(`❌ Gagal menyimpan ke database`)
        }
    }
    
    if (isDel) {
        if (!legacyData.sellers.includes(targetUser)) {
            return m.reply(`❌ \`${targetUser}\` bukan seller.`)
        }
        
        legacyData.sellers = legacyData.sellers.filter(s => s !== targetUser)
        
        if (saveLegacyData(legacyData)) {
            m.react('✅')
            return m.reply(
                `✅ *sᴇʟʟᴇʀ ᴅɪʜᴀᴘᴜs*\n\n` +
                `> Nomor: \`${targetUser}\`\n` +
                `> Total: *${legacyData.sellers.length}* seller`
            )
        } else {
            return m.reply(`❌ Gagal menyimpan ke database`)
        }
    }
}

export { pluginConfig as config, handler }