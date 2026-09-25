import { games } from '../../src/lib/games.js'

games.register('tekateki', {
    alias: ['teka'],
    emoji: '🧩',
    title: 'TEKA-TEKI',
    description: 'Game teka-teki tradisional'
})

const { config: pluginConfig, handler, answerHandler } = games.createPlugin('tekateki')
export { pluginConfig as config, handler, answerHandler }
