use crate::common::{data_plugin, data_reply};
use oktz_plugin_api::{Ctx, Reply, plugin};

/// `src/data/dare.json` — 63 entris, verbatim.
pub(crate) const DARE: [&str; 63] = [
    "Ajak orang yg tidak kamu kenal itu selfie berdua dengan mu lalu upload ke snapgram",
    "Ambil beberapa nomor dari kontakmu secara acak dan kirim sms 'Aku hamil' sama mereka.",
    "Ambil minuman apa saja yg ada didekat mu lalu campurkan dengan cabai dan minum!",
    "Ambil nomor secara acak dari kontakmu, telepon dia, dan bilang 'Aku mencintaimu'",
    "Beli makanan paling murah di kantin (atau beli sebotol aqua) dan bilang sambil tersedu-sedu pada teman sekelasmu 'Ini.. adalah makanan yang paling mahal yang pernah kubeli.. Hiks'",
    "Beli satu botol coca cola dan siram bunga dengan coca cola itu di depan orang banyak.",
    "Berdiri deket kulkas, tutup mata, pilih makanan secara acak didalemnya, pas makanpun mata harus tetep ditutup.",
    "Berdiri di tengah lapangan basket dan berteriak, 'AKU MENCINTAIMU PANGERANKU/PUTRIKU'",
    "Beri hormat pada seseorang di kelas, lalu bilang 'Hamba siap melayani Anda, Yang Mulia.'",
    "Berjalan sambil bertepuk tangan dan menyanyi lagu 'Selamat Ulang Tahun' dari kelas ke koridor.",
    "Berlutut satu kaki dan bilang 'Marry me?' sama orang pertama yang masuk ke ruangan.",
    "Bikin hiasan kepala absurd dari tisu, apapun itu, terus suruh pose didepan kamera, terus upload",
    "Bilang 'KAMU CANTIK BANGET NGGAK BOHONG' sama cewek yang menurutmu paling cantik di kelas ini",
    "Bilang pada seorang guru, 'Bu/Pak, baju saya terasa sempit' dengan ekspresi memelas.",
    "Bilang pada seseorang di kelas, 'Aku baru saja diberi tahu aku adalah kembaranmu dulu, kita dipisahkan, lalu aku menjalani operasi plastik. Dan ini adalah hal paling serius yang pernah aku katakan.'",
    "Buang buku catatan seseorang ke tempat sampah, di depan matanya, sambil bilang 'Buku ini isinya tidak ada yang bisa memahami'",
    "Cabut bulu kaki mu sendiri sebanyak 3x",
    "Chat kedua orangtuamu, katakan bahwa kamu kangen dengan mereka lengkap dengan emoticon sedih.",
    "Coba searcing google mengenai hal-hal yang mengerikan atau menggelikan seperti trypophobia, dll.",
    "Duduk relaks di tengah lapangan basket sambil berpura-pura itu adalah pantai untuk berjemur.",
    "isi mulut penuh dengan air dan harus tahan hingga dua putaran. Jika tertawa dan tumpah atau terminum, maka harus ngisi ulang dan ditambah satu putaran lagi.",
    "Jabat tangan orang pertama yang masuk ke ruangan ini dan bilang 'Selamat datang di Who Wants To Be a Millionaire!'",
    "Kirim sms pada orangtuamu 'Hai, bro! Aku baru beli majalah Playboy edisi terbaru!'",
    "Kirim sms pada orangtuamu, 'Ma, Pa, aku sudah tahu bahwa aku adalah anak adopsi dari Panti Asuhan. Jangan menyembunyikan hal ini lagi.'",
    "Kirim sms pada tiga nomor acak di kontakmu dan tulis 'Aku baru saja menjadi model majalah Playboy.'",
    "Makan 1 Sendok makan kecap manis dan kecap asin!",
    "Makan sesuatu tapi gak pake tangan.",
    "Marah-marah ketemen kamu yang gak dateng padahal udah janjian mau main 'truth or dare' bareng-bareng",
    "Mecahin telur pake kepala.",
    "Memakan makanan yang sudah dicampur-campur dan rasanya pasti aneh, namun pastikan bahwa makanan itu tidak berbahaya untuk kesehatan jangka panjang maupun jangka pendek.",
    "Menari ala Girls' Generation untuk cowok di depan kelas, atau menari ala Super Junior untuk cewek.",
    "Mengerek tiang bendera tanpa ada benderanya.",
    "Menggombali orang yang ditaksir, sahabat terdekat, lawan jenis yang tidak dikenal sama sekali dan  sejenisnya.",
    "Meniru style rambut semua temen kamu.",
    "Menyanyikan lagu HAI TAYO di depan banyak orang sambil menari",
    "Menyanyikan lagu Iwak Peyek dengan keras di ruang kelas.",
    "Minjem sesuatu ke tetangga",
    "Minta tandatangan pada seorang guru yang paling kamu benci sambil bilang 'Anda benar-benar orang yang paling saya kagumi di dunia.'",
    "Minta uang pada seseorang (random/acak) di jalan sambil bilang 'Saya tidak punya uang untuk naik angkot.'",
    "Minum sesuatu yang udah dibuat/disepakatin, tapi pastiin gak berbahaya, bisa kayak minum sirup yang digaremin terus ditambah kecap.",
    "Minum tiga teguk teh atau coke (coca-cola atau sprite) yang dicampur sambal.",
    "Ngomong ke gebetannya emoticon-Takut, ngobrol ngalurngidul apapun lah boleh ,via manapun juga bisa.",
    "Nyanyi-nyanyi lagu favorit difilm disney diluar rumah sambil teriak-teriak.",
    "Nyebutin 1 biru sampai 10 biru dengan cepat dan tidak boleh melakukan kesalahan. Jika salah maka harus diulang dari awal.",
    "Pakai mahkota tiruan dari kertas buku dan bilang sama setiap orang di ruangan 'BERI PENGHORMATAN PADA YANG MULIA RAJA' sambil menunjuk setiap orang dengan penggaris.",
    "Pake celana kebalik sampe besok paginya.",
    "Pegang bola basket, berdiri di depan kelas, dan berteriak, 'ADA YANG TAHU MENGAPA BOLA GOLF INI SANGAT BESAR? APA PABRIKNYA SALAH CETAK?'",
    "Peluk orang yang NGGAK kamu sukai di kelas dan bilang, 'Terimakasih banyak kamu sudah bersedia menjadi orang paling baik untukku.'",
    "Pergi ke lapangan yg luas, lalu berlari sekencang kencangnya sambil mengatakan 'aku gila aku gila'",
    "Petik 1 bunga lalu tancapkan bunga itu ke orang yg tidak kamu kenal (harus lawan jenis ya)",
    "Pilih orang secara acak di jalan, lalu bilang 'You don't know you're beautiful' (ala One Direction)",
    "Pura pura kerasukan ex: kerasukan macan dll",
    "Suruh bersiul pas mulutnya lagi penuh dijejelin makanan.",
    "Suruh jadi pelayan buat ngelayanin kamu sama temen-temen kamu buat makan siang.",
    "Suruh pake kaos kaki buat dijadiin sarung tangan.",
    "Suruh pake topi paling aneh/helm paling absurd selama 3 putaraann kedepan.",
    "Telpon mama kamu dan bilang 'ma, aku mau nikah secepatnya'",
    "Telpon mantan kamu dan bialng 'aku rindu kamu'",
    "Teriak 'WOI GW JACK, DENGER NIH RAUNGAN GW, ROAAAAR!' ditempat rame",
    "Tuker baju sama orang terdekat sampe ronde berikutnya.",
    "Update status di BBM, Line, WA, atau apapun itu dengan kata kata yang semuanya berawalan 'T'",
    "Upload video dia nyanyi ke youtube yang lagi nyanyiin lagu-lagu populer",
    "Warnain kuku kaki dan tangan tapi dengan warna berbeda-beda buat seminggu",
];

data_plugin!(
    dare,
    "dare",
    ["dareq", "tantang"],
    "Random tantangan dare",
    ".dare",
    ".dare",
    DARE
);

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::find;

    const SENDER: &str = "628111@s.whatsapp.net";

    fn run(text: &str) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "dare".into(),
            sender: SENDER.into(),
            chat: SENDER.into(),
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            ..Default::default()
        };
        (find("dare").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> String {
        r.text_body().expect("plugin harus membalas teks").to_string()
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let m = &find("dare").unwrap().meta;
        assert_eq!(m.name, "dare");
        assert_eq!(m.alias, ["dareq", "tantang"]);
        assert_eq!(m.category, "fun");
        assert_eq!(m.description, "Random tantangan dare");
        assert_eq!(m.usage, ".dare");
        assert_eq!(m.example, ".dare");
        assert_eq!(m.cooldown, 3);
        assert_eq!(m.energi, 0);
        assert!(!m.is_private && !m.is_group && !m.is_owner && !m.is_premium);
        for a in ["dareq", "tantang"] {
            assert_eq!(find(a).unwrap().meta.name, "dare", "alias {a}");
        }
    }

    #[test]
    fn the_reply_wraps_the_dare_in_a_code_fence_without_quotes() {
        // JS: `` ````${challenge}```` — tanpa kutip, beda dari bucin.js.
        for _ in 0..50 {
            let t = text_of(&run("")).to_string();
            assert!(t.starts_with("```") && t.ends_with("```"), "{t}");
            assert!(!t.starts_with("```\""), "tidak ada kutip: {t}");
            let inner = &t[3..t.len() - 3];
            assert!(DARE.contains(&inner), "di luar daftar JS: {inner}");
        }
    }

    #[test]
    fn an_argument_does_not_change_the_shape_of_the_reply() {
        let __balasan = run("teman");
        let t = text_of(&__balasan);
        assert!(t.starts_with("```") && t.ends_with("```"), "{t}");
    }

    #[test]
    fn the_table_is_the_sixty_three_entries_of_dare_json() {
        assert_eq!(DARE.len(), 63);
        assert_eq!(DARE[0], "Ajak orang yg tidak kamu kenal itu selfie berdua dengan mu lalu upload ke snapgram");
        assert!(DARE.iter().all(|d| !d.is_empty()));
    }
}
