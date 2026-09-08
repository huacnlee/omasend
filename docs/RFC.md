下面这版可以直接保存成 `RFC-OmaSend.md`，也适合直接交给 AI Coding Agent 作为开发指引。

# Omasend — Omarchy Native LocalSend Client

> Status: Draft
> Target: v0.1
> Platform: Omarchy / Linux / Wayland
> UI: GPUI + GPUI Omarchy
> Compatibility: LocalSend Protocol

## 1. 项目目标

Omasend 是一个专门面向 Omarchy 的 LocalSend 兼容客户端。

第一阶段的目标非常克制：

> **用 GPUI 实现一个真正符合 Omarchy 视觉、交互和键盘操作习惯的 LocalSend 客户端，作为 Omarchy 当前 LocalSend Flutter GUI 的原生替代。**

Omasend **不是重新设计 LocalSend 协议**，也暂时不是 Omarchy 的系统级 AirDrop。

核心价值是：

* GPUI Native UI
* Omarchy 原生视觉
* Keyboard-first
* 快速启动
* 低资源占用
* 与 Omarchy Screenshot / Screen Recording / Clipboard 工作流自然结合
* 与现有 LocalSend Android / iOS / macOS / Windows / Linux 客户端完全互通

未来如果 Omasend 被社区接受，再逐步向 Omarchy 的系统级文件传输能力演进。

---

# 2. 第一原则：先替代 LocalSend GUI

v0.1 不做：

* daemon
* CLI
* systemd service
* 开机自启动
* 后台常驻服务
* 自研协议
* 公网传输
* WebRTC
* Omasend 手机客户端
* 系统级 AirDrop replacement
* 复杂账号体系
* 云端服务

第一阶段就是：

```text
omarchy-launch-or-focus omasend
              │
              ▼
        ┌──────────────┐
        │   Omasend    │
        │    GPUI      │
        └──────┬───────┘
               │
       LocalSend Protocol
               │
       ┌───────┼────────┐
       ▼       ▼        ▼
     iPhone  Android   Mac/PC
    LocalSend LocalSend LocalSend
```

应用打开时存在。

应用退出时不存在。

这与普通桌面应用一致。

---

# 3. LocalSend 兼容性

不要自己发明传输协议。

LocalSend 当前公开协议已经发展到 **Protocol v2.2**，默认使用 UDP multicast 做发现，默认 multicast 地址 `224.0.0.167:53317`，HTTP/HTTPS 默认也使用 TCP `53317`。协议包括 Discovery、Prepare Upload、Upload、Cancel、Reverse Transfer 和 Info 等能力。([GitHub][1])

官方协议：

[LocalSend Protocol](https://github.com/localsend/protocol?utm_source=chatgpt.com)

官方 LocalSend：

[LocalSend](https://github.com/localsend/localsend?utm_source=chatgpt.com)

**重要：不要按照旧资料从头实现 LocalSend。**

LocalSend 在 2026 年已经完成核心网络和文件 I/O 的 Rust 化，v1.18.0 的 CLI 与 Flutter App 使用相同的 Rust library。([GitHub][2])

因此 Omasend 应首先研究并复用：

```text
localsend/localsend
└── packages/
    └── core/
```

例如官方 core 已经存在 v2 DTO：

```text
packages/core/src/http/dto_v2.rs
```

([GitHub][3])

### 实现原则

优先级：

```text
1. 直接依赖 / vendoring 官方 LocalSend Rust core
        ↓
2. 如果 API 不适合外部使用，做很薄的 Omasend adapter
        ↓
3. 只有确实无法复用时，才自行实现协议部分
```

不要一开始自己实现 UDP discovery、TLS、HTTP transfer 等基础设施。

---

# 4. v0.1 功能范围

## 4.1 Device Discovery

应用启动以后自动发现局域网中的 LocalSend 设备。

主界面直接展示：

```text
Nearby

┌─────────────────────┐
│ 📱 Jason's iPhone   │
│ iPhone              │
└─────────────────────┘

┌─────────────────────┐
│ 💻 MacBook Pro      │
│ macOS               │
└─────────────────────┘
```

设备应该动态：

* 出现
* 消失
* 更新状态

不需要用户点击 Refresh 才能发现。

---

# 5. 发送内容

v0.1 支持：

```text
File
Files
Folder
Text
Clipboard Image
Clipboard Video
Clipboard File
```

其中 **Clipboard 是 Omasend 的重点体验之一。**

---

# 6. Clipboard First

Omasend 必须把：

> **复制 → 粘贴 → 发送**

作为一级工作流，而不是附属功能。

用户不应该必须：

```text
点击 Add File
→ 打开 File Picker
→ 找文件
→ 选择
→ Send
```

应该支持：

```text
Copy
↓
打开 Omasend
↓
Ctrl+V
↓
选择设备
↓
Send
```

甚至可以进一步做到：

```text
打开 Omasend
↓
Ctrl+V
↓
直接出现 Send Preview
```

---

# 7. Clipboard 类型

不能假设 Clipboard 永远是文本。

必须先枚举 Wayland Clipboard MIME types。

例如：

```text
text/plain
text/uri-list
image/png
image/jpeg
video/mp4
...
```

再决定如何构造发送内容。

Omarchy 自己就是基于 Wayland clipboard 工作的，而且这不是理论问题：2026 年 8 月 Omarchy 已经出现过一个真实 bug——截图后 clipboard 中提供的是 `image/png`，旧分享脚本却按 `text/plain` 去读取，于是 LocalSend 收到了 0-byte 文件。([GitHub][4])

所以：

> **禁止直接 `wl-paste` 然后假设结果是 text。**

应该首先：

```bash
wl-paste --list-types
```

或等价 API 获取 MIME types。

---

# 8. Clipboard File

如果 clipboard 包含：

```text
text/uri-list
```

例如：

```text
file:///home/jason/Pictures/demo.png
```

Omasend 应解析成本地文件：

```rust
SendItem::File(PathBuf)
```

UI 显示：

```text
┌────────────────────────────────┐
│ 🖼 demo.png                     │
│ 2.4 MB                         │
└────────────────────────────────┘
```

然后发送原文件。

不要复制一份临时文件。

---

# 9. Clipboard Image

这是 Omasend 的重要场景。

Omarchy 官方截图工作流本身就会：

1. 保存 PNG 到 Pictures
2. 同时把图片写入 Clipboard

官方文档明确描述截图结果同时进入 `~/Pictures` 和 clipboard。([GitHub][5])

所以典型 Omasend 工作流是：

```text
Print Screen
     ↓
Omarchy Screenshot
     ↓
Clipboard: image/png
     ↓
打开 Omasend
     ↓
Ctrl+V
     ↓
Screenshot Preview
     ↓
选择 iPhone
     ↓
Send
```

如果 Clipboard 提供的是：

```text
image/png
```

而不是文件 URI：

Omasend 创建临时文件：

```text
$XDG_RUNTIME_DIR/omasend/
    screenshot-xxxx.png
```

然后：

```rust
SendItem::TemporaryFile(...)
```

发送完成或 item 被移除后清理。

---

# 10. Clipboard Video / Screen Recording

Screen Recording 应采用与 File 相同的模型。

Omarchy 官方录屏/转码工作流本身已经大量围绕 `~/Videos`、文件以及 clipboard file URI 工作。例如 Transcode 完成后会把生成文件的 file URI 写入 clipboard，供支持 file drop 的应用直接粘贴。([GitHub][5])

因此 Omasend 不应该特殊设计一套：

```text
ScreenRecording
```

数据类型。

统一为：

```rust
enum SendItem {
    File(PathBuf),
    TemporaryFile {
        path: PathBuf,
        mime: MimeType,
    },
    Text(String),
}
```

这样：

```text
Screenshot
Screen Recording
Downloaded File
Copied File
Dragged File
```

最终都进入统一的 Send Pipeline。

---

# 11. Paste 行为

窗口任何合理区域获得焦点时：

```text
Ctrl+V
```

都应该尝试读取 Clipboard。

如果 clipboard 是文件：

```text
Ctrl+V

→ Add File
```

如果是图片：

```text
Ctrl+V

→ Materialize temporary PNG
→ Add File
```

如果是视频：

```text
Ctrl+V

→ Add / resolve Video File
```

如果是纯文本：

```text
Ctrl+V

→ Add Text
```

---

# 12. Send Composer

建议不要模仿 LocalSend 当前 GUI。

可以更接近 AirDrop + Omarchy。

例如：

```text
Omasend
────────────────────────────────────

Nearby

  ┌──────────┐   ┌──────────┐
  │    📱    │   │    💻    │
  │  iPhone  │   │ MacBook  │
  └──────────┘   └──────────┘


Ready to Send

┌──────────────────────────────────┐
│                                  │
│         screenshot.png           │
│                                  │
│          [ Preview ]             │
│                                  │
│             2.4 MB               │
└──────────────────────────────────┘

Drop files or paste with Ctrl+V
```

选择设备以后发送。

---

# 13. Screenshot / Video Preview

这里不需要一开始做成复杂媒体浏览器。

## Image

应该直接提供 thumbnail。

```text
┌──────────────────────────┐
│                          │
│       IMAGE PREVIEW      │
│                          │
├──────────────────────────┤
│ screenshot.png    2.4 MB │
└──────────────────────────┘
```

点击可以打开大图 preview。

## Video

v0.1 **不需要内置视频播放器**。

只需要：

```text
┌──────────────────────────┐
│           ▶              │
│                          │
│     screenrecord.mp4     │
│          18 MB           │
└──────────────────────────┘
```

如果能够低成本获取首帧 thumbnail，可以后续增加。

不要为了 Preview 引入复杂视频播放栈。

---

# 14. Drag & Drop

GPUI 应支持：

```text
File Manager
     │
     │ drag
     ▼
  Omasend
```

与 Clipboard 共用：

```rust
add_send_item()
```

因此输入来源统一：

```text
File Picker ─────┐
                 │
Drag & Drop ─────┤
                 ▼
Clipboard ───► SendItem
                 │
Share Action ────┘
```

不要为不同入口写不同传输逻辑。

---

# 15. Folder

Folder 需要支持。

但 LocalSend 协议本身并没有真正的 directory object；目录实际上是多个文件，`fileName` 携带 relative path，空目录因此无法保留。([GitHub][6])

所以：

```text
Folder
  ├── a.png
  └── src/
      └── main.rs
```

内部转换为：

```text
a.png
src/main.rs
```

然后作为一个 transfer session 发送。

---

# 16. 接收

收到 LocalSend 请求：

```text
Jason's iPhone
wants to send

IMG_3021.HEIC
4.8 MB

[ Decline ]        [ Accept ]
```

接受以后：

```text
Receiving
██████████████░░░░ 72%

IMG_3021.HEIC
```

完成：

```text
Received

IMG_3021.HEIC

[ Open ]   [ Show in Files ]
```

---

# 17. 下载目录

默认：

```text
~/Downloads
```

不要：

```text
~/Downloads/Omasend
```

不要创建 Omasend 专属目录。

目标是和 AirDrop 类似：

> 收到的东西就是普通 Downloads 内容。

实际实现不要硬编码 `$HOME/Downloads`，应通过 XDG user dirs 获取 Downloads directory。

Fallback 才使用：

```text
$HOME/Downloads
```

---

# 18. 文件冲突

例如：

```text
~/Downloads/image.png
```

已经存在。

新文件：

```text
image (1).png
image (2).png
```

禁止默认覆盖。

---

# 19. Transfer Progress

Core 层需要提供事件：

```rust
enum TransferEvent {
    IncomingRequest(...),

    Started(...),

    Progress {
        id: TransferId,
        transferred: u64,
        total: u64,
    },

    Completed(...),

    Cancelled(...),

    Failed(...),
}
```

GPUI View 订阅这些事件并：

```text
update state
→ cx.notify()
```

不要让网络线程直接碰 GPUI View。

---

# 20. 架构

第一阶段保持简单。

不需要 daemon。

```text
┌─────────────────────────────────────┐
│               Omasend               │
│                                     │
│  ┌───────────────────────────────┐  │
│  │           GPUI UI             │  │
│  │                               │  │
│  │ Nearby / Composer / Receive   │  │
│  │ Progress / History            │  │
│  └───────────────┬───────────────┘  │
│                  │                  │
│  ┌───────────────▼───────────────┐  │
│  │        Omasend App State      │  │
│  └───────────────┬───────────────┘  │
│                  │                  │
│  ┌───────────────▼───────────────┐  │
│  │       LocalSend Adapter       │  │
│  └───────────────┬───────────────┘  │
│                  │                  │
│  ┌───────────────▼───────────────┐  │
│  │     LocalSend Rust Core       │  │
│  │                               │  │
│  │ Discovery / TLS / Transfer    │  │
│  └───────────────────────────────┘  │
└─────────────────────────────────────┘
```

---

# 21. 推荐目录

不用一开始 workspace 拆成很多 crate。

先保持一个应用：

```text
omasend/
├── Cargo.toml
└── src/
    ├── main.rs
    ├── app.rs
    │
    ├── localsend/
    │   ├── mod.rs
    │   ├── discovery.rs
    │   ├── transfer.rs
    │   └── adapter.rs
    │
    ├── clipboard/
    │   ├── mod.rs
    │   └── wayland.rs
    │
    ├── model/
    │   ├── device.rs
    │   ├── send_item.rs
    │   └── transfer.rs
    │
    └── views/
        ├── home.rs
        ├── device.rs
        ├── composer.rs
        ├── preview.rs
        ├── receive.rs
        └── progress.rs
```

**Less is better。**

等 core 真有独立复用需求再拆 crate。

---

# 22. GPUI / UI

Omasend 必须使用：

```text
GPUI
+
GPUI Kit
+
GPUI Omarchy
```

不要自己重新实现 Omarchy Theme。

应用本身应该成为：

> **GPUI Omarchy 的真实生产应用 / dogfooding 项目。**

因此 Omasend 中遇到的通用 UI 能力缺口：

```text
GPUI Omarchy
```

应该优先补回主题库，而不是在 Omasend 中 hack。

---

# 23. Keyboard First

必须可以基本不用鼠标完成：

```text
Ctrl+V       Paste
Ctrl+O       Select File
Esc          Cancel / Back
Enter        Confirm
↑ ↓ ← →      Device navigation
Tab          Focus navigation
```

设备选择最好支持：

```text
← → ↑ ↓
```

然后：

```text
Enter
```

发送。

这才符合 Omarchy 的使用方式。

---

# 24. v0.1 页面

尽量只有三个主要状态：

### Home

```text
Nearby devices
+
Send Composer
```

### Receiving

```text
Incoming Request
```

### Transfer

```text
Sending / Receiving Progress
```

Settings 不应该成为 v0.1 的重点。

---

# 25. History

v0.1 可以有轻量 session history：

```text
Today

✓ screenshot.png
  → Jason's iPhone
  09:32

✓ IMG_3312.HEIC
  ← Jason's iPhone
  09:21
```

但不要一开始做数据库。

可以先：

```text
in-memory
```

应用退出即消失。

后续确认有价值，再做 persistence。

---

# 26. Security

不要因为追求 MVP 而自行降级 LocalSend 的安全模型。

LocalSend 当前默认通过 HTTPS 传输，并动态生成 TLS certificate。([GitHub][7])

因此原则是：

> **Omasend 应继承官方 Rust core 的 TLS / fingerprint / verification 行为，而不是自己做一套“简化版 HTTP”。**

这也是优先复用官方 core 的重要原因。

---

# 27. 开发顺序

AI Agent **严格按照这个顺序开发**。

[1]: https://github.com/localsend/protocol/blob/main/README.md?utm_source=chatgpt.com "protocol/README.md at main · localsend/protocol · GitHub"
[2]: https://github.com/localsend/localsend/discussions/3271?utm_source=chatgpt.com "v1.18.0 · localsend localsend · Discussion #3271 · GitHub"
[3]: https://github.com/localsend/localsend/blob/main/packages/core/src/http/dto_v2.rs?utm_source=chatgpt.com "localsend/packages/core/src/http/dto_v2.rs at main · localsend/localsend · GitHub"
[4]: https://github.com/basecamp/omarchy/issues/8340?utm_source=chatgpt.com "omarchy share clipboard sends an empty file when the clipboard holds an image (e.g. a screenshot) · Issue #8340 · basecamp/omarchy · GitHub"
[5]: https://github.com/omacom/omarchy/blob/quattro/manual/12-screenshots-recording.md?utm_source=chatgpt.com "omarchy/manual/12-screenshots-recording.md at quattro · omacom/omarchy · GitHub"
[6]: https://github.com/zebroc/localsend-cli?utm_source=chatgpt.com "GitHub - zebroc/localsend-cli: CLI server & client for localsend in go · GitHub"
[7]: https://github.com/localsend/localsend/blob/main/README.md?plain=1&utm_source=chatgpt.com "localsend/README.md at main · localsend/localsend · GitHub"
