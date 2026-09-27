import FormData from "form-data";
import fetch from "node-fetch";
import mime from "mime-types";

// Host upload keyless, diekstrak dari plugins/tools/tourl.js supaya uploader
// internal dan .tourl bisa pakai satu daftar yang sama.
//
// SEMUA host di daftar ini sudah diverifikasi sungguhan 2026-09-27: upload
// 165KB lalu URL di-fetch dan dicek byte-nya == byte asli. Host yang dibuang
// beserta alasannya:
//   Qu.ax     - balas HTML 7.8KB, bukan file
//   TmpFiles  - /dl/ 302 redirect balik ke halaman preview
//   Top4top   - pola URL di respons HTML tidak ketemu
// Tidak ada satu pun yang butuh API key.

const UA =
  "Mozilla/5.0 (Linux; Android 10; K) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/147.0.0.0 Mobile Safari/537.36";

const ct = (filename) => mime.lookup(filename) || "application/octet-stream";

async function litterbox(buffer, filename) {
  const form = new FormData();
  form.append("reqtype", "fileupload");
  form.append("time", "72h");
  form.append("fileToUpload", buffer, { filename, contentType: ct(filename) });

  const res = await fetch("https://litterbox.catbox.moe/resources/internals/api.php", {
    method: "POST",
    body: form,
    headers: form.getHeaders(),
    timeout: 30000,
  });
  if (!res.ok) throw new Error("Litterbox gagal");
  const url = (await res.text()).trim();
  if (!url.startsWith("http")) throw new Error("Invalid response");
  return { host: "Litterbox", url, expires: "72 jam" };
}

async function kappa(buffer, filename) {
  const form = new FormData();
  form.append("file", buffer, { filename, contentType: ct(filename) });

  const res = await fetch("https://kappa.lol/api/upload", {
    method: "POST",
    body: form,
    headers: {
      ...form.getHeaders(),
      "User-Agent": UA,
      Accept: "*/*",
      Origin: "https://kappa.lol",
      Referer: "https://kappa.lol/",
    },
    timeout: 60000,
  });
  if (!res.ok) throw new Error("Kappa gagal");
  const url = JSON.parse(await res.text())?.link || null;
  if (!url) throw new Error("Invalid response");
  return { host: "Kappa", url, expires: "Permanent" };
}

async function uguu(buffer, filename) {
  const form = new FormData();
  form.append("files[]", buffer, { filename, contentType: ct(filename) });

  const res = await fetch("https://uguu.se/upload.php", {
    method: "POST",
    body: form,
    headers: {
      ...form.getHeaders(),
      accept: "*/*",
      origin: "https://uguu.se",
      referer: "https://uguu.se/",
      "user-agent": UA,
    },
    timeout: 120000,
  });
  if (!res.ok) throw new Error("Uguu gagal");
  const data = await res.json();
  const url = data?.files?.[0]?.url || null;
  if (!data?.success || !url) throw new Error("Invalid response");
  return { host: "Uguu", url, expires: "48 jam" };
}

async function nekohime(buffer, filename) {
  const form = new FormData();
  form.append("file", buffer, { filename, contentType: ct(filename) });

  const res = await fetch("https://cdn.nekohime.site/upload", {
    method: "POST",
    body: form,
    headers: form.getHeaders(),
    timeout: 120000,
  });
  if (!res.ok) throw new Error("Nekohime gagal");
  const data = await res.json();
  // files[0] adalah OBJEK {url, filename, status}, bukan string URL
  const f = data?.files;
  const url = Array.isArray(f) ? f[0]?.url : f?.url;
  if (!url) throw new Error("Nekohime gagal");
  return { host: "Nekohime", url, expires: "Permanent" };
}

async function leopard(buffer, filename) {
  const uploadPage = "https://leopard.hosting.pecon.us/upload.php";
  await fetch(uploadPage, { headers: { "User-Agent": UA }, timeout: 30000 });

  const form = new FormData();
  form.append("uploadContent", buffer, { filename, contentType: ct(filename) });
  form.append("password", "");
  form.append("showname", "yes");

  const html = await fetch(uploadPage, {
    method: "POST",
    body: form,
    headers: {
      ...form.getHeaders(),
      "User-Agent": UA,
      Origin: "https://leopard.hosting.pecon.us",
      Referer: uploadPage,
    },
    timeout: 120000,
  }).then((r) => r.text());

  const url = html.match(/Download link:\s*<a href=([^>\s]+)>/i)?.[1] || null;
  if (!url) throw new Error("Leopard gagal");
  return { host: "Leopard", url, expires: "Permanent" };
}

async function uploadEe(buffer, filename) {
  const ext = (filename.match(/\.([^.]+)$/) || [])[1] || "bin";
  const isImage = ["jpg", "jpeg", "png", "webp", "gif", "bmp", "svg", "avif"].includes(
    ext.toLowerCase(),
  );

  await fetch("https://www.upload.ee/?", { headers: { "User-Agent": UA }, timeout: 30000 });

  const idBody = await fetch(
    `https://www.upload.ee/ubr_link_upload.php?rnd_id=${Date.now()}`,
    { headers: { "User-Agent": UA, Referer: "https://www.upload.ee/?" }, timeout: 30000 },
  ).then((r) => r.text());

  const uploadId = idBody.match(/startUpload\("([^"]+)"/)?.[1];
  if (!uploadId) throw new Error("Upload ID tidak ditemukan");

  const form = new FormData();
  form.append("upfile_0", buffer, { filename, contentType: ct(filename) });
  form.append("link", "");
  form.append("email", "");
  form.append("category", isImage ? "cat_picture" : "cat_file");
  form.append("big_resize", "none");
  form.append("small_resize", "120x90");

  const uploadUrl = `https://www.upload.ee/cgi-bin/ubr_upload.pl?X-Progress-ID=${uploadId}&upload_id=${uploadId}`;
  await fetch(uploadUrl, {
    method: "POST",
    body: form,
    headers: {
      ...form.getHeaders(),
      "User-Agent": UA,
      Origin: "https://www.upload.ee",
      Referer: "https://www.upload.ee/?",
    },
    timeout: 120000,
  });

  const html = await fetch(
    `https://www.upload.ee/?page=finished&upload_id=${uploadId}`,
    { headers: { "User-Agent": UA, Referer: uploadUrl }, timeout: 30000 },
  ).then((r) => r.text());

  const rawUrl =
    html.match(/id=["']file_src["'][^>]*value=["']([^"']+)["']/i)?.[1] ||
    html.match(/View file:\s*<br\s*\/?>\s*<a href=["']?([^"'>\s]+)["']?/i)?.[1];
  if (!rawUrl) throw new Error("Upload.ee gagal");

  let url = rawUrl.replaceAll("&amp;", "&").replaceAll("&quot;", '"');
  if (isImage) url = url.replace("/files/", "/image/").replace(/\.html$/, "");
  return { host: "Upload.ee", url, expires: "Permanent" };
}

// Urutan = urutan percobaan. Yang paling ringan dan paling sering hidup dulu
// supaya fallback jarang dipakai.
const PROVIDERS = [
  { name: "Litterbox", run: (b, f) => litterbox(b, f) },
  { name: "Kappa", run: (b, f) => kappa(b, f) },
  { name: "Uguu", run: (b, f) => uguu(b, f) },
  { name: "Nekohime", run: (b, f) => nekohime(b, f) },
  { name: "Leopard", run: (b, f) => leopard(b, f) },
  { name: "Upload.ee", run: (b, f) => uploadEe(b, f) },
];

export function createUploadProviders(overrides) {
  return overrides ? overrides.slice() : PROVIDERS.slice();
}

export { PROVIDERS as UPLOAD_PROVIDERS };
