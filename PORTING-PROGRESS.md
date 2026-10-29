# Proses port gel4-7 (20 plugin JS → Rust)

Status per plugin: DONE = berkas Rust penuh + tes, BLOKIRAN = tidak di-port (dokumentasi alasannya di berkas + ringkasan).

## Daftar plugin (dari gel4-7.md)

| # | JS | Rust target | Status |
|---|----|-------------|--------|
| 1 | plugins/owner/addsewa.js | owner/addsewa.rs |  |
| 2 | plugins/owner/bcpc.js | owner/bcpc.rs |  |
| 3 | plugins/owner/buatgrup.js | owner/buatgrup.rs |  |
| 4 | plugins/ai/characterai.js | ai/characterai.rs |  |
| 5 | plugins/owner/delplugin.js | owner/delplugin.rs |  |
| 6 | plugins/owner/eval.js | owner/eval.rs |  |
| 7 | plugins/game/fisch.js | game/fisch.rs |  |
| 8 | plugins/owner/ganti-namabot.js | owner/ganti-namabot.rs |  |
| 9 | plugins/group/groupinfo.js | group/groupinfo.rs |  |
| 10 | plugins/canvas/igstoryimg.js | canvas/igstoryimg.rs |  |
| 11 | plugins/panel/linode.js | panel/linode.rs |  |
| 12 | plugins/tools/musikapaini.js | tools/musikapaini.rs |  |
| 13 | plugins/search/playvid.js | search/playvid.rs |  |
| 14 | plugins/tools/removebg.js | tools/removebg.rs |  |
| 15 | plugins/owner/sessionstats.js | owner/sessionstats.rs |  |
| 16 | plugins/search/spotify.js | search/spotify.rs |  |
| 17 | plugins/tools/styleteks.js | tools/styleteks.rs |  |
| 18 | plugins/fun/tembak.js | fun/tembak.rs |  |
| 19 | plugins/canvas/topixel.js | canvas/topixel.rs |  |
| 20 | plugins/search/ttsearch.js | search/ttsearch.rs |  |

Aturan: tidak menyentuh mod.rs; tidak menjalankan cargo; JS adalah acuan mutlak.
