import { games } from '../../src/lib/games.js'

games.register('tebakdrakor', {
    alias: ['drakor', 'kdrama'],
    emoji: '🇰🇷',
    title: 'TEBAK DRAKOR',
    description: 'Tebak judul drama Korea',
    hasImage: true
})

const { config: pluginConfig, handler, answerHandler } = games.createPlugin('tebakdrakor')
export { pluginConfig as config, handler, answerHandler }
