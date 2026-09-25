import { games } from '../../src/lib/games.js'

games.register('asahotak', {
    alias: ['asah', 'quiz'],
    emoji: '🧠',
    title: 'ASAH OTAK',
    description: 'Game asah otak - tebak jawaban'
})

const { config: pluginConfig, handler, answerHandler } = games.createPlugin('asahotak')
export { pluginConfig as config, handler, answerHandler }
