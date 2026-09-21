const pluginConfig = {
    name: 'cekkaya',
    alias: ['kaya', 'rich'],
    category: 'cek',
    description: 'Cek seberapa kaya kamu',
    usage: '.cekkaya <nama>',
    example: '.cekkaya Budi',
    isOwner: false,
    isPremium: false,
    isGroup: false,
    isPrivate: false,
    cooldown: 5,
    energi: 0,
    isEnabled: true
}

async function handler(m) {
        const percent = Math.floor(Math.random() * 101)
    const mentioned = m.mentionedJid[0] || m.sender
                    
    let desc
    if (percent >= 90) {
        desc = 'Sultan! Crazy rich! 💎'
    } else if (percent >= 70) {
        desc = 'Tajir melintir! 💰'
    } else if (percent >= 50) {
        desc = 'Lumayan berada 💵'
    } else if (percent >= 30) {
        desc = 'Cukup lah buat hidup 😊'
    } else {
        desc = 'Semangat nabung! 🙏'
    }
    
    let txt = mentioned === m.sender ? `Hai @${mentioned.split('@')[0]}
    
Tingkat kekayaan kamu *${percent}%*
\`\`\`${desc}\`\`\`` : `Kamu ingin ngecek tingkat kekayaan @${mentioned.split('@')[0]} yak? 
    
Tingkat kekayaan dia sebesar *${percent}%*
\`\`\`${desc}\`\`\``
    
    await m.reply(txt, { mentions: [mentioned] })
}

export { pluginConfig as config, handler }