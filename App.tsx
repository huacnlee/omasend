// Omasend's window. The LocalSend client, its state and its views are Rust in
// ext/omasend, an nshm of the app's own; this entry mounts its root view.
import { App as Omasend } from "omasend/native";

export default function App() {
  return <Omasend style={{ flex: 1 }} />;
}
