import axios from "axios";
import { getDatabase } from "../../src/lib/ourin-database.js";
import te from "../../src/lib/ourin-error.js";
import { prepareWAMessageMedia, generateWAMessageFromContent } from "ourin";
import { addExpWithLevelCheck } from "../../src/lib/ourin-level.js";
import { rollHusbu, applyAction, rollEvent, getDailyMood, DOWRY } from "../../src/lib/ourin-husbu.js";
import { angerEffMood, angerUpdate, applyNeglect, finalGain, moodLabel, todayStr } from "../../src/lib/ourin-romance.js";

const pluginConfig = {
  name: ["gachahusbu", "husbuaction", "tinggalinhusbu", "husbuku", "suamiku"],
  alias: ["gachasuami"],
  category: "fun",
  description: "Gacha husbu impianmu, rebut hatinya, dan jadikan dia pasanganmu!",
  usage: ".gachahusbu | .husbuku | .tinggalinhusbu",
  example: ".gachahusbu",
  isOwner: false,
  isPremium: false,
  isGroup: false,
  isPrivate: false,
  cooldown: 5,
  energi: 2,
  isEnabled: true,
};

async function getHusbuImage(keyword) {
  try {
    const res = await axios.get(`https://api.cuki.biz.id/api/search/pinterest?apikey=cuki-x&query=${encodeURIComponent(keyword)}&type=image`);
    const results = res.data?.data?.results;
    if (results && results.length > 0) {
      const validImages = results.filter((item) => item.image_url);
      if (validImages.length > 0) {
        const limit = Math.min(15, validImages.length);
        return validImages[Math.floor(Math.random() * limit)].image_url;
      }
    }
  } catch (e) {
    console.error("[GachaHusbu] Pinterest API error:", e.message);
  }
  return "https://i.pinimg.com/736x/8f/3e/2a/8f3e2a77ec65cdbcfad4ff3bc17e825f.jpg";
}

async function getBuffer(url) {
  try {
    const res = await axios.get(url, { responseType: "arraybuffer", timeout: 15000 });
    return Buffer.from(res.data);
  } catch {
    return null;
  }
}

function moodState(user, m) {
  const h = user.husbu;
  if (h.moodUntil && Date.now() < new Date(h.moodUntil).getTime() && h.mood?.type) return h.mood.type;
  return getDailyMood(m.sender, todayStr());
}

function angerMeter(h) {
  const a = h.anger || 0;
  const n = Math.round(a / 10);
  return "█".repeat(n) + "░".repeat(10 - n) + ` (${a}/100)`;
}

async function sendHusbuMessage(m, sock, husbu, textContent, customButtons = null) {
  let imgBuffer = null;
  if (husbu.imageUrl) imgBuffer = await getBuffer(husbu.imageUrl);
  if (!imgBuffer) {
    const newUrl = await getHusbuImage(husbu.keyword);
    husbu.imageUrl = newUrl;
    imgBuffer = await getBuffer(newUrl) || Buffer.alloc(0);
  }
  const media = await prepareWAMessageMedia({ image: imgBuffer }, { upload: sock.waUploadToServer });
  let buttons = customButtons;
  if (!buttons) {
    if (husbu.affection < 80) {
      buttons = [
        { name: "quick_reply", buttonParamsJson: JSON.stringify({ display_text: "🚶 Jalan-jalan", id: `${m.prefix}husbuaction menu_jalanjalan` }) },
        { name: "quick_reply", buttonParamsJson: JSON.stringify({ display_text: "☕ Kafe", id: `${m.prefix}husbuaction menu_kafe` }) },
        { name: "quick_reply", buttonParamsJson: JSON.stringify({ display_text: "🎬 Bioskop", id: `${m.prefix}husbuaction menu_bioskop` }) },
        { name: "quick_reply", buttonParamsJson: JSON.stringify({ display_text: "🛍️ Belanja", id: `${m.prefix}husbuaction menu_belanja` }) },
        { name: "quick_reply", buttonParamsJson: JSON.stringify({ display_text: "🎮 Lainnya", id: `${m.prefix}husbuaction menu_lainnya` }) },
      ];
    } else if (husbu.affection < 100) {
      buttons = [
        { name: "quick_reply", buttonParamsJson: JSON.stringify({ display_text: "🫂 Peluk", id: `${m.prefix}husbuaction menu_peluk` }) },
        { name: "quick_reply", buttonParamsJson: JSON.stringify({ display_text: "💋 Cium", id: `${m.prefix}husbuaction menu_cium` }) },
        { name: "quick_reply", buttonParamsJson: JSON.stringify({ display_text: "🛏️ Tidur", id: `${m.prefix}husbuaction menu_tidur` }) },
        { name: "quick_reply", buttonParamsJson: JSON.stringify({ display_text: "🛁 Mandi", id: `${m.prefix}husbuaction menu_mandi` }) },
        { name: "quick_reply", buttonParamsJson: JSON.stringify({ display_text: "🎁 Lainnya", id: `${m.prefix}husbuaction menu_lainnya` }) },
      ];
    } else if (!husbu.married) {
      buttons = [
        { name: "quick_reply", buttonParamsJson: JSON.stringify({ display_text: "💍 Terima Lamaran", id: `${m.prefix}husbuaction nikah` }) },
        { name: "quick_reply", buttonParamsJson: JSON.stringify({ display_text: "💝 Hadiah", id: `${m.prefix}husbuaction hadiah` }) },
      ];
    } else {
      buttons = [
        { name: "quick_reply", buttonParamsJson: JSON.stringify({ display_text: "👩‍❤️‍👨 Mesra", id: `${m.prefix}husbuaction mesra` }) },
        { name: "quick_reply", buttonParamsJson: JSON.stringify({ display_text: "💌 Rayu", id: `${m.prefix}husbuaction rayu` }) },
        { name: "quick_reply", buttonParamsJson: JSON.stringify({ display_text: "🌴 Bulan Madu", id: `${m.prefix}husbuaction menu_bulanmadu` }) },
        { name: "quick_reply", buttonParamsJson: JSON.stringify({ display_text: "🎁 Klaim Harian", id: `${m.prefix}husbuaction klaim` }) },
      ];
    }
  }
  let footerText = "❤️ Buat dia luluh dan jatuh cinta padamu!";
  if (customButtons) footerText = "💭 Dia menunggu jawabanmu...";
  else if (husbu.married) footerText = "❤️ Kamu adalah istrinya yang sah!";

  const msg = generateWAMessageFromContent(m.chat, {
    viewOnceMessage: {
      message: {
        messageContextInfo: { deviceListMetadata: {}, deviceListMetadataVersion: 2 },
        interactiveMessage: {
          body: { text: textContent },
          footer: { text: footerText },
          header: {
            title: `🌟 *${husbu.tier.toUpperCase()} TIER HUSBANDO* 🌟`,
            subtitle: husbu.name,
            hasMediaAttachment: true,
            imageMessage: media.imageMessage,
          },
          nativeFlowMessage: { buttons },
        },
      },
    },
  }, { quoted: m });

  await sock.relayMessage(m.chat, msg.message, { messageId: msg.key.id });
}

function marrageDay(husbu) {
  if (!husbu.marriedDate) return 0;
  const a = new Date(husbu.marriedDate);
  const b = new Date();
  return Math.floor((b - a) / 86400000) + 1;
}

function initStats(user) {
  if (!user.husbuStats) user.husbuStats = { totalGacha: 0, byTier: {}, pityCounter: 0, rarest: null, marriedCount: 0 };
  return user.husbuStats;
}

function recordPull(user, husbu, stats) {
  stats.totalGacha++;
  stats.byTier[husbu.tier] = (stats.byTier[husbu.tier] || 0) + 1;
  const isEpicPlus = ["Epic", "Legendary", "Mythic"].includes(husbu.tier);
  stats.pityCounter = isEpicPlus ? 0 : (stats.pityCounter || 0) + 1;
  if (!stats.rarest || ["Common", "Rare", "Epic", "Legendary", "Mythic"].indexOf(husbu.tier) > ["Common", "Rare", "Epic", "Legendary", "Mythic"].indexOf(stats.rarest.tier)) {
    stats.rarest = { name: husbu.name, tier: husbu.tier };
  }
  if (!user.husbuHistory) user.husbuHistory = [];
  user.husbuHistory.push({ name: husbu.name, series: husbu.series, tier: husbu.tier, at: new Date().toISOString() });
  if (user.husbuHistory.length > 100) user.husbuHistory = user.husbuHistory.slice(-100);
}

const HUSB_PERSONALITIES = ["tsundere", "kuudere", "genki", "yandere", "dandere", "oji-san", "playboy", "prince", "badboy", "sunao", "femboy"];

async function handler(m, { sock }) {
  const db = getDatabase();
  const user = db.getUser(m.sender);
  if (!user) return;

  const cmd = m.command.toLowerCase();

  if (cmd === "husbuku" || cmd === "suamiku") {
    if (!user.husbu) return m.reply(`⚠️ *Kamu belum memiliki husbu!*\nSilakan ketik *${m.prefix}gachahusbu* untuk memulainya!`);
    m.react("🕕");
    const h = user.husbu;
    const mood = moodState(user, m);
    const { decay } = applyNeglect(h);
    let status = h.married ? "Telah Menikah 💍" : "Pendekatan 💖";
    const day = marrageDay(h);
    const moodLine = h.married
      ? `\n🗓️ *Hari ke-${day}*${day >= 7 ? ` ${day >= 100 ? "🏆" : day >= 30 ? "🎖️" : "🎉"}` : ""}`
      : "";
    let textContent = `📸 *STATUS HUSBU KAMU* 📸\n\n` +
      `💖 *Nama:* ${h.name}\n` +
      `💎 *Tier:* ${h.tier}\n` +
      `🎭 *Personality:* ${h.personality}\n` +
      `🌤️ *Mood hari ini:* ${moodLabel(mood)}\n` +
      `😡 *Anger:* ${angerMeter(h)}\n` +
      `💞 *Affection:* ${h.affection}/100\n` +
      `💍 *Status:* ${status}${moodLine}\n`;
    if (decay > 0) textContent += `📉 *Neglect:* Affection turun -${decay} karena kamu jarang interaksi!\n`;
    if ((h.anger || 0) >= 50) textContent += `⚠️ *Dia sedang MARAH!* Perbaiki hubungan sebelum affection habis!\n`;
    textContent += `\nLanjutkan interaksi dengan memilih aksi di bawah!`;
    user.husbu = h;
    db.setUser(m.sender, user);
    m.react("✅");
    return await sendHusbuMessage(m, sock, h, textContent, null);
  }

  if (cmd === "tinggalinhusbu") {
    if (!user.husbu) return m.reply(`⚠️ *Kamu bahkan belum punya husbu!* Cari dulu gih!`);
    const husbuName = user.husbu.name;
    const husbuJid = 'husbu_' + husbuName.replace(/\s+/g, '') + '@s.whatsapp.net';
    if (user.husbu.married) {
      if (user.fun && user.fun.pasangan === husbuJid) user.fun.pasangan = "";
      db.setUser(husbuJid, { fun: { pasangan: "" } });
    }
    delete user.husbu;
    db.setUser(m.sender, user);
    m.react("💔");
    return m.reply(`💔 *KAMU MENCAMPAKKAN ${husbuName.toUpperCase()}!*\n\nKamu mengembalikan barang-barangnya dan memintanya untuk pergi. Dia menatapmu dengan mata kecewa yang mendalam, berbalik tanpa sepatah kata pun, lalu menghilang di tengah hujan.\n\nKalian kini resmi berpisah.`);
  }

  if (cmd === "gachahusbu" || cmd === "gachasuami") {
    if (user.husbu) {
      m.react("😡");
      let pesanStatus = user.husbu.married ? "Dia sudah menjadi suamimu!" : "Dia sedang berusaha meluluhkan hatimu!";
      return m.reply(`⚠️ *Kamu sudah memiliki Husbu!*\n\nNama: *${user.husbu.name}*\nTier: *${user.husbu.tier}*\nAffection: *${user.husbu.affection}/100*\n\nJangan serakah! Jaga husbu yang kamu miliki sekarang. ${pesanStatus} Ketik *${m.prefix}husbuku* untuk berinteraksi dengannya.`);
    }
    const sub = (m.args[0] || "").toLowerCase();
    if (sub !== "start") {
      const panduan = `💕 *SISTEM GACHA HUSBU* 💕\n\n` +
        `Simulasi kencan virtual interaktif untuk mendapatkan laki-laki anime idamanmu! Tarik perhatiannya, buat dia jatuh cinta, dan nikahi dia!\n\n` +
        `*PENGGUNAAN:*\n` +
        `• *${m.prefix}gachahusbu* — Panduan ini\n` +
        `• *${m.prefix}husbuku* — Panel interaksi\n` +
        `• *${m.prefix}husbualbum* — Riwayat & statistik luck\n` +
        `• *${m.prefix}husbupool* — Jelajahi pool husbu\n` +
        `• *${m.prefix}tinggalinhusbu* — Putuskan hubungan\n\n` +
        `*ALUR:*\n1. Tekan tombol **Panggil Husbando**.\n2. 3 Fase: Pendekatan (<80) → Intim (80–99) → Menikah (100).\n3. Mood, personality & anger memengaruhi poin affection.\n4. Pity: 20 roll tanpa Epic+ dijamin dapat Epic+.\n5. Affection makin sulit ditambah makin tinggi + interaksi berulang menyusut.`;
      const msg = generateWAMessageFromContent(m.chat, {
        viewOnceMessage: {
          message: {
            messageContextInfo: { deviceListMetadata: {}, deviceListMetadataVersion: 2 },
            interactiveMessage: {
              body: { text: panduan },
              footer: { text: "Tekan tombol untuk mulai!" },
              nativeFlowMessage: { buttons: [{ name: "quick_reply", buttonParamsJson: JSON.stringify({ display_text: "🎲 Panggil Husbando!", id: `${m.prefix}gachahusbu start` }) }] },
            },
          },
        },
      }, { quoted: m });
      return await sock.relayMessage(m.chat, msg.message, { messageId: msg.key.id });
    }

    m.react("🕕");
    try {
      const stats = initStats(user);
      const picked = rollHusbu(stats.pityCounter);
      const husbu = {
        ...picked,
        affection: 50,
        married: false,
        imageUrl: await getHusbuImage(picked.keyword),
      };
      recordPull(user, husbu, stats);
      user.husbu = husbu;
      db.setUser(m.sender, user);
      const textContent = `🎉 *KYAAA! KAMU MENDAPATKAN HUSBANDO BARU!* 🎉\n\n` +
        `💖 *Nama:* ${husbu.name}\n🎂 *Usia:* ${husbu.age} tahun\n📏 *Tinggi:* ${husbu.height}\n⚖️ *Berat:* ${husbu.weight}\n` +
        `💎 *Tier:* ${husbu.tier}\n🎭 *Personality:* ${husbu.personality}\n💞 *Affection:* ${husbu.affection}/100\n\n` +
        `Pilih interaksi untuk mulai PDKT. Hati-hati jangan sampai affection habis!`;
      m.react("✅");
      await sendHusbuMessage(m, sock, husbu, textContent, null);
    } catch (err) {
      console.error(err);
      m.react("☢");
      return m.reply(te(m.prefix, m.command, m.pushName));
    }
    return;
  }

  if (cmd === "husbuaction") {
    if (!user.husbu) { m.react("❌"); return m.reply(`Kamu belum memiliki husbu! Ketik *${m.prefix}gachahusbu* untuk memulai.`); }

    const action = (m.args[0] || "").toLowerCase();
    const husbu = user.husbu;
    if (!husbu.personality) {
      let h = 0;
      for (const c of husbu.name) h = (h * 31 + c.charCodeAt(0)) >>> 0;
      husbu.personality = HUSB_PERSONALITIES[h % HUSB_PERSONALITIES.length];
      user.husbu = husbu;
      db.setUser(m.sender, user);
    }
    const sendMenu = (title, options) => sendHusbuMessage(m, sock, husbu, title, options.map(([label, id]) => ({ name: "quick_reply", buttonParamsJson: JSON.stringify({ display_text: label, id: `${m.prefix}husbuaction ${id}` }) })));

    const MENUS = {
      menu_jalanjalan: ["🚶 Jalan-jalan ke...", [["🌳 Taman", "jalan_taman"], ["🏢 Mall", "jalan_mall"], ["🏖️ Pantai", "jalan_pantai"], ["🌆 Kota", "jalan_kota"]]],
      menu_kafe: ["☕ Kamu traktir dia...", [["☕ Kopi Pahit", "kafe_kopi"], ["🍵 Matcha Latte", "kafe_matcha"], ["🍰 Kue", "kafe_kue"], ["🥛 Susu Hangat", "kafe_susu"]]],
      menu_bioskop: ["🎬 Pilih genre film...", [["💥 Action", "bioskop_romantis"], ["👻 Horor", "bioskop_horor"], ["🎨 Animasi", "bioskop_animasi"]]],
      menu_belanja: ["🛍️ Minta dibeliin apa?..", [["👕 Baju Keren", "belanja_baju"], ["💍 Aksesoris", "belanja_perhiasan"], ["🧸 Boneka", "belanja_boneka"]]],
      menu_kuliner: ["🍽️ Makan di...", [["🍽️ Restoran", "restoran_makan"], ["🥟 Dimsum", "restoran_dimsum"], ["🍖 BBQ", "restoran_bbq"]]],
      menu_olahraga: ["⚽ Olahraga...", [["⛰️ Hiking", "olahraga_hiking"], ["🏃 Lari Pagi", "olahraga_lari"], ["🧗 Panjat", "olahraga_panjat"]]],
      menu_alam: ["🏕️ Petualangan alam...", [["🏕️ Camping", "alam_camping"], ["🎣 Mancing", "alam_mancing"], ["⛵ Perahu", "alam_perahu"]]],
      menu_seni: ["🎨 Seni & budaya...", [["🖼️ Museum", "seni_museum"], ["🎨 Melukis", "seni_melukis"], ["🎸 Konser", "seni_konser"]]],
      menu_karaoke: ["🎤 Karaoke...", [["🎶 Duet", "karaoke_duet"], ["🎵 Solo", "karaoke_solo"]]],
      menu_arcade: ["🕹️ Di arcade...", [["🎮 Adu Skor", "arcade_duo"], ["🎯 Main Boneka", "arcade_boneka"]]],
      menu_piknik: ["🧺 Piknik...", [["🌳 Di Taman", "piknik_taman"], ["🏝️ Di Pantai", "piknik_pantai"]]],
      menu_masak: ["🍳 Masak bareng...", [["🍳 Masakan Rumah", "masak_bareng"], ["🍰 Kue", "masak_kue"]]],
      menu_lainnya: ["🎮 Aksi lainnya...", husbu.affection < 80
        ? [["🎤 Karaoke", "menu_karaoke"], ["🕹️ Arcade", "menu_arcade"], ["🧺 Piknik", "menu_piknik"], ["🍳 Masak", "menu_masak"], ["🍽️ Kuliner", "menu_kuliner"], ["⚽ Olahraga", "menu_olahraga"], ["🏕️ Alam", "menu_alam"], ["🎨 Seni", "menu_seni"]]
        : husbu.affection < 100
          ? [["🤗 Gendong", "menu_gendong"], ["🖐️ Tepuk Kepala", "tepuk_kepala"]]
          : [["🌴 Bulan Madu", "menu_bulanmadu"], ["💝 Hadiah", "hadiah"]]],
      menu_peluk: ["🫂 Cara memeluk...", [["🤗 Dari Belakang", "peluk_belakang"], ["💑 Berhadapan", "peluk_depan"]]],
      menu_cium: ["💋 Cium di...", [["😚 Kening", "cium_kening"], ["💋 Bibir", "cium_bibir"]]],
      menu_tidur: ["🛏️ Tidur bareng...", [["🫂 Pulas", "tidur_kelon"], ["🌙 Serandu", "tidur_serandu"]]],
      menu_mandi: ["🛁 Mandi bareng...", [["🛁 Gosok Punggung", "mandi_punggung"], ["🧼 Usap Bahu", "mandi_bahu"]]],
      menu_gendong: ["🤗 Gendong...", [["👸 Gaya Putri", "gendong_putri"], ["🏃 Punggung", "gendong_punggung"]]],
      menu_bulanmadu: ["🌴 Bulan madu...", [["🏝️ Pantai", "bulanmadu_pantai"], ["🏨 Hotel", "bulanmadu_hotel"]]],
    };

    if (MENUS[action]) {
      const [title, options] = MENUS[action];
      return sendMenu(title, options);
    }

    if (["peluk", "cium", "tidur", "mandi", "gendong_putri", "gendong_punggung", "tepuk_kepala", "intim_belakang", "intim_depan", "intim_kening", "intim_bibir", "intim_kelon", "intim_panas", "intim_punggung", "intim_bahu"].includes(action) && husbu.affection < 80) {
      const drop = Math.floor(Math.random() * 30) + 30;
      husbu.affection = Math.max(0, husbu.affection - drop);
      user.husbu = husbu;
      db.setUser(m.sender, user);
      m.react("💢");
      return m.reply(`💢 *HENTIKAN!* Kamu bertindak terlalu agresif saat perasaannya masih samar! *${husbu.name}* menepis tanganmu dengan wajah marah dan kecewa.\n💞 *Affection -${drop}*`);
    }

    if (action === "nikah") {
      if (husbu.affection < 100) return m.reply(`⚠️ Affection belum 100! Jangan terburu-buru menerima lamarannya!`);
      if (husbu.married) return m.reply(`⚠️ Kalian kan sudah menikah!`);
      husbu.married = true;
      husbu.marriedDate = new Date().toISOString();
      const dowry = DOWRY[husbu.tier];
      db.updateEnergi(m.sender, dowry.limit);
      db.updateKoin(m.sender, dowry.koin);
      await addExpWithLevelCheck(sock, m, db, user, dowry.exp);
      const stats = initStats(user);
      stats.marriedCount = (stats.marriedCount || 0) + 1;
      if (!user.fun) user.fun = {};
      const husbuJid = 'husbu_' + husbu.name.replace(/\s+/g, '') + '@s.whatsapp.net';
      user.fun.pasangan = husbuJid;
      db.setUser(husbuJid, { fun: { pasangan: m.sender }, name: husbu.name });
      user.husbu = husbu;
      db.setUser(m.sender, user);
      m.react("💍");
      return m.reply(`💍 *KAMU RESMI MENERIMA LAMARAN ${husbu.name.toUpperCase()}!* 💍\n\nSebagai nafkah:\n- ⚡ ${dowry.limit.toLocaleString()} Limit/Energi\n- 💰 ${dowry.koin.toLocaleString()} Koin\n- ✨ ${dowry.exp.toLocaleString()} EXP\n\nStatus \`.cekpacar\` kini resmi berpasangan!`);
    }

    if (action === "hadiah") {
      if (husbu.affection < 100) return m.reply(`⚠️ Dia belum cukup mencintaimu untuk memberi hadiah!`);
      const g = 300 + Math.floor(Math.random() * 401);
      db.updateEnergi(m.sender, g);
      m.react("💝");
      return m.reply(`💝 *${husbu.name}* memberimu hadiah!\nKamu mendapat ⚡ ${g} Limit/Energi!`);
    }

    if (action === "klaim") {
      if (!husbu.married) return m.reply(`⚠️ Hanya suami sah yang bisa klaim hadiah harian!`);
      const last = husbu.lastClaimDate;
      if (last === todayStr()) return m.reply(`⚠️ Kamu sudah klaim hari ini! Coba lagi besok.`);
      const tierMult = { Common: 1, Rare: 1.2, Epic: 1.5, Legendary: 2, Mythic: 3 }[husbu.tier] || 1;
      const koin = Math.floor((5000 + Math.random() * 15001) * tierMult);
      const exp = Math.floor((200 + Math.random() * 601) * tierMult);
      const day = marrageDay(husbu);
      let bonus = "";
      if (day === 7 || day === 30 || day === 100) {
        const bKoin = day * 1000 * tierMult;
        db.updateKoin(m.sender, bKoin);
        bonus = `\n🎉 *MILESTONE HARI KE-${day}!* Bonus +${bKoin.toLocaleString()} Koin!`;
      }
      husbu.lastClaimDate = todayStr();
      user.husbu = husbu;
      db.setUser(m.sender, user);
      db.updateKoin(m.sender, koin);
      await addExpWithLevelCheck(sock, m, db, user, exp);
      m.react("🎁");
      return m.reply(`🎁 *${husbu.name}* memberi hadiah harian!\n💰 +${koin.toLocaleString()} Koin\n✨ +${exp} EXP${bonus}`);
    }

    const { decay: negDecay } = applyNeglect(husbu);
    const mult = husbu.nextMultUntil && Date.now() < new Date(husbu.nextMultUntil).getTime() ? 0.8 : 1;
    const mood = angerEffMood(moodState(user, m), husbu);
    const result = applyAction(action, husbu, mood, undefined, mult);
    if (!result) { m.react("❓"); return m.reply(`Aksi tidak dikenali. Gunakan tombol husbu.`); }

    if (result.phase === "married" && !husbu.married) { m.react("⛔"); return m.reply(`Aksi ini hanya untuk pasangan suami istri!`); }
    if (result.phase === "intim" && !husbu.married && husbu.affection < 80) return;

    if (result.phase === "married" && action !== "nikah") {
      const { change } = finalGain(result, husbu, { actionsToday: 0 });
      husbu.anger = angerUpdate(husbu, result);
      husbu.affection = Math.max(0, Math.min(100, husbu.affection + change));
      const koinGain = Math.floor(Math.max(0, change) * 100);
      user.husbu = husbu;
      db.setUser(m.sender, user);
      if (koinGain > 0) db.updateKoin(m.sender, koinGain);
      await addExpWithLevelCheck(sock, m, db, user, result.exp);
      m.react("❤️");
      const affLine = change >= 0 ? `💞 *Affection +${change}*` : `💞 *Affection ${change}*`;
      return m.reply(`${result.text}\n\n${affLine} (Total: ${husbu.affection}/100)\n💰 +${koinGain} Koin\n✨ +${result.exp} EXP`);
    }

    if (husbu.lastActionDate !== todayStr()) {
      husbu.actionsToday = 0;
      husbu.lastActionDate = todayStr();
    }
    husbu.actionsToday = (husbu.actionsToday || 0) + 1;

    const eff = finalGain(result, husbu, { actionsToday: husbu.actionsToday });
    husbu.anger = angerUpdate(husbu, result);

    const affBefore = husbu.affection;
    let newAff = affBefore + eff.change;
    let eventBlock = "";
    let eventExp = 0;
    let eventKoin = 0;
    const ev = rollEvent({ married: husbu.married, phase: result.phase, personality: husbu.personality, name: husbu.name });
    if (ev) {
      newAff += ev.aff;
      eventKoin = ev.koin || 0;
      if (ev.anger) husbu.anger = Math.min(100, (husbu.anger || 0) + ev.anger);
      if (ev.mood) { husbu.mood = { type: ev.mood, since: new Date().toISOString() }; husbu.moodUntil = new Date(Date.now() + 12 * 3600000).toISOString(); }
      if (ev.nextMult < 1) husbu.nextMultUntil = new Date(Date.now() + 24 * 3600000).toISOString();
      eventBlock = `\n\n✨ *EVENT:* ${ev.text}${ev.aff ? ` (${ev.aff > 0 ? "+" : ""}${ev.aff} aff)` : ""}${eventKoin ? ` (+${eventKoin.toLocaleString()} koin)` : ""}`;
      eventExp = result.exp;
    }
    newAff = Math.max(0, Math.min(100, newAff));
    husbu.affection = newAff;
    user.husbu = husbu;
    db.setUser(m.sender, user);
    if (eventKoin) db.updateKoin(m.sender, eventKoin);
    await addExpWithLevelCheck(sock, m, db, user, result.exp + (eventExp ? 15 : 0));

    let affText = `💞 *Affection:* ${husbu.affection}/100`;
    if (eff.change !== 0) affText = `💞 *Affection berubah:* ${eff.change > 0 ? "+" : ""}${eff.change} (Total: ${husbu.affection}/100)`;
    if (husbu.affection === 100) affText = `💞 *Affection MAKSIMAL! (100/100)* 🎉\n💍 *Dia akan melamarmu!*`;
    if (negDecay > 0) affText += `\n📉 *Neglect: -${negDecay} aff* (jarang interaksi)`;
    if (eff.angry) affText += `\n😡 *Dia MARAH!* Drain -${eff.drain} aff`;
    if (eff.extra.length) affText += `\n${eff.extra.join("\n")}`;

    if (husbu.affection <= 0) {
      const husbuName = husbu.name;
      if (husbu.married) {
        const husbuJid = 'husbu_' + husbuName.replace(/\s+/g, '') + '@s.whatsapp.net';
        if (user.fun && user.fun.pasangan === husbuJid) user.fun.pasangan = "";
        db.setUser(husbuJid, { fun: { pasangan: "" } });
      }
      delete user.husbu;
      db.setUser(m.sender, user);
      m.react("💔");
      return m.reply(`💔 *${husbuName.toUpperCase()} MENINGGALKANMU!* 💔\n\nKarena kasih sayangnya habis (0), dia mengemasi seluruh barang-barangnya dan pergi. Kamu kehilangan husbumu! *(Ketik ${m.prefix}gachahusbu untuk memulai ulang)*`);
    }

    m.react(husbu.affection === 100 ? "💍" : "✨");
    const updated = `${result.text}${eventBlock}\n\n${affText}\n✨ +${result.exp} EXP${eventKoin ? `\n💰 +${eventKoin.toLocaleString()} Koin` : ""}`;
    await sendHusbuMessage(m, sock, husbu, updated, null);
  }
}

export { pluginConfig as config, handler };
