# Plugin terblokir — daftar dan alasannya

Disusun dari laporan batch agent porting (B01–B09). Acuan: file JS masing-masing.
Daftar ini bukan belgekan: plugin di sini **tidak bisa** di-port tanpa menambah
kapabilitas di `oktz-plugin-api` atau crate inti. Kalau salah satu kapabilitas di bawah
ini, daftar ini ikut susut, daftar ini ikut susut.

## 1. Unduh media pesan masuk (blocking terbesar) — SELESAI 2026-10-16

Dibuka oleh `Ctx::media` + `Ctx::media_fetch` (`MediaFetch`), bukan `Ctx::download_media`
saja. `Ctx` sekarang punya:

- `media: Option<IncomingMedia>` — padanan `m.isMedia`, `m.isImage`, `m.mimetype`,
  `m.fileName`, dan caption pesan pemicu. `Ctx::is_media()`, `Ctx::media_type()`
  (`"imageMessage"` dst), `Ctx::is_media_kind(…)`.
- `media_fetch: Arc<dyn MediaFetch>` — `await m.download()` dan
  `await m.quoted.download()` (`serialize.js:1669`, `:543`). Default-nya
  `NoMediaFetch` yang menolak dengan alasan jelas; implementasi nyata
  `oktz_wa::media_fetch::WaMediaFetch` memakai `Client::download` atas media
  pemicu + pesan yang di-reply.

Dua hal yang **tidak** ikut (sengaja, dicatat):

1. `m.download(filename)` yang menulis ke `storage/temp` — butuh `fs` (§4).
2. Retry `sock.updateMediaMessage` saat CDN menjawab 400/404/410/428/429/500/870
   (`serialize.js:1684-1707`). Tidak ada re-enkripsi sisi klien di Rust, jadi
   kegagalan CDN langsung jadi `Err`.

Efek samping: `Ctx` gained dua field, jadi `Caller` di `oktz-router` ikut
membawanya (partial-eq `Caller` sengaja dihapus — `Arc<dyn MediaFetch>` tidak
punya identitas bermakna). Semua `Ctx { .. }` di test tetap jalan karena
`Default` diisi `media: None` + `NoMediaFetch`.

Plugin yang sekarang bisa lanjut: 14 file `ai/to*.js`, `canvas/*` yang butuh
media pemicu, `owner/ganti-*` (15 file), `anime/autoanimewinbu.js`.

## 2. `sock.*` (profile picture, `sendMedia`, `getName`, `sendImageAsSticker`)

Tidak ada jalur equivalent di `Pipeline` sekarang. `Reply` hanya punya
`Image`, `Sticker`, `Interactive`, teks.

File: `canvas/fakecall.js`, `canvas/fakeml.js`, `canvas/fakestory*.js`,
`canvas/gura.js`, `canvas/igstory*.js`, `canvas/musiccard.js`,
`canvas/profileig.js`, `canvas/sroast.js`, `canvas/ttqc.js`,
`convert/audiofx.js`, `download/aio.js`.

## 3. Varian `Reply` yang belum ada

Butuh varian baru di enum `Reply`, bukan cuma isi:
`Document` (`download/alightmotion.js`), `Audio` (`convert/audiofx.js`,
`download/aio.js`), `Video` dengan `gifPlayback` (`cek/cekfemboy.js`,
`asupan/*.js`).

## 4. `fs` / `path` / `node:https` (I/O file dan unduh manual)

File lokal, state file, dan fetch tanpa `HttpGet`.

File: `ai/dpsteai.js`, `ai/txt2img2.js` (state queue),
`asupan/asupan.js`, `asupan/asupantiktok.js`, `asupan/bocil.js`,
`asupan/ukhty.js`, `canvas/iqcpink.js`, `canvas/kalender.js`,
`canvas/ttqc.js`, `cek/cekfemboy.js`, `convert/audiofx.js`,
`anime/autoanimewinbu.js`.

## 5. `@napi-rs/canvas` + `sharp` + `GlobalFonts` (render gambar)

Tidak ada toolchain graphics di workspace dan tidak boleh menambah dependency
baru tanpa keputusan.

File: `canvas/fakedev7.js`, `canvas/fakestory*.js`, `canvas/igstory*.js`,
`canvas/iqcpink.js`, `canvas/kalender.js`, `canvas/sroast.js`,
`canvas/ttqc.js`, `canvas/topixel.js`, `canvas/facepalm.js`,
`canvas/buatquotes.js`.

## 6. `queueFFmpeg` (GIF→MP4, audio effect)

`cek/cekfemboy.js`, `convert/audiofx.js`.

## 7. `config.APIkey.*` dan `config.saluran`

Jalur baca config runtime sudah ada: `Ctx.config()` mengembalikan
`oktz_core::config::Runtime`, cerminan `config.js` untuk kunci yang dibaca
plugin (`info`, `owner`, `bot`, `assets`, `sticker`, `payment`,
`donasi`, `saluran`, `api_keys`). Produksi memasangnya lewat
`Pipeline::with_config`, dipanggil `wiring_from_with_config` di
`connect_device`. Default-nya `Runtime::default()` yang SEMUA stringnya
kosong, bukan nilai produksi, supaya `Ctx::default()` di test tidak bisa
menarik API key sungguhan dari environment.

Yang tersisa per file:
- `ai/txt2img.js`, `ai/anime-gen.js`, `info/jadwalbola.rs` (pola sama),
  `canvas/watercolortext.rs`, `canvas/pakustad.rs`,
  `asupan/asupantiktok.js` — belum rewiring ke `ctx.config()`.
- `download/aio.js` (`saluranCtx()`) masih butuh `sock.sendMessage` ke
  newsletter (§2).

## 8. `games.createPlugin` — 27 plugin `game/*`

Semua file `plugins/game/*` yang cuma memanggil
`games.createPlugin(...)` (lihat daftar di `PLUGIN-CLASSIFICATION.md`).
`handler` dan `answerHandler` adalah closure dari `src/lib/games.js:84-299`,
yang belum di-port. Selain itu butuh `fs`, `sharp`, `sock.sendPreview`,
`setSessionTimer`, data JSON `src/data/*.json`, dan `src/lib/game-data.js`,
`database.js`, `level.js`, `context.js`.

Masalah arsitektur `PluginEntry` cuma punya satu `handler` sudah tertutup:
`oktz-plugin-api::registry` kini punya `ANSWER_HANDLERS` (distributed slice
`linkme` kedua), macro `#[answer_handler]` untuk mendaftarkan fungsi
`fn(Ctx) -> Option<Reply>`, dan `run_answer_handlers` yang berhenti di `Some`
pertama. `oktz_router::Router::answer` membangun `Ctx` lewat `build_ctx` yang
sama dengan dispatch command, dan `oktz-wa::pipeline` memanggilnya untuk pesan
yang bukan command. Yang tersisa untuk port isi: `fs`, `sharp`,
`sock.sendPreview`, `setSessionTimer`, data JSON `src/data/*.json`, dan
`src/lib/game-data.js`, `database.js`, `level.js`, `context.js`.

Catatan parity: `handleGameAnswer` di JS dijaga predicate
`hasActiveSession`/`hasSuitGame`/`hasTTTGame`/`hasUTGame`
(`src/handler.js:1294-1304`). Predicate itu belum ada di pipeline Rust, jadi
rantai answer handler dipanggil untuk setiap pesan non-command dan masing-masing
handler yang memutuskan sendiri; ini ditandai `ponytail:` di `pipeline.rs`.

Nilainya sudah didokumentasikan di laporan batch B09 (alias, description,
`timeout`, `hasImage`, `questionField`, `answerField`, `hintCount`) supaya
nanti tidak perlu baca ulang.

## 9. `answerHandler` di luar game

`ai/characterai.js` (`caiAnswerHandler`/`caiChatHandler` dipanggil untuk semua
pesan biasa) dan `anime/autoanimewinbu.js`.

Jalur pemanggilannya sudah ada (§8): entry point kedua `answerHandler` bisa
didaftarkan dari crate plugin mana pun dengan `#[answer_handler]`. Yang belum
ada adalah isi kedua plugin itu sendiri.

## 10. `AIRich` dan pesan kaya builder

`ai/ai.js` memakai `AIRich` dari `src/lib/builder.js` (`addTable`, `addCode`,
`addText`) lalu mengirim lewat `relayMessage`. `Reply` cuma satu payload.

## 11. Scraper yang tertinggal

Semua 59 file `src/scraper/*.js` sudah punya padanan di
`oktz-scraper/src/` (dicek ulang: `lib.rs` mendaftarkan 59 modul).

`info/berita.js` dan `islamic/quran.js` tetap terblokir, tapi bukan karena
scrapernya belum ada — keduanya butuh parse HTML/XML `cheerio` yang tidak
punya padanan di crate ini dan tidak boleh menambah dependency baru. Dua plugin
itu belum di-port karena itu.

Kapabilitas yang masih hilang di dalam scraper yang SUDAH ada tercatat di
bagian lain: dekripsi AES untuk `imgdrop`, `ffprobe`/`ffmpeg` plus berkas
sementara untuk `youtube` (SaveTube dan QByte), `sharp` untuk
`removebackground`, dan tulis/hapus berkas temporer untuk `mconverter`,
`hdvid`, `hdvid2`.

## 12. Album dan pesan asosiasi

`anime/mywaifu.js` butuh `albumMessage`, `messageAssociation`,
`sock.relayMessage`, `sock.waUploadToServer`, dan `crypto.randomBytes(32)`
untuk `messageSecret`.

## 13. Unggah biner ke host pihak ketiga

`uploadImage`, `uploadBuf`, `ImageUploadService("pixhost.to")`,
`uploadTo0x0`, `live3d`. `HttpGet` tidak punya PUT dan `HttpBody` tidak punya
nama file untuk multipart.

## 15. `saluranCtx()` — badge newsletter

`_saluranCtx()` (`src/lib/context.js:50-62`) menempelkan
`forwardingScore: 9`, `isForwarded: true`, dan
`forwardedNewsletterMessageInfo` ke `contextInfo` tiap balasan. Di WhatsApp
ini tampil sebagai badge "Newsletter" — jadi bedanya terlihat user, bukan
internal. `Reply` sekarang tidak punya jalur context sama sekali.

Butuh dua kapabilitas sekaligus, bukan salah satu:
1. `Reply` baru (atau field contextInfo) yang bisa membawa data forwarding.
2. `Ctx::bot_number` — `m.botNumber` dipakai `fun/tolak.js:53` dan
   `fun/terima.js:53` untuk menolak "pacaran" dengan bot sendiri. Router
   sudah punya `Env::bot_number`, tapi tidak diteruskan ke plugin.

Contoh plugin yang paling dekat bisa di-port begitu dua hal ini ada:
`fun/terima.js`, `fun/tolak.js` (keduanya cuma tersisih `global.tembakSessions`
dan `saluranCtx`), lalu `fun/tembak.js` (masih butuh `answerHandler`).

## 16. Sinkron `mod.rs` dan `lib.rs`

`mod.rs` tiap kategori dan `lib.rs` crate `oktz-plugins` adalah milik
orkestrator. Agent porting tidak boleh menyentuhnya; daftar modul dibangun
dari isi folder.

## 17. Terblokir dari batch B12-B15

Dikumpulkan dari laporan agent, diacukan ke file JS masing-masing.

| Plugin | Kapabilitas yang hilang |
|---|---|
| `ai/musicmaker` | `Reply` tidak punya varian `Audio` (§3), dan JS mengirim **dua pesan berurutan** (`sock.sendMessage` audio lalu gambar) sedangkan satu `Reply` hanya membawa satu payload (§2) |
| `canvas/tiktokchat` | `downloadContentMessage` untuk pesan pemicu (§1), plus `HttpBody` multipart yang tidak punya `filename` padahal `form.append("avatar", buf, { filename })` (§13) |
| `fun/cekkhodam` | `sock.sendMedia(..., { type: 'audio' })` (§2/§3), `fs.writeFileSync`/`unlinkSync` + `path.join(process.cwd(), 'temp')` (§4), dan TTS dari `google-tts-api` yang tidak ada padanannya |
| `fun/confess` | `sock.onWhatsApp()` untuk verifikasi nomor, `sock.sendMessage()` ke JID pihak ketiga dengan `contextInfo.forwardingScore` (§15), `export { replyHandler }` sebagai entry point kedua (§9), `config.saluran.id`/`config.bot.name` (§7) |
| `download/dailymotiondl` | `src/scraper/dailymotion.js` = POST form + 5 header kustom; `HttpGet` tidak punya header kustom. Perlu `child_process`/ffmpeg + `fs` + `sock.sendMessage` |
| `download/terabox` | `src/scraper/terabox.js` = parse token CSRF dari HTML lalu POST JSON dengan `X-CSRF-TOKEN`/`Cookie`; stub selalu `Err` |
| `download/pindl` | `src/capabilities/pinterest.js` belum di-port; perlu `queueFFmpeg` + `fs.statSync/readFileSync/writeFileSync/unlinkSync/renameSync` + `sock` |
| `download/sfiledl` | `src/capabilities/sfile.js`: scraper lokal butuh `cheerio`, aggregator `neoxr` butuh `config.APIkey.neoxr` (§7) |
| `download/videy` | `src/capabilities/videy.js`, alasan sama seperti `sfiledl` |
| `fun/tembak`, `fun/terima`, `fun/tolak` | `answerHandler` (§9) dan `saluranCtx()` + `Ctx::bot_number` (§15) |

## 17b. Terblokir dari batch B12-B15 (gelombang kedua)

Laporan agent B13-B15, diacukan ke file JS masing-masing.

| Plugin | Kapabilitas yang hilang |
|---|---|
| `main/menu` | `sock.relayMessage` (7 titik, case 7 mengirim **dua** pesan berurutan), `sock.waUploadToServer`, `prepareWAMessageMedia`/`generateWAMessageFromContent`/`wrapInteractive`, `getAssetBuffer`, `fs.readFileSync`, `sharp`, `getCasesByCategory`, `config.saluran`. Default `menuVariant = 2` = kartu interaktif, bukan teks |
| `main/menucat` | `sock.relayMessage` (5 titik), `prepareWAMessageMedia` + `wrapInteractive`, `getAssetBuffer`, `fs.readFileSync(config.assets["mp4"])`, `getCasesByCategory`. Default `menucatVariant = 2` |
| `main/ping` | `@napi-rs/canvas` `createCanvas` + seluruh `ctx.createLinearGradient/arc/fillText/measureText`, `sock.sendMessage`, modul `os` |
| `main/ping2` | `sock.sendTable` — butuh varian `Reply` tabel yang belum ada; `execSync` (`wmic`/`df -k`), modul `os`, akses `getDatabase().data.users/groups` mentah |
| `main/sc` | `sock.relayMessage` + `wrapInteractive` dengan `nativeFlowMessage.buttons` `cta_url`. `Reply::Interactive` merender `Card::new(body)` tanpa header/footer/tombol |
| `main/stats` | `sock.sendTableV2` — varian `Reply` tabel belum ada |
| `main/totalfitur` | `sock.sendTable` — sama seperti `stats` |
| `main/stopjadibot` | `src/lib/jadibot-manager.js`: `isJadibotActive`/`getJadibotStatus` membaca `Map` in-process, `stopJadibot` menutup socket + `clearInterval` |
| `main/system` | modul `os` (`totalmem/freemem/cpus/hostname/platform/arch/uptime`) dan `child_process.exec` untuk `df -h /` + `wmic` |
| `owner/addowner` | `fs`/`path` menulis balik `config.js` (`savePanelConfig`), `config.pterodactyl.*`, `src/lib/lid.js`, `src/lib/jadibot-database.js`, `getGroupMode` |
| `owner/addplugin` | `fs`/`path` (`mkdirSync`/`writeFileSync`/`existsSync`), `hotReloadPlugin` (`src/lib/plugins.js`), `quoted.download()` |
| `owner/addprem` | `db.data.premium` adalah store atas terpisah (`premium.json`), tidak ada di `Ctx` maupun di enum `Mutation` (hanya `SetUser`/`SetGroup`/`SetSetting`); cabang `listprem` seluruhnya bergantung padanya. Plus `jadibot-database.js`, `db.updateExp`/`updateKoin`, `config.energi.premium` |
| `owner/addsewa` | `sock.groupGetInviteInfo`, `sock.groupMetadata`, `sock.groupAcceptInvite`, `sock.sendText` + `saluranCtx()`, `db.db.data.sewa` |
| `owner/akses` | `db.getUser(jid)` untuk JID **arbitrer** (bukan sender) di ketiga cabang add/list/del; plus `sock.parseMention()` dan `sock.sendMessage()` dengan tombol `listMessage` |
| `owner/backupsc` | `fs` (`readdirSync/statSync/createWriteStream/readFileSync/unlinkSync`), `archiver` (zip), `process.cwd()` |
| `owner/bcgc` | enumerasi grup (`sock.groupFetchAllParticipating`), unduh media (`qmsg.download()`), fan-out per grup (`sock.sendMessage`), `global.statusBcgc`/`stopBcgc`, jeda `setTimeout` |
| `owner/bcpc` | `sock.store.chats/messages/contacts`, `sock.sendMedia`/`sock.sendText`, `qmsg.download()` |
| `owner/cekschedule` | `getFullSchedulerStatus()` (`src/lib/scheduler.js:291`) membaca `activeCronJobs` + `scheduledTasks` in-memory, dan `db.getStats()` — fungsi server |
| `owner/clearsessions` | `fs.*` pada `storage/sessions/`. `ctx.sessions` adalah map in-memory yang **berbeda** — memetakannya berarti mengarang perilaku |
| `owner/colongpp` | `sock.profilePictureUrl()`, `sock.updateProfilePicture()`, resize gambar `@napi-rs/canvas` |
| `owner/delplugin` | `fs.readdirSync/unlinkSync` + `unloadPlugin()` (hot-reload registry) |
| `owner/delsewa` | `db.db.data.sewa` (store atas `sewa.json`; `Mutation` tidak punya jalannya), `db.db.write()`, `sock.groupGetInviteInfo()`, `sock.groupLeave()`, `sock.sendText()` |
| `owner/disableplugin`, `owner/enableplugin` | `fs.readdirSync/readFileSync/writeFileSync` + `await import()` dinamis; tujuannya menulis ulang `isEnabled` di sumber plugin |
| `owner/eval`, `owner/exec` | `eval("(async () => { ... })()")`. `eval` juga memakai `customTrigger: body.startsWith('=>')` + `noPrefix` — `PluginMeta::custom_trigger` ada tapi macro selalu `None` dan router belum mendukung trigger tanpa prefix |

Pola yang berulang dan murah dibuka: **`sock.sendTable`/`sendTableV2`** (3 file)
dan **`Reply::Interactive` yang bisa membawa header/footer/buttons** (1 file).
Menutup salah satunya langsung membuka `stats`, `ping2`, `totalfitur`, `sc`.

## 17c. Terblokir dari batch B12-B15 (gelombang ketiga)

Laporan agent B12-B15, diacukan ke file JS masing-masing. 45 plugin, 4 di
antaranya berhasil di-port (`group/acc`, `autoreply`, `warn`, `welcome`).

### `game/*` — 4 file, TIDAK memakai `games.createPlugin`

| Plugin | Kapabilitas yang hilang |
|---|---|
| `game/fisch` | `sock.sendPreview` (§2), `getAssetBuffer`, lib `src/lib/fisch.js` + `fisch-data.js`, `config.info.website` (§7), `db.db.data.users` mentah + `db.markDirty`, `setTimeout` antar pesan |
| `game/minecraft` | sama seperti `fisch`, lib `src/lib/minecraft.js` + `minecraft-data.js` |
| `game/ulartangga` | §2 `sendPreview`, board `drawBoard` + `DICE_STICKERS` (`src/lib/game-ulartangga.js`), **`answerHandler` "kocok"** (§9), `config.saluran` (§15), `db.updateKoin`/`updateEnergi`, kirim sticker |
| `game/werewolf` | §2 `sendPreview` + fan-out PM ke JID arbitrer, timer fase 60-90 detik (`setTimeout`/`clearTimeout`), `config.saluran` (§15), `db.updateKoin` |

### lainnya — 41 file

| Plugin | Kapabilitas yang hilang |
|---|---|
| `fun/sulap` | `export replyHandler` = entry point kedua (§9), sesi berkunci **id pesan terkirim**, `groupParticipantsUpdate` (§2), `sock.user.id`, 3 pesan berurutan + `sleep(2000)` |
| `group/add` | `groupGetInviteInfo`, `groupMetadata` JID arbitrer, `groupParticipantsUpdate` (§2), `sock.user.id`; teks gerbang butuh `subject` yang tidak ada di `GroupRoster` |
| `group/cekidgc` | field metadata (subject/creation/owner/desc/restrict/announce/joinApprovalMode) tidak ada di `GroupRoster`, `sharp` (§5), `profilePictureUrl` (§2), `prepareWAMessageMedia`+`relayMessage`+tombol `cta_copy`, `config.saluran` (§15) |
| `group/cekonline` | `sock.presenceSubscribe` + `sock.ev.on('presence.update')` (§2), tunggu 5 detik, `groupMetadata.subject` |
| `group/groupinfo` | field metadata teks utama tidak ada di roster, `profilePictureUrl` (§2), `src/lib/lid.js` (`resolveAnyLidToJid`), axios unduh PP |
| `group/tam` | `pollResultSnapshotMessage` via `relayMessage` — varian `Reply` poll belum ada + `saluranCtx()` (§15) + `db.getUser(jid)` arbitrer |
| `info/gag2` | mode `watch` = 5 iterasi x 30 detik + minimal 2 pesan berurutan; `HttpGet` tidak punya header kustom |
| `info/infotourney` | scraping `infotourney.com` dengan **cheerio** (§11), kirim via `sock.sendMedia` type image (§2) |
| `islamic/murrotal` | cheerio untuk parse daftar surah (§11) + `sock.sendMedia` type **audio** (§2/§3) |
| `jpm/jpm` | `sock.relayMessage` + `prepareWAMessageMedia`/`generateWAMessageFromContent`/`proto`, `sock.query` IQ newsletter + `getBinaryNodeChild`, `fs`/`path`, `getAssetBuffer`, `config.bot`/`config.saluran`/`config.owner`, `saluranCtx()` (§15), `fetchGroupsSafe`, `qmsg.download()` (§1), scheduler `src/lib/auto-jpm.js`, state `global.statusjpm`/`stopjpm`, `setTimeout` fan-out |
| `main/allmenu` | `sock.relayMessage` (varian 2/5/6), `prepareWAMessageMedia`, `getAssetBuffer`, `fs.readFileSync(config.assets)`, `sharp`, `getCasesByCategory`/`getCaseCount`, `config.saluran`/`config.bot`/`config.assets`, ffmpeg audio + `child_process.spawn`. Keluarga `main/menu` (§17b) |
| `main/carifitur` | `loadAllPlugins()` membaca folder plugins dari disk (`fs.readdirSync` + `import()` dinamis) dan menambah command dari `getCaseCommands()` (`src/case/foto.js`); hasil via `sock.sendButton` + `getAssetBuffer("foto")`. Port dengan registry Rust akan mengubah hasil |
| `main/donasi` | seluruh jalur output lewat `sock.sendButton` (`cta_copy`) / `interactiveButtons` + `contextInfo` forwarding (§2/§15); isinya dari `config.donasi`/`config.bot`/`config.owner`/`config.saluran` (§7) |
| `main/jadibot` | `startJadibot`/`isJadibotActive` dari `src/lib/jadibot-manager.js` (socket WA kedua in-process). Keluarga `main/stopjadibot` (§17b) |
| `main/leaderboard` | membaca **seluruh store users** (`db.data.users` / `db.getAllUsers()`); `Ctx` hanya punya user sender + roster. Cabang default via `sock.sendButton` + `getAssetBuffer('foto')` (§2) |
| `owner/addowner` | (§17b) `fs`/`path` menulis balik `config.js`, `config.pterodactyl.*`, `src/lib/lid.js`, `jadibot-database.js`, `getGroupMode` |
| `owner/ganti-asset` | keluarga `ganti-*` di bawah + entry point kedua `gantiAssetAnswerHandler` (§9) + `config.assets` (§7) |
| `owner/ganti-namabot` | `fs.readFileSync/writeFileSync` `config.js` + regex ubah `bot.name`; baca `config.bot?.name` (§4/§7) |
| `owner/ganti-namadev` | idem, regex `developer`; baca `config.bot?.developer` (§4/§7) |
| `owner/ganti-namaowner` | cabang `main`: `fs` baca/tulis `config.js` regex `owner.name`; baca `config.owner?.number?.[0]`/`config.owner?.name`; `getOwnerName()` (`config.js:537-552`) |
| `owner/ganti-large` | unduh media pemicu (`m.isImage`/`m.isMedia` + `download()`) (§1), lalu `fs.existsSync/mkdirSync/writeFileSync` menulis 3 berkas ke `assets/images/` (§4) |
| `ganti-levelup`, `ganti-mp3`, `ganti-mp4`, `ganti-pp-kosong`, `ganti-promote`, `ganti-rpg`, `ganti-rules`, `ganti-store`, `ganti-v8`, `ganti-welcome`, `ganti-winner` | `m.download()`/`m.quoted.download()` + cek `m.isImage`/`isAudio`/`isVideo` (§1); `updateAssetUrl` (`src/lib/uploader.js:62-115`) baca/tulis `config.assets` + `fs.writeFileSync(config.js)` via regex + `fs.writeFileSync` berkas asset (`src/lib/asset-manager.js:99-111`) (§4/§7); `sharp` resize 1280x720 (§5, dilewati untuk `.mp3`/`.mp4`) |
| `owner/ganticode` | `quoted.download()` (§1), `fs` rekursif `plugins/` + backup + `writeFileSync` (§4), `hotReloadPlugin` (`src/lib/plugins.js`) |
| `owner/gantiscraper` | `fs` (`readdirSync`/`readFileSync`/`writeFileSync`/`copyFileSync`/`unlinkSync`/`statSync`) (§4), `quoted.download()` (§1), `hotReloadPlugin`, dan **`import()` dinamis** dengan cache-buster `?t=${Date.now()}` |
| `owner/get` | `HttpGet` hanya punya `get`/`post_json`/`post_form`/`post_multipart` — JS butuh method arbitrer (PUT/DELETE/PATCH/HEAD/OPTIONS), header kustom, basic auth, timeout, maxRedirects; relay respons butuh varian `Reply` **Document**, **Audio**, **Video** dengan `gifPlayback` (§3) + kirim document dengan `fileName` |
| `owner/getplugin` | `fs` (`readdirSync`/`statSync`/`readFileSync`/`existsSync`, scan dua pass nama+alias) (§4), kirim source sebagai document `.js` — `Reply::Document` dengan `fileName` belum ada (§3) |
| `owner/hapusdata` | `db.resetToDefaults()` — fungsi server yang menulis file + backup (`database.js:788`), enumerasi store atas `stats`/`sewa`/`premium`/`owner`/`partner`, `interactiveButtons`, `Buffer.byteLength(JSON.stringify(store))` atas seluruh store |
| `owner/join` | `sock.groupGetInviteInfo`, `sock.groupAcceptInvite`, `saluranCtx()` (§15), jeda `setTimeout` antar join |
| `owner/leave` | `sock.groupGetInviteInfo`, `sock.groupLeave`, `config.saluran` (§7), contextInfo newsletter `forwardingScore: 9999` (§15), flag `global.sewaLeaving` |
| `owner/listjadibot` | `getAllJadibotSessions`/`getActiveJadibots` (jadibot-manager, Map in-process + session di disk), `interactiveButtons`, mentions session JID |
| `owner/listjadibotaktif` | `getActiveJadibots` (jadibot-manager), `interactiveButtons` |
| `owner/listsewa` | `db.db.data.sewa` — store atas `sewa.json`; `Mutation` tidak punya jalannya; plus `db.db.write()` |
| `owner/onlythisgrup` | `sock.groupInviteCode(m.chat)`, `sock.user.id`, `groupMetadata.subject`, `findParticipantByNumber` dari `src/lib/lid.js`. Cabang lock (fungsi utamanya) butuh 4 kapabilitas itu; port cabang unlock saja = port sebagian |
| `owner/payment` | `config.payment.*` (`methods`/`banks`/`qrisUrl`/`customText`), `config.bot?.name`, `config.owner?.name` — config runtime yang tidak dibawa `Ctx` (§7). Cabang kirim QRIS sudah tertutup `Reply::image`, hanya datanya yang tidak ada |
| `owner/ptvch` | `quoted.download()`/`m.download()` (§1), `config.saluran?.id` (§7), `sock.sendMessage` video + `ptv: true` ke JID channel pihak ketiga (§2/§3) |
| `owner/renewsewa` | `db.db.data.sewa` (baca+tulis per-grup), `sock.groupGetInviteInfo`, `sock.sendText` + `saluranCtx()` (§15) |
| `owner/resetdb` | `fs` backup `db.json` (`existsSync`/`copyFileSync`) (§4), bulk-clear store `users`/`groups`/`clans` (`Mutation` hanya per-JID; store `clans` tidak ada di `Ctx`), `db.save()` |

Kapabilitas pembuka terbesar dari gelombang ini:

1. ~~**Unduh media pesan pemicu**~~ — **selesai 2026-10-16**, lihat §1
   (`Ctx::media` + `Ctx::media_fetch`). Membuka 15 file `ganti-*` dan 14 file
   `ai/to*.js`.
2. **I/O `config.js` + config runtime** (`config.payment`, `config.donasi`,
   `config.owner`, `config.bot`, `config.assets`) — membuka 8 file.
3. ~~**`Reply::Document` dengan `fileName`**~~ — **selesai** (A2, bersama
   `Reply::Video`/`Reply::Audio`); membuka `getplugin` dan sebagian besar
   cabang output `get`.
4. **Store atas `sewa.json`** — membuka `listsewa`/`renewsewa`.
5. **Enumerasi seluruh store users** — membuka `leaderboard`.

`download/pixeldraindl` **tidak** terblokir: jalur HTTP-nya port penuh, hanya
`config.APIkey.neoxr` kosong — persis fallback `|| ''` di JS.

## 18. `m.pushName` tidak ada di `Ctx`

`te(prefix, command, pushName)` meng-substitusi `{pushName}` dengan nama push
pengirim; di Rust selalu jadi `User`, sama seperti `pushName || 'User'` ketika
nilainya kosong. Ini penyempitan yang diketahui, bukan penyimpangan port.

## 19. `config.errorTemplate` bukan kode mati yang boleh dipakai

`config.errorTemplate` (`config.js:153`) **selalu terisi**, jadi teks fallback
`☢ *ᴇʀʀᴏʀ*` di `src/lib/error.js` tidak pernah dipakai. Enam file Rust yang
memakai teks itu menyimpang dari JS; semuanya sudah dikoreksi ke helper tunggal
`common::error_template(prefix, command)`.

Catatan kecil: `filename.replace(/[<>:"/\\|?*]/g, "")` **menghapus** karakter,
bukan menggantinya. Helper-nya `common::scrub_nama_berkas`.

## 20. Dua jebakan verifikasi yang sudah memakan waktu (bukan blokir plugin)

Keduanya ditemukan saat gelombang scraper pertama, dan keduanya bisa
membuat hasil terlihat hijau padahal tidak:

1. **`r#"` yang ditutup dengan `".`** — file `oktz-scraper/src/sfiledl.rs:741`
   punya `r#"…html…".to_string()`. Penutupnya `"` biasa, bukan `"#`, jadi lexer
   membaca ~37 baris berikutnya (termasuk dua fungsi test) sebagai isi string.
   Akibatnya: type error di dalam tidak pernah muncul, `#[test]` di dalamnya
   tidak terdaftar, dan test yang terdaftar berjalan dengan body milik test
   lain. Tidak ada satu pun error kompilasi. Penjaga: `scripts/cek-raw-string.mjs`
   (mencari raw string yang isinya berisi `fn `, `#[test]`, `assert!`, `let `).
   Jalankan setelah setiap gelombang agent.
2. **Cargo kadang tidak membangun ulang unit test** di lingkungan ini: `cargo
   check`/`build` tetap mem-build ulang, tapi `cargo test` memakai binari lama
   sehingga test yang baru ditulis tidak muncul. Gejalanya "test saya tidak
   jalan". Hindari dengan menyentuh file sumber sebelum build:
   `touch rust/src/*.rs rust/lib/oktz-*/*.rs rust/lib/oktz-*/*/*.rs rust/plugin/src/*/*.rs`. Kalau hasilnya
   tetap janggal, buktikan dulu dengan `compile_error!` di dalam badan fungsi —
   kalau tidak muncul, itu bukan soal staleness, tapi jebakan no-1 di atas.

Aturan tambahan untuk agent: kalau jumlah `#[test]` pada disk berbeda dengan
hasil `cargo test -- --list`, itu gejala yang harus dilaporkan, bukan dibiarkan.

## 12. Stub yang dibuang pada gelombang agent pertama

Plugin berikut sempat dibuatkan berkas stub oleh agent porting (badannya cuma
pesan "belum tersedia"), lalu dibuang dari registrasi karena bukan paritas JS.
Port ulang setelah kapabilitasnya ada.

### system (`rust/plugin/src/main/system.rs`)

`plugins/main/system.js` — info sistem (RAM, CPU, Disk, Latency).

## BLOKIR — tidak di-port paksa.

Alasan: butuh `os` (`totalmem`, `freemem`, `cpus`, `uptime`, `platform`,
`arch`) dan `child_process` (`exec` untuk `df -h /` / `wmic`). Keduanya
adalah I/O sistem yang tidak tersedia di `Ctx` Rust (handler bebas I/O,
jaringan lewat `ctx.http` saja).

`ponytail:` kalau `Ctx` menambah field sistem nanti, port ulang dari
`plugins/main/system.js`.

### ganti-asset (`rust/plugin/src/owner/ganti-asset.rs`)

`plugins/owner/ganti-asset.js` — ganti asset secara interaktif.

NOTE: baris `pub mod ganti_asset;` perlu ditambah ke `owner/mod.rs`
secara manual (mod.rs dilarang disentuh).

BLOKIRAN: butuh `updateAssetUrl` dari `src/lib/uploader.js` (fs + HTTP
upload ke storage), serta `m.download()` / `m.quoted.download()` dan
`global.gantiAssetSessions` (state antar-pesan dengan `setTimeout`).
Kapabilitas ini belum ada di Rust API (`Ctx` tidak punya `sock`, dan
`MediaFetch` belum dipasang pipeline untuk owner). Jangan port paksa.

### ganti-demote.jpg (`rust/plugin/src/owner/ganti-demote.rs`)

`plugins/owner/ganti-demote.js` — ganti gambar demote.jpg.
BLOKIRAN: butuh kapasitas berkas (fs) + image processing (sharp) +
upload ke penyimpanan aset melalui `updateAssetUrl` (`src/lib/uploader.js`),
yang semua TIDAK tersedia di API Rust (`Ctx` tidak punya `sock`, tidak
ada `download_media` ke berkas aset, dan tidak ada crate sharp).
Tidak dapat dipaksa di-port. Buka lagi kalau fs + sharp + upload aset
tersedia. Jika dibuka, daftarkan di `owner/mod.rs` dengan path karena
nama berkas mengandung titik dan hubung:
`#[path = "ganti-demote.rs"] pub mod ganti_demote;`.

### ganti-foto3.jpg (`rust/plugin/src/owner/ganti-foto3.rs`)

`plugins/owner/ganti-foto3.js` — ganti gambar foto3.jpg.

## BLOKIR — tidak di-port paksa.

Alasan: `updateAssetUrl` di `src/lib/uploader.js` membutuhkan kapasitas
yang belum ada di `Ctx` Rust:
- `fs` (baca/tulis `config.js`, `mkdir`, `writeFileSync` file aset),
- `sharp` (resize + konversi JPEG),
- `updateAssetAndSave` dari `src/lib/asset-manager.js` (store I/O).
Tidak ada `MediaFetch`/filesystem yang cukup di `Ctx`, dan mengubah
`config.js` dari dalam plugin melanggar aturan file scope eksklusif.

`ponytail:` kalau `MediaFetch` + write-asset tersedia di `Ctx` nanti,
port ulang dari `plugins/owner/ganti-foto3.js` (download media, upload,
balas URL baru).

### ganti-goodbye.jpg (`rust/plugin/src/owner/ganti-goodbye.rs`)

`plugins/owner/ganti-goodbye.js` — ganti gambar goodbye.jpg.

NOTE: baris `pub mod ganti_goodbye;` perlu ditambah ke `owner/mod.rs`
secara manual (mod.rs dilarang disentuh).

BLOKIRAN: butuh `updateAssetUrl` dari `src/lib/uploader.js` (fs + HTTP
upload ke storage) serta `m.download()` / `m.quoted.download()`.
Kapabilitas belum ada di Rust API. Jangan port paksa.

### ganti-levelup.jpg (`rust/plugin/src/owner/ganti-levelup.rs`)

`plugins/owner/ganti-levelup.js` — ganti gambar levelup.jpg.
BLOKIRAN: butuh kapasitas berkas (fs) + image processing (sharp) +
upload ke penyimpanan aset melalui `updateAssetUrl` (`src/lib/uploader.js`),
yang semua TIDAK tersedia di API Rust. Tidak dapat dipaksa di-port.
Buka lagi kalau fs + sharp + upload aset tersedia. Jika dibuka, daftarkan
di `owner/mod.rs` dengan path karena nama berkas mengandung titik dan hubung:
`#[path = "ganti-levelup.rs"] pub mod ganti_levelup;`.

### ganti-pp-kosong.jpg (`rust/plugin/src/owner/ganti-pp-kosong.rs`)

`plugins/owner/ganti-pp-kosong.js` — ganti gambar pp-kosong.jpg.

## BLOKIR — tidak di-port paksa.

Alasan: sama dengan `ganti-foto3.js` — `updateAssetUrl` butuh `fs`,
`sharp`, dan modifikasi `config.js`, yang belum tersedia di `Ctx` Rust.

`ponytail:` kalau kapasitas itu ditambahkan nanti, port ulang dari
`plugins/owner/ganti-pp-kosong.js`.

### ganti-rpg.jpg (`rust/plugin/src/owner/ganti-rpg.rs`)

`plugins/owner/ganti-rpg.js` — ganti gambar rpg.jpg.

NOTE: baris `pub mod ganti_rpg;` perlu ditambah ke `owner/mod.rs`
secara manual (mod.rs dilarang disentuh).

BLOKIRAN: butuh `updateAssetUrl` dari `src/lib/uploader.js` (fs + HTTP
upload ke storage) serta `m.download()` / `m.quoted.download()`.
Kapabilitas belum ada di Rust API. Jangan port paksa.

### ganti-rules.jpg (`rust/plugin/src/owner/ganti-rules.rs`)

`plugins/owner/ganti-rules.js` — ganti gambar rules.jpg (thumbnail rules).
BLOKIRAN: butuh kapasitas berkas (fs) + image processing (sharp) +
upload ke penyimpanan aset melalui `updateAssetUrl` (`src/lib/uploader.js`),
yang semua TIDAK tersedia di API Rust. Tidak dapat dipaksa di-port.
Buka lagi kalau fs + sharp + upload aset tersedia. Jika dibuka, daftarkan
di `owner/mod.rs` dengan path karena nama berkas mengandung titik dan hubung:
`#[path = "ganti-rules.rs"] pub mod ganti_rules;`.

### ganti-welcome.jpg (`rust/plugin/src/owner/ganti-welcome.rs`)

`plugins/owner/ganti-welcome.js` — ganti gambar welcome.jpg.

## BLOKIR — tidak di-port paksa.

Alasan: sama dengan `ganti-foto3.js` — `updateAssetUrl` butuh `fs`,
`sharp`, dan modifikasi `config.js`, yang belum tersedia di `Ctx` Rust.

`ponytail:` kalau kapasitas itu ditambahkan nanti, port ulang dari
`plugins/owner/ganti-welcome.js`.

### sessionstats (`rust/plugin/src/owner/sessionstats.rs`)

`plugins/owner/sessionstats.js` — statistik session_keys per kategori.

NOTE: baris `pub mod sessionstats;` perlu ditambah ke `owner/mod.rs`
secara manual (mod.rs dilarang disentuh).

BLOKIRAN: butuh `sessionKeyStats` dari `src/lib/turso-session.js` (Turso
DB). Rust crate ini sengaja tidak punya dependensi store/Turso, dan `Ctx`
tidak mengekspos DB. Jangan port paksa.

### linode2gb (`rust/plugin/src/panel/linode.rs`)

`plugins/panel/linode.js` — manajemen VPS Linode.

## BLOKIR — tidak di-port paksa.

Alasan: bagian `linode2gb/4gb/8gb/16gb` (create) butuh
`setTimeout(60000)` — tunggu 60 detik lalu ambil info instance. Handler
Rust adalah `Fn(Ctx) -> Reply` yang sinkron (tidak async), jadi tidak ada
mekanisme menunda/delay di dalam handler. Sisa command
(`listlinode`, `onlinode`, `offlinode`, `rebootlinode`, `rebuildlinode`,
`delinode`, `saldolinode`, `sisalinode`, `cekvpslinode`) bisa di-port
lewat `ctx.http`, tapi aturan melarang port sebagian saja: kalau satu
cabang butuh kapasitas yang belum ada, jangan port paksa.

`ponytail:` kalau `sleep`/delay tersedia di handler nanti, port ulang
dari `plugins/panel/linode.js` (Linode API v4, token dari
`ctx.config().api_keys.linode`).

### lookup (`rust/plugin/src/tools/lookup.rs`)

`plugins/tools/lookup.js` — DNS Lookup untuk domain.

NOTE: baris `pub mod lookup;` perlu ditambah ke `tools/mod.rs` secara
manual (mod.rs dilarang disentuh).

BLOKIRAN: butuh `sendToolsPreview(sock, chat, text, ...)` dari
`src/lib/context.js` — yang membutuhkan `sock` (WhatsApp client) untuk
mengirim pesan preview. `Ctx` di Rust tidak punya `sock`. Jangan port paksa.

### cekvps (`rust/plugin/src/vps/cekvps.rs`)

`plugins/vps/cekvps.js` — cek detail VPS DigitalOcean.
BLOKIRAN: butuh `config.digitalocean` (token, sellers, ownerPanels), yang
TIDAK ada di `oktz_core::config::Runtime` (hanya `api_keys` dan field
standar config.js lain). Tidak ada jalan lain untuk membaca token
DigitalOcean di `Ctx`, jadi plugin tidak dapat dipaksa di-port.
Buka lagi kalau `config.digitalocean` ditambahkan ke `Runtime`.
Jika dibuka, tambahkan baris `pub mod cekvps;` ke `vps/mod.rs`.

### delvps (`rust/plugin/src/vps/delvps.rs`)

`plugins/vps/delvps.js` — hapus VPS DigitalOcean.

## BLOKIR — tidak di-port.

Alasan: membutuhkan `config.digitalocean.token` dan
`config.digitalocean.sellers` / `config.digitalocean.ownerPanels`, yang
tidak ada pada `oktz_core::config::Runtime` (hanya `api_keys`, `owner`,
`assets`, dst. — lihat `rust/lib/oktz-core/config.rs`).
Tidak ada jalur membaca kunci DigitalOcean dari `Ctx`, jadi tidak bisa
meniru perilaku JS apa adanya.

`ponytail:` kalau `config.digitalocean` ditambahkan ke `Runtime` nanti,
port ulang dari `plugins/vps/delvps.js` (axios DELETE ke
`https://api.digitalocean.com/v2/droplets/{id}`, header
`Authorization: Bearer <token>`, pakai `ctx.http.delete` bila tersedia,
atau `HttpCall::delete` lewat `ctx.http.call`).

### sisavps (`rust/plugin/src/vps/sisavps.rs`)

`plugins/vps/sisavps.js` — cek sisa kuota VPS DigitalOcean.
BLOKIRAN: butuh `config.digitalocean` (token, sellers, ownerPanels), yang
TIDAK ada di `oktz_core::config::Runtime`. Tidak ada jalan lain untuk
membaca token DigitalOcean di `Ctx`, jadi plugin tidak dapat dipaksa
di-port. Buka lagi kalau `config.digitalocean` ditambahkan ke `Runtime`.
Jika dibuka, tambahkan baris `pub mod sisavps;` ke `vps/mod.rs`.


## 21. `AIRich` dan `sock.sendTable` (5 plugin, gelombang 4)

Kapabilitas ini belum ada di Rust dan TIDAK bisa ditiru dengan teks biasa
tanpa melanggar paritas tampilan.

- `plugins/main/ping2.js:129` — `sock.sendTable(...)`
- `plugins/main/totalfitur.js:62` — `sock.sendTable(...)`
- `plugins/main/stats.js:88` — `sock.sendTableV2(...)`
- `plugins/ai/ai.js:70` dan `:94` — `new AIRich(sock)` lalu `.addTable(...)`
- `plugins/search/ytplay.js:891` — `new AIRich(sock)`

Sisi JS: `sock.sendTable` dan `sock.sendTableV2` didefinisikan di
`src/lib/socket.js:1300-1311`, keduanya memanggil `generateTableContent*`
lalu `sock.relayMessage`. `AIRich` ada di `src/lib/builder.js:993`.

Yang hilang di Rust:

1. `Reply` (`rust/lib/oktz-plugin-api/registry.rs`) hanya punya
   teks, kutipan, media, reaksi, dan mutasi — belum ada varian tabel atau
   pesan interaktif.
2. `oktz-wa` belum mengekspos pengiriman pesan proto mentah
   (`relayMessage`) untuk tabel/interactive message; pipeline masih
   menutup jalur itu.

Jalan naik: tambah `Reply::Tabel { judul, header, baris }`, port
`generateTableContent*` ke pembangun proto di `oktz-wa`, lalu kirim lewat
jalur `relayMessage` yang setara. Untuk `AIRich`, port `src/lib/builder.js`
bagian `AIRich` (kira-kira 900 baris) ke modul Rust dan pakai renderer yang
sama. Setelah keduanya ada, kelima plugin di atas bisa di-port.

## 22. `random/anime` (`plugins/random/anime.js`)

Nama plugin di JS: `["loli", ...nexrayTypes]` (waifu, neko, shinobu, megumin,
bully, cuddle, cry, hug, awoo, kiss, lick, pat, smug, bonk, yeet, blush,
smile, wave, highfive, handhold, nom, bite, glomp, slap, kill, ...).

Tidak bisa di-port: `prepareWAMessageMedia` / `generateWAMessageFromContent`
dari `onigis`, `sock.relayMessage`, `viewOnceMessage` + `interactiveMessage`
(native flow), dan deteksi GIF lewat header byte — semuanya belum ada di
`Reply`/`Ctx`/`Sock`. `Reply::Sequence` bisa mengirim beberapa gambar biasa,
tetapi bukan pesan `viewOnce` interaktif dengan quick reply.

Berkas catatan lama (`rust/plugin/src/random/anime.rs`) dihapus
supaya tidak muncul sebagai plugin palsu di `bandingkan-nama-plugin.mjs`.
Buka lagi kalau `Reply` punya varian native-flow.

## 23. Port agen yang belum lolos kompilasi (dicabut dari registrasi)

Berkas-berkas ini ditulis agen port tapi belum bisa dikompilasi. Registrasinya
dicabut lewat `scripts/cabut-modul.mjs` supaya pohon tetap hijau; berkasnya
TIDAK dihapus. Port ulang per berkas, lalu daftarkan lagi dengan
`scripts/pasang-modul.mjs --terapkan`.

| berkas | galat pertama |
|---|---|
| `ai/bananagen.rs` | 196:24 — use of moved value: `media`; 295:20 — the type `Arc` does not implement `Copy` |
| `ai/musicmaker.rs` | 287:26 — `Teks` doesn't implement `Debug`; 312:43 — mismatched types |
| `ai/to3d.rs` | 87:39 — `ai::to3d::tests::Stub` doesn't implement `Debug` |
| `ai/tocartoon.rs` | 68:24 — cannot find function `async_download_media` in this scope; 68:9 — the size for values of type `[u8]` cannot be known at compilation time |
| `ai/tochibi.rs` | 100:22 — the trait bound `Result<Vec<u8>, oktz_plugin_api::MediaError>: Default` is not satisfied |
| `ai/tofigurine.rs` | 154:9 — the trait bound `Result<Vec<u8>, oktz_plugin_api::MediaError>: Default` is not satisfied; 190:27 — mismatched types |
| `ai/tomanga.rs` | 101:22 — the trait bound `Result<Vec<u8>, oktz_plugin_api::MediaError>: Default` is not satisfied |
| `anime/mywaifu.rs` | 64:30 — mismatched types; 65:30 — mismatched types |
| `canvas/applemusic.rs` | 304:26 — `UpFail` doesn't implement `Debug`; 324:29 — `UpFail` doesn't implement `Debug` |
| `canvas/fakecall.rs` | 86:25 — the trait bound `Arc<(dyn HttpGet + 'static)>: HttpGet` is not satisfied; 86:25 — the trait bound `std::sync::Arc<(dyn oktz_plugin_api::HttpGet + 'static)>: oktz_plugin_api::HttpGet` is not satisfied |
| `canvas/tiktokchat.rs` | 246:26 — `canvas::tiktokchat::tests::api_gagal_mengembalikan_pesan_gagal_dan_reaksi_silang::UpOk` doesn't implement `Debug`; 266:29 — `canvas::tiktokchat::tests::api_gagal_mengembalikan_pesan_gagal_dan_reaksi_silang::UpOk` doesn't implement `Debug` |
| `download/aio.rs` | 173:17 — the type `Arc` does not implement `Copy`; 90:9 — the trait bound `Result<std::string::String, ()>: Default` is not satisfied |
| `download/alightmotion.rs` | 275:13 — the type `Arc` does not implement `Copy`; 91:10 — no method named `with_mimetype` found for enum `oktz_plugin_api::Reply` in the current scope |
| `fun/cekkhodam.rs` | 13:37 — mismatched types; 219:33 — mismatched types |
| `fun/waifupool.rs` | 151:18 — this function takes 0 arguments but 1 argument was supplied; 152:23 — no field `nav` on type `()` |
| `group/add.rs` | 315:38 — cannot find struct, variant or union type `GroupRoster` in crate `oktz_plugin_api`; 365:20 — the type `Arc` does not implement `Copy` |
| `info/gag2.rs` | 208:39 — `info::gag2::tests::Stub` doesn't implement `Debug`; 50:78 — mismatched types |
| `info/infotourney.rs` | 110:17 — cannot find value `abs` in this scope; 148:32 — the `?` operator can only be used in a function that returns `Result` or `Option` (or another type that implements `FromResidual`) |
| `main/donasi.rs` | 295:25 — mismatched types; 53:36 — no method named `as_deref` found for struct `std::string::String` in the current scope |
| `owner/addprem.rs` | 287:63 — mismatched types; 38:16 — type mismatch resolving `<Option<&str> as IntoIterator>::Item == &&str` |
| `owner/akses.rs` | 105:61 — type mismatch in function arguments; 109:49 — type mismatch in function arguments |
| `owner/ganti_namadev.rs` | 127:11 — no method named `tulis_teks` found for struct `BerkasPalsu` in the current scope; 173:15 — no method named `baca_teks` found for struct `std::sync::Arc<BerkasPalsu>` in the current scope |
| `owner/leave.rs` | 100:13 — mismatched types; 108:13 — mismatched types |
| `owner/payment.rs` | 30:5 — mismatched types; 34:13 — mismatched types |
| `owner/prefix.rs` | 129:34 — mismatched types; 214:30 — mismatched types |
| `owner/sampah.rs` | 82:11 — no method named `buat_folder` found for struct `BerkasPalsu` in the current scope; 90:11 — no method named `buat_folder` found for struct `BerkasPalsu` in the current scope |
| `owner/setgoodbyetype.rs` | 86:44 — can't compare `&str` with `str` |
| `random/lahelu.rs` | 108:39 — `random::lahelu::tests::Stub` doesn't implement `Debug`; 54:9 — mismatched types |
| `random/quotesimage.rs` | 39:9 — mismatched types; 40:9 — mismatched types |
| `religi/audioquran.rs` | 169:83 — temporary value dropped while borrowed; 181:77 — temporary value dropped while borrowed |
| `religi/jadwalsholat.rs` | 104:25 — mismatched types; 178:25 — mismatched types |
| `rpg/dice.rs` | 107:14 — no method named `insert` found for enum `serde_json::Value` in the current scope; 107:14 — no method named `insert` found for enum `Value` in the current scope |
| `search/playsoundcloud.rs` | 69:41 — the trait bound `Arc<(dyn HttpGet + 'static)>: HttpGet` is not satisfied; 69:41 — the trait bound `std::sync::Arc<(dyn oktz_plugin_api::HttpGet + 'static)>: oktz_plugin_api::HttpGet` is not satisfied |
| `search/ptvsearch.rs` | 60:16 — cannot move out of index of `Vec<serde_json::Value>`; 60:16 — cannot move out of index of `Vec<Value>` |
| `search/searchthatsong.rs` | 306:13 — mismatched types; 322:13 — mismatched types |
| `search/soundcloud.rs` | 351:32 — the trait bound `std::sync::Arc<search::soundcloud::tests::StubHttp>: oktz_plugin_api::HttpGet` is not satisfied |
| `search/tiktokfoto.rs` | 156:22 — expected `Chain<Once<char>, Once<char>>` to be an iterator that yields `u8`, but it yields `char`; 156:63 — type mismatch resolving `<Chain<Once<char>, Once<char>> as IntoIterator>::Item == u8` |
| `search/wikipedia.rs` | 365:64 — type mismatch in function arguments; 368:26 — no method named `contains` found for enum `Option<T>` in the current scope |
| `stalker/discordstalk.rs` | 372:13 — the type `Arc` does not implement `Copy` |
| `stalker/ffstalk.rs` | 108:40 — mismatched types |
| `stalker/robloxstalk.rs` | 157:56 — no method named `first` found for reference `&serde_json::Value` in the current scope; 157:56 — no method named `first` found for reference `&Value` in the current scope |
| `tools/am_data.rs` | 99:40 — mismatched types |
| `tools/cekxl.rs` | 191:40 — mismatched types; 192:14 — the trait bound `std::string::String: Copy` is not satisfied |
| `tools/emojitoanimasi.rs` | 207:17 — the type `Arc` does not implement `Copy`; 257:21 — mismatched types |
| `tools/emojitoimage.rs` | 96:49 — `if` and `else` have incompatible types |
| `tools/hdvid.rs` | 31:13 — mismatched types; 36:7 — mismatched types |
| `tools/imgtoprompt.rs` | 226:19 — the trait bound `std::sync::Arc<tools::imgtoprompt::tests::Stub>: oktz_plugin_api::HttpGet` is not satisfied; 78:23 — cannot find function `img2prompt` in this scope |
| `tools/invoicemaker.rs` | 187:80 — mismatched types; 316:21 — mismatched types |
| `tools/lookup.rs` | 94:27 — no method named `and_then` found for type `bool` in the current scope; 95:31 — no method named `and_then` found for type `bool` in the current scope |
| `tools/sendngl.rs` | 54:17 — cannot find type `HttpError` in this scope |
| `tools/spamngl.rs` | 111:39 — `tools::spamngl::tests::Stub` doesn't implement `Debug` |
| `tools/ssweb.rs` | 185:17 — the type `Arc` does not implement `Copy`; 245:21 — mismatched types |
| `tools/tempmail.rs` | 69:27 — no method named `insert` found for enum `serde_json::Value` in the current scope; 69:27 — no method named `insert` found for enum `Value` in the current scope |
| `tools/wink.rs` | 118:22 — the trait bound `Result<Vec<u8>, oktz_plugin_api::MediaError>: Default` is not satisfied; 196:20 — the type `Arc` does not implement `Copy` |

### 23.1 Status terkini gelombang 4

Seluruh berkas baru gelombang 4 (`docs/rust-rewrite/gelombang4-belum-terpasang.txt`,
204 berkas plugin, 92 modul sempat terdaftar) sudah DICABUT dari registrasi
karena:
- 54 berkas gagal dikompilasi (tabel di atas), dan
- 29 test gagal setelah registrasi (bug perilaku: `ai/toblack` mengembalikan
  galat unduh saat tidak ada gambar padahal JS mengirim kartu bantuan,
  `sticker/emojimix` mengambil emoji dari string yang salah,
  `owner/bcgc` parse delay, `rpg/duel` ambang health, `fun/top` alias
  `leaderboard` yang bertabrakan dengan `main/leaderboard`, dan lain-lain).

Berkas tidak dihapus. Cara memasang lagi per berkas: perbaiki, tambahkan test,
lalu `node scripts/pasang-modul.mjs --terapkan`. Jangan pasang massal sebelum
`cargo test --release` hijau untuk berkas itu.

## 24. Owner gelombang 4 — hanya `akses` yang layak didaftarkan

- `owner/akses.rs` — **didaftarkan**. Satu-satunya dari batch ini yang semua
  cabangnya bekerja lewat API yang ada (`Ctx.user_data` + `Mutation::SetUser`).
- `owner/addprem.rs` — **tidak didaftarkan**: seluruh operasi tulis (add/del/
  list, jadibot maupun reguler) berakhir `error_template` + reaksi ☢ karena
  `Ctx` belum punya store premium (`db.data.premium`). Badan handler jadi
  "belum tersedia" untuk aksi intinya → kena aturan stub.
- `owner/arsip.rs` — **tidak didaftarkan**: `sock.chatModify({archive})` tidak
  ada padanannya; port mengirim `Mutation::SetUser` kosong yang bukan aksi
  arsip (menulis data palsu ke user). Tunggu kapabilitas `Sock` §2.
- `scripts/pasang-modul.mjs` kini punya `--hanya=<jalur,...>` supaya batch
  pendaftaran bisa dibatasi; tanpa itu skrip mendaftarkan seluruh berkas.

## 25. Gelombang 4 massal — 60 modul lolos, 73 tidak

Pendaftaran massal 133 berkas (`pasang-modul.mjs` tanpa `--hanya=`), lalu
penyaringan otomatis: 52 dicabut karena gagal kompilasi, 19 dicabut karena
testnya gagal, 2 (`owner/addprem`, `owner/arsip`) dicabut karena stub (§24).
Sisanya 60 modul terdaftar.

Gagal kompilasi (API/tipe belum ada) — 52 berkas:
`ai/bananagen ai/musicmaker ai/to3d ai/tocartoon ai/tochibi ai/tofigurine
ai/tomanga anime/mywaifu canvas/applemusic canvas/fakecall canvas/tiktokchat
download/aio download/alightmotion fun/cekkhodam fun/waifupool group/add
info/gag2 info/infotourney main/donasi owner/ganti_namadev owner/leave
owner/payment owner/prefix owner/sampah owner/setgoodbyetype random/lahelu
random/quotesimage religi/audioquran religi/jadwalsholat rpg/dice
search/playsoundcloud search/ptvsearch search/searchthatsong search/soundcloud
search/tiktokfoto search/wikipedia stalker/discordstalk stalker/ffstalk
stalker/robloxstalk tools/am_data tools/cekxl tools/emojitoanimasi
tools/emojitoimage tools/hdvid tools/imgtoprompt tools/invoicemaker tools/lookup
tools/sendngl tools/spamngl tools/ssweb tools/tempmail tools/wink`

Gagal test (perilaku belum sama dengan JS) — 19 berkas, semuanya sudah
dibereskan dan didaftarkan kecuali `owner/bcgc` (stub, §26):
`ai/toblack ai/toisland ai/txt2img canvas/musiccard group/pinchat owner/bcgc
owner/hapusgrup owner/mutechat owner/renewsewa random/meme rpg/duel rpg/nulis
search/playtiktok stalker/igstalk stalker/stalkml sticker/emojimix tools/ipwho
tools/tovideo utility/inspect`

Catatan: berkas-berkas itu tetap ada di disk (dicabut, bukan dihapus), jadi
perbaikannya cukup memanggil ulang `pasang-modul.mjs --terapkan --hanya=<berkas>`
setelah galatnya dibereskan.

Struktur: crate pendukung tetap di `rust/lib/oktz-*`, tapi isi `src/`
dinaikkan satu level (`rust/lib/oktz-core/config.rs`, `lib.rs` di akar crate
dengan `[lib] path = "lib.rs"`) — folder `src/` dihapus.

## 26. Tiga berkas owner yang tadi gagal test

- `owner/mutechat.rs` — **terdaftar**. Dua kesalahan: teks bantuan menambah baris
  `Prefix saat ini: ...` yang tidak ada di JS, dan kasus "tanpa target" di chat
  pribadi dianggap kartu bantuan padahal JS memakai `m.chat` sebagai target.
  Test ikut dibetulkan. `sock.chatModify` tetap tak ada di `Ctx` (teks balasan
  yang diuji, bukan aksinya).
- `owner/hapusgrup.rs` — **terdaftar**. Port menolak argumen yang tersaring
  habis, padahal JS membentuk `"@g.us"` lalu masuk cabang `catch`. Resolusi JID
  dipecah ke `target_jid_dari_arg` + test kuirk `123-456@g.us` → `123456@.@g.us`.
- `owner/bcgc.rs` — **tidak didaftarkan**: badannya stub (selalu membalas
  `STOP_TEXT`). Sekaligus perbaiki bug `parse_delay` `"1d"` = 24 menit
  (kurang satu `* 60`) supaya berkasnya benar saat `sock`/`db` tersedia.
- Sisa `Prefix saat ini: ...` yang tidak ada di JS: `ai/tomekah.rs`,
  `ai/toemotebatu.rs`, `rpg/dice.rs` (belum terdaftar) — buang saat
  mendaftarkannya.

## 27. Kapabilitas API yang diminta 52 berkas gelombang 4

Dikumpulkan dari catatan subagent saat mendaftarkan batch ini. Semuanya sudah
diberi `ponytail:` di masing-masing berkas; daftar ini yang dipakai kalau
`Ctx`/`Reply` diperluas.

`Reply`:
- `contextInfo` — `forwardingScore`/`isForwarded` (`playtiktok`),
  `forwardedNewsletterMessageInfo`/`saluranCtx()` (`donasi`, `leave`,
  `emojitoanimasi`, `jadwalsholat`, `invoicemaker`).
- `quoted: m` per balasan (dipakai hampir semua plugin JS).
- Tombol interaktif + `buttonParamsJson`/footer (`donasi`, `waifupool`).
- `fileName` untuk video (`hdvid`: `HDVID-<ms>.mp4`), flag `ptv` (`ptvsearch`).
- Album WA (`albumMessage` + `relayMessage`) — `mywaifu`, `tiktokfoto`.
- Reaksi berurutan (JS `m.react('🕕')` lalu `✅`/`☢`): `Reply` cuma membawa satu
  reaksi. Terdampak `lahelu`, `quotesimage`, `wink`, `imgtoprompt`, `dice`,
  `leave`, `jadwalsholat`.

`Ctx`:
- `m.pushName` (teks error template selalu "User").
- Akses env/`process.env` (mis. `TERMAI_UPLOAD_KEY` di `applemusic`).
- Jam/tanggal: `Date.now()` dan format ISO + zona `Asia/Jakarta` tanpa crate
  tanggal (dipakai `wink`, `imgtoprompt`, `discordstalk`, `robloxstalk`).
- Timer/jeda async (`setTimeout` 4000 `spamngl`, 2500 `dice`) dan kiriman
  bertahap (pesan progres).
- `sendToolsPreview` (`lookup`), `saluranCtx()` (`donasi`).
- Status per-nomor pada `ubah_peserta` (`group/add`: bot admin vs bukan).
- Parser HTML ala cheerio (`wikipedia`, `soundcloud`, `infotourney`) — sekarang
  parser manual dengan selector terbatas.
- `crypto.randomBytes` + `waUploadToServer` (`mywaifu`).
- Folder nyata di `BerkasPalsu` (`sampah` memakai trik entri penanda).
- Map paginasi/`searchPool` global (`waifupool`).

Cacat kecil yang masih terbuka (bukan penghalang kompilasi):
- `fakecall.js:100` memakai reaksi `☢`, port memakai `❌`.
- `ganti-namadev.js:31-34` selalu menulis kutip tunggal; port mempertahankan
  kutip asli.
- `musicmaker.js:20` memakai `m.text?.trim() || m.args.join(" ")`; port cuma
  cek `is_empty()` sehingga teks spasi-saja berbeda.
- `imgtoprompt`: unduhan gagal → `GAGAL_UNDUH`, JS `catch` → `te(...)`.

## 28. Survei stub yang tersisa (gelombang 4 lanjutan)

Tiga survei read-only atas 56 berkas stub + 11 berkas catatan. Klasifikasi
diambil dari stub + JS aslinya, bukan dari tebakan.

PORTABEL (bisa diimplementasi penuh dengan API sekarang):
- `cek/cekfemboy` — `oktz_scraper::lufemboy`, ffmpeg `jalankan_dengan_masukan`,
  `Reply::video_gif` semua sudah ada; stub lama salah menyatakan terblokir.
- `tools/dafont` — scraper `dafont` ada, `ctx.session_*` + `#[answer_handler]`
  ada; satu-satunya batas: TTL sesi 120s diatur manual.
- `owner/listsewa` — `db.db.data.sewa` dipetakan ke `settings.sewa`
  (preseden `group/checksewa.rs:18`).

SEBAGIAN (jalur utama bisa, ada bagian yang tetap tidak bisa):
- `ai/toemotebatu`, `ai/tofigurev2`, `ai/toghibli`, `ai/tomekah` — jalur API
  lengkap; yang hilang hanya reaksi kedua (`🕕` lalu `✅`/`❌`/`☢`).
- `canvas/fakeml`, `canvas/profileig` — unggah multipart sudah bisa
  (`HttpCall::put`/`multipart`/`MultipartPart::file`), yang hilang reaksi kedua.
- `download/tiktokdl2` — jalur video/audio bisa; album slide tidak (§12).
- `fun/gachawaifu`, `nsfw/nsfw` — `is_premium` tidak ada di `Ctx`; kartu
  `viewOnce` hilang.
- `group/cekidgc` — butuh internet `ctx.sock` nyata (masih `SockMati`);
  potongan teks deskripsi 120 char + tombol `cta_copy` hilang.
- `main/carifitur` — tanpa tombol `single_select`.
- `store/addproduk` — `uploadImage()` yang mengembalikan URL tidak ada (§13),
  jadi `imageUrl`/`videoUrl` tak bisa diisi.
- `owner/ganti-namaowner` — daftar + set nama bisa; cabang `main` yang menulis
  `config.js` tidak.
- `owner/setreply`, `owner/setwelcometype` — set varian bisa via
  `Mutation::SetSetting`; menu tombol + `getAssetBuffer("foto")` tidak.
- `owner/sewabot` — `status`/`off`/`on`/`cancel` bisa; `confirm`/`leave` butuh
  fan-out ke JID lain.

TERBLOKIR (jalur utama mustahil sekarang), alasan singkat:
- `canvas/fakedev7`, `canvas/iqcpink`, `sticker/bratcewek`, `sticker/bratvid`,
  `sticker/ttp` — rasterizer teks/vektor/gradien; `gambar.rs` cuma
  resize/grayscale/encode. `iqcpink` juga butuh `m.messageTimestamp`.
- `game/ulartangga`, `game/werewolf` — `sock.sendPreview` (semua balasan),
  PM ke JID pemain, timer fase, hadiah ke pemain lain.
- `owner/addowner`, `owner/backupsc`, `owner/cekschedule`, `owner/chatclear`,
  `owner/colongpp`, `owner/disableplugin`, `owner/ganti-games`,
  `owner/ganti-mp3`, `owner/ganti-promote`, `owner/ganti-winner`,
  `owner/gantiscraper`, `owner/hapusdata`, `owner/listjadibot`, `owner/restart`,
  `owner/schedule`, `owner/srt`, `owner/stopalljadibot`, `owner/swgc`,
  `owner/swgcall` — `updateAssetUrl` (unggah + tulis `config.js`), OS/shell,
  cron/map in-process, `sock.chatModify`, `sock.user.id`, `sendButton`,
  `groupStatusMessage`, `randomBytes`+album, TTL berbasis timer, fan-out JID.
- `main/menucat`, `main/sc` — tombol `single_select`/`cta_url`.
- `main/system` — tanpa eksekusi OS (CPU/RAM/disk).
- `panel/createserver`, `vps/cekvps`, `vps/delvps`, `vps/sisavps` —
  `config.pterodactyl`/`config.digitalocean`/`config.vercel` tidak ada di
  `Runtime` (`config.rs:225-234`).
- `search/playch` — kirim audio ke JID lain + unggah media.
- `tools/deploy` — token vercel + ekstrak ZIP.
- `tools/ocr` — mesin OCR tidak ada di repo.
- `fun/confess` — kirim ke JID target (`pipeline.rs:3058` selalu ke chat
  pemicu).

Catatan lintas-berkas yang perlu dibaca kalau kapabilitas ditambah:
- `ctx.sock` sudah dipasang di produksi (`SockKlien`, `bot.rs` wiring) —
  `nama_kontak` (`sock.getName`) tetap tidak bisa karena klien tidak menyimpan
  toko kontak.
- `ctx.berkas` masih `BerkasPalsu` (memori), jadi efek tulis berkas hilang.
- `Reply::Interactive` cuma `body`; `cards.rs` punya tombol penuh tapi belum
  dijembatani.
- `Ctx` tidak punya `is_premium`, `pushName`, `messageTimestamp`.

## 29. Config file, launcher Node, dan sock produksi

`rust/config.json` + `Config::load()` (`lib/oktz-core/config.rs`) jadi pengganti
`config.js` untuk yang paling sering diubah user: `info`, `owner`, `bot`,
`session`, `database`, `turso`, `sticker`, `saluran`, `apiKeys`.

Aturan timpa: nilai yang diisi menang atas env/`.env`; string kosong berarti
"jangan sentuh"; kunci tak dikenal diabaikan (jadi `_petunjuk` aman); file rusak
atau salah tipe = error saat start, bukan fallback diam. Lokasi file lewat
`OKTZ_CONFIG`. `owner.number` kosong di env DAN file jatuh ke bawaan
`config.js`.

Sudah ada konsumennya: `owner.name`/`owner.number` (identitas router), `bot`,
`info`, `sticker`, `saluran`, `apiKeys` (lewat `Config::runtime()` → `Ctx::config()`),
`database.path`, `turso.url`/`authToken`, `session.pairingNumber` (argumen
`oktz pair-code` kosong), `session.usePairingCode` (mode `oktz pair`).

Sudah dibaca tapi BELUM ada konsumen — jangan diklaim berfungsi:
`session.qrDir`, `qrPngWidth`, `qrMaxPrints`, `pairingRateLimitCooldownMs`,
`pairingFallbackToQr`, `keepAliveIntervalMs`; `turso.enabled` masih sekadar
ditampilkan saat boot.

Blok `config.js` yang belum punya jalur file: `messages`, `features`,
`autoWork`, `registration`, `ui`, `energi`, `groupProtection`, `payment`,
`donasi`, `effectApi`, `pterodactyl`, `digitalocean`, `geminiApiKey`,
`autoaiPersonas`, `mode`, `command`, `dev`.

Launcher panel: `rust/launcher.js` (+ `package.json`) menjalankan
`target/release/oktz` sebagai child process, stdio + sinyal + exit code
diteruskan; binary bisa dialihkan lewat `OKTZ_BIN`, argumen diteruskan apa
adanya (tanpa argumen = `run`).

Sock produksi: `SockKlien` (`lib/oktz-wa/sock_client.rs`) mengisi 15 dari 16
method `Sock` (grup, foto profil, presence, read receipt, status teks, profil
bisnis). `nama_kontak` (`sock.getName`) tidak bisa: klien tidak menyimpan toko
kontak — errornya eksplisit. Sinkron→asinkron lewat `block_in_place` +
`Handle::block_on`; runtime current-thread ditolak sebagai `SockError`.

Dua panggilan sync itu dipakai `execute_group_decision`: `KickAndReply` kini
benar-benar menghapus → kick (`ubah_peserta(..., Keluar)`) → balas, urutannya
sama dengan `src/lib/group-protection.js:1234-1242`; kegagalan kick ditelan
dengan `warn` seperti JS. Fallback `KICK_FALLBACK` (cabang `catch` di
`handleAntilinkGc/All`) belum dijalankan.

Roster grup juga sudah dipasang dari `ClientSender` (`bot.rs` wiring), jadi
`Ctx.roster` tidak lagi kosong di produksi.

Db sesi kini durable: `prepare_store` memakai ulang db yang sidik creds-nya
cocok (`<db>.ident` berisi `registration_id:identitas`), jadi device yang
dirotasi library + sesi ratchet bertahan antar-restart. Db tanpa marker atau
ber-sidik beda tetap dibangun ulang. Write-back `creds.json` tetap TIDAK ada
dan itu disengaja (`bot.rs:302-331`: rollback kunci senyap).

Belum: `ctx.berkas` nyata, `Reply::Interactive`, daemon shutdown, handler event
lanjutan.

## 30. Event peserta grup (welcome/goodbye, anticulik, sewa)

`EventKind::GroupUpdate` sekarang dilanggan (`Pipeline::attach`) dan mengalir
lewat antrean sendiri ke `Pipeline::tangani_update_grup`. Satu event = satu aksi
+ satu daftar peserta (`wacore-0.7.0/src/types/events.rs:1780`), bukan array
besar seperti Baileys.

Yang sudah jalan:
- Default grup baru ditulis persis `db.setGroup(groupJid, {...})`
  (`src/handler.js:1999-2003`). Nilainya `false` karena `config.js:186-187`
  memang memasang `defaultEnabled: false` untuk welcome/goodbye.
- Welcome (`add`) dan goodbye (`remove`) tipe teks (1): teks di
  `lib/oktz-wa/pesan_grup.rs` (placeholder, sapaan, tips acak, potong 120
  karakter) dan gerbang di `lib/oktz-wa/grup_event.rs`. Quirk JS ditiru: yang
  diserahkan sebagai "deskripsi grup" adalah `groupMeta.descOwner`
  (`plugins/group/welcome.js:137`, `goodbye.js:165`), bukan `desc`.
- LID → JID lewat `groupMeta.participants` (`resolveAnyLidToJid`); mention
  memakai nomor hasil terjemahan.
- Bot ditambahkan ke grup (`src/connection.js:1199-1328`): anticulik (keluar
  setelah 2 detik kalau `anticulik: "on"` dan pengundang bukan owner), sewa
  (grup tidak terdaftar/tidak aktif → beri tahu lalu keluar), dan salam
  "hai, salam kenal" saat grup terdaftar.

Batas yang diketahui (jangan diklaim jalan):
- `welcomeType`/`goodbyeType` 2..8 (kartu carousel, upload foto profil, video
  gif, tombol, teks type 8) — dilewati dengan `warn`. Juga banner
  `promote`/`demote` (`src/handler.js:2090-2130`, butuh `config.assets`).
- Badge newsletter `saluranCtx()` dan `forwardedNewsletterMessageInfo`
  (`src/connection.js:1288-1313`) tidak ikut; jalur context itu belum ada.
- `config.bot.support` belum punya field di `Runtime`; baris "Support: -" dan
  fallback kontak sewa memakai `developer` lalu `"owner"`.
- Penjaga 15 detik setelah connect (`src/connection.js:1200`), cache metadata
  300 detik, dan `options.onParticipantsUpdate` (tidak dipakai di JS) tidak
  diport.
- Penerjemahan LID untuk `pelaku` (pengundang) hanya lewat pencocokan nomor;
  tabel LID global JS (`src/lib/lid.js`) tidak ada di modul murni.
- `waktu_lokal()` memakai `Local::now()`; host harus TZ `Asia/Jakarta` supaya
  tanggal/jam/hari sama dengan bot JS.

Tes: `pipeline::tests::update_grup_*`, `anticulik_*`, `sewa_*` (memakai `SockUji`
yang mengembalikan metadata + mencatat `keluar_grup`).
