import { games } from '../../src/lib/games.js'

games.register('tebakepep', {
    alias: ['tebakff', 'tebakfreefire'],
    emoji: '🔫',
    title: 'TEBAK EPEP',
    description: 'Tebak karakter Free Fire',
    hasImage: true
})

const { config: pluginConfig, handler, answerHandler } = games.createPlugin('tebakepep')
export { pluginConfig as config, handler, answerHandler }