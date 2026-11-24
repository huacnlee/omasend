// Receiving and the transfers: the request to accept, the live transfers and
// the history sheet. Finished transfers also raise a toast.
import { Text, View, signal } from "nsh";
import { Button, Dialog, IconButton, Progress, Row, Section, Sheet, EmptyState, toast } from "./ui";
import { Omasend, sizeLabel, state, t } from "./state";

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

/** One transfer with its progress and the actions that fit its state. */
function TransferRow(props: { item: () => Transfer }) {
  const item = () => props.item();
  const detail = () =>
    item().status === "active" && !item().awaitingAcceptance
      ? `${statusLabel(item())} · ${sizeLabel(item().transferred)} / ${sizeLabel(item().total)}`
      : statusLabel(item());
  return (
    <View class="gap-1">
      <Row
        testID={`transfer-${item().id}`}
        title={`${fill(item().sending ? "history.to" : "history.from", item().peer)} · ${item().files.join(", ")}`}
        detail={detail()}
        trailing={
          <View class="flex-row gap-1">
            {item().status === "active" ? (
              <IconButton icon="x" label={t("action.cancel")} size="small" onPress={() => Omasend.cancel(item().id)} />
            ) : null}
            {item().text ? (
              <IconButton icon="copy" label={t("action.copy")} size="small" onPress={() => Omasend.copyReceived(item().id)} />
            ) : null}
            {item().paths.length > 0 ? (
              <IconButton
                icon="folder"
                label={t("action.show_in_files")}
                size="small"
                onPress={() => Omasend.openPath(item().paths[0], true)}
              />
            ) : null}
          </View>
        }
      />
      {item().status === "active" && !item().awaitingAcceptance ? (
        <Progress value={percent(item())} label={null} />
      ) : null}
    </View>
  );
}

/** The live transfers, newest first; nothing while there are none. */
export function Active() {
  const live = () => state().transfers.filter((item) => item.status === "active").reverse();
  return live().length === 0 ? null : (
    <Section title={t("history.session")}>
      <Each of={live()} key={(item: Transfer) => item.id}>
        {(item: () => Transfer) => <TransferRow item={item} />}
      </Each>
    </Section>
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
        <View class="gap-2">
          <Each of={items()} key={(item: Transfer) => item.id}>
            {(item: () => Transfer) => <TransferRow item={item} />}
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
