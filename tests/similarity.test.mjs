import { describe, it } from 'node:test';
import assert from 'node:assert';
import { findSimilarCommands } from '../src/lib/ourin-similarity.js';

const COMMANDS = [
  'menu', 'help', 'ping', 'sticker', 'calculator',
  'timelapse', 'animelist', 'music', 'profile', 'level',
];

describe('findSimilarCommands behavior', () => {
  it('exact command returns exact match with consumer shape', () => {
    const results = findSimilarCommands('menu', COMMANDS);
    assert.ok(results.length > 0);
    const top = results[0];
    assert.strictEqual(top.command, 'menu');
    assert.strictEqual(top.type, 'exact');
    assert.strictEqual(top.similarity, 1.0);
    for (const key of ['command', 'similarity', 'distance', 'type', 'emoji', 'reason']) {
      assert.ok(key in top, `result must keep field: ${key}`);
    }
  });

  it('typo meny suggests menu (handler options: maxResults 1, minSimilarity 0.8, maxDistance 2)', () => {
    const results = findSimilarCommands('meny', COMMANDS, {
      maxResults: 1,
      minSimilarity: 0.8,
      maxDistance: 2,
    });
    assert.strictEqual(results.length, 1);
    assert.strictEqual(results[0].command, 'menu');
  });

  it('typo helo suggests help', () => {
    const results = findSimilarCommands('helo', COMMANDS);
    assert.strictEqual(results[0].command, 'help');
  });

  it('transposition at first position still resolves (ehlp -> help)', () => {
    const results = findSimilarCommands('ehlp', COMMANDS, {
      maxResults: 1,
      minSimilarity: 0.8,
      maxDistance: 2,
    });
    assert.strictEqual(results.length, 1);
    assert.strictEqual(results[0].command, 'help');
  });

  it('short input bypasses first-char filter (elp -> help)', () => {
    const results = findSimilarCommands('elp', COMMANDS);
    assert.ok(results.some((r) => r.command === 'help'));
  });

  it('long random input yields no suggestions', () => {
    const results = findSimilarCommands('xqzwvjkgdlqpzmx', COMMANDS);
    assert.strictEqual(results.length, 0);
  });

  it('first-char mismatch on long input is filtered out (xalculator)', () => {
    const results = findSimilarCommands('xalculator', COMMANDS);
    assert.ok(!results.some((r) => r.command === 'calculator'));
  });
});
