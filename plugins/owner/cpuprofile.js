import te from "../../src/lib/ourin-error.js";
import {
  startCpuProfile,
  stopCpuProfile,
  isCpuProfiling,
  CPU_MAX_DURATION_MS,
} from "../../src/lib/ourin-profiler.js";

const pluginConfig = {
  name: "cpuprofile",
  alias: ["cpup", "cpuprof"],
  category: "owner",
  description: "CPU profiling via node:inspector (Owner Only)",
  usage: ".cpuprofile start [menit] | .cpuprofile stop | .cpuprofile status",
  example: ".cpuprofile start 5",
  isOwner: true,
  isPremium: false,
  isGroup: false,
  isPrivate: false,
  cooldown: 0,
  energi: 0,
  isEnabled: true,
};

async function handler(m, { sock }) {
  const args = (m.fullArgs || m.text || "").trim().split(/\s+/);
  const sub = args[0]?.toLowerCase();

  if (sub === "start") {
    const minutes = Math.min(parseFloat(args[1]) || 5, CPU_MAX_DURATION_MS / 60000);
    const durationMs = Math.round(minutes * 60 * 1000);
    const result = await startCpuProfile({ name: `cmd-${m.sender?.split("@")[0] || "owner"}`, durationMs, trigger: "command" });
    if (!result.ok) {
      return m.reply(`❌ ${result.error}`);
    }
    await m.react("🟢");
    return m.reply(
      `🟢 *CPU Profiling Dimulai*\n\n` +
      `> Durasi: ${minutes} menit\n` +
      `> Nama: ${result.name}\n` +
      `> Auto-stop: ✅ (max ${CPU_MAX_DURATION_MS / 60000} menit)\n\n` +
      `Gunakan *${m.prefix}cpuprofile stop* untuk stop lebih awal\n` +
      `Gunakan *${m.prefix}cpuprofile status* untuk cek status`,
    );
  }

  if (sub === "stop") {
    const result = await stopCpuProfile();
    if (!result.ok) {
      return m.reply(`❌ ${result.error}`);
    }
    await m.react("⏹️");
    return m.reply(
      `⏹️ *CPU Profiling Selesai*\n\n` +
      `> Durasi: ${(result.elapsedMs / 1000).toFixed(1)}s\n` +
      `> File: \`${result.filePath}\`\n\n` +
      `File .cpuprofile bisa diambil lewat file manager panel atau command *.get*.`,
    );
  }

  if (sub === "status" || !sub) {
    if (!isCpuProfiling()) {
      const mem = process.memoryUsage();
      return m.reply(
        `📊 *Status Profiler*\n\nCPU: ❌ Tidak aktif\n\n` +
        `> RSS: ${(mem.rss / 1024 / 1024).toFixed(1)}MB\n` +
        `> Heap: ${(mem.heapUsed / 1024 / 1024).toFixed(1)}/${(mem.heapTotal / 1024 / 1024).toFixed(1)}MB\n\n` +
        `Gunakan *${m.prefix}cpuprofile start [menit]* untuk memulai.`,
      );
    }
    const { cpu } = (await import("../../src/lib/ourin-profiler.js")).profilerStatus();
    const mem = process.memoryUsage();
    return m.reply(
      `📊 *Status Profiler*\n\nCPU: 🟢 Aktif\n` +
      `> Nama: ${cpu.name}\n` +
      `> Berjalan: ${cpu.elapsedS}s\n` +
      `> Sisa: ${cpu.remainingS}s\n` +
      `> Durasi: ${cpu.durationS}s\n\n` +
      `> RSS: ${(mem.rss / 1024 / 1024).toFixed(1)}MB\n` +
      `> Heap: ${(mem.heapUsed / 1024 / 1024).toFixed(1)}/${(mem.heapTotal / 1024 / 1024).toFixed(1)}MB`,
    );
  }

  return m.reply(
    `⚙️ *CPU Profiler*\n\n` +
    `> *${m.prefix}cpuprofile start [menit]* — Mulai profiling (default 5, max ${CPU_MAX_DURATION_MS / 60000})\n` +
    `> *${m.prefix}cpuprofile stop* — Stop profiling lebih awal\n` +
    `> *${m.prefix}cpuprofile status* — Cek status`,
  );
}

export { pluginConfig as config, handler };