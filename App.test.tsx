// The TSX window over the Rust LocalSend session (`omasend/native`).
import { expect, open, test } from "nsh/test";

test("opens with an empty Outbox and nothing to send", async () => {
  const app = await open("./App.tsx");
  await app.waitFor("Outbox");
  expect(app.texts()).toContain("Files, folders, images or a few words.");
});

test("the menu offers the languages and Exit", async () => {
  const app = await open("./App.tsx");
  await app.press("label:Menu");
  // Theme modes only show outside Omarchy, which follows its own theme.
  for (const item of ["English", "简体中文", "Exit"]) {
    await app.waitFor(item);
  }
});

// Omasend's keys, with the native module standing in: two devices, one item.
test("arrows choose a device and Return sends", async () => {
  const calls: string[] = [];
  const device = (id: string, alias: string) => ({ id, alias, model: "", host: "" });
  const app = await open("./App.tsx", {
    modules: {
      "omasend/native": {
        "Omasend.snapshot": {
          quit: false,
          identity: device("me", "This computer"),
          running: true,
          discovering: false,
          devices: [device("a", "Laptop"), device("b", "Phone")],
          selected: "a",
          composer: [{ id: "i", kind: "text", name: "Hi", size: 2, path: "", transfer: "", text: "Hi" }],
          transfers: [],
          incoming: null,
          error: "",
          canSend: true,
          language: "en",
          update: "",
          updateActionable: false,
          updateBusy: false,
          diagnosing: false,
          diagnosisStep: "",
          diagnosis: "",
        },
        "Omasend.navigate": (delta: number) => void calls.push(`navigate ${delta}`),
        "Omasend.send": () => void calls.push("send"),
      },
    },
  });
  await app.waitFor("Laptop");
  await app.keyboard.press("right");
  await app.keyboard.press("left");
  await app.keyboard.press("enter");
  expect(calls).toEqual(["navigate 1", "navigate -1", "send"]);
});
