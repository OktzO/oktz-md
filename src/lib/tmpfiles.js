import { uploadImage } from './uploader.js'

// Sebelumnya: hardcode c.termai.cc + wajib TERMAI_UPLOAD_KEY, satu host.
// Sekarang lewat uploader resilient (6 host keyless, sequential fallback).
// Nama export & bentuk return dipertahankan karena fakeml.js memakai
// `.directUrl`.
export async function uploadTo0x0(buffer, opts) {
  if (!Buffer.isBuffer(buffer)) throw new Error("buffer harus Buffer");

  const url = await uploadImage(buffer, opts?.filename || 'image.jpg', {
    perProviderMs: opts?.timeoutMs,
    providers: opts?.providers,
  });

  return { url, directUrl: url };
}
