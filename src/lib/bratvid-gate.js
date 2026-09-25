// RAM gate untuk command yang lazy-import modul gemuk (brat-canvas/video, +200MB RSS).
// Cek RSS sebelum import: kalau sudah lewat limit, tolak render — jangan import.
const BRATVID_RSS_LIMIT = 480 * 1024 * 1024

function bratvidRssBlocked(getRss = () => process.memoryUsage().rss) {
    if (getRss() <= BRATVID_RSS_LIMIT) return null
    return '🎬 *ʙʀᴀᴛ ᴀɴɪᴍᴀᴛᴇᴅ*\n\n> Bot lagi low RAM, coba lagi nanti ya 🥺'
}

export { BRATVID_RSS_LIMIT, bratvidRssBlocked }