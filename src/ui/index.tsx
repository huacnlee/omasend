// Omasend's own components. Screens use these, not nsh-omarchy directly, so
// the app keeps one small vocabulary of controls.
import { Text, View, platform, reduceMotion } from "nsh";
import {
  Button as OmarchyButton,
  Caption,
  Dialog,
  EmptyState,
  Icon,
  IconButton,
  Input,
  ListItem,
  Menu,
  MenuItem,
  MenuLabel,
  MenuSeparator,
  Omarchy,
  Progress,
  Sheet,
  TitleBar,
  Textarea,
  Toaster,
  toast,
} from "nsh-omarchy";

export {
  Icon,
  Dialog,
  EmptyState,
  IconButton,
  Input,
  Menu,
  MenuItem,
  MenuLabel,
  MenuSeparator,
  Omarchy,
  Progress,
  Sheet,
  Textarea,
  TitleBar,
  Toaster,
  toast,
};

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

/** nsh-omarchy's button with Omasend's press: it shrinks a little while held. */
export function Button(props: Parameters<typeof OmarchyButton>[0]) {
  return (
    <OmarchyButton
      {...props}
      style={{ transitionProperty: "scale", transitionDuration: reduceMotion() ? 0 : 90, transitionTimingFunction: "linear" }}
      activeStyle={{ scale: reduceMotion() ? 1 : 0.98 }}
    />
  );
}

/** The terminal: no icons, a grid of character cells. */
export const TERMINAL = platform.idiom === "terminal";

/**
 * A small action in a bar: an icon button, or its label as a text button in
 * the terminal, where icons cannot draw (as the Rust version did).
 */
export function ToolButton(props: {
  icon: Parameters<typeof IconButton>[0]["icon"];
  label: string;
  onPress?: () => void;
  testID?: string;
}) {
  return TERMINAL ? (
    <OmarchyButton testID={props.testID} label={props.label} size="small" variant="ghost" onPress={props.onPress} />
  ) : (
    <IconButton testID={props.testID} icon={props.icon} label={props.label} size="small" onPress={props.onPress} />
  );
}
