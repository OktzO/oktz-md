import axios from "axios";
import { getDatabase } from "../../src/lib/ourin-database.js";
import te from "../../src/lib/ourin-error.js";
import { prepareWAMessageMedia, generateWAMessageFromContent } from "ourin";
import { addExpWithLevelCheck } from "../../src/lib/ourin-level.js";
import { rollWaifu, applyAction, rollEvent, getDailyMood, DOWRY } from "../../src/lib/ourin-waifu.js";
import { angerEffMood, angerUpdate, applyNeglect, finalGain } from "../../src/lib/ourin-romance.js";

const pluginConfig = {
  name: ["gachawaifu", "waifuaction", "tinggalinwaifu", "waifuku", "istriku"],
  alias: ["gachaistri"],
  category: "fun",
  description: "Gacha waifu impianmu, jaga perasaannya, dan jadikan dia pasanganmu!",
  usage: ".gachawaifu | .waifuku | .tinggalinwaifu",
  example: ".gachawaifu",
  isOwner: false,
  isPremium: false,
  isGroup: false,
  isPrivate: false,
  cooldown: 5,
  energi: 2,
  isEnabled: true,
};

async function getWaifuImage(keyword) {
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
    console.error("[GachaWaifu] Pinterest API error:", e.message);
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

function todayStr() {
  return new Date().toLocaleDateString("id-ID", { timeZone: "Asia/Jakarta" });
}

function moodLabel(mood) {
  return { ceria: "ceria 😄", romantis: "romantis 💘", biasa: "biasa 🙂", sedih: "sedih 😢", marah: "marah 😡" }[mood] || mood;
}

function moodState(user, m) {
  const w = user.waifu;
  if (w.moodUntil && Date.now() < new Date(w.moodUntil).getTime() && w.mood?.type) return w.mood.type;
  return getDailyMood(m.sender, todayStr());
}

function angerMeter(w) {
  const a = w.anger || 0;
  const n = Math.round(a / 10);
  return "█".repeat(n) + "░".repeat(10 - n) + ` (${a}/100)`;
}

async function sendWaifuMessage(m, sock, waifu, textContent, customButtons = null) {
  let imgBuffer = null;
  if (waifu.imageUrl) imgBuffer = await getBuffer(waifu.imageUrl);
  if (!imgBuffer) {
    const newUrl = await getWaifuImage(waifu.keyword);
    waifu.imageUrl = newUrl;
    imgBuffer = await getBuffer(newUrl) || Buffer.alloc(0);
  }
  const media = await prepareWAMessageMedia({ image: imgBuffer }, { upload: sock.waUploadToServer });
  let buttons = customButtons;
  if (!buttons) {
    if (waifu.affection < 80) {
      buttons = [
        { name: "quick_reply", buttonParamsJson: JSON.stringify({ display_text: "🚶 Jalan-jalan", id: `${m.prefix}waifuaction menu_jalanjalan` }) },
        { name: "quick_reply", buttonParamsJson: JSON.stringify({ display_text: "☕ Kafe", id: `${m.prefix}waifuaction menu_kafe` }) },
        { name: "quick_reply", buttonParamsJson: JSON.stringify({ display_text: "🎬 Bioskop", id: `${m.prefix}waifuaction menu_bioskop` }) },
        { name: "quick_reply", buttonParamsJson: JSON.stringify({ display_text: "🛍️ Belanja", id: `${m.prefix}waifuaction menu_belanja` }) },
        { name: "quick_reply", buttonParamsJson: JSON.stringify({ display_text: "🎮 Lainnya", id: `${m.prefix}waifuaction menu_lainnya` }) },
      ];
    } else if (waifu.affection < 100) {
      buttons = [
        { name: "quick_reply", buttonParamsJson: JSON.stringify({ display_text: "🫂 Peluk", id: `${m.prefix}waifuaction menu_peluk` }) },
        { name: "quick_reply", buttonParamsJson: JSON.stringify({ display_text: "💋 Cium", id: `${m.prefix}waifuaction menu_cium` }) },
        { name: "quick_reply", buttonParamsJson: JSON.stringify({ display_text: "🛏️ Tidur", id: `${m.prefix}waifuaction menu_tidur` }) },
        { name: "quick_reply", buttonParamsJson: JSON.stringify({ display_text: "🛁 Mandi", id: `${m.prefix}waifuaction menu_mandi` }) },
        { name: "quick_reply", buttonParamsJson: JSON.stringify({ display_text: "🎁 Lainnya", id: `${m.prefix}waifuaction menu_lainnya` }) },
      ];
    } else if (!waifu.married) {
      buttons = [
        { name: "quick_reply", buttonParamsJson: JSON.stringify({ display_text: "💍 Nikahi", id: `${m.prefix}waifuaction nikah` }) },
        { name: "quick_reply", buttonParamsJson: JSON.stringify({ display_text: "💝 Hadiah", id: `${m.prefix}waifuaction hadiah` }) },
      ];
    } else {
      buttons = [
        { name: "quick_reply", buttonParamsJson: JSON.stringify({ display_text: "👩‍❤️‍👨 Mesra", id: `${m.prefix}waifuaction mesra` }) },
        { name: "quick_reply", buttonParamsJson: JSON.stringify({ display_text: "💌 Rayu", id: `${m.prefix}waifuaction rayu` }) },
        { name: "quick_reply", buttonParamsJson: JSON.stringify({ display_text: "🌴 Bulan Madu", id: `${m.prefix}waifuaction menu_bulanmadu` }) },
        { name: "quick_reply", buttonParamsJson: JSON.stringify({ display_text: "🎁 Klaim Harian", id: `${m.prefix}waifuaction klaim` }) },
      ];
    }
  }
  let footerText = "❤️ Jaga terus perasaannya ya!";
  if (customButtons) footerText = "💭 Dia menunggu jawabanmu...";
  else if (waifu.married) footerText = "❤️ Kamu sudah menikahinya!";

  const msg = generateWAMessageFromContent(m.chat, {
    viewOnceMessage: {
      message: {
        messageContextInfo: { deviceListMetadata: {}, deviceListMetadataVersion: 2 },
        interactiveMessage: {
          body: { text: textContent },
          footer: { text: footerText },
          header: {
            title: `🌟 *${waifu.tier.toUpperCase()} TIER WAIFU* 🌟`,
            subtitle: waifu.name,
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

function marrageDay(waifu) {
  if (!waifu.marriedDate) return 0;
  const a = new Date(waifu.marriedDate);
  const b = new Date();
  return Math.floor((b - a) / 86400000) + 1;
}

function initStats(user) {
  if (!user.waifuStats) user.waifuStats = { totalGacha: 0, byTier: {}, pityCounter: 0, rarest: null, marriedCount: 0 };
  return user.waifuStats;
}

function recordPull(user, waifu, stats) {
  stats.totalGacha++;
  stats.byTier[waifu.tier] = (stats.byTier[waifu.tier] || 0) + 1;
  const isEpicPlus = ["Epic", "Legendary", "Mythic"].includes(waifu.tier);
  stats.pityCounter = isEpicPlus ? 0 : (stats.pityCounter || 0) + 1;
  if (!stats.rarest || ["Common", "Rare", "Epic", "Legendary", "Mythic"].indexOf(waifu.tier) > ["Common", "Rare", "Epic", "Legendary", "Mythic"].indexOf(stats.rarest.tier)) {
    stats.rarest = { name: waifu.name, tier: waifu.tier };
  }
  if (!user.waifuHistory) user.waifuHistory = [];
  user.waifuHistory.push({ name: waifu.name, series: waifu.series, tier: waifu.tier, at: new Date().toISOString() });
  if (user.waifuHistory.length > 100) user.waifuHistory = user.waifuHistory.slice(-100);
}

async function handler(m, { sock }) {
  const db = getDatabase();
  const user = db.getUser(m.sender);
  if (!user) return;

  const cmd = m.command.toLowerCase();

  if (cmd === "waifuku" || cmd === "istriku") {
    if (!user.waifu) return m.reply(`⚠️ *Kamu belum memiliki waifu!*\nSilakan ketik *${m.prefix}gachawaifu* untuk memulainya!`);
    m.react("🕕");
    const w = user.waifu;
    const mood = moodState(user, m);
    const { decay } = applyNeglect(w);
    let status = w.married ? "Telah Menikah 💍" : "Pendekatan 💖";
    const day = marrageDay(w);
    const moodLine = w.married
      ? `\n🗓️ *Hari ke-${day}*${day >= 7 ? ` ${day >= 100 ? "🏆" : day >= 30 ? "🎖️" : "🎉"}` : ""}`
      : "";
    let textContent = `📸 *STATUS WAIFU KAMU* 📸\n\n` +
      `💖 *Nama:* ${w.name}\n` +
      `💎 *Tier:* ${w.tier}\n` +
      `🎭 *Personality:* ${w.personality}\n` +
      `🌤️ *Mood hari ini:* ${moodLabel(mood)}\n` +
      `😡 *Anger:* ${angerMeter(w)}\n` +
      `💞 *Affection:* ${w.affection}/100\n` +
      `💍 *Status:* ${status}${moodLine}\n`;
    if (decay > 0) textContent += `📉 *Neglect:* Affection turun -${decay} karena kamu jarang interaksi!\n`;
    if ((w.anger || 0) >= 50) textContent += `⚠️ *Dia sedang MARAH!* Perbaiki hubungan sebelum affection habis!\n`;
    textContent += `\nLanjutkan interaksi dengan memilih aksi di bawah!`;
    user.waifu = w;
    db.setUser(m.sender, user);
    m.react("✅");
    return await sendWaifuMessage(m, sock, w, textContent, null);
  }

  if (cmd === "tinggalinwaifu") {
    if (!user.waifu) return m.reply(`⚠️ *Kamu bahkan belum punya waifu!* Halu ya?`);
    const waifuName = user.waifu.name;
    const waifuJid = 'waifu_' + waifuName.replace(/\s+/g, '') + '@s.whatsapp.net';
    if (user.waifu.married) {
      if (user.fun && user.fun.pasangan === waifuJid) user.fun.pasangan = "";
      db.setUser(waifuJid, { fun: { pasangan: "" } });
    }
    delete user.waifu;
    db.setUser(m.sender, user);
    m.react("💔");
    return m.reply(`💔 *KAMU MEMUTUSKAN UNTUK MENINGGALKAN ${waifuName.toUpperCase()}!*\n\nKamu mengemas barang-barangmu dan pergi. Dia menangis tersedu-sedu. Kalian kini resmi berpisah.`);
  }

  if (cmd === "gachawaifu" || cmd === "gachaistri") {
    if (user.waifu) {
      m.react("😡");
      return m.reply(`⚠️ *Kamu sudah memiliki waifu!*\nNama: *${user.waifu.name}*\nTier: *${user.waifu.tier}*\nAffection: *${user.waifu.affection}/100*\n\nJangan serakah! Ketik *${m.prefix}waifuku* untuk berinteraksi dengannya.`);
    }
    const sub = (m.args[0] || "").toLowerCase();
    if (sub !== "start") {
      const panduan = `💕 *SISTEM GACHA WAIFU* 💕\n\n` +
        `Simulasi kencan virtual interaktif. Dapatkan waifu impianmu, dekati hatinya, dan nikahi dia!\n\n` +
        `*PENGGUNAAN:*\n` +
        `• *${m.prefix}gachawaifu* — Panduan ini\n` +
        `• *${m.prefix}waifuku* — Panel interaksi\n` +
        `• *${m.prefix}waifualbum* — Riwayat & statistik luck\n` +
        `• *${m.prefix}waifupool* — Jelajahi pool waifu\n` +
        `• *${m.prefix}tinggalinwaifu* — Putuskan hubungan\n\n` +
        `*ALUR:*\n1. Tekan tombol **Mulai Gacha**.\n2. 3 Fase: Pendekatan (<80) → Intim (80–99) → Menikah (100).\n3. Mood & personality memengaruhi poin affection.\n4. Pity: 20 roll tanpa Epic+ dijamin dapat Epic+.\n5. Aksi memberi EXP + reward ekonomi.`;
      const msg = generateWAMessageFromContent(m.chat, {
        viewOnceMessage: {
          message: {
            messageContextInfo: { deviceListMetadata: {}, deviceListMetadataVersion: 2 },
            interactiveMessage: {
              body: { text: panduan },
              footer: { text: "Tekan tombol untuk mulai!" },
              nativeFlowMessage: { buttons: [{ name: "quick_reply", buttonParamsJson: JSON.stringify({ display_text: "🎲 Mulai Gacha!", id: `${m.prefix}gachawaifu start` }) }] },
            },
          },
        },
      }, { quoted: m });
      return await sock.relayMessage(m.chat, msg.message, { messageId: msg.key.id });
    }

    m.react("🕕");
    try {
      const stats = initStats(user);
      const picked = rollWaifu(stats.pityCounter);
      const waifu = {
        ...picked,
        affection: 50,
        married: false,
        imageUrl: await getWaifuImage(picked.keyword),
      };
      recordPull(user, waifu, stats);
      user.waifu = waifu;
      db.setUser(m.sender, user);
      const textContent = `🎉 *SELAMAT! KAMU MENDAPATKAN WAIFU BARU!* 🎉\n\n` +
        `💖 *Nama:* ${waifu.name}\n🎂 *Usia:* ${waifu.age} tahun\n📏 *Tinggi:* ${waifu.height}\n⚖️ *Berat:* ${waifu.weight}\n` +
        `💎 *Tier:* ${waifu.tier}\n🎭 *Personality:* ${waifu.personality}\n💞 *Affection:* ${waifu.affection}/100\n\n` +
        `Pilih interaksi untuk mulai PDKT. Hati-hati jangan sampai affection habis!`;
      m.react("✅");
      await sendWaifuMessage(m, sock, waifu, textContent, null);
    } catch (err) {
      console.error(err);
      m.react("☢");
      return m.reply(te(m.prefix, m.command, m.pushName));
    }
    return;
  }

  if (cmd === "waifuaction") {
    if (!user.waifu) { m.react("❌"); return m.reply(`Kamu belum memiliki waifu! Ketik *${m.prefix}gachawaifu* untuk memulai.`); }

    const action = (m.args[0] || "").toLowerCase();
    const waifu = user.waifu;
    if (!waifu.personality) {
      let h = 0;
      for (const c of waifu.name) h = (h * 31 + c.charCodeAt(0)) >>> 0;
      waifu.personality = ["tsundere", "deredere", "kuudere", "dandere", "genki"][h % 5];
      user.waifu = waifu;
      db.setUser(m.sender, user);
    }
    const sendMenu = (title, options) => sendWaifuMessage(m, sock, waifu, title, options.map(([label, id]) => ({ name: "quick_reply", buttonParamsJson: JSON.stringify({ display_text: label, id: `${m.prefix}waifuaction ${id}` }) })));

    const MENUS = {
      menu_jalanjalan: ["🚶 Jalan-jalan ke...", [["🌳 Taman", "jalan_taman"], ["🏢 Mall", "jalan_mall"], ["🏖️ Pantai", "jalan_pantai"], ["🌆 Kota", "jalan_kota"]]],
      menu_kafe: ["☕ Pesan untuknya...", [["☕ Kopi Pahit", "kafe_kopi"], ["🍵 Matcha Latte", "kafe_matcha"], ["🍰 Kue", "kafe_kue"], ["🥛 Susu Hangat", "kafe_susu"]]],
      menu_bioskop: ["🎬 Pilih genre film...", [["💞 Romantis", "bioskop_romantis"], ["👻 Horor", "bioskop_horor"], ["🎨 Animasi", "bioskop_animasi"]]],
      menu_belanja: ["🛍️ Belikan dia apa?..", [["👗 Baju Lucu", "belanja_baju"], ["💎 Perhiasan", "belanja_perhiasan"], ["🧸 Boneka", "belanja_boneka"]]],
      menu_karaoke: ["🎤 Karaoke...", [["🎶 Duet", "karaoke_duet"], ["🎵 Solo", "karaoke_solo"]]],
      menu_arcade: ["🕹️ Di arcade...", [["🎮 Adu Skor", "arcade_duo"], ["🎯 Main Boneka", "arcade_boneka"]]],
      menu_piknik: ["🧺 Piknik...", [["🌳 Di Taman", "piknik_taman"], ["🏝️ Di Pantai", "piknik_pantai"]]],
      menu_masak: ["🍳 Masak bareng...", [["🍳 Masakan Rumah", "masak_bareng"], ["🍰 Kue", "masak_kue"]]],
      menu_kuliner: ["🍽️ Makan di...", [["🍽️ Restoran", "restoran_makan"], ["🥟 Dimsum", "restoran_dimsum"], ["🍖 BBQ", "restoran_bbq"]]],
      menu_olahraga: ["⚽ Olahraga...", [["⛰️ Hiking", "olahraga_hiking"], ["🏃 Lari Pagi", "olahraga_lari"], ["🧗 Panjat", "olahraga_panjat"]]],
      menu_alam: ["🏕️ Petualangan alam...", [["🏕️ Camping", "alam_camping"], ["🎣 Mancing", "alam_mancing"], ["⛵ Perahu", "alam_perahu"]]],
      menu_seni: ["🎨 Seni & budaya...", [["🖼️ Museum", "seni_museum"], ["🎨 Melukis", "seni_melukis"], ["🎸 Konser", "seni_konser"]]],
      menu_lainnya: ["🎮 Aksi lainnya...", waifu.affection < 80
        ? [["🎤 Karaoke", "menu_karaoke"], ["🕹️ Arcade", "menu_arcade"], ["🧺 Piknik", "menu_piknik"], ["🍳 Masak", "menu_masak"], ["🍽️ Kuliner", "menu_kuliner"], ["⚽ Olahraga", "menu_olahraga"], ["🏕️ Alam", "menu_alam"], ["🎨 Seni", "menu_seni"]]
        : waifu.affection < 100
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

    if (["peluk", "cium", "tidur", "mandi", "gendong_putri", "gendong_punggung", "tepuk_kepala", "intim_belakang", "intim_depan", "intim_kening", "intim_bibir", "intim_kelon", "intim_panas", "intim_punggung", "intim_bahu"].includes(action) && waifu.affection < 80) {
      const drop = Math.floor(Math.random() * 30) + 30;
      waifu.affection = Math.max(0, waifu.affection - drop);
      user.waifu = waifu;
      db.setUser(m.sender, user);
      m.react("💢");
      return m.reply(`💢 *PLAKK!!* Kamu mencoba bersikap mesum kepada *${waifu.name}*, tapi kalian belum sedekat itu! Dia menamparmu keras.\n💞 *Affection -${drop}*`);
    }

    if (action === "nikah") {
      if (waifu.affection < 100) return m.reply(`⚠️ Affection belum 100! Jangan terburu-buru melamar!`);
      if (waifu.married) return m.reply(`⚠️ Kalian kan sudah menikah!`);
      waifu.married = true;
      waifu.marriedDate = new Date().toISOString();
      const dowry = DOWRY[waifu.tier];
      db.updateEnergi(m.sender, dowry.limit);
      db.updateKoin(m.sender, dowry.koin);
      await addExpWithLevelCheck(sock, m, db, user, dowry.exp);
      const stats = initStats(user);
      stats.marriedCount = (stats.marriedCount || 0) + 1;
      if (!user.fun) user.fun = {};
      const waifuJid = 'waifu_' + waifu.name.replace(/\s+/g, '') + '@s.whatsapp.net';
      user.fun.pasangan = waifuJid;
      db.setUser(waifuJid, { fun: { pasangan: m.sender }, name: waifu.name });
      user.waifu = waifu;
      db.setUser(m.sender, user);
      m.react("💍");
      return m.reply(`💍 *KAMU RESMI MENIKAH DENGAN ${waifu.name.toUpperCase()}!* 💍\n\nSebagai dowry:\n- ⚡ ${dowry.limit} Limit/Energi\n- 💰 ${dowry.koin.toLocaleString()} Koin\n- ✨ ${dowry.exp.toLocaleString()} EXP\n\nStatus \`.cekpacar\` kini resmi berpasangan!`);
    }

    if (action === "hadiah") {
      if (waifu.affection < 100) return m.reply(`⚠️ Dia belum cukup mencintaimu untuk memberi hadiah!`);
      const g = 300 + Math.floor(Math.random() * 401);
      db.updateEnergi(m.sender, g);
      m.react("💝");
      return m.reply(`💝 *${waifu.name}* memberimu hadiah!\nKamu mendapat ⚡ ${g} Limit/Energi!`);
    }

    if (action === "klaim") {
      if (!waifu.married) return m.reply(`⚠️ Hanya istri sah yang bisa klaim hadiah harian!`);
      const last = waifu.lastClaimDate;
      if (last === todayStr()) return m.reply(`⚠️ Kamu sudah klaim hari ini! Coba lagi besok.`);
      const tierMult = { Common: 1, Rare: 1.2, Epic: 1.5, Legendary: 2, Mythic: 3 }[waifu.tier] || 1;
      const koin = Math.floor((5000 + Math.random() * 15001) * tierMult);
      const exp = Math.floor((200 + Math.random() * 601) * tierMult);
      const day = marrageDay(waifu);
      let bonus = "";
      if (day === 7 || day === 30 || day === 100) {
        const bKoin = day * 1000 * tierMult;
        db.updateKoin(m.sender, bKoin);
        bonus = `\n🎉 *MILESTONE HARI KE-${day}!* Bonus +${bKoin.toLocaleString()} Koin!`;
      }
      waifu.lastClaimDate = todayStr();
      user.waifu = waifu;
      db.setUser(m.sender, user);
      db.updateKoin(m.sender, koin);
      await addExpWithLevelCheck(sock, m, db, user, exp);
      m.react("🎁");
      return m.reply(`🎁 *${waifu.name}* memberi hadiah harian!\n💰 +${koin.toLocaleString()} Koin\n✨ +${exp} EXP${bonus}`);
    }

    const { decay: negDecay } = applyNeglect(waifu);
    const mult = waifu.nextMultUntil && Date.now() < new Date(waifu.nextMultUntil).getTime() ? 0.8 : 1;
    const mood = angerEffMood(moodState(user, m), waifu);
    const result = applyAction(action, waifu, mood, undefined, mult);
    if (!result) { m.react("❓"); return m.reply(`Aksi tidak dikenali. Gunakan tombol waifu.`); }

    if (result.phase === "married" && !waifu.married) { m.react("⛔"); return m.reply(`Aksi ini hanya untuk pasangan suami istri!`); }
    if (result.phase === "intim" && !waifu.married && waifu.affection < 80) return;

    if (result.phase === "married" && action !== "nikah") {
      const { change } = finalGain(result, waifu, { actionsToday: 0 });
      waifu.anger = angerUpdate(waifu, result);
      waifu.affection = Math.max(0, Math.min(100, waifu.affection + change));
      const koinGain = Math.floor(Math.max(0, change) * 100);
      user.waifu = waifu;
      db.setUser(m.sender, user);
      if (koinGain > 0) db.updateKoin(m.sender, koinGain);
      await addExpWithLevelCheck(sock, m, db, user, result.exp);
      m.react("❤️");
      const affLine = change >= 0 ? `💞 *Affection +${change}*` : `💞 *Affection ${change}*`;
      return m.reply(`${result.text}\n\n${affLine} (Total: ${waifu.affection}/100)\n💰 +${koinGain} Koin\n✨ +${result.exp} EXP`);
    }

    // aksi approach/intim biasa
    if (waifu.lastActionDate !== todayStr()) {
      waifu.actionsToday = 0;
      waifu.lastActionDate = todayStr();
    }
    waifu.actionsToday = (waifu.actionsToday || 0) + 1;

    const eff = finalGain(result, waifu, { actionsToday: waifu.actionsToday });
    waifu.anger = angerUpdate(waifu, result);

    const affBefore = waifu.affection;
    let newAff = affBefore + eff.change;
    let eventBlock = "";
    let eventExp = 0;
    let eventKoin = 0;
    const ev = rollEvent({ married: waifu.married, phase: result.phase, personality: waifu.personality, name: waifu.name });
    if (ev) {
      newAff += ev.aff;
      eventKoin = ev.koin || 0;
      if (ev.anger) waifu.anger = Math.min(100, (waifu.anger || 0) + ev.anger);
      if (ev.mood) { waifu.mood = { type: ev.mood, since: new Date().toISOString() }; waifu.moodUntil = new Date(Date.now() + 12 * 3600000).toISOString(); }
      if (ev.nextMult < 1) waifu.nextMultUntil = new Date(Date.now() + 24 * 3600000).toISOString();
      eventBlock = `\n\n✨ *EVENT:* ${ev.text}${ev.aff ? ` (${ev.aff > 0 ? "+" : ""}${ev.aff} aff)` : ""}${eventKoin ? ` (+${eventKoin.toLocaleString()} koin)` : ""}`;
      eventExp = result.exp;
    }
    newAff = Math.max(0, Math.min(100, newAff));
    waifu.affection = newAff;
    user.waifu = waifu;
    db.setUser(m.sender, user);
    if (eventKoin) db.updateKoin(m.sender, eventKoin);
    await addExpWithLevelCheck(sock, m, db, user, result.exp + (eventExp ? 15 : 0));

    let affText = `💞 *Affection:* ${waifu.affection}/100`;
    if (eff.change !== 0) affText = `💞 *Affection berubah:* ${eff.change > 0 ? "+" : ""}${eff.change} (Total: ${waifu.affection}/100)`;
    if (waifu.affection === 100) affText = `💞 *Affection MAKSIMAL! (100/100)* 🎉\n💍 *Nikahi dia sekarang!*`;
    if (negDecay > 0) affText += `\n📉 *Neglect: -${negDecay} aff* (jarang interaksi)`;
    if (eff.angry) affText += `\n😡 *Dia MARAH!* Drain -${eff.drain} aff`;
    if (eff.extra.length) affText += `\n${eff.extra.join("\n")}`;

    if (waifu.affection <= 0) {
      const waifuName = waifu.name;
      if (waifu.married) {
        const waifuJid = 'waifu_' + waifuName.replace(/\s+/g, '') + '@s.whatsapp.net';
        if (user.fun && user.fun.pasangan === waifuJid) user.fun.pasangan = "";
        db.setUser(waifuJid, { fun: { pasangan: "" } });
      }
      delete user.waifu;
      db.setUser(m.sender, user);
      m.react("💔");
      return m.reply(`💔 *${waifuName.toUpperCase()} MENINGGALKANMU!* 💔\n\nKarena kasih sayangnya habis (0), dia pergi meninggalkan surat basah air mata. Kamu kehilangan waifumu! *(Ketik ${m.prefix}gachawaifu untuk memulai ulang)*`);
    }

    m.react(waifu.affection === 100 ? "💍" : "✨");
    const updated = `${result.text}${eventBlock}\n\n${affText}\n✨ +${result.exp} EXP${eventKoin ? `\n💰 +${eventKoin.toLocaleString()} Koin` : ""}`;
    await sendWaifuMessage(m, sock, waifu, updated, null);
  }
}

export { pluginConfig as config, handler };
