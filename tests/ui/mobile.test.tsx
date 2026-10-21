import { expect, open, test } from "nsh/test";

test("phone menu supports sublevel navigation and keeps app styling", async () => {
  const app = await open("../..", { platform: "ios", width: 402, height: 874 });
  await app.waitFor("id:app-menu");
  await app.waitFor("id:outbox-actions");
  await app.wait(200);
  await app.screenshot("/tmp/omasend-phone.png");
  await app.press("id:app-menu");
  const chinese = app.texts().includes("语言");
  await app.waitFor(`label:${chinese ? "主题" : "Theme"}`);
  expect(app.menu()).toBe(null);
  await app.screenshot("/tmp/omasend-phone-menu.png");
  await app.press(`label:${chinese ? "主题" : "Theme"}`);
  await app.waitFor(`label:${chinese ? "返回" : "Back"}`);
  await app.screenshot("/tmp/omasend-phone-theme.png");
  const modes = chinese ? ["跟随系统", "浅色", "深色"] : ["System", "Light", "Dark"];
  const original = modes.find(mode => app.shows(`label:${mode} ✓`));
  await app.press(`label:${chinese ? "深色" : "Dark"}`);
  await app.wait(200);
  await app.screenshot("/tmp/omasend-phone-dark-home.png");
  await app.press("id:app-menu");
  await app.press(`label:${chinese ? "主题" : "Theme"}`);
  await app.waitFor(`label:${chinese ? "深色" : "Dark"} ✓`);
  await app.screenshot("/tmp/omasend-phone-dark.png");
  if (original) {
    await app.press(`label:${original === (chinese ? "深色" : "Dark") ? original + " ✓" : original}`);
    await app.press("id:app-menu");
    await app.press(`label:${chinese ? "主题" : "Theme"}`);
  }
  await app.press(`label:${chinese ? "返回" : "Back"}`);
  await app.waitFor(`label:${chinese ? "语言" : "Language"}`);
  await app.press(`label:${chinese ? "关于…" : "About…"}`);
  await app.waitFor(`label:${chinese ? "关闭" : "Close"}`);
  await app.press(`label:${chinese ? "关闭" : "Close"}`);
  await app.waitFor("id:outbox-actions");
  await app.press("id:app-menu");
  await app.press(`label:${chinese ? "传输记录" : "Transfer history"}`);
  await app.waitFor("id:history-sheet");
  await app.press(`label:${chinese ? "关闭" : "Close"}`);
  await app.waitFor("id:history-sheet", { state: "gone" });
  await app.press("id:add-files-button");
  await app.waitFor("id:add-files-menu");
  const photos = chinese ? "照片与视频" : "Photos and videos";
  const files = chinese ? "浏览文件" : "Browse files";
  await app.waitFor(`label:${photos}`);
  await app.waitFor(`label:${files}`);
  expect(app.menu()).toBe(null);
  expect(app.texts().indexOf(photos) < app.texts().indexOf(files)).toBe(true);
  await app.screenshot("/tmp/omasend-add-files-menu.png");
  // Tap the header outside the source menu to dismiss it.
  await app.press("id:app-menu");
  await app.waitFor("id:add-files-menu", { state: "gone" });
});
