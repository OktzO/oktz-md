import sharp from 'sharp'

async function normalizeToJpeg(img) {
  const meta = await sharp(img).metadata().catch(() => null)
  const buf = await sharp(img).jpeg().toBuffer()
  return { buf, width: meta?.width || 0, height: meta?.height || 0 }
}

async function pixelcutRemove(buf) {
  const form = new FormData()
  form.append('image', new Blob([buf], { type: 'image/jpeg' }), 'image.jpg')
  form.append('format', 'png')
  form.append('model', 'v1')

  const res = await fetch('https://api2.pixelcut.app/image/matte/v1', {
    method: 'POST',
    headers: {
      'User-Agent': 'Mozilla/5.0 (Linux; Android 10; K) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/139.0.0.0 Mobile Safari/537.36',
      'Accept': 'application/json, text/plain, */*',
      'sec-ch-ua': '"Chromium";v="139", "Not;A=Brand";v="99"',
      'x-locale': 'en',
      'x-client-version': 'web:pixa.com:4a5b0af2',
      'sec-ch-ua-mobile': '?1',
      'sec-ch-ua-platform': '"Android"',
      'origin': 'https://www.pixelcut.ai',
      'sec-fetch-site': 'cross-site',
      'sec-fetch-mode': 'cors',
      'sec-fetch-dest': 'empty',
      'referer': 'https://www.pixelcut.ai/',
      'accept-language': 'id-ID,id;q=0.9,en-AU;q=0.8,en;q=0.7,en-US;q=0.6'
    },
    body: form
  })

  const contentType = res.headers.get('content-type') || ''
  if (!res.ok || !contentType.includes('image/')) {
    const text = await res.text().catch(() => '')
    try {
      const json = JSON.parse(text)
      throw new Error(json.error || json.message || `pixelcut failed: ${res.status}`)
    } catch (e) {
      if (e instanceof SyntaxError) throw new Error(`pixelcut failed: ${res.status} ${text.slice(0, 200)}`, { cause: e })
      throw e
    }
  }
  return Buffer.from(await res.arrayBuffer())
}

async function bgninjaRemove(buf) {
  const form = new FormData()
  form.append('file', new Blob([buf], { type: 'image/jpeg' }), 'image.jpg')
  form.append('src', 'wa-bot')

  const res = await fetch('https://bgninja.com/api/remove', {
    method: 'POST',
    headers: { 'User-Agent': 'Mozilla/5.0' },
    body: form
  })

  const contentType = res.headers.get('content-type') || ''
  if (!res.ok || !contentType.includes('image/')) {
    const text = await res.text().catch(() => '')
    try {
      const json = JSON.parse(text)
      throw new Error(json.error || json.detail || `bgninja failed: ${res.status}`)
    } catch (e) {
      if (e instanceof SyntaxError) throw new Error(`bgninja failed: ${res.status} ${text.slice(0, 200)}`, { cause: e })
      throw e
    }
  }
  return Buffer.from(await res.arrayBuffer())
}

async function pixa(img) {
  const { buf } = await normalizeToJpeg(img)
  try {
    return await pixelcutRemove(buf)
  } catch (err) {
    try {
      return await bgninjaRemove(buf)
    } catch (err2) {
      throw new Error(`removebg gagal (pixelcut: ${err.message}; bgninja: ${err2.message})`, { cause: err2 })
    }
  }
}

export { pixa }
