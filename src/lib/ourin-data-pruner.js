import { logger } from './ourin-logger.js'
import { getDatabase } from './ourin-database.js'
const INACTIVE_THRESHOLD = 14 * 24 * 60 * 60 * 1000
const PRUNE_INTERVAL = 6 * 60 * 60 * 1000

let prunerTimer = null

function startDailyPruner() {
    if (prunerTimer) return

    prunerTimer = setInterval(() => {
        try {
            
            const db = getDatabase()
            if (!db || !db.db?.data) return

            const now = Date.now()
            const threshold = now - INACTIVE_THRESHOLD
            let prunedUsers = 0
            let prunedGroups = 0
            let prunedPremium = 0
            let prunedPartner = 0
            let prunedContacts = 0

            const users = db.db.data.users
            if (users && typeof users === 'object') {
                for (const [jid, user] of Object.entries(users)) {
                    const isProtected =
                        user.premium ||
                        user.owner ||
                        user.partner ||
                        user.banned

                    if (!isProtected && user.lastSeen && user.lastSeen < threshold) {
                        delete users[jid]
                        prunedUsers++
                    }
                }
            }

            const groups = db.db.data.groups
            if (groups && typeof groups === 'object') {
                for (const [jid, group] of Object.entries(groups)) {
                    if (group.lastActivity && group.lastActivity < threshold) {
                        delete groups[jid]
                        prunedGroups++
                    }
                }
            }

            if (Array.isArray(db.db.data.premium)) {
                const arr = db.db.data.premium;
                for (let i = arr.length - 1; i >= 0; i--) {
                    const p = arr[i];
                    if (typeof p === 'string') continue;
                    const expire = p.expired || (p.expiredAt ? new Date(p.expiredAt).getTime() : 0);
                    if (expire && expire <= now) { arr.splice(i, 1); prunedPremium++; }
                }
            }
            if (Array.isArray(db.db.data.partner)) {
                const arr = db.db.data.partner;
                for (let i = arr.length - 1; i >= 0; i--) {
                    const p = arr[i];
                    if (typeof p === 'string') continue;
                    const expire = p.expired || (p.expiredAt ? new Date(p.expiredAt).getTime() : 0);
                    if (expire && expire <= now) { arr.splice(i, 1); prunedPartner++; }
                }
            }

            const contacts = db.setting?.('contacts')
            if (contacts && typeof contacts === 'object') {
                for (const jid of Object.keys(contacts)) {
                    if (!users?.[jid]) {
                        delete contacts[jid]
                        prunedContacts++
                    }
                }
                if (prunedContacts > 0) db.setting('contacts', contacts)
            }

            if (users && typeof users === 'object') {
                for (const user of Object.values(users)) {
                    if (!user.cooldowns) continue
                    for (const [cmd, until] of Object.entries(user.cooldowns)) {
                        if (until <= now) delete user.cooldowns[cmd]
                    }
                }
            }

            if (prunedUsers > 0 || prunedGroups > 0 || prunedPremium > 0 || prunedPartner > 0 || prunedContacts > 0) {
                db.save()
                logger.system('pruner', `removed ${prunedUsers} users, ${prunedGroups} groups, ${prunedPremium} premium, ${prunedPartner} partner, ${prunedContacts} contacts (>${INACTIVE_THRESHOLD / 86400000}d inactive)`)
            }
        } catch (error) {
            logger.error('pruner', error.message)
        }
    }, PRUNE_INTERVAL)

    if (prunerTimer.unref) prunerTimer.unref()
    logger.success('pruner', `auto-prune >${INACTIVE_THRESHOLD / 86400000}d inactive, every ${PRUNE_INTERVAL / 3600000}h`)
}

function stopDailyPruner() {
    if (prunerTimer) {
        clearInterval(prunerTimer)
        prunerTimer = null
    }
}

export { startDailyPruner, stopDailyPruner }