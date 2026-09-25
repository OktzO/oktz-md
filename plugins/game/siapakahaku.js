import { games } from '../../src/lib/games.js'

games.register('siapakahaku', {
    alias: ['siapa', 'whoami'],
    emoji: '🎭',
    title: 'SIAPAKAH AKU',
    description: 'Tebak dari deskripsi'
})

const { config: pluginConfig, handler, answerHandler } = games.createPlugin('siapakahaku')
export { pluginConfig as config, handler, answerHandler }
