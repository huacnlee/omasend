// The TSX window over the Rust LocalSend session (`omasend/native`).
import { expect, open, test } from "nsh/test";

test("opens with an empty Outbox and nothing to send", async () => {
  const app = await open("./App.tsx");
  await app.waitFor("Outbox");
  expect(app.texts()).toContain("Nothing added yet");
});

test("the menu offers the languages and Exit", async () => {
  const app = await open("./App.tsx");
  await app.press("label:Menu");
  // Theme modes only show outside Omarchy, which follows its own theme.
  for (const item of ["English", "简体中文", "Exit"]) {
    await app.waitFor(item);
  }
});
