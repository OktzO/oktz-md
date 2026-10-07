import { getDatabase } from "./database.js";
import { logger } from "./logger.js";
import { CronJob } from "cron";
import config from "../../config.js";
import { saluranCtx } from "./context.js";
import { getTodaySchedule, extractPrayerTimes } from "./sholat-api.js";
import { getGroupCache, AsyncPool, yieldToEventLoop } from "./async-pool.js";

const TZ = "Asia/Jakarta";

const SHOLAT_MESSAGES = {
  imsak:
    "🌙 *WAKTU IMSAK*\n\n> Hai Sahabat, waktu Imsak telah tiba.\n> Segera makan sahur sebelum waktu habis.",
  subuh:
    "🌅 *WAKTU SUBUH*\n\n> Hai Sahabat, waktu Sholat Subuh telah tiba.\n> Ambilah air wudhu dan segeralah sholat.",
  terbit:
    "☀️ *WAKTU TERBIT*\n\n> Matahari telah terbit.\n> Selamat beraktivitas hari ini!",
  dhuha:
    "🌤️ *WAKTU DHUHA*\n\n> Hai Sahabat, waktu Sholat Dhuha telah tiba.\n> Jangan lupa sholat Dhuha 2-8 rakaat.",
  dzuhur:
    "🌞 *WAKTU DZUHUR*\n\n> Hai Sahabat, waktu Sholat Dzuhur telah tiba.\n> Ambilah air wudhu dan segeralah sholat.",
  ashar:
    "🌇 *WAKTU ASHAR*\n\n> Hai Sahabat, waktu Sholat Ashar telah tiba.\n> Ambilah air wudhu dan segeralah sholat.",
  maghrib:
    "🌆 *WAKTU MAGHRIB*\n\n> Hai Sahabat, waktu Sholat Maghrib telah tiba.\n> Ambilah air wudhu dan segeralah sholat.",
  isya: "🌙 *WAKTU ISYA*\n\n> Hai Sahabat, waktu Sholat Isya telah tiba.\n> Ambilah air wudhu dan segeralah sholat.",
};

const GAMBAR_SUASANA = {
  imsak: "https://cdn.gimita.id/download/images_1769502277606_04d594fe.jfif",
  subuh: "https://cdn.gimita.id/download/images_1769502277606_04d594fe.jfif",
  terbit: "https://cdn.gimita.id/download/images_1769502277606_04d594fe.jfif",
  dhuha: "https://cdn.gimita.id/download/images_1769502277606_04d594fe.jfif",
  dzuhur:
    "https://cdn.gimita.id/download/qf2d6868_sheikh-zayed-grand-mosque_625x300_04_March_25_1769502237718_92212561.webp",
  ashar:
    "https://cdn.gimita.id/download/18537d69-a2e0-4dc2-a144-57dde0f359b5_1769502389063_5c004902.jpg",
  maghrib:
    "https://cdn.gimita.id/download/mosque-5950407_1280_1769502206553_660ae15c.webp",
  isya: "https://cdn.gimita.id/download/pngtree-nighttime-mosque-illustration-with-realistic-details-celebrating-ramadan-kareem-mubarak-image_3814083_1769502091988_e4cf3326.jpg",
};

const AUDIO_ADZAN = "https://files.catbox.moe/z2bj5s.mp3";

let sock = null;
const sholatCronJobs = new Map();
let dailyRefreshJob = null;

function getTodayDateString() {
  const now = new Date(new Date().toLocaleString("en-US", { timeZone: TZ }));
  return `${now.getFullYear()}-${now.getMonth() + 1}-${now.getDate()}`;
}

const scheduleCache = new Map();

async function loadTodaySchedule(kotaId = "1301") {
  const todayStr = getTodayDateString();
  const cacheKey = `${kotaId}|${todayStr}`;
  if (scheduleCache.has(cacheKey)) return scheduleCache.get(cacheKey);

  try {
    const jadwalData = await getTodaySchedule(kotaId);
    const schedule = extractPrayerTimes(jadwalData);
    const daerah = jadwalData.daerah || "DKI JAKARTA";

    let cityTz = "Asia/Jakarta";
    const d = daerah.toUpperCase();
    if (d.includes("SULAWESI") || d.includes("BALI") || d.includes("NUSA TENGGARA") || d.includes("KALIMANTAN SELATAN") || d.includes("KALIMANTAN TIMUR") || d.includes("KALIMANTAN UTARA")) {
      cityTz = "Asia/Makassar";
    } else if (d.includes("MALUKU") || d.includes("PAPUA")) {
      cityTz = "Asia/Jayapura";
    }

    const entry = { schedule, cityTz };
    scheduleCache.set(cacheKey, entry);
    return entry;
  } catch (e) {
    logger.error("SholatScheduler", `Gagal fetch jadwal: ${e.message}`);
    return null;
  }
}

function clearSholatCronJobs() {
  for (const [, job] of sholatCronJobs) job.stop();
  sholatCronJobs.clear();
}

async function schedulePrayerTimes() {
  clearSholatCronJobs();

  const db = getDatabase();

  let groupsObj;
  try {
    groupsObj = await getGroupCache(sock);
  } catch {
    return;
  }

  const globalAuto = db.setting("autoSholat");
  const globalKotaId = db.setting("autoSholatKota")?.id || "1301";

  // Kelompokkan grup berdasarkan kota: hanya grup yang aktif secara eksplisit
  // (autoSholat=true per grup) atau mewarisi global, dan belum mematikan notif.
  const groupsByKota = new Map();
  for (const groupId of Object.keys(groupsObj)) {
    const g = db.data?.groups?.[groupId] || {};
    const enabled = g.autoSholat ?? globalAuto;
    if (!enabled) continue;
    if (g.notifSholat === false) continue;
    const kotaId = g.autoSholatKota?.id || globalKotaId;
    if (!groupsByKota.has(kotaId)) groupsByKota.set(kotaId, []);
    groupsByKota.get(kotaId).push(groupId);
  }

  if (groupsByKota.size === 0) return;

  let totalJobs = 0;
  for (const [kotaId, groupIds] of groupsByKota) {
    const data = await loadTodaySchedule(kotaId);
    if (!data) continue;

    const { schedule, cityTz } = data;

    for (const [sholat, waktu] of Object.entries(schedule)) {
      if (waktu === "-") continue;

      const [hour, minute] = waktu.split(":").map(Number);
      const cronExpr = `${minute} ${hour} * * *`;

      const job = new CronJob(
        cronExpr,
        async () => {
          await sendSholatNotifications(sholat, waktu, kotaId, groupIds);
        },
        null,
        true,
        cityTz,
      );
      job.threshold = 10000;

      sholatCronJobs.set(`${kotaId}:${sholat}`, job);
      totalJobs++;
    }
  }

  logger.info(
    "SholatScheduler",
    `Scheduled ${totalJobs} prayer times across ${groupsByKota.size} kota(s)`,
  );
}

async function sendSholatNotifications(sholat, waktu, kotaId, groupIds) {
  try {
    const db = getDatabase();

    const saluranId = config.saluran?.id || "120363400911374213@newsletter";
    const saluranName = config.saluran?.name || config.bot?.name || "Bot";

    const globalAuto = db.setting("autoSholat");

    // Re-validasi ulang saat kirim: grup bisa saja sudah mematikan fitur
    // sejak cron dijadwalkan.
    const groupList = (groupIds || []).filter((groupId) => {
      const g = db.data?.groups?.[groupId] || {};
      const enabled = g.autoSholat ?? globalAuto;
      if (!enabled) return false;
      if (g.notifSholat === false) return false;
      const gKotaId = g.autoSholatKota?.id || db.setting("autoSholatKota")?.id || "1301";
      return gKotaId === kotaId;
    });

    if (groupList.length === 0) return;

    let sentCount = 0;
    const closedGroups = [];
    const isSholatTime = [
      "subuh",
      "dzuhur",
      "ashar",
      "maghrib",
      "isya",
    ].includes(sholat);

    const pool = new AsyncPool(5);

    for (const groupId of groupList) {
      pool.add(async () => {
        const groupData = db.data?.groups?.[groupId] || {};
        const closeGroup = groupData.autoSholatCloseGroup ?? db.setting("autoSholatCloseGroup") ?? false;
        const duration = groupData.autoSholatDuration ?? db.setting("autoSholatDuration") ?? 5;
        const sendAudio = groupData.autoSholatAudio ?? (db.setting("autoSholatAudio") !== false);
        const kotaNama = groupData.autoSholatKota?.nama ?? db.setting("autoSholatKota")?.nama ?? "KOTA JAKARTA";

        let message = `${SHOLAT_MESSAGES[sholat] || `🕌 *WAKTU ${sholat.toUpperCase()}*`}\n\n⏰ *${waktu} WIB*\n📍 *${kotaNama}*`;
        if (closeGroup && isSholatTime) {
          message += `\n\n> 🔒 _Grup ditutup ${duration} menit untuk sholat_`;
        }

        try {
          if (sendAudio && isSholatTime) {
            try {
              await sock.sendMessage(groupId, {
                audio: { url: AUDIO_ADZAN },
                mimetype: "audio/mpeg",
                ptt: false,
                contextInfo: {
                  ...saluranCtx(),
                  forwardedNewsletterMessageInfo: {
                    newsletterJid: saluranId,
                    newsletterName: saluranName,
                    serverMessageId: 127,
                  },
                },
              });
            } catch (audioErr) {
              logger.error("SholatScheduler", `Failed to send audio to ${groupId}: ${audioErr.message}`);
            }
          }

          await sock.sendMessage(groupId, {
            text: message,
            contextInfo: {
              ...saluranCtx(),
              forwardedNewsletterMessageInfo: {
                newsletterJid: saluranId,
                newsletterName: saluranName,
                serverMessageId: 127,
              },
            },
          });

          if (closeGroup && isSholatTime) {
            try {
              await sock.groupSettingUpdate(groupId, "announcement");
              closedGroups.push({ groupId, duration });
            } catch (e) {
              logger.error(
                "SholatScheduler",
                `Failed to close ${groupId}: ${e.message}`,
              );
            }
          }

          sentCount++;
        } catch (err) {
          logger.error(
            "SholatScheduler",
            `Failed to send to ${groupId}: ${err.message}`,
          );
        }
      });

      // yield every 20 groups so event loop breathes
      if (sentCount > 0 && sentCount % 20 === 0) await yieldToEventLoop();
    }

    await pool.onIdle();

    for (const { groupId, duration } of closedGroups) {
      setTimeout(
        async () => {
          try {
            await sock.groupSettingUpdate(groupId, "not_announcement");
            await sock.sendMessage(groupId, {
              text: `✅ Grup dibuka kembali setelah sholat ${sholat}.\n\n> Semoga sholat kita diterima. Aamiin 🤲`,
              contextInfo: {
                forwardingScore: 9999,
                isForwarded: true,
                forwardedNewsletterMessageInfo: {
                  newsletterJid: saluranId,
                  newsletterName: saluranName,
                  serverMessageId: 127,
                },
              },
            });
          } catch (e) {
            logger.error(
              "SholatScheduler",
              `Failed to open ${groupId}: ${e.message}`,
            );
          }
        },
        duration * 60 * 1000,
      );
    }

    if (closedGroups.length > 0) {
      logger.info(
        "SholatScheduler",
        `Closed ${closedGroups.length} groups after ${sholat}, will reopen per-group durations`,
      );
    }

    if (sentCount > 0) {
      logger.info(
        "SholatScheduler",
        `Sent ${sholat} notification to ${sentCount} groups` +
          (closedGroups.length > 0 ? ` (${closedGroups.length} closed)` : ""),
      );
    }
  } catch (error) {
    logger.error("SholatScheduler", `Error: ${error.message}`);
  }
}

function initSholatScheduler(socketInstance) {
  sock = socketInstance;

  if (dailyRefreshJob) dailyRefreshJob.stop();

  dailyRefreshJob = new CronJob(
    "1 0 * * *",
    async () => {
      scheduleCache.clear();
      await schedulePrayerTimes();
    },
    null,
    true,
    TZ,
  );
  dailyRefreshJob.threshold = 10000;

  schedulePrayerTimes();
  logger.info(
    "SholatScheduler",
    "Prayer time scheduler started (CronJob, precise per-prayer)",
  );
}

function stopSholatScheduler() {
  clearSholatCronJobs();
  if (dailyRefreshJob) {
    dailyRefreshJob.stop();
    dailyRefreshJob = null;
  }
  logger.info("SholatScheduler", "Prayer time scheduler stopped");
}

export {
  initSholatScheduler,
  stopSholatScheduler,
  SHOLAT_MESSAGES,
  GAMBAR_SUASANA,
  AUDIO_ADZAN,
};
