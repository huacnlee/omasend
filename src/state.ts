// The app's state lives in Rust (ext/omasend's `Omasend` module); this keeps
// one signal with its latest snapshot, refreshed on every change.
import { signal } from "nsh";
import { Omasend } from "omasend/native";

export { Omasend };

// Rust's state is only reachable while components run, so the signal starts
// on first read (from App), not when this file loads.
type Snapshot = ReturnType<typeof Omasend.snapshot>;
let snapshot: { (): Snapshot; set(value: Snapshot): void } | undefined;

/** The latest snapshot; reading it subscribes. */
export function state(): Snapshot {
  if (!snapshot) {
    const current = signal(Omasend.snapshot());
    Omasend.onChange(() => current.set(Omasend.snapshot()));
    snapshot = current;
  }
  return snapshot();
}

/** The text for `key` in the interface language; reading it subscribes. */
export function t(key: string): string {
  state().language;
  return Omasend.text(key);
}

/** A byte count as people read it: 1.2 MB. */
export function sizeLabel(bytes: number): string {
  const units = ["B", "KB", "MB", "GB", "TB"];
  let value = bytes;
  let unit = 0;
  while (value >= 1000 && unit < units.length - 1) {
    value /= 1000;
    unit += 1;
  }
  return unit === 0 ? `${value} ${units[0]}` : `${value.toFixed(value < 10 ? 1 : 0)} ${units[unit]}`;
}
