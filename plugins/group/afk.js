import { formatAfkDuration } from '../../src/lib/ourin-middleware.js'

const afkStorage = global.afkStorage || (global.afkStorage = new Map())
// ponytail: throttle pair bisa tumbuh selama sesi panjang; dibersihkan saat user balik/restart. Upgrade: TTL sweeper berkala kalau jumlah user besar.
const notifyThrottle = global.afkNotifyThrottle || (global.afkNotifyThrottle = new Map())

const pluginConfig = {
    name: 'afk',
    alias: ['away', 'brb'],
    category: 'group',
    description: 'Set status AFK dengan alasan',
    usage: '.afk <alasan>',
    example: '.afk lagi makan',
    isOwner: false,
    isPremium: false,
    isGroup: false,
    isPrivate: false,
    cooldown: 5,
    energi: 0,
    isEnabled: true
}

const AFK_CMDS = [pluginConfig.name, ...pluginConfig.alias]

function dbReady(db) {
    return typeof db?.getUser === 'function' && typeof db?.setUser === 'function'
}

function canPersist(jid) {
    const clean = jid.replace(/@.+/g, '')
    return clean.length <= 15 && !clean.startsWith('120')
}

function getAfkUser(jid, db) {
    if (dbReady(db) && canPersist(jid)) {
        const user = db.getUser(jid)
        if (user?.afk) return user.afk
    }
    return afkStorage.get(jid) || null
}

function setAfkUser(jid, reason, db) {
    const record = { reason: reason || 'Tidak ada alasan', since: Date.now() }
    if (dbReady(db) && canPersist(jid)) {
        db.setUser(jid, { afk: record })
    }
    afkStorage.set(jid, record)
}

function removeAfkUser(jid, db) {
    if (dbReady(db) && canPersist(jid)) {
        db.setUser(jid, { afk: null })
    }
    afkStorage.delete(jid)
    for (const key of notifyThrottle.keys()) {
        if (key.endsWith(`|${jid}`)) notifyThrottle.delete(key)
    }
}

function isUserAfk(jid, db) {
    return !!getAfkUser(jid, db)
}

async function handler(m, { sock }) {
    const reason = m.text || 'Tidak ada alasan'
    setAfkUser(m.sender, reason)
    await m.reply(
        `💤 *ᴀꜰᴋ ᴀᴋᴛɪꜰ*\n\n` +
        `\`\`\`@${m.sender.split('@')[0]} sekarang AFK\`\`\`\n` +
        `🍀 \`Alasan:\` *${reason}*\n\n` +
        `_Ketik apapun untuk menonaktifkan AFK._`,
        { mentions: [m.sender] }
    )
}

async function checkAfk(m, sock, db) {
    const afkData = getAfkUser(m.sender, db)
    if (afkData) {
        if (m.isCommand && AFK_CMDS.includes(m.command?.toLowerCase())) return
        removeAfkUser(m.sender, db)
        const duration = formatAfkDuration(Date.now() - afkData.since)
        await m.reply(`👋 *ᴀꜰᴋ ʙᴇʀᴀᴋʜɪʀ*\n\n` +
                `\`\`\`@${m.sender.split('@')[0]} sudah kembali!\`\`\`\n` +
                `🍀 \`Durasi AFK:\` *${duration}*`, { mentions: [m.sender] })
    }
    if (m.isGroup && m.mentionedJid && m.mentionedJid.length > 0) {
        const replies = []
        for (const mentioned of m.mentionedJid) {
            const mentionedAfk = getAfkUser(mentioned, db)
            if (!mentionedAfk) continue
            const throttleKey = `${m.sender}|${mentioned}`
            if (notifyThrottle.has(throttleKey)) continue
            notifyThrottle.set(throttleKey, Date.now())
            const duration = formatAfkDuration(Date.now() - mentionedAfk.since)
            replies.push(
                m.reply(`💤 *ᴜsᴇʀ ᴀꜰᴋ*\n\n` +
                    `\`\`\`Hustt, jangan di ganggu!\`\`\` \`@${mentioned.split('@')[0]}\` lagi AFK\n` +
                    `🍀 \`Alasan:\` *${mentionedAfk.reason}*\n` +
                    `🍀 \`Sejak:\` *${duration} yang lalu*`, { mentions: [mentioned] })
            )
        }
        await Promise.all(replies)
    }
}

export { pluginConfig as config, handler, checkAfk, getAfkUser, setAfkUser, removeAfkUser, isUserAfk }
