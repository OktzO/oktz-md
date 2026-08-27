import { describe, it } from 'node:test';
import assert from 'node:assert';

const SENDER = '6281234567890@s.whatsapp.net';

async function loadLogger() {
  return await import('../src/lib/ourin-logger.js');
}

function captureLog(fn) {
  const lines = [];
  const orig = console.log;
  console.log = (...args) => lines.push(args.map((a) => String(a)).join(' '));
  try {
    fn();
  } finally {
    console.log = orig;
  }
  return lines.map((l) => l.replace(/\x1B\[\d+m/g, ''));
}

describe('ourin-logger logCommand', () => {
  it('logs single CMD line with command and name, never the sender number', async () => {
    const mod = await loadLogger();
    const lines = captureLog(() =>
      mod.logCommand({
        prefix: '.',
        command: 'ping',
        pushName: 'Zann',
        sender: SENDER,
        chatType: 'private',
      })
    );
    assert.strictEqual(lines.length, 1);
    assert.ok(lines[0].includes('CMD ]'), `missing [ CMD ] tag: ${lines[0]}`);
    assert.ok(lines[0].includes('.ping'), `missing command: ${lines[0]}`);
    assert.ok(lines[0].includes('Zann'), `missing name: ${lines[0]}`);
    assert.ok(!lines[0].includes('6281234567890'), `number leaked: ${lines[0]}`);
  });

  it('falls back to generic name when pushName missing, still no number', async () => {
    const mod = await loadLogger();
    const lines = captureLog(() =>
      mod.logCommand({
        prefix: '.',
        command: 'menu',
        pushName: '',
        sender: SENDER,
        chatType: 'private',
      })
    );
    assert.strictEqual(lines.length, 1);
    assert.ok(!lines[0].includes('6281234567890'), `number leaked: ${lines[0]}`);
    assert.ok(lines[0].includes('.menu'));
  });

  it('shows group name for group chats', async () => {
    const mod = await loadLogger();
    const lines = captureLog(() =>
      mod.logCommand({
        prefix: '.',
        command: 'sticker',
        pushName: 'Zann',
        sender: SENDER,
        chatType: 'group',
        groupName: 'Test Group',
      })
    );
    assert.strictEqual(lines.length, 1);
    assert.ok(lines[0].includes('Test Group'), `missing group: ${lines[0]}`);
    assert.ok(!lines[0].includes('6281234567890'));
  });

  it('skips empty command', async () => {
    const mod = await loadLogger();
    const lines = captureLog(() =>
      mod.logCommand({ prefix: '.', command: '', sender: SENDER })
    );
    assert.strictEqual(lines.length, 0);
  });
});
