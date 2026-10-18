# Porting Plan: Gelombang Cheerio

## Plugins to port (9 files)

### New categories (create folder + mod.rs)
1. `src/berita/berita.rs` + `src/berita/mod.rs` — `plugins/info/berita.js` (category: berita)
2. `src/islamic/mod.rs` + `src/islamic/murrotal.rs` + `src/islamic/quran.rs` — `plugins/islamic/` (category: islamic)

### Existing categories (add plugin files only, note mod.rs registration needed)
3. `src/info/infotourney.rs` — `plugins/info/infotourney.js` (category: info)
4. `src/search/soundcloud.rs` — `plugins/search/soundcloud.js` (category: search)
5. `src/search/wikipedia.rs` — `plugins/search/wikipedia.js` (category: search)
6. `src/download/tiktokdl.rs` — `plugins/download/tiktokdl.js` (category: download)
7. `src/download/tiktokdl2.rs` — `plugins/download/tiktokdl2.js` (category: download)
8. `src/tools/bandingkan_hp.rs` — `plugins/tools/bandingkan-hp.js` (category: tools)

### Registration notes (cannot touch lib.rs or existing mod.rs per rules)
- Add `pub mod berita;` and `pub mod islamic;` to `src/lib.rs`
- Add `pub mod infotourney;` to `src/info/mod.rs`
- Add `pub mod soundcloud;` and `pub mod wikipedia;` to `src/search/mod.rs`
- Add `pub mod tiktokdl;` and `pub mod tiktokdl2;` to `src/download/mod.rs`
- Add `pub mod bandingkan_hp;` to `src/tools/mod.rs`

## Progress
- [ ] berita.rs + berita/mod.rs
- [ ] islamic/mod.rs + murrotal.rs + quran.rs
- [ ] infotourney.rs
- [ ] soundcloud.rs
- [ ] wikipedia.rs
- [ ] tiktokdl.rs
- [ ] tiktokdl2.rs
- [ ] bandingkan_hp.rs
- [ ] Registration comments in each file