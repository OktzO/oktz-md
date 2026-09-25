import { games } from '../../src/lib/games.js'

games.register('tebakkimia', {
    alias: ['kimia', 'chemistry', 'unsur'],
    emoji: '🧪',
    title: 'TEBAK KIMIA',
    description: 'Tebak unsur kimia',
    questionField: 'unsur',
    answerField: 'lambang'
})

const { config: pluginConfig, handler, answerHandler } = games.createPlugin('tebakkimia')
export { pluginConfig as config, handler, answerHandler }
