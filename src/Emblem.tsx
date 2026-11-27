// Omasend's mark. The large one in the empty Outbox animates: its eight outer
// pieces light up in turn, clockwise, while someone can see it — as the Rust
// version did. The title bar's stays still.
import { Canvas, View, cleanup, effect, lifecycle, reduceMotion, signal, token } from "nsh";

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
  // Each canvas fills the box the layout gives the mark, not `size`: a short
  // terminal shrinks it, and a fixed canvas would draw over the text below.
  return (
    <View testID="emblem" class="relative" style={{ width: props.size, height: props.size }}>
      {PIECES.map((rects, index) => (
        // Fade a plain wrapper, not the canvas (as the Rust version did): a
        // canvas drawn through an animated value redraws every frame. A
        // canvas, not views, so the terminal draws the mark in braille.
        <View class="absolute inset-0" style={{ opacity: levels[index](), transitionProperty: "opacity", transitionDuration: 90, transitionTimingFunction: "linear" }}>
          <Canvas
            class="w-full h-full"
            draw={(cx, { width, height }) => {
              const side = Math.min(width, height);
              const unit = side / 32;
              const left = (width - side) / 2;
              const top = (height - side) / 2;
              cx.fillStyle = token("primary" as never) as string;
              for (const [x, y, w, h] of rects) cx.fillRect(left + x * unit, top + y * unit, w * unit, h * unit);
            }}
          />
        </View>
      ))}
    </View>
  );
}
