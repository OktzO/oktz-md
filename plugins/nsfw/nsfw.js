import fs from "fs"
import path from "path"
import axios from "axios"
import { getDatabase } from "../../src/lib/ourin-database.js"
import te from "../../src/lib/ourin-error.js"
import { prepareWAMessageMedia, generateWAMessageFromContent } from "ourin"

const NSFW_DATA_DIR = path.join(process.cwd(), "src", "data", "nsfw")

const jsonCache = new Map()

function loadJsonUrls(filename) {
  if (jsonCache.has(filename)) return jsonCache.get(filename)
  try {
    const filePath = path.join(NSFW_DATA_DIR, filename)
    if (!fs.existsSync(filePath)) return []
    const data = JSON.parse(fs.readFileSync(filePath, "utf-8"))
    if (Array.isArray(data) && data.length > 0) {
      jsonCache.set(filename, data)
      return data
    }
    return []
  } catch {
    return []
  }
}

function getRandomItem(arr) {
  return arr[Math.floor(Math.random() * arr.length)]
}

const JSON_CATEGORIES = {
  ass: { file: "ass.json", emoji: "🍑", label: "Ass Anime" },
  bdsm: { file: "bdsm.json", emoji: "⛓️", label: "BDSM Anime" },
  cum: { file: "cum.json", emoji: "💦", label: "Cum Anime" },
  gangbang: { file: "gangbang.json", emoji: "👥", label: "Gangbang Anime" },
  hentai: { file: "hentai.json", emoji: "🔞", label: "Hentai" },
  kasedaiki: { file: "kasedaiki.json", emoji: "🎎", label: "Kasedaiki" },
  manstrubation: { file: "manstrubation.json", emoji: "✋", label: "Manstrubation Anime" },
  opaianime: { file: "opaianime.json", emoji: "🍈", label: "Oppai Anime" },
}

const API_CATEGORIES = {
  blowjob: { endpoint: "blowjob", emoji: "👄", label: "Blowjob Anime" },
  neko: { endpoint: "neko", emoji: "🐱", label: "Neko NSFW" },
  trap: { endpoint: "trap", emoji: "🎭", label: "Trap Anime" },
  waifunsfw: { endpoint: "waifu", emoji: "💕", label: "Waifu NSFW" },
}

const ALL_COMMANDS = [
  ...Object.keys(JSON_CATEGORIES),
  ...Object.keys(API_CATEGORIES).filter((k) => k !== "neko"),
  "nsfw",
  "nsfwon",
  "nsfwoff",
  "nsfwmenu",
]

const pluginConfig = {
  name: ALL_COMMANDS,
  alias: ["oppai", "oppaianimee"],
  category: "nsfw",
  description: "Koleksi gambar NSFW anime dari berbagai kategori (Hanya untuk 18+)",
  usage: ".nsfw atau .<kategori>",
  example: ".nsfw hentai",
  isOwner: false,
  isPremium: false,
  isGroup: false,
  isPrivate: false,
  cooldown: 8,
  energi: 2,
  isEnabled: true,
}

async function fetchFromApi(endpoint) {
  const res = await axios.get(`https://api.waifu.pics/nsfw/${endpoint}`, { timeout: 15000 })
  if (!res.data?.url) throw new Error("Gagal mengambil gambar dari API")
  const imgRes = await axios.get(res.data.url, { responseType: "arraybuffer", timeout: 30000 })
  return Buffer.from(imgRes.data)
}

async function fetchFromJson(filename) {
  const urls = loadJsonUrls(filename)
  if (urls.length === 0) throw new Error("Data gambar kosong atau file tidak ditemukan")
  const url = getRandomItem(urls)
  const res = await axios.get(url, { responseType: "arraybuffer", timeout: 30000 })
  return Buffer.from(res.data)
}

function buildCategoryList(prefix) {
  let text = `🔞 *NSFW MENU*\n\n`
  text += `Kumpulan gambar anime NSFW dari berbagai kategori.\n`
  text += `Fitur ini hanya untuk pengguna berusia *18 tahun ke atas*.\n\n`
  text += `*📂 KATEGORI TERSEDIA:*\n\n`

  text += `*— Dari Database Lokal —*\n`
  for (const [cmd, info] of Object.entries(JSON_CATEGORIES)) {
    const count = loadJsonUrls(info.file).length
    text += `- ${info.emoji} *${prefix}${cmd}* — ${info.label} (${count} gambar)\n`
  }

  text += `\n*— Dari API Online —*\n`
  for (const [cmd, info] of Object.entries(API_CATEGORIES)) {
    text += `- ${info.emoji} *${prefix}${cmd}* — ${info.label}\n`
  }

  text += `\n*⚙️ PENGATURAN GRUP:*\n`
  text += `- *${prefix}nsfwon* — Mengaktifkan fitur NSFW di grup ini\n`
  text += `- *${prefix}nsfwoff* — Menonaktifkan fitur NSFW di grup ini\n`

  text += `\n*📌 CATATAN PENTING:*\n`
  text += `- Fitur ini bisa digunakan langsung di *chat pribadi* bot\n`
  text += `- Untuk grup, admin harus mengaktifkan dulu dengan *${prefix}nsfwon*\n`
  text += `- Gunakan dengan bijak dan bertanggung jawab\n`
  text += `- Konten ini hanya untuk pengguna *18+*`

  return text
}

function isNsfwAllowed(m, db) {
  if (!m.isGroup) return true
  const groupData = db.getGroup(m.chat) || {}
  return groupData.nsfw === true
}

function requirePremium(m) {
  if (m.isPremium || m.isOwner) return true
  return false
}

async function handler(m, { sock }) {
  const db = getDatabase()
  const cmd = m.command.toLowerCase()

  if (cmd === "nsfwon") {
    if (!m.isGroup) return m.reply(`❌ *Perintah ini hanya untuk grup*\n\nFitur NSFW bisa langsung digunakan di chat pribadi tanpa perlu diaktifkan.`)
    if (!m.isAdmin && !m.isOwner) return m.reply(`❌ *Akses Ditolak*\n\nHanya admin grup yang bisa mengaktifkan atau menonaktifkan fitur NSFW di grup ini.`)

    const groupData = db.getGroup(m.chat) || {}
    groupData.nsfw = true
    db.setGroup(m.chat, groupData)
    return m.reply(
      `✅ *NSFW DIAKTIFKAN*\n\n` +
      `Fitur NSFW telah berhasil diaktifkan untuk grup ini.\n\n` +
      `*Perhatian:*\n` +
      `- Pastikan semua anggota grup sudah berusia *18+*\n` +
      `- Admin bertanggung jawab atas konten yang muncul di grup\n` +
      `- Gunakan *${m.prefix}nsfwoff* untuk menonaktifkan kembali\n\n` +
      `Ketik *${m.prefix}nsfw* untuk melihat daftar kategori yang tersedia.`
    )
  }

  if (cmd === "nsfwoff") {
    if (!m.isGroup) return m.reply(`❌ *Perintah ini hanya untuk grup*\n\nDi chat pribadi, fitur NSFW selalu tersedia.`)
    if (!m.isAdmin && !m.isOwner) return m.reply(`❌ *Akses Ditolak*\n\nHanya admin grup yang bisa mengaktifkan atau menonaktifkan fitur NSFW di grup ini.`)

    const groupData = db.getGroup(m.chat) || {}
    groupData.nsfw = false
    db.setGroup(m.chat, groupData)
    return m.reply(
      `✅ *NSFW DINONAKTIFKAN*\n\n` +
      `Fitur NSFW telah berhasil dinonaktifkan untuk grup ini.\n` +
      `Semua perintah NSFW tidak akan bisa digunakan di sini sampai diaktifkan kembali.`
    )
  }

  if (!isNsfwAllowed(m, db)) {
    return m.reply(
      `🔒 *FITUR NSFW BELUM AKTIF*\n\n` +
      `Fitur NSFW belum diaktifkan untuk grup ini.\n` +
      `Minta admin grup untuk mengaktifkannya terlebih dahulu dengan perintah *${m.prefix}nsfwon*\n\n` +
      `Atau kamu bisa menggunakan fitur ini di *chat pribadi* bot secara langsung.`
    )
  }

  if (cmd === "nsfw" || cmd === "nsfwmenu") {
    const args = m.args
    if (args[0]) {
      const sub = args[0].toLowerCase()
      if (JSON_CATEGORIES[sub] || API_CATEGORIES[sub]) {
        if (!requirePremium(m)) {
          return m.reply(
            `🔞 *Konten Premium 18+*\n\n` +
            `Konten NSFW hanya bisa diakses pengguna *Premium*.\n` +
            `> Ketik *${m.prefix}benefitpremium* untuk info upgrade`
          )
        }
        return await sendNsfwImage(m, sock, sub)
      }
    }
    return m.reply(buildCategoryList(m.prefix))
  }

  if (JSON_CATEGORIES[cmd] || API_CATEGORIES[cmd]) {
    if (!requirePremium(m)) {
      return m.reply(
        `🔞 *Konten Premium 18+*\n\n` +
        `Konten NSFW hanya bisa diakses pengguna *Premium*.\n` +
        `> Ketik *${m.prefix}benefitpremium* untuk info upgrade`
      )
    }
    return await sendNsfwImage(m, sock, cmd)
  }

  return m.reply(buildCategoryList(m.prefix))
}

async function sendNsfwImage(m, sock, category) {
  await m.react("🕕")

  try {
    let buffer
    let info

    if (JSON_CATEGORIES[category]) {
      info = JSON_CATEGORIES[category]
      buffer = await fetchFromJson(info.file)
    } else if (API_CATEGORIES[category]) {
      info = API_CATEGORIES[category]
      buffer = await fetchFromApi(info.endpoint)
    } else {
      return m.reply(`❌ Kategori *${category}* tidak ditemukan.`)
    }

    const media = await prepareWAMessageMedia(
      { image: buffer },
      { upload: sock.waUploadToServer }
    )

    const msg = generateWAMessageFromContent(m.chat, {
      viewOnceMessage: {
        message: {
          messageContextInfo: {
            deviceListMetadata: {},
            deviceListMetadataVersion: 2,
          },
          interactiveMessage: {
            body: { text: `${info.emoji} *${info.label.toUpperCase()}*` },
            footer: { text: "🔞 Konten ini hanya untuk 18+ — Gunakan dengan bijak" },
            header: {
              hasMediaAttachment: true,
              imageMessage: media.imageMessage,
            },
            nativeFlowMessage: {
              buttons: [
                {
                  name: "quick_reply",
                  buttonParamsJson: JSON.stringify({
                    display_text: `${info.emoji} Lanjut Lagi?`,
                    id: `${m.prefix}${category}`,
                  }),
                },
                {
                  name: "quick_reply",
                  buttonParamsJson: JSON.stringify({
                    display_text: "📂 Lihat Semua Kategori",
                    id: `${m.prefix}nsfw`,
                  }),
                },
              ],
            },
          },
        },
      },
    }, { quoted: m })

    await sock.relayMessage(m.chat, msg.message, { messageId: msg.key.id })
    await m.react("✅")
  } catch (err) {
    await m.react("☢")
    m.reply(te(m.prefix, m.command, m.pushName))
  }
}

export { pluginConfig as config, handler }
