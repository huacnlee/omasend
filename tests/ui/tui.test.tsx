// Run with: nsh test --target tui tests/ui/tui.test.tsx
import { expect, open, test } from "nsh/test";

for (const [columns, rows] of [[80, 24], [60, 20]]) {
  test(`terminal toolbar and menu fit ${columns}x${rows}`, async () => {
    const app = await open("../..", { width: columns * 8, height: rows * 16 });
    await app.waitFor("label:Menu");
    expect(app.texts().includes("Transfer history")).toBe(true);
    await app.press("id:app-menu");
    await app.waitFor("label:Language");
    await app.press("label:Language");
    await app.waitFor("id:app-submenu-popup");
    expect(app.texts().includes("English")).toBe(true);
  });
}
