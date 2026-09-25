let client = null;

async function createTursoClient(cfg) {
  if (!cfg || !cfg.enabled || !cfg.url) return null;
  try {
    const { createClient } = await import("@libsql/client");
    client = createClient({
      url: cfg.url,
      authToken: cfg.authToken,
    });
    return client;
  } catch (e) {
    console.warn("[turso] failed to create client:", e.message);
    return null;
  }
}

function getTursoClient() {
  return client;
}

async function initTursoTables(client) {
  if (!client) return;
  const sql = `
    CREATE TABLE IF NOT EXISTS stores (
      key TEXT PRIMARY KEY,
      data TEXT NOT NULL,
      updated_at INTEGER NOT NULL
    );
    CREATE TABLE IF NOT EXISTS session_creds (
      scope TEXT PRIMARY KEY,
      creds TEXT NOT NULL,
      updated_at INTEGER NOT NULL
    );
    CREATE TABLE IF NOT EXISTS session_keys (
      scope TEXT NOT NULL,
      category TEXT NOT NULL,
      id TEXT NOT NULL,
      data TEXT NOT NULL,
      updated_at INTEGER NOT NULL,
      PRIMARY KEY (scope, category, id)
    );
    CREATE TABLE IF NOT EXISTS backup_snapshots (
      id INTEGER PRIMARY KEY AUTOINCREMENT,
      data TEXT NOT NULL,
      created_at INTEGER NOT NULL
    );
  `;
  await client.executeMultiple(sql);
}

async function closeTurso() {
  if (client) { try { client.close(); } catch {} client = null; }
}

export { createTursoClient, getTursoClient, initTursoTables, closeTurso };
