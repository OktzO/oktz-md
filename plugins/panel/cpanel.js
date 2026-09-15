import config from '../../config.js'
import { getRoles, getUserRole, getAccessibleServers, VALID_SERVERS } from '../../src/lib/ourin-roles-cpanel.js'
const pluginConfig = {
    name: 'cpanel',
    alias: ['panelmenu', 'menupanel'],
    category: 'panel',
    description: 'Menu panel pterodactyl (v1-v5)',
    usage: '.cpanel',
    example: '.cpanel',
    isOwner: false,
    isPremium: false,
    isGroup: false,
    isPrivate: false,
    cooldown: 5,
    energi: 0,
    isEnabled: true
}

async function handler(m, { sock }) {
    const pteroConfig = config.pterodactyl
    const prefix = m.prefix || '.'
    
    const getStatus = (cfg) => (cfg?.domain && cfg?.apikey) ? '✅' : '❌'
    
    const serverStatuses = {
        v1: getStatus(pteroConfig?.server1),
        v2: getStatus(pteroConfig?.server2),
        v3: getStatus(pteroConfig?.server3),
        v4: getStatus(pteroConfig?.server4),
        v5: getStatus(pteroConfig?.server5)
    }
    
    const userServers = getAccessibleServers(m.sender)
    const userRoleList = userServers.map(s => `${s.server.toUpperCase()}:${s.role}`).join(', ') || 'Tidak ada'
    
    let txt = `🖥️ *ᴄᴘᴀɴᴇʟ ᴍᴇɴᴜ v2.0*\n\n`
    txt += `> V1: ${serverStatuses.v1} | V2: ${serverStatuses.v2} | V3: ${serverStatuses.v3} | V4: ${serverStatuses.v4} | V5: ${serverStatuses.v5}\n`
    txt += `> Role kamu: *${m.isOwner ? 'Bot Owner' : userRoleList}*\n\n`
    
    txt += `╭─「 📦 *ᴄʀᴇᴀᴛᴇ sᴇʀᴠᴇʀ* 」\n`
    for (const ver of VALID_SERVERS) {
        txt += `┃ \`${prefix}1gb${ver}\` - \`${prefix}10gb${ver}\` | \`${prefix}unli${ver}\`\n`
    }
    txt += `╰───────────────\n\n`
    
    txt += `╭─「 👑 *ᴏᴡɴᴇʀ ᴍᴀɴᴀɢᴇᴍᴇɴᴛ* 」\n`
    for (const ver of VALID_SERVERS) {
        txt += `┃ \`${prefix}addowner${ver}\` | \`${prefix}delowner${ver}\` | \`${prefix}listowner${ver}\`\n`
    }
    txt += `╰───────────────\n\n`
    
    txt += `╭─「 🎯 *ᴄᴇᴏ ᴍᴀɴᴀɢᴇᴍᴇɴᴛ* 」\n`
    for (const ver of VALID_SERVERS) {
        txt += `┃ \`${prefix}addceo${ver}\` | \`${prefix}delceo${ver}\` | \`${prefix}listceo${ver}\`\n`
    }
    txt += `╰───────────────\n\n`
    
    txt += `╭─「 👥 *ʀᴇsᴇʟʟᴇʀ ᴍᴀɴᴀɢᴇᴍᴇɴᴛ* 」\n`
    for (const ver of VALID_SERVERS) {
        txt += `┃ \`${prefix}addreseller${ver}\` | \`${prefix}delreseller${ver}\` | \`${prefix}listreseller${ver}\`\n`
    }
    txt += `╰───────────────\n\n`
    
    txt += `╭─「 🔐 *ᴀᴅᴍɪɴ ᴘᴀɴᴇʟ* 」\n`
    for (const ver of VALID_SERVERS) {
        txt += `┃ \`${prefix}cadmin${ver}\` | \`${prefix}deladmin${ver}\` | \`${prefix}listadmin${ver}\`\n`
    }
    txt += `╰───────────────\n\n`
    
    txt += `╭─「 🖥️ *sᴇʀᴠᴇʀ ᴍᴀɴᴀɢᴇᴍᴇɴᴛ* 」\n`
    for (const ver of VALID_SERVERS) {
        txt += `┃ \`${prefix}listserver${ver}\` | \`${prefix}delserver${ver}\` | \`${prefix}serverinfo${ver}\`\n`
    }
    txt += `╰───────────────\n\n`
    
    txt += `╭─「 👤 *ᴜsᴇʀ ᴍᴀɴᴀɢᴇᴍᴇɴᴛ* 」\n`
    for (const ver of VALID_SERVERS) {
        txt += `┃ \`${prefix}listuser${ver}\`\n`
    }
    txt += `╰───────────────\n\n`
    
    txt += `╭─「 🏪 *ɢᴄ sᴇʟʟᴇʀ ᴘᴀɴᴇʟ* 」\n`
    for (const ver of VALID_SERVERS) {
        txt += `┃ \`${prefix}addgcseller${ver}\` | \`${prefix}resetgcseller${ver}\`\n`
    }
    txt += `╰───────────────\n\n`
    
    const doConfig = config.digitalocean || {}
    const doHasToken = doConfig.token ? '✅' : '❌'
    
    txt += `╭─「 🌊 *ᴅɪɢɪᴛᴀʟᴏᴄᴇᴀɴ ᴠᴘs* 」\n`
    txt += `┃ Status: ${doHasToken} Token\n`
    txt += `┃\n`
    txt += `┃ 📦 *ᴄʀᴇᴀᴛᴇ ᴠᴘs:*\n`
    txt += `┃ \`${prefix}vps1g1c\` - 1GB/1CPU\n`
    txt += `┃ \`${prefix}vps2g1c\` - 2GB/1CPU\n`
    txt += `┃ \`${prefix}vps4g2c\` - 4GB/2CPU\n`
    txt += `┃ \`${prefix}vps8g4c\` - 8GB/4CPU\n`
    txt += `┃\n`
    txt += `┃ 🔧 *ᴍᴀɴᴀɢᴇ:*\n`
    txt += `┃ \`${prefix}listvps\` | \`${prefix}cekvps\` | \`${prefix}delvps\` | \`${prefix}sisavps\`\n`
    txt += `┃\n`
    txt += `┃ ⚡ *ᴋᴏɴᴛʀᴏʟ:*\n`
    txt += `┃ \`${prefix}turnon\` | \`${prefix}turnoff\` | \`${prefix}restartvps\`\n`
    txt += `╰───────────────\n\n`
    
    txt += `> _Powered by ${config.info?.website || 'Panel'}_`
    
    await m.reply(txt)
}

export { pluginConfig as config, handler }