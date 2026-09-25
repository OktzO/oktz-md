import { games } from '../../src/lib/games.js'

games.register('tebakkalimat', {
    alias: ['tkl', 'peribahasa'],
    emoji: '📖',
    title: 'TEBAK KALIMAT',
    description: 'Tebak kalimat atau peribahasa'
})

const { config: pluginConfig, handler, answerHandler } = games.createPlugin('tebakkalimat')
export { pluginConfig as config, handler, answerHandler }
