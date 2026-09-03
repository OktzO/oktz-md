export function yieldToEventLoop() {
  return new Promise(resolve => setImmediate(resolve));
}

export class AsyncPool {
  constructor(concurrency) {
    this.concurrency = concurrency
    this.queue = []
    this.active = 0
    this._idleResolve = null
  }
  async add(fn) {
    return new Promise((resolve, reject) => {
      this.queue.push({ fn, resolve, reject })
      this._process()
    })
  }
  _process() {
    while (this.active < this.concurrency && this.queue.length > 0) {
      const { fn, resolve, reject } = this.queue.shift()
      this.active++
      fn().then(resolve, reject).finally(() => {
        this.active--
        this._process()
        if (this.active === 0 && this.queue.length === 0 && this._idleResolve) {
          this._idleResolve()
          this._idleResolve = null
        }
      })
    }
  }
  async onIdle() {
    if (this.active === 0 && this.queue.length === 0) return
    return new Promise(resolve => { this._idleResolve = resolve })
  }
  get size() { return this.queue.length }
  get pending() { return this.active }
}

const GROUP_CACHE_TTL = 60000
let _groupCache = null
let _groupCacheTime = 0
let _groupCachePromise = null

export async function getGroupCache(sock) {
  const now = Date.now()
  if (_groupCache && now - _groupCacheTime < GROUP_CACHE_TTL) return _groupCache
  if (_groupCachePromise) return _groupCachePromise
  _groupCachePromise = (async () => {
    const groups = await sock.groupFetchAllParticipating()
    _groupCache = groups
    _groupCacheTime = Date.now()
    return groups
  })()
  try {
    return await _groupCachePromise
  } finally {
    _groupCachePromise = null
  }
}

export function invalidateGroupCache() {
  _groupCache = null
  _groupCacheTime = 0
}