// The devices LocalSend finds on this network.
import { EmptyState, Row, Section } from "./ui";
import { Omasend, state, t } from "./state";

export function Nearby() {
  return (
    <Section title={t("nearby.title")} class="flex-1">
      {state().devices.length === 0 ? (
        <EmptyState title={t("nearby.looking")} icon="search" />
      ) : (
        <Each of={state().devices} key={(d: any) => d.id}>
          {(device: any) => (
            <Row
              testID={`device-${device().id}`}
              title={device().alias}
              detail={[device().model, device().host].filter(Boolean).join(" · ")}
              selected={state().selected === device().id}
              onPress={() => Omasend.select(device().id)}
            />
          )}
        </Each>
      )}
    </Section>
  );
}
