const { chromium } = require('playwright');
const crypto = require('crypto');
const path = require('path');
const os = require('os');

const PROFILE_DIR = path.join(os.homedir(), '.whatsapp-assistant', 'profile');
const WHATSAPP_URL = 'https://web.whatsapp.com';
const LAUNCH_OPTS = {
  args: ['--no-sandbox', '--disable-blink-features=AutomationControlled'],
  userAgent: 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36',
};

function sha256(str) {
  return crypto.createHash('sha256').update(str).digest('hex');
}

function computeMessageId(contact, timestamp, body) {
  return sha256(`${contact}|${timestamp}|${body}`);
}

function output(data) {
  process.stdout.write(JSON.stringify(data) + '\n');
}

async function waitForReady(page) {
  // Wait for either QR canvas or chat interface (up to 20s)
  await page.waitForFunction(
    () => document.querySelector('canvas') !== null || document.querySelector('#side') !== null,
    { timeout: 20000 }
  ).catch(() => {});
  await page.waitForTimeout(1000); // small settle time
}

async function isLoggedIn(page) {
  const side = await page.$('#side').catch(() => null);
  if (side) return true;
  const canvas = await page.$('canvas').catch(() => null);
  // No canvas and no #side means page still loading — treat as not logged in
  return false;
}

async function getQrScreenshot(page) {
  const canvas = await page.$('canvas').catch(() => null);
  if (!canvas) return null;
  try {
    const dataUrl = await canvas.evaluate(el => el.toDataURL('image/png'));
    return dataUrl;
  } catch {
    // Fallback: screenshot the canvas bounding box
    try {
      const buf = await canvas.screenshot();
      return 'data:image/png;base64,' + buf.toString('base64');
    } catch {
      return null;
    }
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

  // Check login status headlessly first
  const headless = await chromium.launchPersistentContext(PROFILE_DIR, {
    headless: true,
    ...LAUNCH_OPTS,
  });
  const p = headless.pages()[0] || await headless.newPage();
  await p.goto(WHATSAPP_URL, { waitUntil: 'domcontentloaded', timeout: 30000 });
  await waitForReady(p);
  const loggedIn = await isLoggedIn(p);
  await headless.close();

  if (checkLoginOnly) {
    output({ logged_in: loggedIn });
    return;
  }

  if (!loggedIn) {
    // Open headed browser so user can scan QR directly from the window
    const headed = await chromium.launchPersistentContext(PROFILE_DIR, {
      headless: false,
      ...LAUNCH_OPTS,
    });
    const hp = headed.pages()[0] || await headed.newPage();
    await hp.goto(WHATSAPP_URL, { waitUntil: 'domcontentloaded', timeout: 30000 });
    await waitForReady(hp);

    // Try to get QR screenshot to show in app too
    const qrData = await getQrScreenshot(hp) || '';
    output({ status: 'qr_required', qr_data: qrData });

    // Wait up to 5 minutes for login
    const scanned = await hp.waitForFunction(
      () => document.querySelector('#side') !== null && document.querySelector('canvas') === null,
      { timeout: 5 * 60 * 1000 }
    ).then(() => true).catch(() => false);

    await headed.close();

    if (!scanned) {
      output({ status: 'error', message: 'QR scan timeout — please try again' });
      return;
    }

    // Signal success — next scheduled sync will scrape messages
    output({ status: 'ok', messages: [] });
    return;
  }

  // Logged in — scrape messages headlessly
  const browser = await chromium.launchPersistentContext(PROFILE_DIR, {
    headless: true,
    ...LAUNCH_OPTS,
  });
  const page = browser.pages()[0] || await browser.newPage();
  await page.goto(WHATSAPP_URL, { waitUntil: 'domcontentloaded', timeout: 30000 });
  await waitForReady(page);

  const messages = [];
  try {
    // Scroll chat list to load more items (WhatsApp uses virtual list)
    const chatPanel = await page.$('#pane-side, #side');
    if (chatPanel) {
      for (let i = 0; i < 5; i++) {
        await chatPanel.evaluate(el => el.scrollBy(0, 600));
        await page.waitForTimeout(300);
      }
      await chatPanel.evaluate(el => el.scrollTo(0, 0));
      await page.waitForTimeout(300);
    }

    const chatItems = await page.$$('[data-testid="cell-frame-container"], [role="listitem"]');

    // Separate unread chats from the rest — unread badge shows a number in the row
    const unreadItems = [];
    const otherItems = [];
    for (const item of chatItems) {
      const hasUnread = await item.evaluate(el => {
        const spans = [...el.querySelectorAll('span')];
        return spans.some(s => {
          const t = s.textContent.trim();
          return /^\d{1,3}$/.test(t) || t === '99+';
        });
      }).catch(() => false);
      if (hasUnread) unreadItems.push(item);
      else otherItems.push(item);
    }

    // Process unread first, then recent — up to 40 total
    const toProcess = [...unreadItems, ...otherItems].slice(0, 40);

    for (const chatItem of toProcess) {
      try {
        await chatItem.click();
        await page.waitForTimeout(600);

        const chatName = await page.evaluate(() => {
          const el = document.querySelector('header [dir="auto"], header span[title]');
          return el ? el.textContent.trim() : 'Unknown';
        }).catch(() => 'Unknown');

        const msgEls = await page.$$('[data-testid="msg-container"], [class*="message-"]');
        for (const msgEl of msgEls) {
          try {
            const body = await msgEl.$eval(
              '[data-testid="msg-text"] span, [class*="selectable-text"] span',
              el => el.textContent
            ).catch(() => null);
            if (!body) continue;

            const tsTitle = await msgEl.$eval(
              '[data-testid="msg-meta"] span[title], [class*="copyable-text"]',
              el => el.getAttribute('data-pre-plain-text') || el.getAttribute('title')
            ).catch(() => null);
            const timestamp = tsTitle
              ? Math.floor(new Date(tsTitle).getTime() / 1000)
              : Math.floor(Date.now() / 1000);

            if (timestamp < since) continue;

            const isMine = await msgEl.evaluate(el => {
              return el.classList.contains('message-out') ||
                el.closest('[class*="message-out"]') !== null;
            }).catch(() => false);
            const contact = isMine ? 'você' : chatName;

            messages.push({
              id: computeMessageId(contact, timestamp, body),
              contact,
              chat: chatName,
              body,
              timestamp,
              is_mine: isMine,
            });
          } catch { /* skip */ }
        }
      } catch { /* skip */ }
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
