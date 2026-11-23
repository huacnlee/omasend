import { expect, open, test } from "nsh/test";

test("the themed Menu anchors to its button, selects submenus and opens Dialog", async () => {
  const app = await open("../..", { platform: "macos", width: 840, height: 630 });
  await app.waitFor("id:app-menu");
  const item = (text: string) => app.shows(`label:${text} ✓`) ? `${text} ✓` : text;
  const pressItem = async (text: string) => { await app.press(`label:${item(text)}`); };
  const openMenu = async () => { await app.press("id:app-menu"); };
  let originalLanguage = "English";
  let originalTheme = "System";
  let haveTheme = false;
  await openMenu();
  try {
    // Menu is an NSH overlay with the application's own styles.
    expect(app.menu()).toBe(null);
    await pressItem(app.texts().includes("Language") ? "Language" : "语言");
    await app.waitFor("id:app-submenu-popup");
    originalLanguage = app.shows("label:简体中文 ✓") ? "简体中文" : "English";
    await pressItem("English");
    await openMenu();
    await app.wait(150);
    await app.screenshot("/tmp/omasend-themed-menu.png");
    await pressItem("Theme");
    await app.waitFor("id:app-submenu-popup");
    await app.wait(150);
    await app.screenshot("/tmp/omasend-themed-submenu.png");
    originalTheme = ["System", "Light", "Dark"].find(mode => app.shows(`label:${mode} ✓`))!;
    haveTheme = true;
    await pressItem("Light");
    await openMenu();
    await pressItem("Theme");
    await app.waitFor("label:Light ✓");
    await pressItem("Dark");
    await app.setViewport(640, 520);
    await openMenu();
    await pressItem("Theme");
    await app.waitFor("label:Dark ✓");
    await app.wait(150);
    await app.screenshot("/tmp/omasend-themed-submenu-small.png");
    await pressItem("Dark");
    await openMenu();
    await pressItem("About…");
    await app.waitFor("label:Close");
    await app.press("label:Close");
    if (app.shows("id:error-notice")) {
      await app.screenshot("/tmp/omasend-error-notice.png");
      await app.press("label:Dismiss error");
      await app.waitFor("id:error-notice", { state: "gone" });
    }
    await app.press("id:history-button");
    await app.waitFor("id:history-sheet");
    await app.screenshot("/tmp/omasend-history-sheet.png");
    await app.keyboard.press("escape");
    await app.waitFor("id:history-sheet", { state: "gone" });
    await app.press("id:history-button");
    await app.waitFor("id:history-sheet");
    await app.press("label:Close");
    await app.waitFor("id:history-sheet", { state: "gone" });
    await openMenu();
    await pressItem("Language");
    await app.waitFor("id:app-submenu-popup");
    await pressItem("简体中文");
    await openMenu();
    await pressItem("语言");
    await app.waitFor("label:简体中文 ✓");
    await pressItem("English");
  } finally {
    if (app.texts().includes("Close")) await app.press("label:Close");
    if (!app.texts().includes("Theme")) await openMenu();
    if (haveTheme) {
      await pressItem("Theme");
      await app.waitFor("id:app-submenu-popup");
      await pressItem(originalTheme);
      await openMenu();
    }
    await pressItem("Language");
    await app.waitFor("id:app-submenu-popup");
    await pressItem(originalLanguage);
  }
});
