import fs from 'fs'
import fsp from 'fs/promises'
import path from 'path'
import { logger } from './ourin-logger.js'
import { yieldToEventLoop } from './ourin-async-pool.js'

const CLEAN_INTERVAL = 30 * 60 * 1000
const MAX_AGE_MS = 60 * 60 * 1000
const MIN_AGE_MS = 5 * 60 * 1000

let cleanerTimer = null

async function scanDir(dirPath, ageThreshold) {
  let total = 0
  let size = 0
  try {
    await fsp.access(dirPath)
  } catch { return { total, size } }
  try {
    const entries = await fsp.readdir(dirPath, { withFileTypes: true })
    for (let i = 0; i < entries.length; i++) {
      const entry = entries[i]
      const fullPath = path.join(dirPath, entry.name)
      try {
        if (entry.isDirectory()) {
          const sub = await scanDir(fullPath, ageThreshold)
          total += sub.total
          size += sub.size
          const remaining = await fsp.readdir(fullPath)
          if (remaining.length === 0) await fsp.rmdir(fullPath)
        } else if (entry.isFile()) {
          const stat = await fsp.stat(fullPath)
          const age = Date.now() - stat.mtimeMs
          if (age > ageThreshold) {
            size += stat.size
            await fsp.unlink(fullPath)
            total++
          }
        }
      } catch {}
      if (i % 50 === 49) await yieldToEventLoop()
    }
  } catch {}
  return { total, size }
}

function startTempCleaner() {
  if (cleanerTimer) return
  cleanerTimer = setInterval(async () => {
    const now = Date.now()
    const ageThreshold = now - MAX_AGE_MS
    let grandTotal = 0, grandSize = 0
    for (const dir of ['temp', 'tmp']) {
      const dirPath = path.join(process.cwd(), dir)
      try {
        await fsp.access(dirPath)
      } catch { continue }
      const result = await scanDir(dirPath, ageThreshold)
      grandTotal += result.total
      grandSize += result.size
    }
    if (grandTotal > 0) {
      const sizeMB = (grandSize / 1024 / 1024).toFixed(2)
      logger.system('temp', `cleaned ${grandTotal} file(s) (${sizeMB}MB)`)
    }
  }, CLEAN_INTERVAL)
  if (cleanerTimer.unref) cleanerTimer.unref()
  logger.success('temp', `auto-clean age >${MAX_AGE_MS/60000}m recursive, every ${CLEAN_INTERVAL/60000}m`)
}

function stopTempCleaner() {
  if (cleanerTimer) { clearInterval(cleanerTimer); cleanerTimer = null }
}

export { startTempCleaner, stopTempCleaner }
