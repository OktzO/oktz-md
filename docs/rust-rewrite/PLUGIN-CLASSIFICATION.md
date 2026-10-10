# Plugin Classification — `oktz-md` → Rust Rewrite

Generated: **2026-10-10** · Source: `plugins/**/*.js` · Branch: `rust-rewrite`
Machine-readable: [`plugin-manifest.json`](./plugin-manifest.json) · Generator: `scripts/extract-plugin-manifest.mjs`

> Read-only inventory. No source file was modified.

## Ringkasan

| Metrik | Nilai |
|---|---:|
| Total file plugin | **830** |
| Parse failure | **0** |
| Plugin dengan `needsRuntimeConfig` | **41** |
| Host eksternal unik terdeteksi | **89** |
| Plugin menunggu keputusan Oktz (`awaitingDecision`) | **20** |

Total cocok dengan spesifikasi (830) dan `parseFailures` kosong. Cakupan file 1:1 —
tidak ada file hilang, tidak ada entri tambahan, tidak ada duplikat.

## Distribusi per Kelas

| Kelas | Jumlah | % | Arti |
|---|---:|---:|---|
| **A** | 396 | 47.7% | logika/teks murni |
| **B** | 262 | 31.6% | HTTP/scraper/API eksternal |
| **C** | 44 | 5.3% | media/canvas/ffmpeg/OCR |
| **D** | 103 | 12.4% | butuh fitur WA khusus |
| **E** | 5 | 0.6% | mati/endpoint hilang |
| **F** | 20 | 2.4% | berisiko platform — menunggu keputusan |
| **Total** | **830** | 100% | |

Arti kelas:

- **A** — tidak ada HTTP, tidak ada media, tidak ada fitur WA khusus. Port paling murah.
- **B** — menyentuh jaringan. `externalEndpoints` diisi bila host bisa di-resolve statis; bila tidak (URL dari config/dinamis) host tetap kosong tetapi kelas tetap B.
- **C** — butuh canvas/ffmpeg/OCR/image processing, termasuk yang lewat helper lokal `src/lib/ffmpeg.js`.
- **D** — memakai fitur WA non-portabel: `interactiveMessage`, `nativeFlowMessage`, `buttonParamsJson`, `prepareWAMessageMedia`, `generateWAMessageFromContent`, `waUploadToServer`, `relayMessage`, `newsletter`, `forwardedNewsletterMessageInfo`, `groupParticipantsUpdate`, `viewOnce`, `jadibot`.
- **E** — upstream terverifikasi mati. **Tetap disimpan, tidak dibuang.**
- **F** — berisiko. Ditandai `awaitingDecision: true`, **tidak diputuskan di sini.**

Prioritas saat multi-kategori: **F > E > D > C > B > A**. Seluruh alasan dicatat di `classReason` (dipisah `|`).

## Distribusi per Kategori

| Kategori | Total | A | B | C | D | E | F |
|---|---:|---:|---:|---:|---:|---:|---:|
| `owner` | 151 | 80 | 35 | 2 | 34 | 0 | 0 |
| `group` | 104 | 82 | 6 | 0 | 15 | 1 | 0 |
| `rpg` | 67 | 60 | 7 | 0 | 0 | 0 | 0 |
| `tools` | 56 | 13 | 24 | 10 | 8 | 0 | 1 |
| `cek` | 48 | 47 | 0 | 1 | 0 | 0 | 0 |
| `ai` | 46 | 0 | 45 | 0 | 0 | 1 | 0 |
| `search` | 46 | 4 | 25 | 1 | 15 | 1 | 0 |
| `fun` | 40 | 26 | 8 | 0 | 6 | 0 | 0 |
| `game` | 36 | 10 | 24 | 0 | 2 | 0 | 0 |
| `canvas` | 30 | 1 | 6 | 8 | 0 | 0 | 15 |
| `download` | 24 | 5 | 12 | 5 | 2 | 0 | 0 |
| `panel` | 20 | 11 | 8 | 0 | 1 | 0 | 0 |
| `sticker` | 20 | 1 | 6 | 12 | 1 | 0 | 0 |
| `main` | 19 | 9 | 0 | 1 | 9 | 0 | 0 |
| `user` | 16 | 13 | 0 | 0 | 3 | 0 | 0 |
| `stalker` | 14 | 1 | 11 | 0 | 0 | 2 | 0 |
| `store` | 14 | 8 | 4 | 0 | 2 | 0 | 0 |
| `info` | 13 | 4 | 8 | 0 | 1 | 0 | 0 |
| `clan` | 9 | 9 | 0 | 0 | 0 | 0 | 0 |
| `primbon` | 8 | 0 | 8 | 0 | 0 | 0 | 0 |
| `random` | 6 | 1 | 4 | 0 | 1 | 0 | 0 |
| `cecan` | 6 | 6 | 0 | 0 | 0 | 0 | 0 |
| `vps` | 6 | 0 | 6 | 0 | 0 | 0 | 0 |
| `asupan` | 4 | 2 | 2 | 0 | 0 | 0 | 0 |
| `religi` | 4 | 1 | 3 | 0 | 0 | 0 | 0 |
| `anime` | 3 | 0 | 1 | 0 | 2 | 0 | 0 |
| `maker` | 2 | 0 | 0 | 2 | 0 | 0 | 0 |
| `downloader` | 2 | 0 | 2 | 0 | 0 | 0 | 0 |
| `islamic` | 2 | 0 | 2 | 0 | 0 | 0 | 0 |
| `media` | 2 | 2 | 0 | 0 | 0 | 0 | 0 |
| `nsfw` | 2 | 0 | 0 | 0 | 0 | 0 | 2 |
| `tts` | 2 | 0 | 1 | 1 | 0 | 0 | 0 |
| `convert` | 1 | 0 | 0 | 1 | 0 | 0 | 0 |
| `ephoto` | 1 | 0 | 1 | 0 | 0 | 0 | 0 |
| `berita` | 1 | 0 | 1 | 0 | 0 | 0 | 0 |
| `jpm` | 1 | 0 | 0 | 0 | 0 | 0 | 1 |
| `linode` | 1 | 0 | 1 | 0 | 0 | 0 | 0 |
| `pushkontak` | 1 | 0 | 0 | 0 | 0 | 0 | 1 |
| `stalker␠␠` | 1 | 0 | 1 | 0 | 0 | 0 | 0 |
| `utility` | 1 | 0 | 0 | 0 | 1 | 0 | 0 |

> Kategori diambil dari `pluginConfig.category`, sehingga beberapa nama tidak sama dengan direktorinya (mis. `downloader`, `linode`, `maker`, `berita`, `cecan`). `stalker␠␠` adalah nilai mentah dengan trailing space di sumber — dipertahankan apa adanya.

## Kelas F — menunggu keputusan Oktz (20)

Semuanya ber-`awaitingDecision: true`. **Tidak ada yang diputuskan di dokumen ini.**

| File | Alasan |
|---|---|
| `plugins/canvas/fakebankjago.js` | canvas/artefak palsu (rekening/saldo/chat palsu) untuk menipu |
| `plugins/canvas/fakecall.js` | fake call (penipuan visual) |
| `plugins/canvas/fakedana.js` | canvas/artefak palsu untuk menipu |
| `plugins/canvas/fakedev7.js` | canvas/artefak palsu untuk menipu |
| `plugins/canvas/fakegopay.js` | canvas/artefak palsu untuk menipu |
| `plugins/canvas/fakeml.js` | canvas/artefak palsu untuk menipu |
| `plugins/canvas/fakeovo.js` | canvas/artefak palsu untuk menipu |
| `plugins/canvas/fakestory.js` | canvas/artefak palsu untuk menipu |
| `plugins/canvas/fakestory2.js` | canvas/artefak palsu untuk menipu |
| `plugins/canvas/fakestory3.js` | canvas/artefak palsu untuk menipu |
| `plugins/canvas/fakestory4.js` | canvas/artefak palsu untuk menipu |
| `plugins/canvas/iqc.js` | canvas/artefak palsu untuk menipu |
| `plugins/canvas/iqc2.js` | canvas/artefak palsu untuk menipu |
| `plugins/canvas/iqcpink.js` | canvas/artefak palsu untuk menipu |
| `plugins/canvas/ttqc.js` | canvas/artefak palsu untuk menipu |
| `plugins/jpm/jpm.js` | kategori `jpm/*` (kiriman massal / bom chat) |
| `plugins/nsfw/nsfw.js` | kategori `nsfw/*` |
| `plugins/nsfw/paptt.js` | kategori `nsfw/*` |
| `plugins/pushkontak/pushkontak.js` | kategori `pushkontak/*` (broadcast kontak massal) |
| `plugins/tools/spamngl.js` | `tools/*` yang bisa disalahgunakan (hack/crack/nmap/ddos/spam/phishing) |

### Kandidat F yang sengaja TIDAK ditandai (allowlist)

Diperiksa manual karena keyword-nya mencurigakan, ternyata aman:

- `plugins/tools/phisingataubukan.js` — presentarse mencurigakan, tetapi ini **pemeriksa URL phishing** (defensif): ia memindai URL dan melaporkan `is_phishing`. Bukan alat serangan → **B**.
- `plugins/tools/carbon.js` — the string `"Hack"` is a **font family name** in the font list, not a hacking tool → **A**.
- `plugins/tools/lookup.js` — matches only because it queries `api.hackertarget.com` → **B**.

## Kelas E — endpoint mati (5)

Diverifikasi langsung terhadap jaringan pada 2026-10-10, bukan asumsi.

| File | Alasan |
|---|---|
| `plugins/ai/text2img.js` | `firefly.maiku.my.id` — Cloudflare **HTTP 530**, body memuat `error code: 1033` (Argo Tunnel: origin down) |
| `plugins/group/autoai.js` | `firefly.maiku.my.id` — 530 / 1033 |
| `plugins/search/pinvid.js` | `firefly.maiku.my.id` — 530 / 1033 |
| `plugins/stalker/igstalk.js` | `firefly.maiku.my.id` — 530 / 1033 |
| `plugins/stalker/ytstalk.js` | `firefly.maiku.my.id` — 530 / 1033 |

Bukti verifikasi:

```
$ curl -o /dev/null -w '%{http_code}' https://firefly.maiku.my.id/
530
$ curl -s https://firefly.maiku.my.id/ | grep -oE 'error code: [0-9]+'
error code: 1033
```

> **Catatan `ssyoutube.com`:** spesifikasi menyebutnya sebagai kandidat E, tetapi grep menunjukkan **tidak ada satu pun file `plugins/**/*.js` yang menyebut `ssyoutube.com`**. Host itu hanya ada di `src/scraper/yt.js`, dan **tidak ada plugin yang meng-import helper tersebut** (`grep -rl "scraper/yt" plugins/` → kosong). Jadi `ssyoutube` tidak menghasilkan kelas E untuk plugin mana pun. Perlu diperiksa terpisah saat porting `src/scraper/yt.js`.

## `needsRuntimeConfig: true` (41)

Config tidak bisa di-resolve tanpa eksekusi runtime. **Tidak di-eval.**

**Kelompok 1 — `games.createPlugin(...)` (23 file, config di-generate runtime):**
`asahotak`, `caklontong`, `kataacak`, `riddle`, `siapakahaku`, `susunkata`, `tebakbendera`, `tebakdrakor`, `tebakepep`, `tebakfilm`, `tebakgambar`, `tebakhewan`, `tebakjkt48`, `tebakkalimat`, `tebakkata`, `tebakkimia`, `tebaklagu`, `tebaklirik`, `tebakkakanan`, `tebaknegara`, `tebakprofesi`, `tebaktebakan`, `tekateki` — semuanya di `plugins/game/`.
(Catatan: spesifikasi menyebut `plugins/game/tebak*.js`; faktanya 23 file yang memakai `games.createPlugin`, hanya 16 yang bernama `tebak*`, 7 lainnya bernama lain.)

**Kelompok 2 — field config non-literal (18 file):**

| File | Alasan |
|---|---|
| `plugins/convert/audiofx.js` | `alias` dibangun dari loop |
| `plugins/group/giveaway.js` | config dinamis |
| `plugins/media/mengkane.js` | config dinamis |
| `plugins/media/music.js` | `alias` dibangun dari loop (`MUSIC_LIST`) |
| `plugins/nsfw/nsfw.js` | config dinamis |
| `plugins/owner/eval.js` | **`customTrigger` adalah function** + config dinamis |
| `plugins/panel/cadmin.js` | config dinamis |
| `plugins/panel/createserver.js` | config dinamis |
| `plugins/panel/deladmin.js` | config dinamis |
| `plugins/panel/delserver.js` | config dinamis |
| `plugins/panel/gcseller.js` | config dinamis |
| `plugins/panel/listadmin.js` | `name`/`alias` dari `VALID_SERVERS.map(...)` |
| `plugins/panel/listserver.js` | config dinamis |
| `plugins/panel/listuser.js` | config dinamis |
| `plugins/panel/rolemanager.js` | config dinamis |
| `plugins/panel/serverinfo.js` | config dinamis |
| `plugins/random/anime.js` | `name` pakai spread array |
| `plugins/vps/createvps.js` | config dinamis |

`customTrigger` di `plugins/owner/eval.js` ditulis sebagai string `"<function>"` — **tidak di-eval**.

## Beban port Rust

- **D (103)** adalah blokir utama: `usesInteractive` 50, `usesNewsletter` 39, `usesRelay` 31, `usesWaUpload` 27, `usesGroupMeta` 10, `usesJadibot` 9.
- **B (262)** butuh transport HTTP + daftar 89 host; teratas: `cdn.nekohime.site` (36), `kappa.lol` (36), `leopard.hosting.pecon.us` (36), `litterbox.catbox.moe` (36), `uguu.se` (36), `upload.ee` (36), `api.cuki.biz.id` (24), `api.nexray.eu.cc` (18).
- **A (396)** adalah target pertama yang masuk akal — hampir separuh repo tanpa dependency eksternal.
- Semua plugin ber-`portStatus: "pending"`.

## Reproduksi

```bash
node scripts/extract-plugin-manifest.mjs   # exit 0
```

Deterministik: tanpa timestamp clock selain `generatedAt`, tanpa eval, tanpa eksekusi modul plugin.