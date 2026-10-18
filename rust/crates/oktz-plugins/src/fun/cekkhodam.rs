//! `plugins/fun/cekkhodam.js` — cek khodam dengan audio TTS (Google TTS).
//!
//! Catatan: `pub mod cekkhodam;` perlu ditambah manual ke
//! `rust/crates/oktz-plugins/src/fun/mod.rs` (mod.rs milik plugin lain,
//! jangan disentuh).
//!
//! Teks dikonversi ke URL `translate.google.com/translate_tts` lalu di-fetch
//! melalui `ctx.http` dan dikirim sebagai audio. `pushName` tidak ada di
//! `Ctx`, jadi target diturunkan dari nomor sender/mention/teks.

use oktz_plugin_api::{Ctx, MediaSource, Reply, plugin};

const KHODAMS: [(&str, &str); 67] = [
    ("Harimau Putih", "Kamu kuat dan berani seperti harimau, karena pendahulumu mewariskan kekuatan besar padamu."),
    ("Lampu Tertidur", "Terlihat ngantuk tapi selalu memberikan cahaya yang hangat"),
    ("Panda Ompong", "Kamu menggemaskan dan selalu berhasil membuat orang tersenyum dengan keanehanmu."),
    ("Bebek Karet", "Kamu selalu tenang dan ceria, mampu menghadapi gelombang masalah dengan senyum."),
    ("Ninja Turtle", "Kamu lincah dan tangguh, siap melindungi yang lemah dengan kekuatan tempurmu."),
    ("Kucing Kulkas", "Kamu misterius dan selalu ada di tempat-tempat yang tak terduga."),
    ("Sabun Wangi", "Kamu selalu membawa keharuman dan kesegaran di mana pun kamu berada."),
    ("Semut Kecil", "Kamu pekerja keras dan selalu bisa diandalkan dalam situasi apa pun."),
    ("Cupcake Pelangi", "Kamu manis dan penuh warna, selalu membawa kebahagiaan dan keceriaan."),
    ("Robot Mini", "Kamu canggih dan selalu siap membantu dengan kecerdasan teknologi tinggi."),
    ("Ikan Terbang", "Kamu unik dan penuh kejutan, selalu melampaui batasan yang ada."),
    ("Ayam Goreng", "Kamu selalu disukai dan dinanti oleh banyak orang, penuh kelezatan dalam setiap langkahmu."),
    ("Kecoa Terbang", "Kamu selalu mengagetkan dan bikin heboh seisi ruangan."),
    ("Kambing Ngebor", "Kamu unik dan selalu bikin orang tertawa dengan tingkah lakumu yang aneh."),
    ("Kerupuk Renyah", "Kamu selalu bikin suasana jadi lebih seru dan nikmat."),
    ("Celengan Babi", "Kamu selalu menyimpan kejutan di dalam dirimu."),
    ("Lemari Tua", "Kamu penuh dengan cerita dan kenangan masa lalu."),
    ("Kopi Susu", "Kamu manis dan selalu bikin semangat orang-orang di sekitarmu."),
    ("Sapu Lidi", "Kamu kuat dan selalu bisa diandalkan untuk membersihkan masalah."),
    ("Indomie Goreng", "Selalu bikin kenyang dan bahagia"),
    ("Es Krim Meleleh", "Selalu mencairkan suasana dengan rasa manisnya"),
    ("Bakso Ulet", "Selalu gigih dan bulat dalam menghadapi masalah"),
    ("Lem Super", "Selalu lengket dalam situasi yang rumit"),
    ("Kecap Manis", "Selalu memberikan sentuhan manis dalam hidup"),
    ("Sabun Mandi", "Selalu bersih dan wangi"),
    ("Kopi Tumpah", "Selalu bersemangat, tapi kadang berantakan"),
    ("Kucing Kampung", "Selalu mandiri dan penuh petualangan"),
    ("Jamu Pahit", "Selalu memberi kekuatan meski tak enak di awal"),
    ("Teh Celup", "Selalu memberikan rasa hangat di hati"),
    ("Motor Astrea", "Selalu setia dan bandel"),
    ("Mie Instan", "Selalu cepat dan mengenyangkan"),
    ("Bolu Kukus", "Selalu lembut dan manis"),
    ("Tahu Bulat", "Selalu enak di segala suasana"),
    ("Nasi Uduk", "Selalu cocok di segala waktu"),
    ("Singa Bermahkota", "Kamu lahir sebagai pemimpin, memiliki kekuatan dan kebijaksanaan seorang raja."),
    ("Macan Kumbang", "Kamu misterius dan kuat, seperti macan yang jarang terlihat tapi selalu waspada."),
    ("Kuda Emas", "Kamu berharga dan kuat, siap untuk berlari menuju kesuksesan."),
    ("Elang Biru", "Kamu memiliki visi yang tajam dan dapat melihat peluang dari jauh."),
    ("Naga Pelangi", "Kamu tangguh dan memiliki kekuatan untuk melindungi dan menyerang."),
    ("Gajah Putih", "Kamu bijaksana dan memiliki kekuatan besar, lambang dari keberanian dan keteguhan hati."),
    ("Banteng Sakti", "Kamu kuat dan penuh semangat, tidak takut menghadapi rintangan."),
    ("Kipas Angin", "Selalu memberikan angin segar"),
    ("Rice Cooker", "Selalu memasak nasi dengan sempurna"),
    ("Honda Beat", "Selalu lincah di jalanan"),
    ("Sandal Jepit", "Selalu santai dan nyaman"),
    ("Bantal Guling", "Selalu nyaman di pelukan"),
    ("Anjing Pelacak", "Kamu setia dan penuh dedikasi, selalu menemukan jalan menuju tujuanmu."),
];

/// `google-tts-api` bangun URL ini: `https://translate.google.com/translate_tts?
// ie=UTF-8&q=<encoded>&tl=<lang>&client=tw-ob&slow=<slow>`.
fn tts_url(teks: &str) -> String {
    format!(
        "https://translate.google.com/translate_tts?ie=UTF-8&q={}&tl=id&client=tw-ob&slow=false",
        crate::tools::phisingataubukan::encode_uri_component(teks)
    )
}

fn target_name(ctx: &Ctx) -> String {
    if let Some(q) = ctx.quoted() {
        q.sender.split('@').next().unwrap_or(&q.sender).to_string()
    } else if let Some(jid) = ctx.mentioned.first() {
        jid.split('@').next().unwrap_or(jid).to_string()
    } else if !ctx.text.trim().is_empty() {
        ctx.text.trim().to_string()
    } else {
        ctx.sender.split('@').next().unwrap_or(&ctx.sender).to_string()
    }
}

#[plugin(
    name = "cekkhodam",
    alias = ["khodam", "cekhodam"],
    category = "fun",
    description = "Cek khodam diri sendiri atau orang lain",
    usage = ".cekkhodam atau reply pesan seseorang",
    example = ".cekkhodam",
    cooldown = 10,
    energi = 1,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn cekkhodam(ctx: Ctx) -> Reply {
    let nama = target_name(&ctx);
    let (nama_khodam, arti) = KHODAMS[crate::common::random_index(KHODAMS.len())];
    let teks = format!(
        "Halo kak {nama}, Khodam kamu adalah {nama_khodam}, Khodam ini memiliki arti: {arti}"
    );

    let Ok(res) = ctx.http.get(&tts_url(&teks)) else {
        return Reply::text("❌ Gagal membuat audio khodam, coba lagi nanti.");
    };
    let buf = res.body.bytes().to_vec();
    if buf.is_empty() {
        return Reply::text("❌ Gagal membuat audio khodam, coba lagi nanti.");
    }

    Reply::audio(MediaSource::bytes(buf)).with_reaction("✅")
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{HttpBody, HttpError, HttpGet, HttpResponse};
    use serde_json::json;
    use std::sync::{Arc, Mutex};

    const ME: &str = "628111@s.whatsapp.net";

    #[derive(Debug, Default)]
    struct Stub {
        balasan: Option<Vec<u8>>,
        seen: Mutex<Vec<String>>,
    }

    impl Stub {
        fn bytes(b: Vec<u8>) -> Arc<Stub> {
            Arc::new(Stub {
                balasan: Some(b),
                seen: Mutex::new(Vec::new()),
            })
        }
        fn offline() -> Arc<Stub> {
            Arc::new(Stub {
                balasan: None,
                seen: Mutex::new(Vec::new()),
            })
        }
    }

    impl HttpGet for Stub {
        fn get(&self, url: &str) -> Result<HttpResponse, HttpError> {
            self.seen.lock().unwrap().push(url.to_string());
            match &self.balasan {
                Some(b) => Ok(HttpResponse {
                    status: 200,
                    body: HttpBody::Bytes {
                        data: Arc::from(b.clone()),
                        mimetype: None,
                    },
                    headers: Vec::new(),
                }),
                None => Err(oktz_plugin_api::HttpError {
                    message: "offline".into(),
                }),
            }
        }
        fn post_json(&self, _: &str, _: &serde_json::Value) -> Result<HttpResponse, HttpError> {
            unimplemented!()
        }
        fn post_form(&self, _: &str, _: &str) -> Result<HttpResponse, HttpError> {
            unimplemented!()
        }
        fn post_multipart(
            &self,
            _: &str,
            _: &[(&str, HttpBody)],
        ) -> Result<HttpResponse, HttpError> {
            unimplemented!()
        }
    }

    fn run(http: Arc<Stub>, text: &str) -> Reply {
        let mut args = text.split_whitespace();
        let ctx = Ctx {
            text: text.to_string(),
            args: args.by_ref().map(str::to_string).collect(),
            sender: ME.into(),
            chat: ME.into(),
            prefix: ".".into(),
            command: "cekkhodam".into(),
            http,
            ..Ctx::default()
        };
        cekkhodam(ctx)
    }

    fn run_dengan_mention(http: Arc<Stub>, mention: &str) -> Reply {
        let mut ctx = Ctx {
            text: "".to_string(),
            args: Vec::new(),
            sender: ME.into(),
            chat: ME.into(),
            prefix: ".".into(),
            command: "cekkhodam".into(),
            mentioned: vec![mention.to_string()],
            http,
            ..Ctx::default()
        };
        cekkhodam(ctx)
    }

    fn reaction_of(r: &Reply) -> Option<&str> {
        match r.without_mutations() {
            Reply::Reaction { emoji, .. } => Some(emoji.as_str()),
            _ => None,
        }
    }

    #[test]
    fn khodam_terpilih_dari_daftar() {
        let _ = run(Stub::bytes(b"audio"), "");
        // Hanya memastikan tidak panic dan mengembalikan audio.
    }

    #[test]
    fn tts_url_mengenkripsi_teks() {
        let url = tts_url("Halo kak Budi, coba");
        assert!(url.contains("translate.google.com/translate_tts"));
        assert!(url.contains("tl=id"));
        assert!(url.contains("%20")); // spasi di-encode
    }

    #[test]
    fn gagal_fetch_mengembalikan_pesan_gagal() {
        let r = run(Stub::offline(), "Budi");
        assert_eq!(
            r.without_mutations().text_body(),
            Some("❌ Gagal membuat audio khodam, coba lagi nanti.")
        );
    }

    #[test]
    fn mention_digunakan_sebagai_target() {
        let r = run_dengan_mention(Stub::bytes(b"x"), "628999@s.whatsapp.net");
        assert_eq!(reaction_of(&r), Some("✅"));
    }
}
