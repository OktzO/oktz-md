import axios from 'axios'
import config from '../../config.js'
import te from '../../src/lib/ourin-error.js'
const pluginConfig = {
    name: 'delpanel',
    alias: ['hapuspanel', 'deletepanel'],
    category: 'panel',
    description: 'Hapus panel (server + user)',
    usage: '.delpanel [s1/s2/s3] serverid [full]',
    example: '.delpanel 5 atau .delpanel s2 5 full',
    isOwner: true,
    isPremium: false,
    isGroup: false,
    isPrivate: false,
    cooldown: 10,
    energi: 0,
    isEnabled: true
}

function getServerConfig(pteroConfig, serverKey) {
    const serverConfigs = {
        's1': pteroConfig.server1,
        's2': pteroConfig.server2,
        's3': pteroConfig.server3
    }
    return serverConfigs[serverKey] || pteroConfig.server1
}

function validateServerConfig(serverConfig) {
    const missing = []
    if (!serverConfig?.domain) missing.push('domain')
    if (!serverConfig?.apikey) missing.push('apikey (PTLA)')
    return missing
}

function getAvailableServers(pteroConfig) {
    const available = []
    if (pteroConfig.server1?.domain && pteroConfig.server1?.apikey) available.push('s1')
    if (pteroConfig.server2?.domain && pteroConfig.server2?.apikey) available.push('s2')
    if (pteroConfig.server3?.domain && pteroConfig.server3?.apikey) available.push('s3')
    return available
}

async function handler(m, { sock }) {
    const pteroConfig = config.pterodactyl
    
    const args = m.text?.trim().split(' ') || []
    let serverKey = 's1'
    let restArgs = args
    
    if (args[0] && ['s1', 's2', 's3'].includes(args[0].toLowerCase())) {
        serverKey = args[0].toLowerCase()
        restArgs = args.slice(1)
    }
    
    const serverConfig = getServerConfig(pteroConfig, serverKey)
    const missingConfig = validateServerConfig(serverConfig)
    
    if (missingConfig.length > 0) {
        const available = getAvailableServers(pteroConfig)
        let txt = `⚠️ *sᴇʀᴠᴇʀ ${serverKey.toUpperCase()} ʙᴇʟᴜᴍ ᴋᴏɴꜰɪɢ*\n\n`
        if (available.length > 0) {
            txt += `> Server tersedia: *${available.join(', ')}*`
        }
        return m.reply(txt)
    }
    
    const serverId = restArgs[0]
    const option = restArgs[1]?.toLowerCase()
    const serverLabel = serverKey.toUpperCase()
    
    if (!serverId) {
        return m.reply(
            `⚠️ *ᴄᴀʀᴀ ᴘᴀᴋᴀɪ*\n\n` +
            `> \`${m.prefix}delpanel ID\` - Hapus server saja\n` +
            `> \`${m.prefix}delpanel ID full\` - Hapus server + user\n` +
            `> \`${m.prefix}delpanel s2 ID\` - Dari server 2\n\n` +
            `> Lihat ID dengan \`${m.prefix}listserver\``
        )
    }
    
    if (isNaN(serverId)) {
        return m.reply(`❌ Server ID harus berupa angka.`)
    }
    
    try {
        const serverRes = await axios.get(`${serverConfig.domain}/api/application/servers/${serverId}`, {
            headers: {
                'Authorization': `Bearer ${serverConfig.apikey}`,
                'Content-Type': 'application/json',
                'Accept': 'Application/vnd.pterodactyl.v1+json'
            }
        })
        
        const server = serverRes.data.attributes
        const userId = server.user
        
        let userInfo = null
        let isUserAdmin = false
        try {
            const userRes = await axios.get(`${serverConfig.domain}/api/application/users/${userId}`, {
                headers: { 'Authorization': `Bearer ${serverConfig.apikey}` }
            })
            userInfo = userRes.data.attributes
            isUserAdmin = userInfo.root_admin
        } catch (e) {}
        
        await m.reply(`🗑️ *ᴍᴇɴɢʜᴀᴘᴜs ᴘᴀɴᴇʟ...*\n\n> Server: *${serverLabel}*\n> Panel: \`${server.name}\`\n> Mode: *${option === 'full' ? 'Server + User' : 'Server saja'}*`)
        
        await axios.delete(`${serverConfig.domain}/api/application/servers/${serverId}`, {
            headers: {
                'Authorization': `Bearer ${serverConfig.apikey}`,
                'Content-Type': 'application/json',
                'Accept': 'Application/vnd.pterodactyl.v1+json'
            }
        })
        
        let result = `✅ *sᴇʀᴠᴇʀ ᴅɪʜᴀᴘᴜs [${serverLabel}]*\n\n`
        result += `> Nama: \`${server.name}\`\n`
        result += `> ID: \`${serverId}\`\n`
        
        if (option === 'full' && userInfo && !isUserAdmin) {
            try {
                await axios.delete(`${serverConfig.domain}/api/application/users/${userId}`, {
                    headers: {
                        'Authorization': `Bearer ${serverConfig.apikey}`,
                        'Content-Type': 'application/json',
                        'Accept': 'Application/vnd.pterodactyl.v1+json'
                    }
                })
                result += `\n✅ *ᴜsᴇʀ ᴅɪʜᴀᴘᴜs*\n`
                result += `> Username: \`${userInfo.username}\`\n`
                result += `> ID: \`${userId}\``
            } catch (err) {
                result += `\n⚠️ User gagal dihapus (mungkin masih punya server lain)`
            }
        } else if (option === 'full' && isUserAdmin) {
            result += `\n⚠️ User adalah Admin, tidak dihapus`
        }
        
        return m.reply(result)
        
    } catch (err) {
        return m.reply(te(m.prefix, m.command, m.pushName))
    }
}

export { pluginConfig as config, handler }