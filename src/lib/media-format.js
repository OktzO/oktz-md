/**
 * Kosakata format media: apa yang dilaporkan host, dan bagaimana labelnya
 * dibungkus jadi `mimetype` + ekstensi untuk WhatsApp.
 *
 * Module ini ada karena format hasil unduhan tidak lagi selalu mp3. Probe
 * langsung pada 2026-10-03 ke `my.izuka-api.xyz/api/downloader/ytmp3`
 * menjawab `"format":"webm"` dengan URL unduhan ber-`mime=audio%2Fwebm`
 * (`itag=251`, WebM/Opus). Plugin yang menulis `audio/mpeg` dan `<judul>.mp3`
 * mengirim byte WebM dengan container yang tidak cocok — itu sebabnya pemutar
 * dan WhatsApp menolak file, dan tidak ada satu pun test yang menangkapnya
 * karena kontrak kapabilitas masih berbohong soal format.
 *
 * Satu modul untuk semuanya: tiga plugin mp3 memakai pemetaan yang sama, dan
 * tiga salinan tabel akan langsung mulai berbeda satu per satu. Arah pemetaan
 * juga sengaja dua: `formatFrom*` untuk membaca apa yang benar-benar dikirim
 * host, `mimetypeFor`/`extensionFor` untuk menulis label yang benar saat
 * mengirim.
 */

/**
 * Format audio → MIME untuk WhatsApp.
 *
 * `mp4` dan `mkv` ikut ada karena aggregator boleh melaporkan video-only
 * container untuk trek audio; audio di dalam MP4 adalah `audio/mp4`, bukan
 * `audio/mpeg`. Yang tidak ada di sini adalah token video murni (`mov`, `flv`,
 * `avi`): `mimetypeFor` untuk mereka jatuh ke default dan dokumentasinya di
 * bawah, karena tidak ada plugin yang mengirim video lewat jalur mp3.
 */
const MIMETYPE_DARI_FORMAT = new Map([
  ["mp3", "audio/mpeg"],
  ["m4a", "audio/mp4"],
  ["mp4", "audio/mp4"],
  ["m4v", "audio/mp4"],
  ["aac", "audio/aac"],
  ["ogg", "audio/ogg"],
  ["oga", "audio/ogg"],
  ["opus", "audio/ogg"],
  ["webm", "audio/webm"],
  ["wav", "audio/wav"],
  ["flac", "audio/flac"],
  ["amr", "audio/amr"],
  ["mkv", "audio/x-matroska"],
  ["3gp", "audio/3gpp"],
]);

/**
 * MIME → format, untuk bacaan sisi kapabilitas.
 *
 * Lengkap dengan video karena kapabilitas harus bisa menyebut apa yang
 * diterima, bukan hanya audio: pemisahan dua arahnya tidak boleh menebak dari
 * ekstensi. `audio/ogg; codecs=opus` dan `audio/webm` dipetakan ke token yang
 * berbeda karena pemutar memperlakukannya berbeda — itu sebabnya `webm` tidak
 * pernah disatukan ke `ogg`.
 */
const FORMAT_DARI_MIME = new Map([
  ["audio/mpeg", "mp3"],
  ["audio/mpeg3", "mp3"],
  ["audio/x-mpeg-3", "mp3"],
  ["audio/mp4", "m4a"],
  ["audio/x-m4a", "m4a"],
  ["audio/aac", "aac"],
  ["audio/ogg", "ogg"],
  ["audio/opus", "opus"],
  ["audio/webm", "webm"],
  ["audio/wav", "wav"],
  ["audio/x-wav", "wav"],
  ["audio/vnd.wave", "wav"],
  ["audio/flac", "flac"],
  ["audio/x-flac", "flac"],
  ["audio/amr", "amr"],
  ["audio/3gpp", "3gp"],
  ["audio/x-matroska", "mkv"],
  ["video/mp4", "mp4"],
  ["video/webm", "webm"],
  ["video/quicktime", "mov"],
  ["video/x-matroska", "mkv"],
  ["video/3gpp", "3gp"],
  ["video/x-flv", "flv"],
  ["video/x-msvideo", "avi"],
]);

// Diturunkan, bukan ditulis ulang: daftar token yang sah adalah seluruh kunci
// dan nilai kedua tabel di atas. Tanpa ini `formatFromToken` harus punya daftar
// sendiri, dan daftar kedua yang berbeda adalah tempat bug "format yang
// berbohong" akan kembali muncul.
const TOKEN_FORMAT = new Set([...MIMETYPE_DARI_FORMAT.keys(), ...FORMAT_DARI_MIME.values()]);

/**
 * Format default kalau host tidak melaporkan apa pun.
 *
 * `mp3`, bukan tebakan acak: setiap jalur audio di Kapabilitas ini menghasilkan
 * mp3 (`ytdl` dengan `format: "mp3"`), dan jalur video dikunci terpisah lewat
 * filter `ext: "mp4"` — bukan lewat default ini.
 */
export const FORMAT_DEFAULT = "mp3";

/** Parameter MIME (`; codecs=opus`) dibuang sebelum looked up. */
function dasarMime(nilai) {
  return String(nilai ?? "").split(";")[0].trim().toLowerCase();
}

/**
 * Format dari nama yang sudah dilaporkan host (`"webm"`, `"MP3"`, `".mp4"`).
 * String kosong kalau nama itu bukan format — `"360"` milik SaveTube adalah
 * nomor kualitas, bukan format, dan menerimanya akan mengubah nomor kualitas
 * menjadi label media.
 */
export function formatFromToken(nilai) {
  const token = dasarMime(nilai).replace(/^\./, "");
  return TOKEN_FORMAT.has(token) ? token : "";
}

/** Format dari MIME yang dilaporkan (`"audio/webm"`, `"audio/ogg; codecs=opus"`). */
export function formatFromMime(nilai) {
  return FORMAT_DARI_MIME.get(dasarMime(nilai)) ?? "";
}

/**
 * MIME untuk `sendMedia`. Default `audio/mpeg` dipertahankan supaya plugin yang
 * menerima `format` kosong atau token yang tidak dikenal tetap berperilaku seperti
 * sebelum Capability ini melaporkan format yang sebenarnya.
 */
export function mimetypeFor(format) {
  return MIMETYPE_DARI_FORMAT.get(formatFromToken(format) || format) ?? MIMETYPE_DARI_FORMAT.get(FORMAT_DEFAULT);
}

/**
 * Ekstensi untuk nama berkas dan nama file sementara. Default `FORMAT_DEFAULT`,
 * dengan alasan yang sama seperti `mimetypeFor`.
 */
export function extensionFor(format) {
  return formatFromToken(format) || FORMAT_DEFAULT;
}