import test from 'node:test';
import assert from 'node:assert/strict';
import { adminFlagsFor } from '../src/lib/ourin-serialize.js';

const BOT_PN = '628999888777:12@s.whatsapp.net';
const BOT_LID = '9900011122233344@lid';
const botJids = [BOT_PN, BOT_LID];

// Bentuk participant ourin per addressing_mode (node_modules/ourin/lib/Socket/groups.js:338)
const lidAddrAdmin = { id: '9911122233344455@lid', phoneNumber: '6281234567890@s.whatsapp.net', admin: 'admin' };
const pnAddrAdmin = { id: '6281234567890@s.whatsapp.net', lid: '9911122233344455@lid', admin: 'admin' };
const member = { id: '6285551112223@s.whatsapp.net', lid: '99777888999000111@lid', admin: undefined };

test('admin terdeteksi dari LID maupun PN (grup addressing_mode=lid)', () => {
	const participants = [member, lidAddrAdmin];
	assert.equal(adminFlagsFor(participants, '9911122233344455@lid', botJids).isAdmin, true, 'sender LID');
	assert.equal(adminFlagsFor(participants, '6281234567890@s.whatsapp.net', botJids).isAdmin, true, 'sender PN');
	assert.equal(adminFlagsFor([member, pnAddrAdmin], '9911122233344455@lid', botJids).isAdmin, true, 'bentuk PN+lid');
});

test('bukan admin tetap ditolak', () => {
	const participants = [member, lidAddrAdmin];
	assert.equal(adminFlagsFor(participants, '6285551112223@s.whatsapp.net', botJids).isAdmin, false);
	assert.equal(adminFlagsFor(participants, '6287777777777@s.whatsapp.net', botJids).isAdmin, false);
});

test('bot admin dikenali walau di-list sebagai LID tanpa phoneNumber', () => {
	const participants = [{ id: BOT_LID, admin: 'admin' }, member];
	assert.equal(adminFlagsFor(participants, member.id, botJids).isBotAdmin, true);
	const pnOnly = [{ id: '628999888777@s.whatsapp.net', lid: BOT_LID, admin: 'superadmin' }];
	assert.equal(adminFlagsFor(pnOnly, member.id, botJids).isBotAdmin, true);
	assert.equal(adminFlagsFor([member], member.id, botJids).isBotAdmin, false);
});

test('admin baru langsung dikenali (tidak ada TTL cache lama 5 menit)', () => {
	const before = [member];
	assert.equal(adminFlagsFor(before, '6281111000111@s.whatsapp.net', botJids).isAdmin, false);
	const after = [...before, { id: '6281111000111@s.whatsapp.net', lid: '9933344455566677@lid', admin: 'admin' }];
	assert.equal(adminFlagsFor(after, '6281111000111@s.whatsapp.net', botJids).isAdmin, true);
	assert.equal(adminFlagsFor(after, '9933344455566677@lid', botJids).isAdmin, true);
});
