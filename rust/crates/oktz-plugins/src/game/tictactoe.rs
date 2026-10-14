//! `plugins/game/tictactoe.js` — buat/join room TicTacToe di grup.
//!
//! `ponytail:` hanya `handler` yang dipindah. `answerHandler` (balasan angka
//! 1-9, menyerah, deteksi pemenang, hadiah koin) dipanggil router global
//! lewat `plugin.answerHandler` (`src/handler.js:453`), sedangkan
//! `PluginEntry` hanya punya satu `handler` — tidak ada tempat untuk fungsi
//! kedua itu. Akibatnya room yang dibuat di sini belum bisa dimainan; sama
//! seperti `suitpvp.rs` dan `kyubigame.rs`.
//!
//! `ponytail:` JS menghapus room `WAITING` lewat `setTimeout(..., 300000)`.
//! Tanpa timer, room yang sudah lewat 5 menit diabaikan saat handler
//! dipanggil lagi — efek yang sama, tanpa pengusir yang berjalan sendiri.
//! Room `PLAYING` tidak pernah dihapus otomatis di JS, jadi tidak juga di
//! sini.
//!
//! `ponytail:` `safeReply`/`safeReact` membungkus `m.reply`/`m.react` dengan
//! retry rate-limit dan `delay()`. `Ctx` sinkron tanpa I/O, jadi retry itu
//! hilang; isi pesannya tidak berubah.

use oktz_plugin_api::{Ctx, Reply, plugin};
use serde_json::{Map, Value, json};

use crate::common::{jid_number, now_ms};

/// `global.tictactoeGames` dipetakan ke session store dengan nama yang sama,
/// supaya whoever mem-port router tic-tac-toe tidak perlu tahu.
pub(crate) const GAMES_KEY: &str = "tictactoeGames";

/// Awalan `roomId` (`tictactoe.js:186`) dan penentu "masih dalam game".
pub(crate) const ROOM_PREFIX: &str = "ttt_";

/// `setTimeout(..., 300000)` (`tictactoe.js:196`) — 5 menit.
pub(crate) const ROOM_TTL_MS: i64 = 300_000;

/// `tictactoe.js:151` — sender sudah punya room.
pub(crate) const BUSY: &str = "❌ Kamu masih dalam game!\n\n> \
    Selesaikan game kamu atau ketik *nyerah* untuk menyerah.";

/// `boardSymbols` (`tictactoe.js:19-31`) — urutan tidak berpengaruh karena
/// dicari per kunci.
const BOARD_SYMBOLS: [(&str, &str); 11] = [
    ("X", "❌"),
    ("O", "⭕"),
    ("1", "1️⃣"),
    ("2", "2️⃣"),
    ("3", "3️⃣"),
    ("4", "4️⃣"),
    ("5", "5️⃣"),
    ("6", "6️⃣"),
    ("7", "7️⃣"),
    ("8", "8️⃣"),
    ("9", "9️⃣"),
];

/// `boardSymbols[String(cell)] || cell` (`tictactoe.js:351`).
pub(crate) fn board_symbol(cell: &str) -> String {
    BOARD_SYMBOLS
        .iter()
        .find(|(key, _)| *key == cell)
        .map_or_else(|| cell.to_string(), |(_, sym)| (*sym).to_string())
}

/// `TicTacToe.render(0, 0)` — `_x` dan `_o` masih nol di room yang baru dibuat
/// atau baru digabung, jadi semua sel kosong dan bernomor 1-9.
fn fresh_cells() -> [String; 9] {
    std::array::from_fn(|i| (i + 1).to_string())
}

/// `renderBoard(arr)` (`tictactoe.js:353-362`).
pub(crate) fn render_board(cells: &[String; 9]) -> String {
    let c: Vec<String> = cells.iter().map(|s| board_symbol(s)).collect();
    format!(
        "┌───┬───┬───┐\n\
         │ {0} │ {1} │ {2} │\n\
         ├───┼───┼───┤\n\
         │ {3} │ {4} │ {5} │\n\
         ├───┼───┼───┤\n\
         │ {6} │ {7} │ {8} │\n\
         └───┴───┴───┘",
        c[0], c[1], c[2], c[3], c[4], c[5], c[6], c[7], c[8]
    )
}

pub(crate) fn fresh_board() -> String {
    render_board(&fresh_cells())
}

/// Room `WAITING` yang lewat TTL — diperlakukan sama seperti sudah dihapus
/// `setTimeout`.
fn is_expired(room: &Value, now: i64) -> bool {
    room.get("state").and_then(Value::as_str) == Some("WAITING")
        && now - room.get("createdAt").and_then(Value::as_i64).unwrap_or(0) > ROOM_TTL_MS
}

/// `room.id.startsWith("ttt_") && [playerX, playerO].filter(Boolean).includes(sender)`.
fn is_active_for(room: &Value, sender: &str) -> bool {
    room.get("id")
        .and_then(Value::as_str)
        .is_some_and(|id| id.starts_with(ROOM_PREFIX))
        && [room.get("playerX"), room.get("playerO")]
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .any(|p| p == sender)
}

/// `r.state === "WAITING" && r.chat === chat && (roomName ? r.name === roomName : true)`.
fn is_waiting(room: &Value, chat: &str, room_name: &str, now: i64) -> bool {
    room.get("state").and_then(Value::as_str) == Some("WAITING")
        && room.get("chat").and_then(Value::as_str) == Some(chat)
        && (room_name.is_empty() || room.get("name").and_then(Value::as_str) == Some(room_name))
        && !is_expired(room, now)
}

/// Keputusan handler terhadap room yang ada. Tidak ada yang dikirim ke user
/// pada poin ini — teksnya dirangkai pemanggil.
enum Decision {
    /// Pengirim masih punya room — tidak ada perubahan state.
    Busy,
    /// Pengirim masuk ke room yang menunggu.
    Joined { player_x: String, player_o: String },
    /// Room baru dibuat untuk pengirim.
    Created,
}

fn decide(
    rooms: &mut Map<String, Value>,
    room_name: &str,
    sender: &str,
    chat: &str,
    now: i64,
) -> Decision {
    if rooms.values().any(|r| is_active_for(r, sender)) {
        return Decision::Busy;
    }

    let waiting = rooms
        .iter()
        .find(|(_, r)| is_waiting(r, chat, room_name, now))
        .map(|(id, _)| id.clone());

    match waiting {
        Some(id) => {
            let room = rooms
                .get_mut(&id)
                .expect("key baru dibaca dari map yang sama");
            if let Some(obj) = room.as_object_mut() {
                obj.insert("playerO".into(), json!(sender));
                obj.insert("state".into(), json!("PLAYING"));
            }
            Decision::Joined {
                player_x: room
                    .get("playerX")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                player_o: sender.to_string(),
            }
        }
        None => {
            rooms.insert(
                format!("{ROOM_PREFIX}{now}"),
                json!({
                    "id": format!("{ROOM_PREFIX}{now}"),
                    "chat": chat,
                    "name": if room_name.is_empty() { Value::Null } else { json!(room_name) },
                    "playerX": sender,
                    "playerO": Value::Null,
                    "state": "WAITING",
                    "createdAt": now,
                }),
            );
            Decision::Created
        }
    }
}

#[plugin(
    name = "tictactoe",
    alias = ["ttt", "xo"],
    category = "game",
    description = "Main TicTacToe dengan player lain",
    usage = ".tictactoe [room name] atau .ttt",
    example = ".tictactoe",
    cooldown = 5,
    energi = 0,
    is_group = true,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn tictactoe(ctx: Ctx) -> Reply {
    let room_name = ctx.args.join(" ").trim().to_string();
    let now = now_ms();

    let mut decided = None;
    ctx.session_update(GAMES_KEY, |slot| {
        if !slot.is_object() {
            *slot = json!({});
        }
        let rooms = slot.as_object_mut().expect("baru diganti objek");
        decided = Some(decide(rooms, &room_name, &ctx.sender, &ctx.chat, now));
    });

    match decided.expect("decide selalu menghasilkan keputusan") {
        Decision::Busy => Reply::text(BUSY),
        Decision::Created => Reply::text(format!(
            "🎮 *ᴛɪᴄ ᴛᴀᴄ ᴛᴏᴇ*\n\n\
             Room dibuat! Menunggu partner...\n\n\
             > Ketik `.tictactoe{}` untuk join\n\
             > Room akan expire dalam 5 menit",
            if room_name.is_empty() {
                String::new()
            } else {
                format!(" {room_name}")
            }
        ))
        .with_reaction("🕕"),
        Decision::Joined { player_x, player_o } => Reply::Text {
            // `game._currentTurn` masih `false` di room yang baru digabung,
            // jadi `currentTurn` = `playerX`.
            text: format!(
                "🎮 *ᴛɪᴄ ᴛᴀᴄ ᴛᴏᴇ*\n\n\
                 Partner ditemukan!\n\n\
                 ❌ @{}\n⭕ @{}\n\n{}\n\n\
                 > Giliran: @{}\n\
                 > Reply pesan ini dengan angka 1-9\n\
                 > Ketik *nyerah* untuk menyerah",
                jid_number(&player_x),
                jid_number(&player_o),
                fresh_board(),
                jid_number(&player_x),
            ),
            mentions: vec![player_x, player_o],
        }
        .with_reaction("🎮"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use oktz_plugin_api::{SessionStore, new_session_store};

    const CHAT: &str = "120363@g.us";
    const SENDER: &str = "628111@s.whatsapp.net";
    const OTHER: &str = "628999@s.whatsapp.net";

    fn ctx(sender: &str, args: &str, sessions: SessionStore) -> Ctx {
        Ctx {
            prefix: ".".into(),
            command: "tictactoe".into(),
            text: args.into(),
            args: args.split_whitespace().map(str::to_string).collect(),
            sender: sender.into(),
            chat: CHAT.into(),
            is_group: true,
            sessions,
            ..Default::default()
        }
    }

    fn run(sender: &str, args: &str, sessions: SessionStore) -> Reply {
        let c = ctx(sender, args, sessions);
        (oktz_plugin_api::find("tictactoe").unwrap().handler)(c)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    fn reaction_of(r: &Reply) -> Option<&str> {
        match r.without_mutations() {
            Reply::Reaction { emoji, .. } => Some(emoji),
            _ => None,
        }
    }

    fn rooms(sessions: &SessionStore) -> Vec<(String, Value)> {
        sessions
            .lock()
            .unwrap()
            .get(GAMES_KEY)
            .and_then(Value::as_object)
            .map(|o| o.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
            .unwrap_or_default()
    }

    fn waiting_room(id: &str, player_x: &str, name: Option<&str>, created: i64) -> (String, Value) {
        (
            id.into(),
            json!({
                "id": id,
                "chat": CHAT,
                "name": name.map_or(Value::Null, |n| json!(n)),
                "playerX": player_x,
                "playerO": Value::Null,
                "state": "WAITING",
                "createdAt": created,
            }),
        )
    }

    fn seed(sessions: &SessionStore, room: (String, Value)) {
        let mut map = Map::new();
        map.insert(room.0, room.1);
        sessions
            .lock()
            .unwrap()
            .insert(GAMES_KEY.into(), Value::Object(map));
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("tictactoe").unwrap();
        assert_eq!(p.meta.name, "tictactoe");
        assert_eq!(p.meta.alias, ["ttt", "xo"]);
        assert_eq!(p.meta.category, "game");
        assert_eq!(p.meta.description, "Main TicTacToe dengan player lain");
        assert_eq!(p.meta.usage, ".tictactoe [room name] atau .ttt");
        assert_eq!(p.meta.example, ".tictactoe");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_premium && !p.meta.is_admin);
        assert!(!p.meta.is_bot_admin);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["tictactoe", "ttt", "xo"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "tictactoe");
        }
    }

    #[test]
    fn the_first_command_creates_a_waiting_room() {
        let s = new_session_store();
        let r = run(SENDER.clone(), "", s.clone());
        assert_eq!(
            text_of(&r),
            "🎮 *ᴛɪᴄ ᴛᴀᴄ ᴛᴏᴇ*\n\n\
             Room dibuat! Menunggu partner...\n\n\
             > Ketik `.tictactoe` untuk join\n\
             > Room akan expire dalam 5 menit"
        );
        assert_eq!(reaction_of(&r), Some("🕕"));

        let list = rooms(&s);
        assert_eq!(list.len(), 1);
        let (id, room) = &list[0];
        assert!(id.starts_with(ROOM_PREFIX), "{id}");
        assert_eq!(room["id"], json!(id.as_str()));
        assert_eq!(room["chat"], json!(CHAT));
        assert_eq!(room["name"], Value::Null, "room tanpa nama disimpan null");
        assert_eq!(room["playerX"], json!(SENDER));
        assert_eq!(room["playerO"], Value::Null);
        assert_eq!(room["state"], json!("WAITING"));
        assert!(room["createdAt"].is_i64());
    }

    #[test]
    fn a_named_room_echoes_the_name_in_the_join_hint() {
        let s = new_session_store();
        let r = run(SENDER.clone(), " opardu   ultimate ", s.clone());
        assert!(
            text_of(&r).contains("> Ketik `.tictactoe opardu ultimate` untuk join"),
            "{}",
            text_of(&r)
        );
        assert_eq!(rooms(&s)[0].1["name"], json!("opardu ultimate"));
    }

    #[test]
    fn the_second_player_joins_the_waiting_room_of_the_same_chat() {
        let s = new_session_store();
        run(SENDER.clone(), "ruang 1", s.clone());
        let r = run(OTHER.clone(), "ruang 1", s.clone());

        assert!(
            text_of(&r).starts_with("🎮 *ᴛɪᴄ ᴛᴀᴄ ᴛᴏᴇ*\n\nPartner ditemukan!\n\n"),
            "{}",
            text_of(&r)
        );
        assert_eq!(reaction_of(&r), Some("🎮"));
        match r.without_mutations() {
            Reply::Text { mentions, .. } => {
                assert_eq!(mentions, &[SENDER.to_string(), OTHER.to_string()])
            }
            other => panic!("harus Text, dapat {other:?}"),
        }

        let list = rooms(&s);
        assert_eq!(list.len(), 1, "tidak boleh membuat room kedua");
        assert_eq!(list[0].1["state"], json!("PLAYING"));
        assert_eq!(list[0].1["playerX"], json!(SENDER));
        assert_eq!(list[0].1["playerO"], json!(OTHER));
    }

    #[test]
    fn the_joined_message_shows_the_empty_board_and_the_first_players_turn() {
        let s = new_session_store();
        run(SENDER.clone(), "", s.clone());
        let r = run(OTHER.clone(), "", s.clone());
        assert!(
            text_of(&r).contains("❌ @628111\n⭕ @628999"),
            "{}",
            text_of(&r)
        );
        assert!(
            text_of(&r).contains("> Giliran: @628111"),
            "giliran pertama = playerX: {}",
            text_of(&r)
        );
        assert!(text_of(&r).contains(&fresh_board()), "{}", text_of(&r));
    }

    #[test]
    fn a_player_who_is_already_in_a_game_is_turned_away() {
        let s = new_session_store();
        run(SENDER.clone(), "", s.clone());
        let before = rooms(&s);
        let r = run(SENDER.clone(), "nama lain", s.clone());
        assert_eq!(text_of(&r), BUSY);
        assert_eq!(reaction_of(&r), None, "JS tidak bereaksi di cabang ini");
        assert_eq!(rooms(&s), before, "tidak boleh ada perubahan state");
    }

    #[test]
    fn the_player_o_is_also_locked_out_of_creating_a_new_room() {
        let s = new_session_store();
        run(SENDER.clone(), "", s.clone());
        run(OTHER.clone(), "", s.clone());
        let r = run(OTHER.clone(), "", s.clone());
        assert_eq!(text_of(&r), BUSY);
        assert_eq!(rooms(&s).len(), 1);
    }

    #[test]
    fn a_different_room_name_does_not_join_the_waiting_one() {
        let s = new_session_store();
        run(SENDER.clone(), "ruang 1", s.clone());
        run(OTHER.clone(), "ruang 2", s.clone());
        let list = rooms(&s);
        assert_eq!(list.len(), 2, "nama room berbeda → dua room terpisah");
        assert!(list.iter().all(|(_, r)| r["state"] == json!("WAITING")));
    }

    #[test]
    fn an_empty_name_matches_any_waiting_room_but_named_join_does_not() {
        let s = new_session_store();
        run(SENDER.clone(), "ruang 1", s.clone());
        // Tanpa nama, filter dilewati (`roomName ? ... : true`).
        let r = run(OTHER.clone(), "", s.clone());
        assert!(
            text_of(&r).contains("Partner ditemukan!"),
            "{}",
            text_of(&r)
        );
        // Dengan nama yang tidak cocok, tidak ada yang di-join.
        let s2 = new_session_store();
        run(SENDER.clone(), "ruang 1", s2.clone());
        let r = run(OTHER.clone(), "ruang 9", s2.clone());
        assert!(text_of(&r).contains("Room dibuat!"), "{}", text_of(&r));
    }

    #[test]
    fn an_expired_waiting_room_is_treated_as_deleted() {
        let s = new_session_store();
        // `createdAt` 0 berarti sudah lewat 5 menit untuk `now_ms()` sekarang.
        seed(&s, waiting_room("ttt_1", SENDER, None, 0));
        let r = run(OTHER.clone(), "", s.clone());
        assert!(text_of(&r).contains("Room dibuat!"), "{}", text_of(&r));
        assert_eq!(
            rooms(&s).len(),
            2,
            "room basi tidak dihapus, room baru ditambahkan"
        );
    }

    #[test]
    fn the_owner_of_an_expired_waiting_room_is_no_longer_locked_out() {
        let s = new_session_store();
        seed(&s, waiting_room("ttt_1", SENDER, None, 0));
        let r = run(SENDER.clone(), "", s.clone());
        assert!(text_of(&r).contains("Room dibuat!"), "{}", text_of(&r));
    }

    #[test]
    fn a_waiting_room_just_inside_the_ttl_can_still_be_joined() {
        let s = new_session_store();
        seed(
            &s,
            waiting_room("ttt_1", SENDER, None, now_ms() - ROOM_TTL_MS),
        );
        let r = run(OTHER.clone(), "", s.clone());
        assert!(
            text_of(&r).contains("Partner ditemukan!"),
            "tepat di batas: {}",
            text_of(&r)
        );
    }

    #[test]
    fn a_playing_room_never_expires_on_its_own() {
        let s = new_session_store();
        run(SENDER.clone(), "", s.clone());
        run(OTHER.clone(), "", s.clone());
        // Setelah join, `createdAt` sudah tua tapi state PLAYING — JS tidak
        // pernah menghapus room ini lewat setTimeout.
        let r = run(SENDER.clone(), "", s.clone());
        assert_eq!(text_of(&r), BUSY);
        assert_eq!(rooms(&s).len(), 1);
    }

    #[test]
    fn a_room_whose_id_has_no_prefix_never_locks_anybody_out() {
        let s = new_session_store();
        s.lock().unwrap().insert(
            GAMES_KEY.into(),
            json!({"lain": {"id": "lain", "chat": CHAT, "playerX": SENDER, "playerO": Value::Null, "state": "WAITING", "createdAt": now_ms()}}),
        );
        let r = run(SENDER.clone(), "", s.clone());
        assert!(
            text_of(&r).contains("Room dibuat!"),
            "id tanpa `ttt_` bukan milik plugin ini: {}",
            text_of(&r)
        );
    }

    #[test]
    fn the_board_maps_every_symbol_and_passes_unknowns_through() {
        assert_eq!(board_symbol("X"), "❌");
        assert_eq!(board_symbol("O"), "⭕");
        for i in 1..=9 {
            assert_eq!(board_symbol(&i.to_string()), format!("{i}️⃣"), "sel {i}");
        }
        assert_eq!(board_symbol("10"), "10", "di luar tabel → apa adanya");
        assert_eq!(board_symbol(""), "");
    }

    #[test]
    fn the_board_frame_has_nine_cells_and_four_rules() {
        let board = fresh_board();
        assert_eq!(board.lines().count(), 7, "3 baris sel + 4 garis: {board}");
        for (i, sym) in BOARD_SYMBOLS.iter().skip(2) {
            assert!(
                board.contains(&format!("│ {sym} │")),
                "sel {i} hilang: {board}"
            );
        }
        assert_eq!(board.matches('│').count(), 18, "6 sel per baris × 3 baris");
    }

    #[test]
    fn the_board_of_a_marked_up_game_uses_the_symbols_not_the_numbers() {
        let cells = ["X", "O", "1", "2", "3", "4", "5", "6", "7"].map(str::to_string);
        let board = render_board(&cells);
        assert!(board.contains("│ ❌ │ ⭕ │ 1️⃣ │"), "{board}");
        assert!(!board.contains("│ X │"), "X harus jadi ❌: {board}");
    }

    #[test]
    fn a_session_slot_holding_junk_is_replaced_by_an_empty_map() {
        let s = new_session_store();
        s.lock()
            .unwrap()
            .insert(GAMES_KEY.into(), json!("bukan objek"));
        let r = run(SENDER.clone(), "", s.clone());
        assert!(text_of(&r).contains("Room dibuat!"), "{}", text_of(&r));
        assert_eq!(rooms(&s).len(), 1);
    }

    #[test]
    fn a_very_long_room_name_is_echoed_whole_without_being_truncated() {
        let s = new_session_store();
        let name = "a".repeat(500);
        let r = run(SENDER.clone(), &name, s.clone());
        assert!(
            text_of(&r).contains(&format!(".tictactoe {name}`. untuk join")),
            "nama panjang harus utuh"
        );
        assert_eq!(rooms(&s)[0].1["name"], json!(name));
    }
}
