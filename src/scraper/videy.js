import axios from 'axios'
import FormData from 'form-data'
import fs from 'fs'
import path from 'path'
import crypto from 'crypto'
import { httpAxios } from '../lib/http.js'
async function videy(file){
    try{
        if(!file) throw new Error('input required')
        if(!fs.existsSync(file)) throw new Error('file not found')

        const form = new FormData()
        form.append(
            'file',
            fs.createReadStream(file),
            {
                filename: path.basename(file),
                contentType: 'video/mp4'
            }
        )

        const r = await axios.post(
            'https://videy.co/api/upload?visitorId=' + crypto.randomUUID(),
            form,
            {
                headers:{
                    ...form.getHeaders(),
                    'User-Agent':'Mozilla/5.0 (Linux; Android 10)',
                    origin:'https://videy.co',
                    referer:'https://videy.co/',
                    accept:'application/json'
                },
                maxBodyLength: Infinity,
                maxContentLength: Infinity
            }
        )

        return {
            output:r.data
        }

    }catch(e){
        return { status:'error', msg:e.message }
    }
}

export default videy

// ── resolve tautan berbagi ────────────────────────────────────────────────────
//
// Default export di atas adalah uploader: ia menerima jalur file lokal lalu
// mengirimkannya ke videy.co. Plugin `download/videy` tidak memakai itu — ia
// menerima `https://videy.co/v?id=...` dan harus mengembalikan URL video yang
// bisa dikirim ke user. Jadi jalur di bawah dibangun dari permukaan yang dipakai
// frontend videy sendiri, bukan dari uploader: id diambil dari query `?id=`,
// lalu dipetakan ke objek CDN `cdn.videy.co/<id>.<ext>`.

// Batas dua tempat, dan keduanya perlu. Panjang id dibatasi karena id masuk ke
// pathname CDN: tanpa itu `?id=../rahasia` menulis keluar dari host. Bentuk
// karakternya dibatasi karena id juga jadi substring regex.
const ID_BENTUK = /^[A-Za-z0-9_-]{1,64}$/;

// Batas probe sengaja kecil dan bukan `Infinity`. Uploader memakai batas tak
// terbatas karena memang mengirim body milik user, dan itu wajar; probe di sini
// hanya perlu membuktikan objeknya ada. Kalau batas `Infinity` ikut terbawa ke
// jalur ini, satu probe ke video multi-GB bisa menarik isinya ke kotak RAM 1GB.
// `Range` satu byte PLUS `maxContentLength` kecil: untuk CDN yang mengabaikan
// `Range`, transfer tetap dimatikan oleh batas kedua.
const PROBE_RANGE = 'bytes=0-0'
const PROBE_BATAS_BYTES = 64 * 1024

// Aturan ini disalin dari frontend videy, bukan ditebak: `cdn.videy.co/${id}.${ext}`
// dengan ext `mov` hanya untuk id 9 karakter yang berakhir "2", selain itu mp4.
// Salah di sini menghasilkan URL 404 yang tetap terlihat seperti tautan hidup.
function ekstensiVidey(id) {
    return id.length === 9 && id.endsWith('2') ? 'mov' : 'mp4'
}

/**
 * Id video dari link berbagi `https://videy.co/v?id=…`, atau `null` kalau
 * bentuknya tidak terbaca. Satu-satunya tempat di repo ini yang tahu seperti
 * apa tautan berbagi videy, jadi kapabilitas tidak perlu menirunya.
 */
export function idDari(url) {
    let parsed
    try {
        parsed = new URL(String(url).trim())
    } catch {
        return null
    }
    const id = parsed.searchParams.get('id')
    return id && ID_BENTUK.test(id) ? id : null
}

/**
 * Ubah link berbagi menjadi URL video langsung, setelah membuktikan objeknya
 * benar-benar ada di CDN. Tanpa probe, backend lokal selalu "berhasil" untuk
 * link yang sudah dihapus: URL 404 naik ke `sendMedia`, dan plugin tetap memberi
 * centang hijau ke user yang tidak menerima apa-apa.
 */
export async function videyShare(url, { signal } = {}) {
    const id = idDari(url)
    if (!id) throw new Error('tautan berbagi videy tanpa id video yang bisa dibaca')

    const ext = ekstensiVidey(id)
    const address = `https://cdn.videy.co/${id}.${ext}`

    // `signal` opsional supaya pemanggil yang hanya mengirim URL tetap jalan, tapi
    // backend resolver boleh teruskannya: AbortController di resolve.js hanya
    // menghentikan resolver menunggu, dan tanpa signal di sini probe-nya tetap
    // hidup sampai timeout 15 detik milik httpAxios sambil memegang socket.
    // 2xx (dan 206 parsial dari Range) berarti objeknya ada; 404 berarti tautan
    // sudah mati, jadi axios melempar dan backend berikutnya yang mendapat giliran.
    await httpAxios.get(address, {
        headers: {
            'User-Agent': 'Mozilla/5.0 (Linux; Android 10)',
            Referer: 'https://videy.co/',
            accept: 'video/mp4,video/quicktime,*/*',
            Range: PROBE_RANGE
        },
        responseType: 'arraybuffer',
        maxContentLength: PROBE_BATAS_BYTES,
        signal
    })

    return { id, ext, url: address }
}
