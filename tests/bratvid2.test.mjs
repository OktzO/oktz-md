import test from 'node:test';
import assert from 'node:assert/strict';
import { execFile } from 'node:child_process';
import { promisify } from 'node:util';
import { writeFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { getFfmpegPath } from '../src/lib/ourin-ffmpeg.js';
import { generateBratVideo } from '../plugins/sticker/bratvid2.js';

const execFileAsync = promisify(execFile);

// Guard regresian pipeline rawvideo: output harus tetap video H.264
// CFR 60fps dengan durasi ~= hold + animasi (~3.4s => ~205 frame).
test('bratvid2 renders a playable 60fps mp4 without PNG intermediates', async () => {
	const ffmpeg = getFfmpegPath();
	assert.ok(ffmpeg, 'ffmpeg harus resolves via PATH/@ffmpeg-installer');

	const buf = await generateBratVideo({ text: 'halo guys selamat datang' });
	assert.equal(buf.subarray(4, 8).toString('latin1'), 'ftyp', 'output harus mp4');

	const out = join(tmpdir(), `bratvid2-${process.pid}.mp4`);
	writeFileSync(out, buf);
	try {
		const { stderr } = await execFileAsync(ffmpeg, ['-hide_banner', '-i', out, '-f', 'null', '-'], {
			maxBuffer: 10 * 1024 * 1024,
		});
		const frames = Number(stderr.match(/frame=\s*(\d+)/g)?.pop()?.match(/\d+/)?.[0] ?? 0);
		assert.ok(frames >= 150, `frame terlalu sedikit: ${frames}`);
		assert.match(stderr, /yuv420p/);
		assert.match(stderr, /60 fps/);
	} finally {
		rmSync(out, { force: true });
	}
});
