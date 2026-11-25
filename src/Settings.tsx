// The app menu and what it opens: theme, language, updates, logs, the network
// diagnosis and About.
import { Scroll, Text, View, signal } from "nsh";
import { Submenu } from "nsh/base";
import { ChevronRight } from "nsh-icon-lucide";
import { Button, Dialog, Icon, ToolButton, Input, Menu, MenuItem, MenuLabel, MenuSeparator, Omarchy, Textarea, toast } from "./ui";
import { Omasend, state, t } from "./state";

type Surface = "logs" | "diagnostics" | "about" | null;
const surface = signal<Surface>(null);
const logs = signal("");

/** Whether logs, the diagnosis or About covers the window. */
export const surfaceOpen = () => surface() !== null;

function openLogs() {
  logs.set(Omasend.logs());
  surface.set("logs");
}

function copy(text: string) {
  Omasend.copyText(text);
  toast({ title: t("logs.copied"), kind: "success" });
}

const LINKS: [string, string][] = [
  ["Omasend…", "https://huacnlee.github.io/omasend"],
  ["GitHub…", "https://github.com/huacnlee/omasend"],
  ["LocalSend…", "https://localsend.org"],
];

const THEMES = ["system", "light", "dark"] as const;
type ThemeMode = (typeof THEMES)[number];
const theme = signal<ThemeMode>("system");

/** Draws the app in the saved theme mode; Omarchy installations follow Omarchy's theme. */
export function applySavedTheme() {
  applyTheme((Omasend.omarchy() ? "system" : Omasend.theme()) as ThemeMode);
}

function applyTheme(mode: ThemeMode) {
  theme.set(mode);
  if (mode === "light") Omarchy.applyTheme("flexoki-light");
  else if (mode === "dark") Omarchy.applyTheme("tokyo-night");
  else Omarchy.followSystem();
}

function setTheme(mode: ThemeMode) {
  applyTheme(mode);
  Omasend.setTheme(mode);
}

/** The title bar's menu. */
export function AppMenu() {
  return (
    <Menu
      testID="app-menu"
      align="end"
      trigger={<ToolButton icon="menu" label={t("action.menu")} />}
    >
      <MenuItem label={t("action.paste")} shortcut="mod-v" onSelect={() => Omasend.paste()} />
      <MenuItem label={t("action.add_files_ellipsis")} shortcut="mod-o" onSelect={() => Omasend.pick(false)} />
      <MenuSeparator />
      {Omasend.omarchy() ? null : (
        <>
          <MenuLabel label={t("action.theme")} />
          {THEMES.map((mode) => (
            <MenuItem label={t(`theme.${mode}`)} checked={theme() === mode} onSelect={() => setTheme(mode)} closeOnSelect={false} />
          ))}
          <MenuSeparator />
        </>
      )}
      <Submenu
        testID="language-menu"
        offset={4}
        trigger={
          <View class="flex-row items-center gap-2 h-6 px-2 hover:bg-foreground/10 focus:bg-foreground/10">
            <Text class="flex-1 text-foreground" style={{ fontSize: 12 }}>{t("action.language")}</Text>
            <Icon icon={ChevronRight} size={12} class="text-muted-foreground" />
          </View>
        }
      >
        <MenuItem label="English" checked={state().language === "en"} onSelect={() => Omasend.setLanguage("en")} />
        <MenuItem label="简体中文" checked={state().language === "zh-CN"} onSelect={() => Omasend.setLanguage("zh-CN")} />
      </Submenu>
      <MenuSeparator />
      <MenuItem
        label={t("action.check_update")}
        disabled={state().updateBusy}
        onSelect={() => Omasend.checkUpdates()}
      />
      <MenuItem label={t("action.about")} onSelect={() => surface.set("about")} />
      {LINKS.map(([label, url]) => (
        <MenuItem label={label} onSelect={() => Omasend.openUrl(url)} />
      ))}
      <MenuSeparator />
      <MenuItem label={t("action.logs")} onSelect={openLogs} />
      <MenuItem label={t("action.diagnostics")} onSelect={() => surface.set("diagnostics")} />
      <MenuSeparator />
      <MenuItem label={t("action.exit")} shortcut="mod-q" onSelect={() => Omasend.exit()} />
    </Menu>
  );
}

/** The update status in the title bar: a button when there is something to install or restart into. */
export function UpdateStatus() {
  const label = () => state().update;
  return (
    <If when={label()}>
      {state().updateActionable ? (
        <Button testID="update" label={label()} size="small" variant="ghost" onPress={() => Omasend.activateUpdate()} />
      ) : (
        <Text class="text-[11px] text-muted-foreground">{label()}</Text>
      )}
    </If>
  );
}

function close(event: { open: boolean }) {
  if (!event.open) surface.set(null);
}

/** The log, over the whole window. */
function Logs() {
  return (
    <If when={surface() === "logs"}>
      <View testID="logs" class="absolute inset-x-0 bottom-0 top-9 bg-background">
        <View class="px-4 py-2 min-h-11 flex-row items-center gap-2 border-b border-border">
          <Text class="flex-1 font-semibold text-foreground">{t("logs.title")}</Text>
          <ToolButton icon="x" label={t("action.close")} onPress={() => surface.set(null)} />
        </View>
        <Scroll class="flex-1 m-3 border border-border bg-foreground/5">
          <View selectable>
          <Text class="p-3 select-text text-xs text-foreground font-mono" accessibilityLabel={t("logs.title")}>
            {logs() || t("logs.empty")}
          </Text>
          </View>
        </Scroll>
        <View class="flex-row gap-2 px-3 pb-3">
          <Button label={t("action.refresh")} onPress={() => logs.set(Omasend.logs())} />
          <Button label={t("action.copy")} disabled={!logs()} onPress={() => copy(logs())} />
          <Button
            label={t("action.clear")}
            disabled={!logs()}
            onPress={() => {
              Omasend.clearLogs();
              logs.set(Omasend.logs());
            }}
          />
        </View>
      </View>
    </If>
  );
}

const peer = signal("");
const invalidPeer = signal(false);

function Diagnostics() {
  const run = () => invalidPeer.set(!Omasend.diagnose(peer()));
  const runLabel = () =>
    t(state().diagnosing ? "diagnostics.running" : state().diagnosis ? "diagnostics.run_again" : "diagnostics.run");
  return (
    <Dialog
      testID="diagnostics"
      title={t("diagnostics.title")}
      description={t("diagnostics.intro")}
      open={surface() === "diagnostics"}
      onOpenChange={close}
      footer={
        <View class="flex-row items-center gap-2">
          <Text class="flex-1 text-xs text-muted-foreground">
            {state().diagnosisStep ? t(state().diagnosisStep) : ""}
          </Text>
          <Button label={t("action.copy")} disabled={state().diagnosing || !state().diagnosis} onPress={() => copy(state().diagnosis)} />
          <Button testID="diagnose" label={runLabel()} variant="primary" disabled={state().diagnosing} onPress={run} />
        </View>
      }
    >
      <View class="gap-3">
        <View class="gap-1">
          <Text class="text-xs text-muted-foreground">{t("diagnostics.peer_label")}</Text>
          <Input
            testID="diagnostics-peer"
            value={peer()}
            placeholder={t("diagnostics.peer_placeholder")}
            invalid={invalidPeer()}
            accessibilityLabel={t("diagnostics.peer_label")}
            onChange={(event) => {
              peer.set(event.value);
              invalidPeer.set(false);
            }}
            onSubmit={run}
          />
          {invalidPeer() ? <Text class="text-xs text-destructive">{t("diagnostics.peer_invalid")}</Text> : null}
        </View>
        <Textarea
          testID="diagnostics-report"
          value={state().diagnosis}
          placeholder={t("diagnostics.empty")}
          readonly
          rows={12}
          accessibilityLabel={t("diagnostics.report_label")}
        />
      </View>
    </Dialog>
  );
}

function About() {
  const lines = () => (state().language, Omasend.about());
  return (
    <Dialog
      testID="about"
      title={t("about.title")}
      open={surface() === "about"}
      onOpenChange={close}
      footer={
        <View class="flex-row gap-2">
          <Button label="localsend.org" onPress={() => Omasend.openUrl("https://localsend.org")} />
          <Button label={t("about.source")} onPress={() => Omasend.openUrl("https://github.com/huacnlee/omasend")} />
        </View>
      }
    >
      <View class="gap-2">
        <Text class="text-sm font-semibold text-foreground">{lines()[0]}</Text>
        <Text class="text-sm text-foreground">{lines()[1]}</Text>
        <Text class="text-sm text-foreground">{t("about.credit")}</Text>
        <Text class="text-xs text-muted-foreground">{t("about.native_shell")}</Text>
      </View>
    </Dialog>
  );
}

/** The dialogs the menu opens. */
export function Surfaces() {
  return (
    <>
      <Logs />
      <Diagnostics />
      <About />
    </>
  );
}
