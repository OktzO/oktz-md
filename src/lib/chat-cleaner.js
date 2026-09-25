import { CronJob } from 'cron'
import { getDatabase } from './database.js'
import { logger } from './logger.js'
import { delay } from './utils.js'

const TZ = 'Asia/Jakarta'
const CLEAR_CRON = '0 0 * * *'
const BATCH_DELAY_MS = 250

let sock = null
let clearJob = null

function isClearcableJid(jid) {
    if (!jid || typeof jid !== 'string') return false
    return jid.endsWith('@g.us') || jid.endsWith('@s.whatsapp.net')
}

async function collectClearTargets(sockInstance) {
    const targets = new Set()
    const store = sockInstance?.store

    if (store?.chats && typeof store.chats.keys === 'function') {
        for (const jid of store.chats.keys()) {
            if (isClearcableJid(jid)) targets.add(jid)
        }
    }

    try {
        const groups = await sockInstance.groupFetchAllParticipating?.()
        if (groups) {
            for (const id of Object.keys(groups)) {
                if (isClearcableJid(id)) targets.add(id)
            }
        }
    } catch {
        logger.warn('AutoClear', 'groupFetchAllParticipating gagal, lanjut pakai store chats')
    }

    return [...targets].sort()
}

function isAutoClearEnabled(db = getDatabase()) {
    return !!db.setting('autoClearChat')
}

function setAutoClearEnabled(value, db = getDatabase()) {
    db.setting('autoClearChat', !!value)
    return !!value
}

async function runClearOnce() {
    if (!sock || !isAutoClearEnabled()) return

    const targets = await collectClearTargets(sock)
    if (targets.length === 0) {
        logger.info('AutoClear', 'tidak ada chat untuk dibersihkan')
        return
    }

    let success = 0
    let failed = 0
    for (const jid of targets) {
        try {
            await sock.chatModify({ delete: true, lastMessages: [] }, jid)
            success++
        } catch {
            failed++
        }
        await delay(BATCH_DELAY_MS)
    }

    logger.info('AutoClear', `cleared ${success}/${targets.length} chats (failed ${failed})`)
}

function initAutoClearScheduler(sockInstance) {
    sock = sockInstance
    if (!sock) return

    if (clearJob) clearJob.stop()

    clearJob = new CronJob(CLEAR_CRON, () => {
        runClearOnce().catch((e) => logger.error('AutoClear', e.message))
    }, null, true, TZ)

    logger.info('AutoClear', `scheduler ${isAutoClearEnabled() ? 'aktif' : 'idle (matikan via .autoclear on)'} — setiap ${CLEAR_CRON} (${TZ})`)
}

function stopAutoClearScheduler() {
    if (clearJob) {
        clearJob.stop()
        clearJob = null
    }
    logger.info('AutoClear', 'scheduler dihentikan')
}

export {
    isClearcableJid,
    collectClearTargets,
    isAutoClearEnabled,
    setAutoClearEnabled,
    runClearOnce,
    initAutoClearScheduler,
    stopAutoClearScheduler,
}