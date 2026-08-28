// Time helpers — Asia/Jakarta, Indonesian locale.
// ponytail: moment-timezone replaced with Intl.DateTimeFormat + Date math.
// Supports the moment format tokens actually used across the codebase:
//   YYYY MM MMMM DD D HH mm ss dddd LLLL
// Ceiling: arbitrary moment tokens (relative time, durations, week math, other
// locale shortcuts like LT/LTS/LLL) are NOT implemented. If a caller needs one,
// either extend formatPattern() here or reach for a real date lib again.
const TIMEZONE = 'Asia/Jakarta'

const NUM_FMT = new Intl.DateTimeFormat('id-ID', {
    timeZone: TIMEZONE,
    year: 'numeric', month: '2-digit', day: '2-digit',
    hour: '2-digit', minute: '2-digit', second: '2-digit',
    hourCycle: 'h23',
})
const NAME_FMT = new Intl.DateTimeFormat('id-ID', {
    timeZone: TIMEZONE,
    weekday: 'long', month: 'long',
})

function pick(parts, type) {
    for (const p of parts) if (p.type === type) return p.value
    return ''
}

// Numeric wall-clock parts in TIMEZONE. month is 1-12.
function getNumericParts(date) {
    const parts = NUM_FMT.formatToParts(date)
    return {
        year: parseInt(pick(parts, 'year'), 10),
        month: parseInt(pick(parts, 'month'), 10),
        day: parseInt(pick(parts, 'day'), 10),
        hour: parseInt(pick(parts, 'hour'), 10),
        minute: parseInt(pick(parts, 'minute'), 10),
        second: parseInt(pick(parts, 'second'), 10),
    }
}

function getNameParts(date) {
    const parts = NAME_FMT.formatToParts(date)
    return {
        weekday: pick(parts, 'weekday'),
        monthLong: pick(parts, 'month'),
    }
}

const pad2 = (n) => String(n).padStart(2, '0')

// Build the replacement map for a single Date.
function buildMap(date) {
    const n = getNumericParts(date)
    const nm = getNameParts(date)
    const HH = pad2(n.hour), mm = pad2(n.minute), ss = pad2(n.second)
    return {
        LLLL: `${nm.weekday}, ${n.day} ${nm.monthLong} ${n.year} pukul ${HH}.${mm}`,
        YYYY: String(n.year),
        MMMM: nm.monthLong,
        MM: pad2(n.month),
        dddd: nm.weekday,
        DD: pad2(n.day),
        D: String(n.day),
        HH, mm, ss,
    }
}

const TOKEN_RE = /LLLL|YYYY|MMMM|MM|dddd|DD|D|HH|mm|ss/g

// Format an arbitrary Date with a moment-style pattern in TIMEZONE.
function formatPattern(date, pattern) {
    const d = date instanceof Date ? date : new Date(date)
    const map = buildMap(d)
    return pattern.replace(TOKEN_RE, (t) => map[t])
}

function now() {
    return new Date()
}

function formatNow(pattern) {
    return formatPattern(new Date(), pattern)
}

function formatTime(format = 'HH:mm:ss') {
    return formatNow(format)
}

function formatDate(format = 'DD-MM-YYYY') {
    return formatNow(format)
}

function formatDateTime(format = 'DD-MM-YYYY HH:mm:ss') {
    return formatNow(format)
}

function formatFull(format = 'dddd, DD MMMM YYYY HH:mm:ss') {
    return formatNow(format)
}

function getHour() {
    return getNumericParts(new Date()).hour
}

function getMinute() {
    return getNumericParts(new Date()).minute
}

function getCurrentTimeString() {
    return formatNow('HH:mm')
}

// timestamp: ms epoch number OR ISO string (new Date handles both, like moment).
function fromTimestamp(timestamp, format = 'DD-MM-YYYY HH:mm:ss') {
    return formatPattern(new Date(timestamp), format)
}

function getLocalDateObject() {
    return new Date()
}

// Epoch ms for a wall-clock time in TIMEZONE (month 1-12). Used for scheduling.
function zonedTimeToEpoch({ year, month, day, hour = 0, minute = 0, second = 0 }) {
    const asUtc = Date.UTC(year, month - 1, day, hour, minute, second)
    // offset = local wall-clock (as UTC) - real UTC, at the guess. Jakarta has no
    // DST so one refinement is exact; loop twice to be safe for other zones.
    let epoch = asUtc
    for (let i = 0; i < 2; i++) {
        epoch = asUtc - tzOffsetAt(epoch)
    }
    return epoch
}

function tzOffsetAt(epochMs) {
    const d = new Date(epochMs)
    const local = getNumericParts(d)
    const localAsUtc = Date.UTC(local.year, local.month - 1, local.day, local.hour, local.minute, local.second)
    const realUtc = Date.UTC(d.getUTCFullYear(), d.getUTCMonth(), d.getUTCDate(), d.getUTCHours(), d.getUTCMinutes(), d.getUTCSeconds())
    return localAsUtc - realUtc
}

export {
    now, formatNow, formatPattern, formatTime, formatDate, formatDateTime, formatFull,
    getHour, getMinute, getCurrentTimeString, fromTimestamp, getLocalDateObject,
    getNumericParts, zonedTimeToEpoch, TIMEZONE,
}
