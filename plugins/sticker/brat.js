import { drawBrat } from "../../src/lib/ourin-brat.js";
import { wrapInteractive } from "../../src/lib/ourin-rich-messages.js";
import { getAssetBuffer } from "../../src/lib/ourin-asset-manager.js";
import { prepareWAMessageMedia, generateWAMessageFromContent } from "ourin";
import config from "../../config.js";
import te from "../../src/lib/ourin-error.js";

const pluginConfig = {
  name: "brat",
  alias: ["bratmenu", "bratimg", "brattext"],
  category: "sticker",
  description: "Menu variant brat dan generator sticker brat",
  usage: ".brat | .bratimg <text>",
  example: ".bratimg Hai semua",
  isOwner: false,
  isPremium: false,
  isGroup: false,
  isPrivate: false,
  cooldown: 10,
  energi: 1,
  isEnabled: true,
};

const BRAT_VARIANTS = [
  {
    title: "Brat Default",
    description: "Sticker brat versi biasa",
    command: "bratimg",
  },
  {
    title: "Brat Green",
    description: "Variant brat warna hijau",
    command: "bratgreen",
  },
  {
    title: "Brat Cewek",
    description: "Variant brat cewek",
    command: "bratcewek",
  },
  {
    title: "Brat Anime",
    description: "Variant brat anime",
    command: "animebrat",
  },
  {
    title: "Brat Video",
    description: "Sticker brat animated",
    command: "bratvid",
  },
  {
    title: "Brat Video V2",
    description: "Sticker brat video v2",
    command: "bratvid2",
  },
];

function buildVariantRows(prefix, text) {
  return BRAT_VARIANTS.map((item) => ({
    title: item.title,
    description: `${item.description} • .${item.command} <text>`,
    id: `${prefix}${item.command} ${text || ""}`.trim(),
  }));
}

async function sendBratMenu(m, sock, text) {
  const caption =
    "🌿 *kamu mau buat brat yak, silahkan pilih variant brat tombol dibawah*";

  // header gambar: thumbnail asset brat diupload dulu (pola goodbye.js)
  let media = null;
  try {
    media = await prepareWAMessageMedia(
      { image: getAssetBuffer("ourin") },
      { upload: sock.waUploadToServer },
    );
  } catch { }

  const content = {
    messageContextInfo: {},
    interactiveMessage: {
      header: media?.imageMessage
        ? {
            title: "",
            subtitle: config.bot?.name,
            hasMediaAttachment: true,
            imageMessage: media.imageMessage,
          }
        : undefined,
      body: { text: caption },
      footer: { text: "Pilih variant brat favorit kamu" },
      nativeFlowMessage: {
        buttons: [
          {
            name: "single_select",
            buttonParamsJson: JSON.stringify({
              title: "🌾 Pilih Variant Brat",
              sections: [
                {
                  title: "Variant Brat",
                  rows: buildVariantRows(m.prefix, text),
                },
              ],
            }),
          },
        ],
      },
    },
  };

  const listMsg = generateWAMessageFromContent(
    m.chat,
    wrapInteractive(content),
    { userJid: sock.user?.id },
  );

  await sock.relayMessage(m.chat, listMsg.message, {
    messageId: listMsg.key.id,
  });
}

async function handler(m, { sock }) {
  const text = m.text;
  const command = String(m.command || "").toLowerCase();

  if (command === "brat" || command === "bratmenu") {
    await sendBratMenu(m, sock, text);
    return;
  }

  if (!text) {
    return m.reply(
      `🖼️ *ʙʀᴀᴛ ɪᴍᴀɢᴇ*\n\n> Masukkan teks\n\n\`Contoh: ${m.prefix}bratimg Hai semua\``,
    );
  }

  m.react("🕕");

  try {
    const buffer = await drawBrat({
      text,
      bgColor: "#8ACE00",
      width: 512,
      height: 512,
      maxWidth: 450,
      maxHeight: 450,
      centerX: 256,
      centerY: 256,
      maxFontSize: 130,
      fontDecrement: 5,
      lineHeightMult: 1.1,
      textColor: "#000000",
    });

    await sock.sendImageAsSticker(m.chat, buffer, m, {
      packname: config.sticker.packname,
      author: config.sticker.author,
    });

    m.react("✅");
  } catch (error) {
    m.react("☢");
    m.reply(te(m.prefix, m.command, m.pushName));
  }
}

export { pluginConfig as config, handler };
