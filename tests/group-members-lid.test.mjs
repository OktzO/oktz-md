import test from 'node:test';
import assert from 'node:assert/strict';
import { findParticipantByNumber } from '../src/lib/ourin-lid.js';
import { handler as promote } from '../plugins/group/promote.js';
import { handler as demote } from '../plugins/group/demote.js';
import { handler as kick } from '../plugins/group/kick.js';

const CHAT = '120365@g.us';
const TARGET_LID = '99777888999000111@lid';
const TARGET_PN = '6285551112223@s.whatsapp.net';
const SENDER_PN = '6281234567890@s.whatsapp.net';

// Bentuk participant ourin utk grup addressing_mode=lid: { id: LID, phoneNumber: PN }
const LIDSHAPE_ADMIN_LID = '9922233344455566@lid';
const LIDSHAPE_ADMIN_PN = '6284445556667@s.whatsapp.net';
const lidGroup = [
  { id: '9900011122233344@lid', phoneNumber: '628999888777@s.whatsapp.net', admin: 'superadmin' },
  { id: TARGET_LID, phoneNumber: TARGET_PN, admin: undefined },
  { id: SENDER_PN, lid: '9911122233344455@lid', admin: 'admin' },
  { id: LIDSHAPE_ADMIN_LID, phoneNumber: LIDSHAPE_ADMIN_PN, admin: 'admin' },
];

const sock = {
	user: { id: '628999888777:12@s.whatsapp.net', lid: '9900011122233344@lid' },
	ops: [],
	async groupParticipantsUpdate(jid, participants, action) {
		this.ops.push({ jid, participants, action });
		return participants.map((p) => ({ status: '200', jid: p }));
	},
};

function makeM(over = {}) {
	return {
		chat: CHAT,
		isGroup: true,
		sender: SENDER_PN,
		groupMetadata: { id: CHAT, subject: 'G', participants: lidGroup },
		mentionedJid: [TARGET_PN],
		prefix: '.',
		command: 'kick',
		pushName: 'Admin',
		replies: [],
		async reply(t) { this.replies.push(String(t)); },
		async react() {},
		...over,
	};
}

test('findParticipantByNumber: LID shape ourin dikenali dari PN maupun LID', () => {
	assert.equal(findParticipantByNumber(lidGroup, TARGET_PN)?.id, TARGET_LID);
	assert.equal(findParticipantByNumber(lidGroup, TARGET_LID)?.id, TARGET_LID);
	assert.equal(findParticipantByNumber(lidGroup, '6285551112223@s.whatsapp.net')?.admin, undefined);
	assert.equal(findParticipantByNumber(lidGroup, '6281111111111@s.whatsapp.net'), null);
	// bentuk grup PN: { id: PN, lid: LID }
	assert.equal(findParticipantByNumber(lidGroup, '9911122233344455@lid')?.admin, 'admin');
});

test('.promote mengirim addressing server (LID), bukan PN hasil resolve', async () => {
	sock.ops.length = 0;
	const m = makeM({ command: 'promote' });
	await promote(m, { sock });
	assert.deepEqual(sock.ops[0]?.participants, [TARGET_LID], `replies: ${m.replies}`);
	assert.equal(sock.ops[0]?.action, 'promote');
});

test('.demote bekerja pada admin ber-shape LID (dulu "User tidak ditemukan")', async () => {
	sock.ops.length = 0;
	// target ditulis sebagai PN (hasil resolve serialize), tapi participant di
	// grup ini hanya punya { id: LID, phoneNumber: PN }.
	const m = makeM({ command: 'demote', mentionedJid: [LIDSHAPE_ADMIN_PN], sender: TARGET_PN });
	await demote(m, { sock });
	assert.equal(sock.ops[0]?.action, 'demote', `replies: ${m.replies}`);
	assert.deepEqual(sock.ops[0]?.participants, [LIDSHAPE_ADMIN_LID]);
});

test('.kick mengirim addressing server (LID)', async () => {
	sock.ops.length = 0;
	const m = makeM({ command: 'kick' });
	await kick(m, { sock });
	assert.deepEqual(sock.ops[0]?.participants, [TARGET_LID], `replies: ${m.replies}`);
	assert.equal(sock.ops[0]?.action, 'remove');
});

test('.kick menolak diri sendiri walau penulisannya beda (LID vs PN)', async () => {
	sock.ops.length = 0;
	const m = makeM({ command: 'kick', sender: TARGET_PN, mentionedJid: [TARGET_LID] });
	await kick(m, { sock });
	assert.equal(sock.ops.length, 0);
	assert.match(m.replies.join(' '), /diri sendiri/i);
});
