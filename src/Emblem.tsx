// Omasend's mark. The large one in the empty Outbox animates: its eight outer
// pieces light up in turn, clockwise, while someone can see it — as the Rust
// version did. The title bar's stays still.
import { View, cleanup, effect, lifecycle, reduceMotion, signal } from "nsh";

type Rect = [number, number, number, number];
// logo.svg's pieces in its 32-unit grid: four corners, four bars, the center.
const PIECES: Rect[][] = [
  [[4, 2, 6, 4], [2, 6, 4, 4]],
  [[22, 2, 6, 4], [26, 6, 4, 4]],
  [[2, 22, 4, 4], [4, 26, 6, 4]],
  [[26, 22, 4, 4], [22, 26, 6, 4]],
  [[12, 4, 8, 2]],
  [[26, 12, 2, 8]],
  [[12, 26, 8, 2]],
  [[4, 12, 2, 8]],
  [[10, 10, 12, 12]],
];
// Top left, top, top right, right, bottom right, bottom, bottom left, left.
const ORDER = [0, 4, 1, 5, 3, 6, 2, 7];

export function Emblem(props: { size: number; animated?: boolean; paused?: () => boolean }) {
  const unit = props.size / 32;
  const levels = PIECES.map(() => signal(1));
  if (props.animated) {
    effect(() => {
      const running = !props.paused?.() && !reduceMotion() && lifecycle() === "active";
      levels.forEach((level, index) => level.set(running && index !== 8 ? 0.6 : 1));
      if (!running) return;
      const timers: number[] = [];
      const burst = () =>
        ORDER.forEach((index, turn) => {
          timers.push(
            setTimeout(() => {
              levels[index].set(1);
              timers.push(setTimeout(() => levels[index].set(0.6), 135));
            }, turn * 55),
          );
        });
      burst();
      const clock = setInterval(burst, 440);
      cleanup(() => {
        clearInterval(clock);
        timers.forEach(clearTimeout);
      });
    });
  }
  return (
    <View testID="emblem" class="relative" style={{ width: props.size, height: props.size }}>
      {PIECES.map((rects, index) => (
        <View class="absolute inset-0" style={{ opacity: levels[index](), transitionProperty: "opacity", transitionDuration: 90, transitionTimingFunction: "linear" }}>
          {rects.map(([x, y, w, h]) => (
            <View
              class="absolute bg-primary"
              style={{ left: x * unit, top: y * unit, width: w * unit, height: h * unit }}
            />
          ))}
        </View>
      ))}
    </View>
  );
}
