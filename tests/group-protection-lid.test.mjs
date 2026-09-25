import test from 'node:test';
import assert from 'node:assert/strict';
import { handleAntilinkGc, handleAntilinkAll } from '../src/lib/group-protection.js';

const CHAT = '120363@g.us';
const BOT_ID = '628999888777:12@s.whatsapp.net';
const BOT_LID = '9900011122233344@lid';
const MEMBER_LID = '99777888999000111@lid';
const MEMBER_PN = '6285551112223@s.whatsapp.net';
const ADMIN_LID = '9911122233344455@lid';
const ADMIN_PN = '6281234567890@s.whatsapp.net';

// Bentuk participant onigis utk grup addressing_mode=lid (groups.js:338):
// { id: LID, phoneNumber: PN } TANPA field `lid`.
const participants = [
  { id: BOT_LID, phoneNumber: '628999888777@s.whatsapp.net', admin: 'superadmin' },
  { id: ADMIN_LID, phoneNumber: ADMIN_PN, admin: 'admin' },
  { id: MEMBER_LID, phoneNumber: MEMBER_PN, admin: undefined },
];

// Stub: handleAntilinkGc/All menerima db lewat argumen, jadi tidak perlu
// menghidupkan database asli (tes ini tidak boleh menulis ke Turso produksi).
function makeDb(groupCfg) {
	return { getGroup: () => groupCfg, setting: () => false };
}

function makeM(senderJid, resolvedSender) {
	return {
		isGroup: true,
		chat: CHAT,
		sender: resolvedSender,
		senderNumber: resolvedSender.replace(/@.+/g, ''),
		key: { remoteJid: CHAT, id: 'OFFENDERMSG1', fromMe: false, participant: senderJid },
		id: 'OFFENDERMSG1',
		body: 'gabung sini https://chat.whatsapp.com/AbCdEfGh',
		text: 'gabung sini https://chat.whatsapp.com/AbCdEfGh',
		isAdmin: false,
		isBotAdmin: false,
		isOwner: false,
		fromMe: false,
	};
}

function makeSock(spy) {
	return {
		user: { id: BOT_ID, lid: BOT_LID, name: 'Bot' },
		groupMetadata: async () => ({ id: CHAT, subject: 'G', desc: '', participants }),
		async sendMessage(jid, content) {
			spy.push({ jid, content });
			return { key: { id: 'SENT' } };
		},
		async groupParticipantsUpdate(jid, ps, action) {
			spy.push({ jid, participants: ps, action });
			return [{ status: '200', jid: ps[0] }];
		},
	};
}

const sent = (spy, what) => spy.filter((s) => s.content?.[what]);

test('antilinkgc: bot admin LID terdeteksi + revoke pakai addressing asli server', async () => {
	const db = makeDb({ antilinkgc: 'on', antilinkgcMode: 'remove' });
	const spy = [];
	const handled = await handleAntilinkGc(makeM(MEMBER_LID, MEMBER_PN), makeSock(spy), db);
	assert.equal(handled, true, 'proteksi harus jalan');

	const del = sent(spy, 'delete');
	assert.equal(del.length, 1, 'harus ada revoke, bukan cuma balas "bot harus admin"');
	assert.equal(del[0].content.delete.id, 'OFFENDERMSG1');
	assert.equal(del[0].content.delete.participant, MEMBER_LID, 'participant harus LID asli server');
});

test('antilinkgc: pesan admin grup tidak dihapus', async () => {
	const db = makeDb({ antilinkgc: 'on', antilinkgcMode: 'remove' });
	const spy = [];
	const handled = await handleAntilinkGc(makeM(ADMIN_LID, ADMIN_PN), makeSock(spy), db);
	assert.equal(handled, false, 'admin tidak boleh kena proteksi');
	assert.equal(sent(spy, 'delete').length, 0);
});

test('antilinkgc mode kick: member di-kick pakai addressing asli server', async () => {
	const db = makeDb({ antilinkgc: 'on', antilinkgcMode: 'kick' });
	const spy = [];
	await handleAntilinkGc(makeM(MEMBER_LID, MEMBER_PN), makeSock(spy), db);
	const kick = spy.find((s) => s.action === 'remove');
	assert.ok(kick, 'harus ada groupParticipantsUpdate remove');
	assert.deepEqual(kick.participants, [MEMBER_LID]);
});

test('antilinkall: ikut jalan di grup LID (dulu always false karena isBotAdminCheck)', async () => {
	const db = makeDb({ antilinkall: 'on', antilinkallMode: 'remove' });
	const spy = [];
	const handled = await handleAntilinkAll(makeM(MEMBER_LID, MEMBER_PN), makeSock(spy), db);
	assert.equal(handled, true);
	assert.equal(sent(spy, 'delete')[0].content.delete.participant, MEMBER_LID);
});
