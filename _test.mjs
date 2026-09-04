import pino from 'pino';
import * as m from 'ourin';

const logger = pino({ level: 'silent' });
const sock = m.default({
  auth: { creds: { me: { id: 'test@s.whatsapp.net' } }, keys: {} },
  logger,
});

console.log('waUploadToServer:', typeof sock.waUploadToServer);
console.log('refreshMediaConn:', typeof sock.refreshMediaConn);
console.log('groupMetadata:', typeof sock.groupMetadata);
console.log('relayMessage:', typeof sock.relayMessage);
console.log('sendMessage:', typeof sock.sendMessage);

// Test interactiveMessage encoding
try {
  const { generateWAMessageFromContent } = m;
  const result = generateWAMessageFromContent('123@s.whatsapp.net', {
    viewOnceMessage: {
      message: {
        messageContextInfo: {},
        interactiveMessage: {
          header: { title: 'Test', subtitle: '', hasMediaAttachment: false },
          body: { text: 'Hello test' },
          nativeFlowMessage: {
            buttons: [{ name: 'quick_reply', buttonParamsJson: JSON.stringify({ display_text: 'OK', id: 'ok' }) }]
          }
        }
      }
    }
  }, { userJid: 'test@s.whatsapp.net' });
  console.log('generateWAMessageFromContent OK:', !!result);
  console.log('Message type:', Object.keys(result.message || {}));
} catch(e) {
  console.error('generateWAMessageFromContent FAIL:', e.message);
}

// Test prepareWAMessageMedia without upload (just check if it loads)
try {
  const { prepareWAMessageMedia } = m;
  const media = await prepareWAMessageMedia({ image: Buffer.from([0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]) }, {});
  console.log('prepareWAMessageMedia no-upload OK:', !!media);
} catch(e) {
  console.error('prepareWAMessageMedia no-upload FAIL:', e.message);
}