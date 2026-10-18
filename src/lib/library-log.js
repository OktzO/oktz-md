// Klasifikasi & agregasi log dari library WhatsApp (onigis) agar kegagalan
// dekripsi yang normal (pemulihan otomatis via retry receipt) tidak membanjiri
// console dengan baris "FAIL library error: ... | No session found to decrypt
// message" berulang.
//
// Dua baris yang sebelumnya muncul untuk SATU kejadian (inner transaction
// rollback + outer "failed to decrypt message") disatukan: baris rollback
// di-drop (duplikat), baris utama diagregasi per jendela waktu.

const DEFAULT_WINDOW_MS = 60_000;
const HARD_PRINT_CAP = 3;

const MISS_RE = /no session found to decrypt|no session found|empty record|session record/i;
const HARD_RE = /bad mac|mac verification|invalid signature|invalid message|untrusted/i;

export function classifyLibraryError(text, detail) {
  const combined = `${text || ""} ${detail || ""}`;
  const isRollback = /transaction failed, rolling back/i.test(text || "");
  const isDecrypt = /failed to decrypt message/i.test(text || "");
  const isRetrySent = /sent retry receipt/i.test(text || "");
  const isRetryExhausted = /reached retry limit/i.test(text || "");
  if (isRetrySent) return "retry-sent";
  if (isRetryExhausted) return "retry-exhausted";
  if (isRollback) {
    if (MISS_RE.test(combined) || HARD_RE.test(combined)) return "decrypt-rollback";
    return "other";
  }
  if (isDecrypt) {
    if (HARD_RE.test(combined)) return "decrypt-hard";
    if (MISS_RE.test(combined)) return "decrypt-miss";
    return "other";
  }
  return "other";
}

export function createLibraryLogRouter({
  windowMs = DEFAULT_WINDOW_MS,
  report,
  now = () => Date.now(),
} = {}) {
  const emit = typeof report === "function" ? report : () => {};
  let windowStart = now();
  const counts = {
    miss: 0,
    hard: 0,
    rollback: 0,
    retrySent: 0,
    retryExhausted: 0,
  };
  let hardPrinted = 0;
  let timer = null;

  function reset() {
    counts.miss = 0;
    counts.hard = 0;
    counts.rollback = 0;
    counts.retrySent = 0;
    counts.retryExhausted = 0;
    hardPrinted = 0;
    windowStart = now();
  }

  function flush() {
    if (timer) {
      clearTimeout(timer);
      timer = null;
    }
    const totalDecrypt =
      counts.miss + counts.hard + counts.rollback;
    if (totalDecrypt === 0 && counts.retryExhausted === 0) {
      reset();
      return;
    }
    const parts = [];
    if (counts.miss)
      parts.push(`${counts.miss} dekripsi gagal (sesi/sender-key hilang)`);
    if (counts.hard)
      parts.push(`${counts.hard} dekripsi gagal (MAC/tanda tangan)`);
    if (counts.rollback)
      parts.push(`${counts.rollback} transaksi di-rollback`);
    if (counts.retrySent)
      parts.push(`${counts.retrySent} retry receipt dikirim ke pengirim`);
    if (counts.retryExhausted)
      parts.push(`${counts.retryExhausted} pesan mencapai batas retry`);
    const elapsed = Math.max(1, Math.round((now() - windowStart) / 1000));
    const summary = `E2EE ringkasan ${elapsed}s: ${parts.join(
      ", ",
    )}. Pemulihan otomatis berjalan (retry receipt).`;
    emit("warn", summary);
    reset();
  }

  function scheduleFlush() {
    if (timer) return;
    timer = setTimeout(() => {
      timer = null;
      flush();
    }, windowMs);
    if (timer && typeof timer.unref === "function") timer.unref();
  }

  function reportFn(level, text, detail) {
    const cat = classifyLibraryError(text, detail);
    if (now() - windowStart >= windowMs) flush();
    scheduleFlush();
    switch (cat) {
      case "decrypt-rollback":
        counts.rollback++;
        return;
      case "decrypt-miss":
        counts.miss++;
        if (counts.miss === 1) emit("warn", text);
        return;
      case "decrypt-hard":
        counts.hard++;
        if (hardPrinted < HARD_PRINT_CAP) {
          hardPrinted++;
          emit("error", text);
        }
        return;
      case "retry-sent":
        counts.retrySent++;
        return;
      case "retry-exhausted":
        counts.retryExhausted++;
        emit("warn", text);
        return;
      default:
        emit(level, text);
    }
  }

  return { report: reportFn, flush, classify: classifyLibraryError };
}
