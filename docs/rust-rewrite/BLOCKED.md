# Plugin terblokir — daftar dan alasannya

Disusun dari laporan batch agent porting (B01–B09). Acuan: file JS masing-masing.
Daftar ini bukan belgekan: plugin di sini **tidak bisa** di-port tanpa menambah
kapabilitas di `oktz-plugin-api` atau crate inti. Kalau salah satu kapabilitas di bawah
ini, daftar ini ikut susut, daftar ini ikut susut.

## 1. Unduh media pesan masuk (blocking terbesar)

Butuh `downloadContentMessage`. `QuotedMessage` di `oktz-plugin-api` hanya
field baca tanpa I/O; `Ctx` tidak punya `isImage`/`isAudio`/`isVideo` untuk
pesan pemicu.

Tren 14 file `ai/to*.js`: `isImage?` → `download` → unggah → efek →
`sock.sendMedia`. Setelah `Ctx` punya `download_media` + `Reply::image`,
14 file ini murah (< 60 baris masing-masing).

File: `ai/bananagen.js`, `ai/to3d.js`, `ai/toanime.js`, `ai/toblack.js`,
`ai/tocartoon.js`, `ai/tocermin.js`, `ai/tochibi.js`, `ai/toemotebatu.js`,
`ai/tofigure.js`, `ai/tofigurev2.js`, `ai/tofigurine.js`, `ai/toghibli.js`,
`ai/tohijab.js`, `ai/toisland.js`, `ai/tojapanese.js`, `ai/tomanga.js`,
`ai/tomekah.js`, `ai/tooilpainting.js`, `ai/txt2img.js`,
`canvas/fakecall.js`, `canvas/fakeml.js`, `canvas/fakestory*.js`,
`canvas/gura.js`, `canvas/igstory*.js`, `canvas/applemusic.js`,
`canvas/facepalm.js`, `canvas/musiccard.js`, `canvas/profileig.js`,
`canvas/starboy.js`, `canvas/topixel.js`, `convert/audiofx.js`.

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

`Ctx` tidak membawa config runtime; `ctx.setting()` membaca `settings.json`,
itu lain. Kunci yang jadi `""` membuat cabang HTTP tak terjangkau.

File: `ai/txt2img.js`, `ai/anime-gen.js`, `info/jadwalbola.rs` (pola sama),
`canvas/watercolortext.rs`, `canvas/pakustad.rs`, `asupan/asupantiktok.js`,
`download/aio.js` (`saluranCtx()`).

## 8. `games.createPlugin` — 27 plugin `game/*`

Semua file `plugins/game/*` yang cuma memanggil
`games.createPlugin(...)` (lihat daftar di `PLUGIN-CLASSIFICATION.md`).
`handler` dan `answerHandler` adalah closure dari `src/lib/games.js:84-299`,
yang belum di-port. Selain itu butuh `fs`, `sharp`, `sock.sendPreview`,
`setSessionTimer`, data JSON `src/data/*.json`, dan `src/lib/game-data.js`,
`database.js`, `level.js`, `context.js`.

Masalah arsitektur: `PluginEntry` cuma punya satu `handler`, sedangkan JS punya
dua (perintah + jawaban bebas). Perlu API registry baru sebelum port mungkin.

Nilainya sudah didokumentasikan di laporan batch B09 (alias, description,
`timeout`, `hasImage`, `questionField`, `answerField`, `hintCount`) supaya
nanti tidak perlu baca ulang.

## 9. `answerHandler` di luar game

`ai/characterai.js` (`caiAnswerHandler`/`caiChatHandler` dipanggil untuk semua
pesan biasa) dan `anime/autoanimewinbu.js`.

## 10. `AIRich` dan pesan kaya builder

`ai/ai.js` memakai `AIRich` dari `src/lib/builder.js` (`addTable`, `addCode`,
`addText`) lalu mengirim lewat `relayMessage`. `Reply` cuma satu payload.

## 11. Scraper belum di-port

`download/aio.js` butuh `src/scraper/aio.js` (cheerio + `Youtube`).
`info/berita.js` dan `islamic/quran.js` butuh parse HTML/XML (cheerio),
dan tidak boleh menambah dependency baru.

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

1. **Unduh media pesan pemicu** (`downloadContentMessage`, §1) — membuka 15 file
   `ganti-*`.
2. **I/O `config.js` + config runtime** (`config.payment`, `config.donasi`,
   `config.owner`, `config.bot`, `config.assets`) — membuka 8 file.
3. **`Reply::Document` dengan `fileName`** — membuka `getplugin` dan sebagian
   besar cabang output `get`.
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