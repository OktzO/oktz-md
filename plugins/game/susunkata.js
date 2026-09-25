import { games } from '../../src/lib/games.js'

games.register('susunkata', {
    alias: ['susun', 'scramble'],
    emoji: '🔠',
    title: 'SUSUN KATA',
    description: 'Susun kata dari huruf'
})

const { config: pluginConfig, handler, answerHandler } = games.createPlugin('susunkata')
export { pluginConfig as config, handler, answerHandler }
