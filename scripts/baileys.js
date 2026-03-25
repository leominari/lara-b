'use strict';

const {
  makeWASocket,
  useMultiFileAuthState,
  DisconnectReason,
  fetchLatestBaileysVersion,
} = require('@whiskeysockets/baileys');
const qrcode = require('qrcode');
const pino = require('pino');
const path = require('path');
const os = require('os');
const fs = require('fs');

const AUTH_DIR = path.join(os.homedir(), '.whatsapp-assistant', 'baileys-auth');

function output(data) {
  process.stdout.write(JSON.stringify(data) + '\n');
}

const groupNameCache = {};

async function connectToWhatsApp() {
  fs.mkdirSync(AUTH_DIR, { recursive: true });

  const { state, saveCreds } = await useMultiFileAuthState(AUTH_DIR);
  const { version } = await fetchLatestBaileysVersion();

  const sock = makeWASocket({
    version,
    auth: state,
    printQRInTerminal: false,
    logger: pino({ level: 'silent' }),
  });

  sock.ev.on('creds.update', saveCreds);

  sock.ev.on('connection.update', async ({ connection, lastDisconnect, qr }) => {
    if (qr) {
      try {
        const url = await qrcode.toDataURL(qr);
        output({ type: 'qr', qr_data: url });
      } catch (e) {
        output({ type: 'error', message: 'QR generation failed: ' + e.message });
      }
    }

    if (connection === 'open') {
      output({ type: 'ready' });
    }

    if (connection === 'close') {
      const statusCode = lastDisconnect?.error?.output?.statusCode;

      if (statusCode === DisconnectReason.loggedOut) {
        output({ type: 'logout' });
        process.exit(0);
      }

      if (statusCode === DisconnectReason.badSession) {
        fs.rmSync(AUTH_DIR, { recursive: true, force: true });
        output({ type: 'error', message: 'bad_session' });
        process.exit(1);
      }

      // All other reasons: reconnect
      connectToWhatsApp();
    }
  });

  sock.ev.on('messages.upsert', async ({ messages, type }) => {
    if (type !== 'notify') return;

    const formatted = [];
    for (const msg of messages) {
      try {
        if (msg.key.fromMe) continue;

        const body =
          msg.message?.conversation ||
          msg.message?.extendedTextMessage?.text ||
          '';
        if (!body) continue;

        const isGroup = msg.key.remoteJid?.endsWith('@g.us');
        const senderJid = isGroup ? msg.key.participant : msg.key.remoteJid;
        const contact = msg.pushName || (senderJid ? senderJid.split('@')[0] : 'Unknown');

        let chat = contact;
        if (isGroup) {
          const gid = msg.key.remoteJid;
          if (!groupNameCache[gid]) {
            try {
              const meta = await sock.groupMetadata(gid);
              groupNameCache[gid] = meta.subject;
            } catch {
              groupNameCache[gid] = gid.split('@')[0];
            }
          }
          chat = groupNameCache[gid];
        }

        formatted.push({
          id: msg.key.id,
          contact,
          chat,
          body,
          timestamp: Number(msg.messageTimestamp),
          is_mine: false,
        });
      } catch { /* skip malformed message */ }
    }

    if (formatted.length > 0) {
      output({ type: 'messages', messages: formatted });
    }
  });
}

connectToWhatsApp().catch(e => {
  output({ type: 'error', message: e.message });
  process.exit(1);
});
