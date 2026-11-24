// The devices LocalSend finds on this network: a strip of cards across the
// window, one row until expanded, as Omasend has always drawn it.
import { Text, View, signal } from "nsh";
import { Button, Icon } from "./ui";
import { Omasend, state, t } from "./state";

const CARD = 240;
const GAP = 8;
const expanded = signal(false);
const width = signal(0);

function columns() {
  return Math.max(1, Math.floor((width() + GAP) / (CARD + GAP)));
}

/** The cards that show: the first row, with a selected card past it taking the last slot. */
function visible() {
  const devices = state().devices;
  if (expanded()) return devices;
  const shown = devices.slice(0, columns());
  const index = devices.findIndex((d) => d.id === state().selected);
  if (index >= shown.length && shown.length > 0) shown[shown.length - 1] = devices[index];
  return shown;
}

function initials(name: string) {
  const words = name.trim().split(/\s+/);
  const letters = words.length > 1 ? words[0].slice(0, 1) + words[1].slice(0, 1) : (words[0] || "?").slice(0, 2);
  return letters.toUpperCase();
}

function DeviceCard(props: { device: () => ReturnType<typeof state>["devices"][number] }) {
  const device = () => props.device();
  const selected = () => state().selected === device().id;
  return (
    <View
      testID={`device-${device().id}`}
      role="button"
      accessibilityLabel={device().alias}
      class={`flex-row items-center gap-3 p-3 h-[58px] border ${
        selected() ? "border-primary bg-foreground/5" : "border-border"
      } hover:bg-foreground/10`}
      style={{ width: Math.max(160, (width() - GAP * (columns() - 1)) / columns()) }}
      onPress={() => Omasend.select(device().id)}
    >
      <View class="size-8 items-center justify-center border border-green-500/45 bg-green-500/10">
        <Text class="text-green-500">{initials(device().alias)}</Text>
      </View>
      <View class="flex-1">
        <Text class="text-foreground truncate">{device().alias}</Text>
        <Text class="text-[11px] text-muted-foreground truncate">{device().model || "LocalSend"}</Text>
      </View>
      <View class="size-3 shrink-0">{selected() ? <Icon name="check" size={12} class="text-primary" /> : null}</View>
    </View>
  );
}

export function Nearby() {
  return (
    <View testID="nearby-panel" class="gap-2 px-4 py-3 border-b border-border">
      <View class="flex-row items-center gap-2">
        <Text class="text-foreground">{t("nearby.title")}</Text>
        <Text class="text-muted-foreground">{String(state().devices.length)}</Text>
        <View class="flex-1" />
        {state().devices.length > columns() ? (
          <Button
            testID="toggle-nearby"
            label={t(expanded() ? "action.collapse" : "action.expand")}
            size="small"
            variant="ghost"
            onPress={() => expanded.set(!expanded())}
          />
        ) : null}
      </View>
      {state().devices.length === 0 ? <Text class="text-muted-foreground">{t("nearby.looking")}</Text> : null}
      <View class="flex-row flex-wrap gap-2" onLayout={(e) => width.set(e.bounds.size.width)}>
        <Each of={visible()} key={(d: any) => d.id}>
          {(device: any) => <DeviceCard device={device} />}
        </Each>
      </View>
    </View>
  );
}
