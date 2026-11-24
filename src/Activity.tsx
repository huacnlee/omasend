// Receiving and the transfers: the request to accept, the live transfers and
// the history sheet. Finished transfers also raise a toast.
import { Image, Text, View, platform, signal } from "nsh";
import { Button, Dialog, Sheet, EmptyState, Textarea, toast } from "./ui";
import { Omasend, sizeLabel, state, t } from "./state";
import { preview } from "./preview";

type Transfer = ReturnType<typeof Omasend.snapshot>["transfers"][number];

function fill(key: string, name: string) {
  return t(key).replace("%{name}", name);
}

function statusLabel(item: Transfer) {
  if (item.status === "active") {
    if (item.awaitingAcceptance) return t("status.waiting");
    return t(item.sending ? "status.sending" : "status.receiving");
  }
  if (item.status === "completed") return t(item.sending ? "status.sent" : "status.received");
  if (item.status === "failed") return item.error || t("status.failed");
  return t("status.cancelled");
}

function percent(item: Transfer) {
  return item.total > 0 ? Math.round((item.transferred / item.total) * 100) : 0;
}

function when(item: Transfer) {
  const d = new Date(item.when);
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}`;
}

function statusColor(item: Transfer) {
  if (item.status === "completed") return "text-green-500";
  if (item.status === "failed") return "text-destructive";
  return "text-muted-foreground";
}

function isImage(path: string) {
  return /\.(png|jpe?g|gif|webp|heic|bmp|svg)$/i.test(path);
}

function basename(path: string) {
  return path.split(/[\\/]/).pop() || path;
}

/** One transfer as a record: peer, status, size and time, its progress and what to do with what arrived. */
export function TransferRecord(props: { item: () => Transfer; onDismiss?: () => void }) {
  const item = () => props.item();
  const expanded = signal(false);
  const size = () =>
    item().status === "active" ? `${sizeLabel(item().transferred)} / ${sizeLabel(item().total)}` : sizeLabel(item().total);
  return (
    <View testID="transfer-record" class="gap-1.5 py-3 border-b border-border">
      <View class="flex-row items-center gap-3">
        <Text class="flex-1 text-foreground truncate">{item().peer}</Text>
        <Text class={`text-[11px] truncate ${statusColor(item())}`}>{statusLabel(item())}</Text>
        {props.onDismiss ? (
          <Button testID="dismiss-received" label={t("action.dismiss")} size="small" onPress={props.onDismiss} />
        ) : null}
      </View>
      <Text class="text-[11px] text-muted-foreground">{`${size()} · ${when(item())}`}</Text>
      {item().status === "active" ? (
        <View class="gap-2">
          {item().awaitingAcceptance ? null : (
            <View testID="transfer-progress" class="h-[3px] bg-border">
              <View class="h-[3px] bg-primary" style={{ width: `${percent(item())}%` }} />
            </View>
          )}
          <View class="flex-row">
            <Button label={t("action.cancel")} size="small" variant="ghost" onPress={() => Omasend.cancel(item().id)} />
          </View>
        </View>
      ) : null}
      {item().paths.map((path) => (
        <View class="flex-row flex-wrap items-center gap-2">
          {isImage(path) ? (
            <Image
              testID="received-image-thumbnail"
              src={`file://${path}`}
              style={props.onDismiss ? { width: "100%", height: 220, objectFit: "contain" } : { width: 48, height: 48, objectFit: "contain" }}
            />
          ) : null}
          <Text class="flex-1 text-foreground truncate">{basename(path)}</Text>
          {isImage(path) ? (
            <Button
              label={t("action.preview")}
              size="small"
              onPress={() => preview.set({ id: path, kind: "image", name: basename(path), size: 0, path, transfer: "", text: "" })}
            />
          ) : null}
          <Button
            label={t(platform.os === "ios" ? "action.share" : "action.open")}
            size="small"
            onPress={() => Omasend.openPath(path, false)}
          />
          {platform.idiom === "desktop" ? (
            <Button label={t("action.show_in_files")} size="small" onPress={() => Omasend.openPath(path, true)} />
          ) : null}
        </View>
      ))}
      {item().text ? (
        <View class="gap-2">
          {expanded() ? (
            <Textarea value={item().text} readonly rows={5} accessibilityLabel={t("message.title")} />
          ) : (
            <Text class="text-[11px] text-muted-foreground" style={{ numberOfLines: 2 }}>
              {item().text}
            </Text>
          )}
          <View class="flex-row gap-2">
            <Button
              label={t(expanded() ? "action.collapse" : "action.expand")}
              size="small"
              variant="ghost"
              onPress={() => expanded.set(!expanded())}
            />
            <Button label={t("action.copy")} size="small" variant="ghost" onPress={() => Omasend.copyReceived(item().id)} />
          </View>
        </View>
      ) : null}
    </View>
  );
}

/** The live transfers, newest first; nothing while there are none. */
export function Active() {
  const live = () => state().transfers.filter((item) => item.status === "active").reverse();
  return (
    <View>
      <Each of={live()} key={(item: Transfer) => item.id}>
        {(item: () => Transfer) => <TransferRecord item={item} />}
      </Each>
    </View>
  );
}

/** The request to receive, as a dialog that asks to accept or decline. */
export function Incoming() {
  const request = () => state().incoming;
  return (
    <Dialog
      testID="incoming"
      title={request() ? fill("receive.title", request()!.peer.alias) : ""}
      description={t("receive.destination")}
      open={!!request()}
      closeOnBackdropPress={false}
      onOpenChange={(event) => {
        if (!event.open && request()) Omasend.decide(request()!.id, false);
      }}
      footer={
        <View class="flex-row gap-2">
          <Button label={t("action.decline")} onPress={() => Omasend.decide(request()!.id, false)} />
          <Button
            testID="accept"
            label={t("action.accept")}
            variant="primary"
            onPress={() => Omasend.decide(request()!.id, true)}
          />
        </View>
      }
    >
      <View class="gap-1">
        {(request()?.files ?? []).map((name) => (
          <Text class="text-sm text-foreground truncate">{name}</Text>
        ))}
        <Text class="text-xs text-muted-foreground">{sizeLabel(request()?.total ?? 0)}</Text>
      </View>
    </Dialog>
  );
}

/** Whether the history sheet shows. */
export const historyOpen = signal(false);

/** Every transfer of this and earlier sessions, newest first. */
export function History() {
  const items = () => state().transfers.slice().reverse();
  return (
    <Sheet
      testID="history-sheet"
      placement="bottom"
      title={t("history.title")}
      open={historyOpen()}
      closeLabel={t("action.close")}
      onOpenChange={(event) => historyOpen.set(event.open)}
      footer={
        <Button
          label={t("action.clear_history")}
          variant="ghost"
          disabled={items().length === 0}
          onPress={() => Omasend.clearHistory()}
        />
      }
    >
      {items().length === 0 ? (
        <EmptyState title={t("history.empty")} icon="inbox" />
      ) : (
        <View>
          <Each of={items()} key={(item: Transfer) => item.id}>
            {(item: () => Transfer) => <TransferRecord item={item} />}
          </Each>
        </View>
      )}
    </Sheet>
  );
}

// Toast each transfer once, when it finishes in this session.
const announced = new Set<string>();
let primed = false;

/** Raises a toast for each transfer that finished since the last call. */
export function announceFinished() {
  const transfers = state().transfers;
  for (const item of transfers) {
    if (item.status === "active" || announced.has(item.id)) continue;
    announced.add(item.id);
    if (!primed) continue;
    toast({
      id: item.id,
      title: statusLabel(item),
      description: `${fill(item.sending ? "history.to" : "history.from", item.peer)} · ${item.files.join(", ")}`,
      kind: item.status === "completed" ? "success" : item.status === "failed" ? "error" : "info",
    });
  }
  primed = true;
}
