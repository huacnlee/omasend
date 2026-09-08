//! Application language catalogs. User content and protocol metadata remain untouched.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Language {
    En,
    ZhCn,
}
impl Language {
    pub fn from_locale(locale: &str) -> Self {
        let locale = locale.to_ascii_lowercase().replace('_', "-");
        if locale == "zh"
            || locale.starts_with("zh-cn")
            || locale.starts_with("zh-sg")
            || locale.starts_with("zh-hans")
        {
            Self::ZhCn
        } else {
            Self::En
        }
    }
    pub fn system() -> Self {
        for key in ["LC_ALL", "LC_MESSAGES", "LANG"] {
            if let Ok(value) = std::env::var(key)
                && !value.is_empty()
            {
                return Self::from_locale(&value);
            }
        }
        #[cfg(target_os = "macos")]
        if let Ok(output) = std::process::Command::new("defaults")
            .args(["read", "-g", "AppleLocale"])
            .output()
            && output.status.success()
        {
            return Self::from_locale(String::from_utf8_lossy(&output.stdout).trim());
        }
        Self::En
    }
    pub fn text(self, key: &str) -> &str {
        if self == Self::En {
            return key;
        }
        CATALOG
            .iter()
            .find(|(en, _)| *en == key)
            .map(|(_, zh)| *zh)
            .unwrap_or(key)
    }
    pub fn named(self, key: &str, value: &str) -> String {
        // Substitute only the catalog template, never re-interpret user content.
        self.text(key).replacen("{name}", value, 1)
    }
    pub fn count(self, count: usize, singular: &str, plural: &str) -> String {
        self.text(if count == 1 { singular } else { plural })
            .replacen("{count}", &count.to_string(), 1)
    }
    /// Transfer failures contain transport internals; only show actionable summaries.
    pub fn transfer_error(self, error: &str) -> String {
        let details = error.to_ascii_lowercase();
        let message = if details.contains("certificate") || details.contains("tls") {
            "Could not verify the receiver. Check its identity and try again"
        } else if details.contains("timed out") || details.contains("timeout") {
            "The receiver did not respond. Try again"
        } else if details.contains("error sending request")
            || details.contains("connection refused")
            || details.contains("connection reset")
            || details.contains("connection closed")
            || details.contains("network is unreachable")
            || details.contains("dns error")
        {
            "Cannot connect to the receiver. Make sure it is online and try again"
        } else if CATALOG.iter().any(|(key, _)| *key == error) {
            error
        } else {
            let formatted = self.error(error);
            if error.split_once(';').is_some_and(|(code, _)| {
                code.parse::<u16>()
                    .is_ok_and(|code| (400..600).contains(&code))
            }) {
                return formatted;
            }
            "Transfer failed. View logs for details"
        };
        self.text(message).to_owned()
    }

    pub fn error(self, error: &str) -> String {
        // The protocol core formats HTTP errors as `409;Some("...")`.
        // Never expose remote response bodies or Rust debug wrappers in UI.
        if let Some((code, _)) = error.split_once(';')
            && let Ok(code) = code.parse::<u16>()
            && (400..600).contains(&code)
        {
            let message = match code {
                401 => "The receiver requires a PIN",
                403 => "The receiver declined the transfer",
                409 => "The receiver is busy. Try again shortly",
                422 => "File verification failed. Try sending again",
                _ => "Transfer failed. View logs for details",
            };
            return self.text(message).to_owned();
        }
        error
            .split(": ")
            .map(|part| self.text(part))
            .collect::<Vec<_>>()
            .join(": ")
    }
}

const CATALOG: &[(&str, &str)] = &[
    ("Done", "完成"),
    ("Average speed", "平均速度"),
    (
        "Cannot connect to the receiver. Make sure it is online and try again",
        "无法连接接收方，请确认对方在线后重试",
    ),
    (
        "The receiver did not respond. Try again",
        "接收方响应超时，请重试",
    ),
    (
        "Could not verify the receiver. Check its identity and try again",
        "无法验证接收方身份，请确认对方身份后重试",
    ),
    ("Install {name}…", "安装 {name}…"),
    ("Downloading update…", "正在下载更新…"),
    ("Installing update…", "正在安装更新…"),
    ("Restart to update", "重启以完成更新"),
    ("Update failed · Retry", "更新失败 · 重试"),
    (
        "Finish transfers and clear Outbox before restarting",
        "请先完成传输并清空待发送内容，再重启",
    ),
    ("Check for update", "检查更新"),
    ("Checking for updates…", "正在检查更新…"),
    ("Up to date", "已是最新版本"),
    ("No releases yet", "暂无发布版本"),
    ("Update check failed", "检查更新失败"),
    ("About…", "关于…"),
    ("Theme", "主题"),
    ("Language", "语言"),
    ("System", "跟随系统"),
    ("Light", "浅色"),
    ("Dark", "深色"),
    ("Connecting…", "正在连接…"),
    ("Disconnected", "未连接"),
    ("Searching for devices…", "正在搜索设备…"),
    ("Connected", "已连接"),
    ("The receiver requires a PIN", "接收方需要 PIN 码"),
    ("The receiver declined the transfer", "接收方拒绝了传输"),
    (
        "The receiver is busy. Try again shortly",
        "接收方正忙，请稍后重试",
    ),
    (
        "File verification failed. Try sending again",
        "文件校验失败，请重新发送",
    ),
    (
        "Transfer failed. View logs for details",
        "传输失败，可查看日志了解详情",
    ),
    ("Logs…", "日志…"),
    ("Logs", "日志"),
    ("Copy", "复制"),
    ("No logs yet", "暂无日志"),
    ("Copied", "已复制"),
    ("to {name}", "至 {name}"),
    ("To {name}", "发送至 {name}"),
    ("From {name}", "来自 {name}"),
    ("Choose a nearby device", "请选择附近的设备"),
    ("Nothing added yet", "尚未添加内容"),
    ("Transfer history", "传输记录"),
    ("Clear", "清空"),
    ("Clear transfer history", "清空传输记录"),
    ("Menu", "菜单"),
    ("Paste", "粘贴"),
    ("Add files…", "添加文件…"),
    ("Add files", "添加文件"),
    ("Exit", "退出"),
    ("Retry", "重试"),
    ("Dismiss", "关闭提示"),
    (
        "Nearby device identity could not be verified",
        "无法验证附近设备的身份，请检查对方是否重启或更换了应用",
    ),
    (
        "Could not connect to nearby device",
        "无法连接附近设备，请检查对方应用和网络连接",
    ),
    ("Outbox", "待发送"),
    ("Preparing…", "正在准备…"),
    ("Send", "发送"),
    ("Preview…", "预览…"),
    ("Preview", "预览"),
    ("Remove", "移除"),
    ("Cancel", "取消"),
    ("Open", "打开"),
    ("Show in Files", "显示位置"),
    ("Decline", "拒绝"),
    ("Accept", "接受"),
    ("Close", "关闭"),
    ("Starting…", "正在启动…"),
    ("Waiting for receiver", "等待对方接受"),
    ("Sending", "正在发送"),
    ("Receiving", "正在接收"),
    ("Sent", "已发送"),
    ("Received", "已接收"),
    ("Cancelled", "已取消"),
    ("Failed", "失败"),
    ("Nearby", "附近设备"),
    ("Expand", "展开"),
    ("Collapse", "收起"),
    ("Show earlier", "展开更早记录"),
    ("This session", "本次会话"),
    ("You", "本机"),
    ("No transfers yet", "暂无传输记录"),
    ("Drop something here", "将内容拖到这里"),
    (
        "Files, folders, images or a few words.",
        "文件、文件夹、图片或文字。",
    ),
    (
        "Looking for devices… Open LocalSend on the same Wi-Fi.",
        "正在查找设备… 请在同一 Wi-Fi 下打开 LocalSend。",
    ),
    ("Save to your Downloads folder", "保存到下载文件夹"),
    ("Add to Omasend", "添加到 Omasend"),
    ("Clipboard is empty", "剪贴板为空"),
    (
        "Copy the image as PNG or JPEG",
        "请将图片复制为 PNG 或 JPEG",
    ),
    ("Network task stopped", "网络任务已停止"),
    ("File preparation stopped", "文件准备任务已停止"),
    ("Clipboard task stopped", "剪贴板任务已停止"),
    ("Select {name}", "选择 {name}"),
    ("Send to {name}", "发送到 {name}"),
    ("{name} wants to send", "{name} 想要发送文件"),
    ("{count} item", "{count} 项"),
    ("{count} items", "{count} 项"),
    ("{count} file", "{count} 个文件"),
    ("{count} files", "{count} 个文件"),
    (
        "XDG_RUNTIME_DIR is not set; start Omasend in your Wayland session",
        "未设置 XDG_RUNTIME_DIR；请在 Wayland 会话中启动 Omasend",
    ),
    (
        "Clipboard temporary directory is not a directory",
        "剪贴板临时路径不是目录",
    ),
    (
        "Clipboard temporary directory must be private (mode 700)",
        "剪贴板临时目录必须为私有目录（权限 700）",
    ),
    ("Clipboard did not respond", "剪贴板未响应"),
    (
        "Install wl-clipboard to paste from Wayland",
        "请安装 wl-clipboard 以读取 Wayland 剪贴板",
    ),
    (
        "Cannot read clipboard; copy something first",
        "无法读取剪贴板，请先复制内容",
    ),
    ("Clipboard media read timed out", "读取剪贴板媒体超时"),
    (
        "Clipboard media is empty or changed; copy it again",
        "剪贴板媒体为空或已变化，请重新复制",
    ),
    ("Cannot read clipboard output", "无法读取剪贴板输出"),
    ("Clipboard text read timed out", "读取剪贴板文本超时"),
    (
        "Clipboard text exceeds 16 MB; send it as a file",
        "剪贴板文本超过 16 MB，请作为文件发送",
    ),
    (
        "Clipboard changed; copy the content again",
        "剪贴板已变化，请重新复制内容",
    ),
    ("Clipboard text is empty", "剪贴板文本为空"),
    (
        "Choose the original file instead of a symbolic link",
        "请选择原始文件，而非符号链接",
    ),
    (
        "This folder contains no files; LocalSend cannot transfer empty folders",
        "该文件夹没有文件；LocalSend 无法传输空文件夹",
    ),
    (
        "Select at most 10,000 files per transfer",
        "每次传输最多选择 10,000 个文件",
    ),
    ("Folder nesting exceeds 128 levels", "文件夹嵌套超过 128 层"),
    ("File name is not valid UTF-8", "文件名不是有效的 UTF-8"),
    ("Missing file name", "缺少文件名"),
    ("Invalid file name", "文件名无效"),
    ("Unsafe relative file path", "相对文件路径不安全"),
    ("Absolute file paths are not accepted", "不接受绝对文件路径"),
    ("Sender stopped responding", "发送方停止响应"),
    ("Transfer cancelled", "传输已取消"),
    ("File checksum did not match", "文件校验失败"),
    (
        "Received more data than offered",
        "接收的数据超出了声明大小",
    ),
    (
        "Clipboard has no supported files, images, videos or text",
        "剪贴板中没有支持的文件、图片、视频或文本",
    ),
    (
        "Clipboard is empty; copy the content again",
        "剪贴板为空，请重新复制内容",
    ),
    ("File clipboard is not UTF-8", "剪贴板文件列表不是 UTF-8"),
    ("Invalid clipboard file URI", "剪贴板文件 URI 无效"),
    (
        "Clipboard URI is not a local file",
        "剪贴板 URI 不是本地文件",
    ),
    (
        "File URI has a query or fragment",
        "文件 URI 包含查询或片段",
    ),
    ("Invalid clipboard file path", "剪贴板文件路径无效"),
    ("Clipboard contains no local files", "剪贴板中没有本地文件"),
    ("Clipboard text is not UTF-8", "剪贴板文本不是 UTF-8"),
    ("Unsupported clipboard media type", "不支持该剪贴板媒体类型"),
    ("Add something to send first", "请先添加要发送的内容"),
    ("The receiver did not respond", "接收方未响应"),
    ("The receiver accepted no files", "接收方未接受任何文件"),
    (
        "Receiver returned an unknown file ID",
        "接收方返回了未知的文件 ID",
    ),
    ("Transfer is too large", "传输内容过大"),
    ("Network service stopped", "网络服务已停止"),
    ("Cannot create Downloads folder", "无法创建下载文件夹"),
    ("Cannot locate your Downloads folder", "无法找到下载文件夹"),
    (
        "Cannot listen for LocalSend transfers; another app may be using port 53317",
        "无法监听 LocalSend 传输；其他应用可能正在使用端口 53317",
    ),
    (
        "Multicast unavailable; trying local network discovery",
        "组播不可用，正在尝试局域网发现",
    ),
    ("Transfer task stopped", "传输任务已停止"),
    ("An encrypted connection is required", "需要加密连接"),
    (
        "Offer must contain between 1 and 10,000 files",
        "传输请求必须包含 1 至 10,000 个文件",
    ),
    (
        "File ID does not match its metadata",
        "文件 ID 与元数据不匹配",
    ),
    ("Folder contains a symbolic link", "文件夹包含符号链接"),
    ("Folder contains a special file", "文件夹包含特殊文件"),
    ("Clipboard file no longer exists", "剪贴板中的文件已不存在"),
];

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn transfer_errors_hide_transport_details_in_both_languages() {
        let request = "error sending request for url (https://127.0.0.1:53317/api/localsend/v2/prepare-upload): client error (Connect): connection refused";
        for (raw, en, zh) in [
            (
                request,
                "Cannot connect to the receiver. Make sure it is online and try again",
                "无法连接接收方，请确认对方在线后重试",
            ),
            (
                "error sending request: operation timed out",
                "The receiver did not respond. Try again",
                "接收方响应超时，请重试",
            ),
            (
                "error sending request: invalid peer certificate",
                "Could not verify the receiver. Check its identity and try again",
                "无法验证接收方身份，请确认对方身份后重试",
            ),
            (
                "409;Some(\"Blocked by another session\")",
                "The receiver is busy. Try again shortly",
                "接收方正忙，请稍后重试",
            ),
            (
                "unrecognized internal details",
                "Transfer failed. View logs for details",
                "传输失败，可查看日志了解详情",
            ),
            (
                "Sender stopped responding",
                "Sender stopped responding",
                "发送方停止响应",
            ),
        ] {
            assert_eq!(Language::En.transfer_error(raw), en);
            assert_eq!(Language::ZhCn.transfer_error(raw), zh);
        }
    }

    #[test]
    fn protocol_errors_hide_raw_response_details() {
        let raw = "409;Some(\"Blocked by another session\")";
        assert_eq!(
            Language::En.error(raw),
            "The receiver is busy. Try again shortly"
        );
        assert_eq!(Language::ZhCn.error(raw), "接收方正忙，请稍后重试");
        assert_eq!(
            Language::En.error("500;Some(\"internal details\")"),
            "Transfer failed. View logs for details"
        );
    }

    #[test]
    fn locale_fallback_and_user_content_are_predictable() {
        for locale in ["zh-CN", "zh_CN.UTF-8", "zh-Hans-CN", "zh_SG"] {
            assert_eq!(Language::from_locale(locale), Language::ZhCn);
        }
        for locale in ["en", "en_US.UTF-8", "ja-JP", "C", ""] {
            assert_eq!(Language::from_locale(locale), Language::En);
        }
        assert_eq!(
            Language::ZhCn.named("Send to {name}", "My {name} iPhone"),
            "发送到 My {name} iPhone"
        );
        assert_eq!(
            Language::En.count(1, "{count} item", "{count} items"),
            "1 item"
        );
        assert_eq!(
            Language::En.count(2, "{count} item", "{count} items"),
            "2 items"
        );
        assert_eq!(
            Language::ZhCn.count(2, "{count} item", "{count} items"),
            "2 项"
        );
        assert_eq!(
            Language::ZhCn.error("Network task stopped: OS error 7"),
            "网络任务已停止: OS error 7"
        );
    }
    #[test]
    fn catalogs_cover_ui_keys_and_preserve_placeholders() {
        let mut seen = std::collections::HashSet::new();
        for (en, zh) in CATALOG {
            assert!(seen.insert(en), "duplicate catalog key: {en}");
            assert!(!zh.is_empty());
            for placeholder in ["{name}", "{count}"] {
                assert_eq!(
                    en.matches(placeholder).count(),
                    zh.matches(placeholder).count(),
                    "{en}"
                );
            }
        }
        for source in [
            include_str!("views/home.rs"),
            include_str!("views/panels.rs"),
        ] {
            for suffix in source.split("self.language.text(\"").skip(1) {
                let key = suffix.split('"').next().unwrap();
                assert!(
                    CATALOG.iter().any(|(en, _)| *en == key),
                    "missing translation: {key}"
                );
            }
        }
    }
}
