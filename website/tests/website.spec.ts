import { expect, test } from '@playwright/test';

test('English and Chinese routes keep correct links, language and metadata', async ({ page }) => {
  const errors: string[] = [];
  page.on('pageerror', error => errors.push(error.message));
  await page.goto('./');
  await expect(page.locator('h1')).toHaveText('Copy. Paste. Send.');
  await expect(page.locator('link[rel="canonical"]')).toHaveAttribute('href', 'https://huacnlee.github.io/omasend/');
  await expect(page.locator('.hero-logo svg')).toHaveCount(9);
  await expect(page.locator('link[rel="icon"]')).toHaveAttribute('href', '/omasend/favicon.svg');
  const favicon = await page.request.get('/omasend/favicon.svg');
  expect(favicon.ok()).toBe(true);
  expect(favicon.headers()['content-type']).toContain('image/svg+xml');
  const markup = await favicon.text();
  for (const path of await page.locator('.brand-mark path').evaluateAll(paths => paths.map(path => path.getAttribute('d')))) {
    expect(markup).toContain(`d="${path}"`);
  }
  expect(markup).not.toContain('<rect');

  await page.locator('#language-toggle').click();
  await page.getByRole('menuitemradio', { name: '简体中文' }).click();
  await expect(page.locator('html')).toHaveAttribute('lang', 'zh-CN');
  await expect(page.locator('h1')).toHaveText('复制粘贴发送');
  await expect(page.locator('link[rel="canonical"]')).toHaveAttribute('href', 'https://huacnlee.github.io/omasend/zh-CN/');
  await expect(page.locator('#download')).toHaveCount(0);
  await expect(page.getByRole('link', { name: 'GitHub', exact: true })).toHaveAttribute('href', 'https://github.com/huacnlee/omasend');
  await expect(page.locator('#command-linux')).toContainText('/main/install.sh | sh');
  await expect(page.locator('#command-windows')).toContainText('/main/install.ps1 | iex');
  expect(errors).toEqual([]);
});

test('light and dark toggle follows system then persists across reload and language navigation', async ({ page }) => {
  await page.emulateMedia({ colorScheme: 'light' });
  await page.goto('./');
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'flexoki-light');
  await expect(page.getByRole('button', { name: 'Dark mode' })).toHaveAttribute('aria-pressed', 'false');
  await page.screenshot({ path: '/tmp/omasend-header-light.png' });
  await page.emulateMedia({ colorScheme: 'dark' });
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'tokyo-night');
  await page.screenshot({ path: '/tmp/omasend-header-dark.png' });
  await page.getByRole('button', { name: 'Dark mode' }).click();
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'flexoki-light');
  await page.reload();
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'flexoki-light');
  await page.locator('#language-toggle').click();
  await page.getByRole('menuitemradio', { name: '简体中文' }).click();
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'flexoki-light');
  await page.getByRole('button', { name: '深色模式' }).click();
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'tokyo-night');
});

test('language menu handles keyboard selection, escape and outside click', async ({ page }) => {
  await page.goto('./');
  await page.getByRole('button', { name: 'Language' }).focus();
  await page.keyboard.press('ArrowDown');
  await expect(page.getByRole('menuitemradio', { name: 'English' })).toBeFocused();
  await expect(page.getByRole('menuitemradio', { name: 'English' })).toHaveAttribute('aria-checked', 'true');
  await page.keyboard.press('End');
  await expect(page.getByRole('menuitemradio', { name: '简体中文' })).toBeFocused();
  await page.keyboard.press('Escape');
  await expect(page.getByRole('button', { name: 'Language' })).toBeFocused();
  await expect(page.getByRole('menu')).toHaveCount(0);
  await page.getByRole('button', { name: 'Language' }).click();
  await page.locator('h1').click();
  await expect(page.getByRole('menu')).toHaveCount(0);
  await expect(page.locator('.brand-mark')).toBeVisible();
  await expect(page.locator('.brand img')).toHaveCount(0);
});

test('tabs switch with mouse and keyboard and copy only the active command', async ({ page, context }) => {
  await context.grantPermissions(['clipboard-read', 'clipboard-write']);
  await page.goto('./');
  const linux = page.getByRole('tab', { name: 'Linux', exact: true });
  await expect(linux).toHaveAttribute('aria-selected', 'true');
  await expect(page.getByRole('tabpanel')).toHaveCount(1);
  await linux.focus();
  await page.keyboard.press('ArrowRight');
  await expect(page.getByRole('tab', { name: 'Windows', exact: true })).toBeFocused();
  await expect(page.getByRole('tabpanel')).toHaveAttribute('id', 'panel-windows');
  await page.getByRole('button', { name: 'Copy', exact: true }).click();
  await expect(page.getByRole('status')).toHaveText('Copied');
  expect(await page.evaluate(() => navigator.clipboard.readText())).toBe('irm https://github.com/huacnlee/omasend/raw/refs/heads/main/install.ps1 | iex');
  await page.getByRole('tab', { name: 'Windows', exact: true }).focus();
  await page.keyboard.press('Home');
  await expect(page.getByRole('tab', { name: 'macOS', exact: true })).toBeFocused();
  await page.keyboard.press('End');
  await expect(page.getByRole('tab', { name: 'Windows', exact: true })).toBeFocused();
  await page.keyboard.press('ArrowRight');
  await expect(page.getByRole('tab', { name: 'macOS', exact: true })).toBeFocused();
  await page.keyboard.press('ArrowLeft');
  await expect(page.getByRole('tab', { name: 'Windows', exact: true })).toBeFocused();
  await linux.click();
  await page.getByRole('button', { name: 'Copy', exact: true }).click();
  expect(await page.evaluate(() => navigator.clipboard.readText())).toBe('curl -fsSL https://github.com/huacnlee/omasend/raw/refs/heads/main/install.sh | sh');
});

for (const [width, height] of [[360, 640], [1440, 900]]) {
  test(`installation follows hero in first viewport at ${width}px`, async ({ page }) => {
    await page.setViewportSize({ width, height });
    for (const route of ['./', './zh-CN/']) {
      await page.goto(route);
      expect(await page.locator('.hero').evaluate(element => element.nextElementSibling?.id)).toBe('install');
      const bounds = await page.locator('#command-linux').boundingBox();
      expect(bounds).not.toBeNull();
      expect(bounds!.y + bounds!.height).toBeLessThanOrEqual(height);
      await page.screenshot({ path: `/tmp/omasend-install-${width}-${route.includes('zh') ? 'zh' : 'en'}.png` });
    }
  });
}

for (const width of [360, 768, 1440]) {
  test(`layout fits ${width}px and reduced motion retains content`, async ({ page }) => {
    await page.setViewportSize({ width, height: 1000 });
    await page.emulateMedia({ reducedMotion: 'reduce' });
    await page.goto('./');
    expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(width);
    await expect(page.locator('.packet').first()).toHaveCSS('display', 'none');
    await expect(page.locator('h1')).toBeVisible();
    await page.locator('#language-toggle').click();
  await page.getByRole('menuitemradio', { name: '简体中文' }).click();
    expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(width);
  });
}

test('keyboard reaches skip link and installer action', async ({ page }) => {
  await page.goto('./');
  await page.keyboard.press('Tab');
  await expect(page.getByRole('link', { name: 'Skip to content' })).toBeFocused();
  await page.keyboard.press('Enter');
  await page.keyboard.press('Tab');
  await expect(page.getByRole('link', { name: 'See how it works' })).toBeFocused();
  await page.keyboard.press('Enter');
  await expect(page).toHaveURL(/#workflow$/);
});

test('logo chases eight outer segments around a fixed center and packets travel quickly', async ({ page }) => {
  await page.goto('./');
  await expect(page.locator('.logo-segment:not(.logo-center)')).toHaveCount(8);
  await expect(page.locator('.logo-center')).toHaveCSS('animation-name', 'none');
  await expect(page.locator('.logo-segment').first()).toHaveCSS('animation-name', 'segment-chase');
  await expect(page.locator('.logo-segment').first()).toHaveCSS('animation-duration', '0.96s');
  await expect(page.locator('.logo-segment').nth(1)).toHaveCSS('animation-delay', '-0.84s');
  await page.locator('.hero-logo').evaluate(element => {
    element.getAnimations({ subtree: true }).forEach(animation => { animation.pause(); animation.currentTime = 0; });
  });
  await expect(page.locator('.logo-segment').first()).toHaveCSS('opacity', '1');
  const trailOpacity = await page.locator('.logo-segment').nth(7).evaluate(el => Number(getComputedStyle(el).opacity));
  expect(trailOpacity).toBeGreaterThan(0.75);
  expect(trailOpacity).toBeLessThan(0.85);
  await expect(page.locator('.logo-center')).toHaveCSS('transform', 'none');
  await page.screenshot({ path: '/tmp/omasend-logo-chase.png' });

  await expect(page.locator('.transfer-route')).toHaveCount(3);
  for (const motion of await page.locator('.packet animateMotion').all()) {
    await expect(motion).toHaveAttribute('dur', '0.8s');
    await expect(motion).toHaveAttribute('repeatCount', 'indefinite');
  }
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await expect(page.locator('.logo-segment').first()).toHaveCSS('animation-name', 'none');
  await expect(page.locator('.logo-segment').first()).toHaveCSS('opacity', '1');
});

test('Chinese headings have no trailing full stop', async ({ page }) => {
  await page.goto('./zh-CN/');
  for (const text of await page.locator('h1, h2, h3, .eyebrow').allTextContents()) {
    expect(text.trim()).not.toMatch(/。/);
  }
});
