import { games } from '../../src/lib/ourin-games.js'

games.register('tebakfilm', {
    alias: ['guessmovie'],
    emoji: '🎬',
    title: 'TEBAK FILM',
    description: 'Tebak judul film'
})

const { config: pluginConfig, handler, answerHandler } = games.createPlugin('tebakfilm')
export { pluginConfig as config, handler, answerHandler }
