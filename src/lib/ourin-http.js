import { request, Agent, setGlobalDispatcher } from 'undici'
import axios from 'axios'
import http from 'http'
import https from 'https'

const REQUEST_TIMEOUT = 15_000

// Shared keep-alive agents — axios default membuat socket TCP/TLS baru per
// request (281 pemakaian di codebase). Dengan keepAlive, koneksi di-reuse:
// latency turun (tanpa handshake ulang) + RAM stabil (tanpa socket churn).
// Sumber: undici docs (Agent/Pool connection pooling) + Node.js cli docs.
const httpAgent = new http.Agent({ keepAlive: true, maxSockets: 50, maxFreeSockets: 10, timeout: 30_000 })
const httpsAgent = new https.Agent({ keepAlive: true, maxSockets: 50, maxFreeSockets: 10, timeout: 30_000 })

try {
  const undiciAgent = new Agent({ connections: 50, keepAliveTimeout: 30_000, keepAliveMaxTimeout: 600_000, pipelining: 1 })
  setGlobalDispatcher(undiciAgent)
} catch { }

export const httpAxios = axios.create({
  timeout: 15000,
  httpAgent,
  httpsAgent,
  maxRedirects: 3,
  maxContentLength: 25 * 1024 * 1024,
  maxBodyLength: 25 * 1024 * 1024,
})

async function f(url, responseType = "json", method = "GET", headers = {}, body = null) {
    const controller = new AbortController()
    const timeoutId = setTimeout(() => controller.abort(), REQUEST_TIMEOUT)

    try {
        const response = await request(url, {
            method,
            headers: {
                "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
                ...headers
            },
            body,
            signal: controller.signal
        })

        if (response.statusCode >= 400) {
            await response.body.dump()
            return null
        }

        if (responseType === "json") return await response.body.json()
        if (responseType === "text") return await response.body.text()
        if (responseType === "arrayBuffer") return await response.body.arrayBuffer()
        if (responseType === "buffer") return Buffer.from(await response.body.arrayBuffer())

        await response.body.dump()
        return null
    } catch (error) {
        const reason = controller.signal.aborted ? "timeout" : (error?.message || error)
        console.error(`[http] request gagal: ${method} ${url} — ${reason}`)
        return null
    } finally {
        clearTimeout(timeoutId)
    }
}

export { f }