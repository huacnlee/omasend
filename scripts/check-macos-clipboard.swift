import AppKit
import Foundation

// Run manually on macOS from a logged-in desktop session. This uses the real
// pasteboard; restore it before returning, including every error path.
let root = URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent()
func snapshot() throws -> [[String: Data]] {
  try (NSPasteboard.general.pasteboardItems ?? []).map { item in
    var contents: [String: Data] = [:]
    for type in item.types {
      guard let data = item.data(forType: type) else {
        throw NSError(
          domain: "ClipboardFixture", code: 3,
          userInfo: [
            NSLocalizedDescriptionKey: "Cannot snapshot every clipboard format; test aborted"
          ])
      }
      contents[type.rawValue] = data
    }
    return contents
  }
}
func verify() throws -> Int32 {
  guard CommandLine.arguments.count > 1,
    ["jpeg", "tiff"].contains(CommandLine.arguments[1])
  else {
    throw NSError(
      domain: "ClipboardFixture", code: 2,
      userInfo: [
        NSLocalizedDescriptionKey:
          "Usage: swift scripts/check-macos-clipboard.swift jpeg|tiff [NSH_SCRIPT]"
      ])
  }
  let board = NSPasteboard.general
  let saved: [NSPasteboardItem] = try snapshot().map { contents in
    let copy = NSPasteboardItem()
    for (type, data) in contents {
      copy.setData(data, forType: NSPasteboard.PasteboardType(type))
    }
    return copy
  }
  defer {
    board.clearContents()
    if !saved.isEmpty { board.writeObjects(saved) }
  }
  let item = NSPasteboardItem()
  let source = try Data(contentsOf: root.appendingPathComponent("assets/omasend.png"))
  let bitmap = NSBitmapImageRep(data: source)!
  let format = CommandLine.arguments[1]
  let data = bitmap.representation(
    using: format == "jpeg" ? .jpeg : .tiff,
    properties: format == "jpeg" ? [.compressionFactor: 0.8] : [:])!
  item.setData(
    data, forType: NSPasteboard.PasteboardType(format == "jpeg" ? "public.jpeg" : "public.tiff"))
  board.clearContents()
  board.writeObjects([item])
  let temporary = URL(fileURLWithPath: NSTemporaryDirectory())
  let before = Set(try FileManager.default.contentsOfDirectory(atPath: temporary.path))
  let process = Process()
  defer {
    if process.isRunning {
      process.terminate()
      process.waitUntilExit()
    }
  }
  process.executableURL = root.appendingPathComponent("target/debug/omasend")
  var environment = ProcessInfo.processInfo.environment
  environment["NSH_SCRIPT"] =
    CommandLine.arguments.count > 2
    ? CommandLine.arguments[2]
    : "wait 2000; tap 130 562; wait 1000; shot /tmp/omasend-\(format)-card.png; tap 730 207; wait 700; shot /tmp/omasend-\(format)-preview.png; key Escape; quit"
  process.environment = environment
  try process.run()
  var exact = false
  let deadline = Date().addingTimeInterval(25)
  while process.isRunning && Date() < deadline {
    let created = Set(try FileManager.default.contentsOfDirectory(atPath: temporary.path))
      .subtracting(before)
    exact = created.filter { $0.hasPrefix("screenshot-") }.contains { name in
      (try? Data(contentsOf: temporary.appendingPathComponent(name))) == data
    }
    if exact { break }
    Thread.sleep(forTimeInterval: 0.05)
  }
  if !exact {
    if process.isRunning { process.terminate() }
    process.waitUntilExit()
    throw NSError(
      domain: "ClipboardFixture", code: 1,
      userInfo: [NSLocalizedDescriptionKey: "No exact clipboard import observed for " + format])
  }
  print("Verified byte-preserving " + format + " clipboard import")
  process.waitUntilExit()
  return process.terminationStatus
}
let original = try snapshot()
var status: Int32 = 1
do { status = try verify() } catch { fputs("Clipboard verification failed: \(error)\n", stderr) }
if try snapshot() != original {
  fputs("Clipboard restoration failed\n", stderr)
  status = 1
} else {
  print("Verified clipboard restoration")
}
exit(status)
