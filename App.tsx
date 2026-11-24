// Omasend's window. The LocalSend client and its state are Rust in ext/omasend
// (an nshm of the app's own, its `Omasend` module); the interface is TSX.
import { Text, View, effect, mount, platform } from "nsh";
import { IconButton, TitleBar, Toaster } from "./src/ui";
import { Active, History, Incoming, announceFinished, historyOpen } from "./src/Activity";
import { Composer } from "./src/Composer";
import { Nearby } from "./src/Nearby";
import { AppMenu, Surfaces, UpdateStatus, applySavedTheme } from "./src/Settings";
import { state, t } from "./src/state";

function App() {
  applySavedTheme();
  // Phones stack the device list above the Outbox; wider screens put them side by side.
  const phone = platform.idiom === "phone";
  effect(announceFinished);
  return (
    <View class="flex-1 bg-background">
      <TitleBar title="Omasend">
        <Text class="text-xs text-muted-foreground">
          {state().identity ? `${t("nearby.you")}: ${state().identity!.alias}` : t("status.starting")}
        </Text>
        <IconButton icon="inbox" label={t("history.title")} size="small" onPress={() => historyOpen.set(true)} />
        <UpdateStatus />
        <AppMenu />
      </TitleBar>
      <View class={phone ? "flex-1 gap-4 p-3" : "flex-1 flex-row gap-6 p-4"}>
        <View class={phone ? "max-h-64" : "w-72"}>
          <Nearby />
        </View>
        <View class="flex-1 gap-6">
          <Active />
          <Composer />
        </View>
      </View>
      <Incoming />
      <History />
      <Surfaces />
      <Toaster />
    </View>
  );
}
mount(App);
