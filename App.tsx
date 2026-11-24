// Omasend's window. The LocalSend client and its state are Rust in ext/omasend
// (an nshm of the app's own, its `Omasend` module); the interface is TSX.
import { Text, View, effect } from "nsh";
import { IconButton, Omarchy, TitleBar, Toaster } from "./src/ui";
import { Active, History, Incoming, announceFinished, historyOpen } from "./src/Activity";
import { Composer } from "./src/Composer";
import { Nearby } from "./src/Nearby";
import { AppMenu, Surfaces, UpdateStatus } from "./src/Settings";
import { state, t } from "./src/state";

export default function App() {
  Omarchy.followSystem();
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
      <View class="flex-1 flex-row gap-6 p-4">
        <View class="w-72">
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
