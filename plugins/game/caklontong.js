import { games } from '../../src/lib/games.js'

games.register('caklontong', {
    alias: ['cak', 'lontong'],
    emoji: '🤔',
    title: 'CAK LONTONG',
    description: 'Game cak lontong - jawaban receh'
})

const { config: pluginConfig, handler, answerHandler } = games.createPlugin('caklontong')
export { pluginConfig as config, handler, answerHandler }
