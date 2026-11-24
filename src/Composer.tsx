// The items to send and the Send button.
import { Text, View, platform } from "nsh";
import { Button, EmptyState, IconButton, Row, Section } from "./ui";
import { Omasend, sizeLabel, state, t } from "./state";

// iOS and Android pick from the photo library; their file pickers take no folders.
const MOBILE = platform.os === "ios" || platform.os === "android";

function sendLabel() {
  const device = state().devices.find((d) => d.id === state().selected);
  return device ? t("action.send_to").replace("%{name}", device.alias) : t("action.send");
}

export function Composer() {
  return (
    <Section
      title={t("composer.outbox")}
      class="flex-1"
      actions={
        <View testID="outbox-actions" class="flex-row gap-1">
          <Button label={t("action.add_files")} size="small" variant="ghost" icon="plus" onPress={() => Omasend.pick(false)} />
          {MOBILE ? (
            <Button label={t("action.photo_library")} size="small" variant="ghost" onPress={() => Omasend.pickPhotos()} />
          ) : (
            <Button label={t("action.add_folder_ellipsis")} size="small" variant="ghost" onPress={() => Omasend.pick(true)} />
          )}
          <Button label={t("action.paste")} size="small" variant="ghost" onPress={() => Omasend.paste()} />
        </View>
      }
    >
      <View class="flex-1 gap-1">
        {state().composer.length === 0 ? (
          <EmptyState title={t("composer.empty")} description={t("composer.drop_hint")} icon="inbox" />
        ) : (
          <Each of={state().composer} key={(item: any) => item.id}>
            {(item: any) => (
              <Row
                testID={`item-${item().id}`}
                title={item().name}
                detail={item().kind === "text" ? undefined : sizeLabel(item().size)}
                trailing={
                  <IconButton icon="x" label={t("action.remove")} size="small" onPress={() => Omasend.remove(item().id)} />
                }
              />
            )}
          </Each>
        )}
      </View>
      {state().error ? <Text class="text-xs text-destructive">{state().error}</Text> : null}
      <View class="flex-row justify-end">
        <Button
          testID="send"
          label={sendLabel()}
          variant="primary"
          icon="upload"
          disabled={!state().canSend}
          onPress={() => Omasend.send()}
        />
      </View>
    </Section>
  );
}
