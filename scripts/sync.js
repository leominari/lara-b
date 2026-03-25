const { chromium } = require('playwright');
const crypto = require('crypto');
const path = require('path');
const os = require('os');

const PROFILE_DIR = path.join(os.homedir(), '.whatsapp-assistant', 'profile');
const WHATSAPP_URL = 'https://web.whatsapp.com';

function sha256(str) {
  return crypto.createHash('sha256').update(str).digest('hex');
}

function computeMessageId(contact, timestamp, body) {
  return sha256(`${contact}|${timestamp}|${body}`);
}

function output(data) {
  process.stdout.write(JSON.stringify(data) + '\n');
}

async function isLoggedIn(page) {
  try {
    await page.waitForSelector('[data-testid="default-user"]', { timeout: 8000 });
    return true;
  } catch {
    return false;
  }
}

async function getQrData(page) {
  try {
    const qrEl = await page.waitForSelector('canvas[aria-label="Scan this QR code to link a device"]', { timeout: 10000 });
    const dataUrl = await qrEl.evaluate(el => el.toDataURL());
    return dataUrl;
  } catch {
    return null;
  }
}

async function main() {
  const args = process.argv.slice(2);
  const checkLoginOnly = args.includes('--check-login-only');
  const sinceIndex = args.indexOf('--since');
  const since = sinceIndex !== -1 ? parseInt(args[sinceIndex + 1], 10) : null;

  if (!checkLoginOnly && (!since || since <= 0)) {
    output({ status: 'error', message: '--since argument is required and must be a positive integer' });
    process.exit(1);
  }

  const browser = await chromium.launchPersistentContext(PROFILE_DIR, {
    headless: true,
    args: ['--no-sandbox'],
  });

  const page = browser.pages()[0] || await browser.newPage();
  await page.goto(WHATSAPP_URL, { waitUntil: 'domcontentloaded' });

  if (checkLoginOnly) {
    const loggedIn = await isLoggedIn(page);
    await browser.close();
    output({ logged_in: loggedIn });
    return;
  }

  const loggedIn = await isLoggedIn(page);
  if (!loggedIn) {
    const qrData = await getQrData(page);
    await browser.close();
    output({ status: 'qr_required', qr_data: qrData || '' });
    return;
  }

  // Collect messages newer than `since`
  const messages = [];
  try {
    // Get all chat list items
    const chatItems = await page.$$('[data-testid="cell-frame-container"]');

    for (const chatItem of chatItems.slice(0, 20)) { // limit to 20 chats for performance
      try {
        await chatItem.click();
        await page.waitForTimeout(500);

        const chatName = await page.$eval('[data-testid="conversation-header"] span[dir="auto"]', el => el.textContent).catch(() => 'Unknown');

        const msgEls = await page.$$('[data-testid="msg-container"]');
        for (const msgEl of msgEls) {
          try {
            const body = await msgEl.$eval('[data-testid="msg-text"] span', el => el.textContent).catch(() => null);
            if (!body) continue;

            const tsEl = await msgEl.$('[data-testid="msg-meta"] span[title]');
            const tsTitle = tsEl ? await tsEl.getAttribute('title') : null;
            const timestamp = tsTitle ? Math.floor(new Date(tsTitle).getTime() / 1000) : Math.floor(Date.now() / 1000);

            if (timestamp < since) continue;

            const isMine = await msgEl.evaluate(el => el.classList.contains('message-out'));
            const contact = isMine ? 'você' : chatName;

            messages.push({
              id: computeMessageId(contact, timestamp, body),
              contact,
              chat: chatName,
              body,
              timestamp,
              is_mine: isMine,
            });
          } catch { /* skip individual message errors */ }
        }
      } catch { /* skip individual chat errors */ }
    }
  } catch (e) {
    await browser.close();
    output({ status: 'error', message: e.message });
    return;
  }

  await browser.close();
  output({ status: 'ok', messages });
}

main().catch(e => {
  output({ status: 'error', message: e.message });
  process.exit(1);
});
