// Omasend's own components. Screens use these, not nsh-omarchy directly, so
// the app keeps one small vocabulary of controls.
import { Text, View } from "nsh";
import {
  Button,
  Caption,
  Dialog,
  EmptyState,
  IconButton,
  ListItem,
  Omarchy,
  Progress,
  Sheet,
  TitleBar,
  Toaster,
  toast,
} from "nsh-omarchy";

export { Button, Dialog, EmptyState, IconButton, Omarchy, Progress, Sheet, TitleBar, Toaster, toast };

/** A titled column: a caption above its content. */
export function Section(props: { title: string; actions?: any; children?: any; class?: string }) {
  return (
    <View class={`gap-2 ${props.class ?? ""}`}>
      <View class="flex-row items-center justify-between min-h-7">
        <Caption>{props.title}</Caption>
        {props.actions}
      </View>
      {props.children}
    </View>
  );
}

/** A row with a primary and a secondary line, pressable when `onPress` is set. */
export function Row(props: {
  testID?: string;
  title: string;
  detail?: string;
  selected?: boolean;
  trailing?: any;
  onPress?: () => void;
}) {
  return (
    <ListItem testID={props.testID} selected={props.selected} accessibilityLabel={props.title} onPress={props.onPress}>
      <View class="flex-1 flex-row items-center gap-3">
        <View class="flex-1 gap-0.5">
          <Text class="text-sm text-foreground truncate">{props.title}</Text>
          {props.detail ? <Text class="text-xs text-muted-foreground truncate">{props.detail}</Text> : null}
        </View>
        {props.trailing}
      </View>
    </ListItem>
  );
}
