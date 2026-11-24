// Omasend's window. The LocalSend client and its state are Rust in ext/omasend
// (an nshm of the app's own, its `Omasend` module); the interface is TSX.
import { Text, View } from "nsh";
import { Omarchy, TitleBar, Toaster } from "./src/ui";
import { Composer } from "./src/Composer";
import { Nearby } from "./src/Nearby";
import { state, t } from "./src/state";

export default function App() {
  Omarchy.followSystem();
  return (
    <View class="flex-1 bg-background">
      <TitleBar title="Omasend">
        <Text class="text-xs text-muted-foreground">
          {state().identity ? `${t("nearby.you")}: ${state().identity!.alias}` : t("status.starting")}
        </Text>
      </TitleBar>
      <View class="flex-1 flex-row gap-6 p-4">
        <View class="w-72">
          <Nearby />
        </View>
        <Composer />
      </View>
      <Toaster />
    </View>
  );
}
