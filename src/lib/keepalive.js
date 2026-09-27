// Library (onigis/Socket/socket.js):
//   if (Date.now() - lastDateRecv > keepAliveIntervalMs + 5000)
//     end(new Boom('Connection was lost', { statusCode: 408 }))
// lastDateRecv hanya disegarkan frame masuk. Bot sepi -> hanya ping yang
// menyegarkan -> slack cuma 5000ms, jadi jedak event loop >15s = koneksi mati.
export const KEEPALIVE_FLOOR_MS = 30000;
export const KEEPALIVE_STALL_MARGIN_MS = 5000;

export function resolveKeepAlive({ configured } = {}) {
  const n = Number(configured);
  const intervalMs =
    Number.isFinite(n) && n > KEEPALIVE_FLOOR_MS ? Math.floor(n) : KEEPALIVE_FLOOR_MS;
  return {
    intervalMs,
    stallToleranceMs: intervalMs + KEEPALIVE_STALL_MARGIN_MS,
  };
}
