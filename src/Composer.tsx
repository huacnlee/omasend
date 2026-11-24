// The Outbox: what to send to the selected device, the drop area, the actions
// under it and the transfers in progress, as Omasend has always drawn them.
import { Image, Scroll, Text, View, platform, signal } from "nsh";
import { Button, Dialog, Textarea } from "./ui";
import { Active, TransferRecord, historyOpen } from "./Activity";
import { surfaceOpen } from "./Settings";
import { Omasend, sizeLabel, state, t } from "./state";
import { Emblem } from "./Emblem";
import { preview } from "./preview";

// iOS and Android pick from the photo library; their file pickers take no folders.
const MOBILE = platform.os === "ios" || platform.os === "android";
const DESKTOP = platform.idiom === "desktop";


function Keycap(props: { label: string }) {
  return (
    <View class="px-1 py-[3px] border border-foreground/25">
      <Text class="text-[11px] text-foreground">{props.label}</Text>
    </View>
  );
}

function Header() {
  const device = () => state().devices.find((d) => d.id === state().selected);
  return (
    <View class="flex-row items-center gap-2">
      <Text class="text-foreground">{t("composer.outbox")}</Text>
      <Text class="flex-1 text-muted-foreground truncate">
        {device() ? t("history.to_lower").replace("%{name}", device()!.alias) : t("composer.choose_device")}
      </Text>
    </View>
  );
}

const dismissed = signal("");

/** The newest received transfer, shown in the empty Outbox until dismissed. */
function latestReceived() {
  if (state().composer.length > 0) return null;
  const item = state()
    .transfers.slice()
    .reverse()
    .find((t) => !t.sending && t.status === "completed");
  return item && item.id !== dismissed() ? item : null;
}

function Empty() {
  return (
    <Scroll class="flex-1">
    <View class="min-h-full items-center justify-center gap-3 p-4">
      <Emblem size={64} animated paused={() => historyOpen() || !!preview() || !!state().incoming || surfaceOpen()} />
      <Text class="text-foreground">{t(DESKTOP ? "composer.drop_hint" : "composer.mobile_hint")}</Text>
      <Text class="text-muted-foreground text-center" style={{ numberOfLines: 2 }}>
        {t("composer.drop_kinds")}
      </Text>
      {DESKTOP ? (
        <View class="flex-row items-center gap-2">
          <Keycap label="ctrl+v" />
          <Text class="text-foreground">{t("action.paste")}</Text>
          <View class="w-2" />
          <Keycap label="ctrl+o" />
          <Text class="text-foreground">{t("action.add_files")}</Text>
        </View>
      ) : null}
    </View>
    </Scroll>
  );
}

type Item = ReturnType<typeof state>["composer"][number];

function ItemCard(props: { item: () => Item }) {
  const item = () => props.item();
  const transfer = () => state().transfers.find((t) => t.id === item().transfer);
  const progress = () => {
    const live = transfer();
    if (!live) return null;
    if (live.status === "active" && live.awaitingAcceptance) return { text: t("status.waiting"), ratio: -1 };
    if (live.status === "active") {
      const ratio = live.total > 0 ? live.transferred / live.total : 0;
      return {
        text: `${sizeLabel(live.transferred)} / ${sizeLabel(live.total)} · ${Math.round(ratio * 100)}% · ${sizeLabel(live.bytesPerSecond)}/s`,
        ratio,
      };
    }
    if (live.status === "completed") return { text: t("status.sent"), ratio: -1 };
    if (live.status === "cancelled") return { text: t("status.cancelled"), ratio: -1 };
    return { text: live.error || t("status.failed"), ratio: -1 };
  };
  const label = () =>
    item().kind === "text" ? sizeLabel(item().size) : `${item().name} · ${sizeLabel(item().size)}`;
  return (
    <View testID="composer-card" class="p-3 gap-2 border border-border">
      {item().kind === "text" ? (
        <Text testID="composer-content-summary" class="text-xs text-muted-foreground" style={{ numberOfLines: 3 }}>
          {item().text.slice(0, 240)}
        </Text>
      ) : null}
      <View class="flex-row items-center gap-2">
        {item().kind === "image" && item().path ? (
          <Image src={`file://${item().path}`} style={{ width: 48, height: 48, objectFit: "contain" }} />
        ) : null}
        <Text class="flex-1 text-foreground truncate">{label()}</Text>
        {item().kind === "image" || item().kind === "text" ? (
          <Button label={t("action.preview")} size="small" onPress={() => preview.set(item())} />
        ) : null}
        <Button
          label={t(transfer()?.status === "active" ? "action.cancel" : "action.remove")}
          size="small"
          onPress={() => (transfer()?.status === "active" ? Omasend.cancel(transfer()!.id) : Omasend.remove(item().id))}
        />
      </View>
      {progress() ? (
        <View class="gap-2">
          <Text class="text-[11px] text-muted-foreground">{progress()!.text}</Text>
          {progress()!.ratio >= 0 ? (
            <View testID="composer-transfer-progress" class="h-[3px] bg-border">
              <View class="h-[3px] bg-primary" style={{ width: `${Math.min(100, progress()!.ratio * 100)}%` }} />
            </View>
          ) : null}
        </View>
      ) : null}
    </View>
  );
}


export function Preview() {
  const item = () => preview();
  return (
    <Dialog
      testID="preview"
      title={t("action.preview")}
      open={!!item()}
      onOpenChange={(event) => {
        if (!event.open) preview.set(null);
      }}
      footer={<Button label={t("action.close")} onPress={() => preview.set(null)} />}
    >
      {item()?.kind === "image" ? (
        <Image src={`file://${item()!.path}`} style={{ width: "100%", height: 420, objectFit: "contain" }} />
      ) : (
        <Textarea value={item()?.text ?? ""} readonly rows={14} accessibilityLabel={t("message.title")} />
      )}
    </Dialog>
  );
}

const hovering = signal(false);

function DropArea() {
  return (
    <View
      testID="composer-area"
      class={hovering() ? "flex-1 border-2 border-primary" : "flex-1 border border-border m-px"}
      onFileDragEnter={() => hovering.set(true)}
      onFileDragLeave={() => hovering.set(false)}
      onFileDrop={(event) => {
        hovering.set(false);
        Omasend.dropFiles(event.uris);
      }}
    >
      {state().composer.length > 0 ? (
        <Scroll class="flex-1">
          <View class="p-3 gap-2">
            <Each of={state().composer} key={(item: Item) => item.id}>
              {(item: () => Item) => <ItemCard item={item} />}
            </Each>
          </View>
        </Scroll>
      ) : latestReceived() ? (
        <Scroll class="flex-1">
          <View class="p-3">
            <TransferRecord item={() => latestReceived()!} onDismiss={() => dismissed.set(latestReceived()!.id)} />
          </View>
        </Scroll>
      ) : (
        <Empty />
      )}
    </View>
  );
}

function Actions() {
  const device = () => state().devices.find((d) => d.id === state().selected);
  return (
    <View testID="outbox-actions" class="flex-row items-center gap-2">
      <Button label={t("action.add_files_ellipsis")} onPress={() => Omasend.pick(false)} />
      {MOBILE ? (
        <Button label={t("action.photo_library")} variant="ghost" onPress={() => Omasend.pickPhotos()} />
      ) : (
        <Button label={t("action.add_folder_ellipsis")} variant="ghost" onPress={() => Omasend.pick(true)} />
      )}
      <Button label={t("action.paste")} variant="ghost" onPress={() => Omasend.paste()} />
      <View class="flex-1" />
      {DESKTOP ? <Button label={t("history.title")} variant="ghost" onPress={() => historyOpen.set(true)} /> : null}
      <Button
        testID="send"
        label={t("action.send")}
        accessibilityLabel={device() ? t("action.send_to").replace("%{name}", device()!.alias) : t("action.send")}
        variant={MOBILE ? "primary" : "default"}
        disabled={!state().canSend}
        onPress={() => Omasend.send()}
      />
    </View>
  );
}

export function Composer() {
  return (
    <View testID="outbox-panel" class="flex-1 px-4 pt-4 pb-3 gap-3">
      <Header />
      <DropArea />
      <Actions />
      <Active />
    </View>
  );
}
