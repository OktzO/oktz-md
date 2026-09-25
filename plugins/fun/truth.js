import { getRandomItem } from '../../src/lib/game-data.js'
const pluginConfig = {
    name: 'truth',
    alias: ['truthq'],
    category: 'fun',
    description: 'Random pertanyaan truth',
    usage: '.truth',
    example: '.truth',
    isOwner: false,
    isPremium: false,
    isGroup: false,
    isPrivate: false,
    cooldown: 3,
    energi: 0,
    isEnabled: true
};

async function handler(m) {
    const question = getRandomItem('truth.json');
    if (!question) {
        await m.reply('❌ Data tidak tersedia!');
        return;
    }
    await m.reply(`\`\`\`${question}\`\`\``);
}

export { pluginConfig as config, handler }