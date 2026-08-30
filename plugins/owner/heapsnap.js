import te from "../../src/lib/ourin-error.js";
import {
  takeHeapSnapshot,
  heapSnapshotEligibility,
  HEAP_SNAP_RSS_CEILING,
  HEAP_SNAP_PROJECTED_CEILING,
} from "../../src/lib/ourin-profiler.js";

const pluginConfig = {
  name: "heapsnap",
  alias: ["heapsnapshot", "heapsnap", "snapheap"],
  category: "owner",
  description: "Ambil heap snapshot (dengan guard RSS — Owner Only)",
  usage: ".heapsnap [label] | .heapsnap status",
  example: ".heapsnap before-ocr",
  isOwner: true,
  isPremium: false,
  isGroup: false,
  isPrivate: false,
  cooldown: 30,
  energi: 0,
  isEnabled: true,
};

function formatMB(bytes) {
  return (bytes / 1024 / 1024).toFixed(1) + "MB";
}

async function handler(m, { sock }) {
  const args = (m.fullArgs || m.text || "").trim().split(/\s+/);
  const sub = args[0]?.toLowerCase();

  if (sub === "status" || !sub) {
    const elig = heapSnapshotEligibility();
    const mem = process.memoryUsage();
    const projectedMb = (mem.rss + 2 * mem.heapTotal) / 1024 / 1024;

    const statusLines = [
      `📊 *Status Heap Snapshot*\n`,
      `RSS: ${(mem.rss / 1024 / 1024).toFixed(1)}MB / ${formatMB(HEAP_SNAP_RSS_CEILING)} (ceiling)`,
      `Heap Total: ${(mem.heapTotal / 1024 / 1024).toFixed(1)}MB`,
      `Proyeksi puncak: ${projectedMb.toFixed(1)}MB / ${formatMB(HEAP_SNAP_PROJECTED_CEILING)} (ceiling)`,
      ``,
      `Snapshot: ${elig.allowed ? "✅ Bisa diambil" : "❌ Tidak aman"}`,
    ];
    if (!elig.allowed) {
      statusLines.push(`> Alasan: ${elig.reason}`);
    }
    statusLines.push(
      `\nGunakan *${m.prefix}heapsnap [label]* untuk mengambil snapshot.`,
    );
    return m.reply(statusLines.join("\n"));
  }

  const label = sub !== "status" ? sub : "snap";
  const result = takeHeapSnapshot({ label, trigger: "command" });

  if (!result.ok) {
    if (result.abort) {
      return m.reply(
        `❌ *Heap Snapshot DIBATALKAN (Safety Guard)*\n\n` +
        `> Alasan: ${result.reason}\n\n` +
        `RSS saat ini: ${result.rssMb}MB\n` +
        `Proyeksi puncak: ${result.projectedMb}MB\n\n` +
        `Coba lagi saat RSS lebih rendah (misal setelah gc atau sepi chat).`,
      );
    }
    return m.reply(`❌ ${result.error}`);
  }

  await m.react("💾");
  return m.reply(
    `💾 *Heap Snapshot Berhasil*\n\n` +
    `> Label: ${label}\n` +
    `> File: \`${result.filePath}\`\n` +
    `> Size: ${(result.sizeBytes / 1024 / 1024).toFixed(1)}MB\n` +
    `> RSS saat ambil: ${result.rssMb}MB\n\n` +
    `File .heapsnapshot bisa diambil lewat file manager panel atau command *.get*.\n\n` +
    `⚠️ Ambil minimal 2 snapshot dengan jeda 2-6 jam untuk perbandingan (diff).`,
  );
}

export { pluginConfig as config, handler };