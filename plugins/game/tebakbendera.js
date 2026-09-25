import { games } from '../../src/lib/games.js'

games.register('tebakbendera', {
    alias: ['tbendera', 'flag'],
    emoji: '🏳️',
    title: 'TEBAK BENDERA',
    description: 'Tebak negara dari bendera',
    dataFile: 'tebakbendera2.json',
    answerField: 'name',
    hasImage: true
})

const { config: pluginConfig, handler, answerHandler } = games.createPlugin('tebakbendera')
export { pluginConfig as config, handler, answerHandler }
