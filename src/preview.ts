// What the preview dialog shows: an item to send (an image, or a text in
// full) or a received image.
import { signal } from "nsh";
import type { state } from "./state";

type Item = ReturnType<typeof state>["composer"][number];

export const preview = signal<Item | null>(null);
