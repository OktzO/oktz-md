import test from 'node:test';
import assert from 'node:assert/strict';
import { existsSync, rmSync } from 'fs';
import { tmpdir } from 'os';
import { join } from 'path';
import { queueFFmpeg } from '../src/lib/ourin-ffmpeg.js';

const marker = join(tmpdir(), `ourin-ffmpeg-pwned-${process.pid}`);

test('queueFFmpeg runs without a shell: $(...) in quoted input is inert', async () => {
	rmSync(marker, { force: true });
	await assert.rejects(queueFFmpeg(
		`ffmpeg -y -i "\$(touch ${marker})" /nonexistent-dir/out.mp4`
	));
	assert.equal(existsSync(marker), false, 'shell metacharacters must not execute');
});

test('queueFFmpeg normal ffmpeg invocation still works', async () => {
	const out = join(tmpdir(), `ourin-ffmpeg-tone-${process.pid}.mp3`);
	rmSync(out, { force: true });
	await queueFFmpeg(`ffmpeg -y -f lavfi -i "sine=frequency=200:duration=1" -t 1 "${out}"`);
	assert.ok(existsSync(out));
	rmSync(out, { force: true });
});
