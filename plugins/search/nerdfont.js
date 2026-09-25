import axios from "axios";
import * as cheerio from "cheerio";

import te from "../../src/lib/error.js";
async function nerdfonts() {
  try {
    const { data } = await axios.get(
      "https://www.nerdfonts.com/font-downloads",
    );
    const $ = cheerio.load(data);
    const result = [];
    $("div.item").each((_, rynn) => {
      const name = $(rynn).find("span.nerd-font-invisible-text").text().trim();
      const textContent = $(rynn).find("div").first().text();
      const version =
        textContent.match(/Version:\s*([^\n\r]+)/)[1].trim() || null;
      const info = textContent.match(/Info:\s*([^\n\r]+)/)[1].trim() || null;
      const preview_image =
        "https://www.nerdfonts.com" +
          $(rynn)
            .find("a.font-preview")
            .attr("style")
            .match(
              /background-image\s*:\s*url\s*\(\s*['"]?([^'"]+)['"]?\s*\)/i,
            )[1] || null;
      const preview_url =
        $(rynn).find("a.nf-oct-link_external").attr("href") || null;
      const download_url = $(rynn).find("a.nf-fa-download").attr("href");
      if (name && download_url) {
        result.push({
          name,
          version,
          info,
          preview_image,
          preview_url,
          download_url,
        });
      }
    });
    return result;
  } catch (error) {
    throw new Error(error.message, { cause: error });
  }
}
const pluginConfig = {
  name: "nerdfont",
  alias: [],
  category: "search",
  description: "Cari font di DaFont",
  usage: ".nerdfont <query>",
  example: ".nerdfont Coolvetica",
  isOwner: false,
  isPremium: false,
  isGroup: false,
  isPrivate: false,
  cooldown: 5,
  energi: 0,
  isEnabled: true,
};
async function handler(m, { sock }) {
  try {
    const res = await nerdfonts();
    const rows = res.map((f, _) => {
      return {
        header: `Font ${f.name}`,
        title: f.info,
        description: `Version: ${f.version}`,
        id: `${m.prefix}nerdfont-ambil ${f.name}`,
      };
    });
    await sock.sendMessage(
      m.chat,
      {
        text: "Silahkan pilih font yang ingin kamu download",
        footer: "Click tombol di bawah ini",
        interactiveButtons: [
          {
            name: "single_select",
            buttonParamsJson: JSON.stringify({
              title: `Pilih Font Disini`,
              sections: [
                {
                  title: "Aku harap, font ini dapat membantu kamu",
                  highlight_label: "Font Pilihan",
                  rows: rows,
                },
              ],
            }),
          },
        ],
      },
      { quoted: m },
    );
  } catch (err) {
    return m.reply(te(m.prefix, m.command, m.pushName));
  }
}
export { pluginConfig as config, handler };
