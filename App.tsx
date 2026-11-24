// Omasend's window. The LocalSend client and its state are Rust in ext/omasend
// (an nshm of the app's own, its `Omasend` module); the interface is TSX.
// The layout is Omasend's own: nearby devices across the top, the Outbox below
// and a status bar at the bottom.
import { Text, View, cleanup, effect, keymap, mount } from "nsh";
import { TERMINAL, ToolButton, TitleBar, Toaster } from "./src/ui";
import { History, Incoming, announceFinished, historyOpen } from "./src/Activity";
import { Composer, Preview } from "./src/Composer";
import { preview } from "./src/preview";
import { Emblem } from "./src/Emblem";
import { Nearby } from "./src/Nearby";
import { AppMenu, Surfaces, UpdateStatus, applySavedTheme, surfaceOpen } from "./src/Settings";
import { Omasend, state, t } from "./src/state";

function ErrorNotice() {
  const raw = () => state().error;
  const port = () => raw().includes("Cannot listen for LocalSend transfers");
  return (
    <If when={raw()}>
    <View testID="error-notice" role="alert" class="mx-4 mt-3 p-3 flex-row items-center gap-3 border border-border bg-foreground/5">
      <View class="size-5 items-center justify-center border border-destructive/40">
        <Text class="text-destructive">!</Text>
      </View>
      <View class="flex-1 gap-1">
        <Text class="font-bold text-foreground">{t(port() ? "error.notice_unavailable" : "error.notice_title")}</Text>
        <Text class="text-[11px] text-muted-foreground">{port() ? t("error.notice_port") : raw()}</Text>
      </View>
      <View class="flex-row items-center gap-1 self-center">
        <ToolButton icon="copy" label={t("action.copy_error")} onPress={() => Omasend.copyText(raw())} />
        <ToolButton icon="x" label={t("action.dismiss_error")} onPress={() => Omasend.dismissError()} />
      </View>
    </View>
    </If>
  );
}

const HOSTNAME = Omasend.hostname();

function StatusBar() {
  const status = () =>
    !state().identity ? "status.connecting" : state().devices.length === 0 ? "status.searching" : "status.connected";
  return (
    <View testID="status-bar" class="h-7 px-4 shrink-0 flex-row items-center gap-3 border-t border-border">
      <Text class="flex-1 text-[11px] text-muted-foreground truncate">{state().identity?.alias ?? HOSTNAME}</Text>
      <UpdateStatus />
      <Text class="text-[11px] text-muted-foreground">{t(status())}</Text>
    </View>
  );
}

// Omasend's keys outside text fields, as the Rust version had them:
// mod+v pastes, mod+o adds files, ←/→ choose a device, Return sends.
// Dialogs and sheets keep their keys.
function bindKeys() {
  const idle = () => !(surfaceOpen() || historyOpen() || preview() || state().incoming);
  const only = (run: () => void) => () => {
    if (idle()) run();
  };
  const off = keymap.bind(
    {
      "mod+v": only(() => Omasend.paste()),
      "mod+o": only(() => Omasend.pick(false)),
      left: only(() => Omasend.navigate(-1)),
      right: only(() => Omasend.navigate(1)),
      enter: only(() => {
        if (state().canSend) Omasend.send();
      }),
    },
    { when: "notEditing" },
  );
  cleanup(off);
}

function App() {
  applySavedTheme();
  effect(announceFinished);
  bindKeys();
  return (
    <View class="flex-1 bg-background">
      <TitleBar testID="app-header">
        {TERMINAL ? null : <Emblem size={24} />}
        <Text testID="app-title" class="flex-1 text-foreground">
          Omasend
        </Text>
        <AppMenu />
      </TitleBar>
      <ErrorNotice />
      <Nearby />
      <Composer />
      <StatusBar />
      <Incoming />
      <History />
      <Preview />
      <Surfaces />
      <Toaster />
    </View>
  );
}
mount(App);
