// The TSX window over the Rust LocalSend session (`omasend/native`).
import { expect, open, test } from "nsh/test";

test("opens with an empty Outbox and nothing to send", async () => {
  const app = await open("./App.tsx");
  await app.waitFor("Outbox");
  expect(app.texts()).toContain("Nothing added yet");
});

test("the menu offers the theme modes", async () => {
  const app = await open("./App.tsx");
  await app.press("label:Menu");
  for (const mode of ["System", "Light", "Dark"]) {
    await app.waitFor(mode);
  }
});
