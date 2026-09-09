//! Application language selection.
//!
//! The catalogs live in `locales/app.yml` and are owned by rust-i18n; this
//! module decides which locale is active and maps the raw error text produced
//! deeper in the application onto translation keys. User content and protocol
//! metadata remain untouched.
use rust_i18n::t;

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
    /// The rust-i18n locale code for this language.
    pub fn code(self) -> &'static str {
        match self {
            Self::En => "en",
            Self::ZhCn => "zh-CN",
        }
    }
    /// Makes this the locale every bare `t!` resolves against.
    pub fn activate(self) {
        rust_i18n::set_locale(self.code());
    }
    pub fn text(self, key: &str) -> String {
        t!(key, locale = self.code()).into_owned()
    }
    pub fn named(self, key: &str, value: &str) -> String {
        // Substitute only the catalog template, never re-interpret user content.
        t!(key, locale = self.code(), name = value).into_owned()
    }
    pub fn count(self, count: usize, singular: &str, plural: &str) -> String {
        t!(
            if count == 1 { singular } else { plural },
            locale = self.code(),
            count = count
        )
        .into_owned()
    }
    /// Transfer failures contain transport internals; only show actionable summaries.
    pub fn transfer_error(self, error: &str) -> String {
        let details = error.to_ascii_lowercase();
        let key = if details.contains("certificate") || details.contains("tls") {
            "error.peer_identity"
        } else if details.contains("timed out") || details.contains("timeout") {
            "error.peer_timeout"
        } else if details.contains("error sending request")
            || details.contains("connection refused")
            || details.contains("connection reset")
            || details.contains("connection closed")
            || details.contains("network is unreachable")
            || details.contains("dns error")
        {
            "error.peer_unreachable"
        } else if let Some(key) = error_key(error) {
            key
        } else {
            let formatted = self.error(error);
            if error.split_once(';').is_some_and(|(code, _)| {
                code.parse::<u16>()
                    .is_ok_and(|code| (400..600).contains(&code))
            }) {
                return formatted;
            }
            "error.transfer_generic"
        };
        self.text(key)
    }

    pub fn error(self, error: &str) -> String {
        // The protocol core formats HTTP errors as `409;Some("...")`.
        // Never expose remote response bodies or Rust debug wrappers in UI.
        if let Some((code, _)) = error.split_once(';')
            && let Ok(code) = code.parse::<u16>()
            && (400..600).contains(&code)
        {
            let key = match code {
                401 => "error.peer_pin",
                403 => "error.peer_declined",
                409 => "error.peer_busy",
                422 => "error.checksum_retry",
                _ => "error.transfer_generic",
            };
            return self.text(key);
        }
        error
            .split(": ")
            .map(|part| match error_key(part) {
                Some(key) => self.text(key),
                None => part.to_owned(),
            })
            .collect::<Vec<_>>()
            .join(": ")
    }
}

/// The translation key for an error phrase produced inside the application.
///
/// Errors travel as English text so that logs stay readable; only the phrases
/// listed here are ever shown to the user, and anything else passes through.
fn error_key(phrase: &str) -> Option<&'static str> {
    ERROR_KEYS
        .iter()
        .find(|(source, _)| *source == phrase)
        .map(|(_, key)| *key)
}

const ERROR_KEYS: &[(&str, &str)] = &[
    (
        "Cannot connect to the receiver. Make sure it is online and try again",
        "error.peer_unreachable",
    ),
    (
        "The receiver did not respond. Try again",
        "error.peer_timeout",
    ),
    (
        "Could not verify the receiver. Check its identity and try again",
        "error.peer_identity",
    ),
    ("The receiver requires a PIN", "error.peer_pin"),
    ("The receiver declined the transfer", "error.peer_declined"),
    ("The receiver is busy. Try again shortly", "error.peer_busy"),
    (
        "File verification failed. Try sending again",
        "error.checksum_retry",
    ),
    (
        "Transfer failed. View logs for details",
        "error.transfer_generic",
    ),
    ("Clipboard is empty", "error.clipboard_empty"),
    (
        "Copy the image as PNG or JPEG",
        "error.clipboard_image_format",
    ),
    ("Network task stopped", "error.network_task"),
    ("File preparation stopped", "error.file_task"),
    ("Clipboard task stopped", "error.clipboard_task"),
    (
        "XDG_RUNTIME_DIR is not set; start Omasend in your Wayland session",
        "error.no_runtime_dir",
    ),
    (
        "Clipboard temporary directory is not a directory",
        "error.clipboard_tmp_kind",
    ),
    (
        "Clipboard temporary directory must be private (mode 700)",
        "error.clipboard_tmp_mode",
    ),
    ("Clipboard did not respond", "error.clipboard_timeout"),
    (
        "Install wl-clipboard to paste from Wayland",
        "error.wl_clipboard_missing",
    ),
    (
        "Cannot read clipboard; copy something first",
        "error.clipboard_unreadable",
    ),
    (
        "Clipboard media read timed out",
        "error.clipboard_media_timeout",
    ),
    (
        "Clipboard media is empty or changed; copy it again",
        "error.clipboard_media_changed",
    ),
    ("Cannot read clipboard output", "error.clipboard_output"),
    (
        "Clipboard text read timed out",
        "error.clipboard_text_timeout",
    ),
    (
        "Clipboard text exceeds 16 MB; send it as a file",
        "error.clipboard_text_large",
    ),
    (
        "Clipboard changed; copy the content again",
        "error.clipboard_changed",
    ),
    ("Clipboard text is empty", "error.clipboard_text_empty"),
    (
        "Choose the original file instead of a symbolic link",
        "error.symlink",
    ),
    (
        "This folder contains no files; LocalSend cannot transfer empty folders",
        "error.empty_folder",
    ),
    (
        "Select at most 10,000 files per transfer",
        "error.too_many_files",
    ),
    ("Folder nesting exceeds 128 levels", "error.deep_folder"),
    ("File name is not valid UTF-8", "error.name_utf8"),
    ("Missing file name", "error.name_missing"),
    ("Invalid file name", "error.name_invalid"),
    ("Unsafe relative file path", "error.path_unsafe"),
    (
        "Absolute file paths are not accepted",
        "error.path_absolute",
    ),
    ("Sender stopped responding", "error.sender_stopped"),
    ("Transfer cancelled", "error.cancelled"),
    ("File checksum did not match", "error.checksum"),
    ("Received more data than offered", "error.overlong"),
    (
        "Clipboard has no supported files, images, videos or text",
        "error.clipboard_unsupported",
    ),
    (
        "Clipboard is empty; copy the content again",
        "error.clipboard_empty_retry",
    ),
    ("File clipboard is not UTF-8", "error.clipboard_list_utf8"),
    ("Invalid clipboard file URI", "error.clipboard_uri"),
    (
        "Clipboard URI is not a local file",
        "error.clipboard_uri_remote",
    ),
    (
        "File URI has a query or fragment",
        "error.clipboard_uri_extra",
    ),
    ("Invalid clipboard file path", "error.clipboard_path"),
    (
        "Clipboard contains no local files",
        "error.clipboard_no_files",
    ),
    ("Clipboard text is not UTF-8", "error.clipboard_text_utf8"),
    (
        "Unsupported clipboard media type",
        "error.clipboard_media_type",
    ),
    ("The receiver did not respond", "error.no_response"),
    ("The receiver accepted no files", "error.no_files_accepted"),
    (
        "Receiver returned an unknown file ID",
        "error.unknown_file_id",
    ),
    ("Transfer is too large", "error.too_large"),
    ("Network service stopped", "error.network_service"),
    ("Cannot create Downloads folder", "error.downloads_create"),
    (
        "Cannot locate your Downloads folder",
        "error.downloads_missing",
    ),
    (
        "Cannot listen for LocalSend transfers; another app may be using port 53317",
        "error.port_in_use",
    ),
    (
        "Multicast unavailable; trying local network discovery",
        "error.multicast",
    ),
    ("Transfer task stopped", "error.transfer_task"),
    (
        "An encrypted connection is required",
        "error.encryption_required",
    ),
    (
        "Offer must contain between 1 and 10,000 files",
        "error.offer_range",
    ),
    (
        "File ID does not match its metadata",
        "error.file_id_mismatch",
    ),
    ("Folder contains a symbolic link", "error.folder_symlink"),
    ("Folder contains a special file", "error.folder_special"),
    (
        "Clipboard file no longer exists",
        "error.clipboard_file_gone",
    ),
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
        assert_eq!(
            Language::ZhCn.error("Network task stopped: OS error 7"),
            "网络任务已停止: OS error 7"
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
        // A device alias that looks like a template must survive verbatim.
        assert_eq!(
            Language::ZhCn.named("action.send_to", "My %{name} iPhone"),
            "发送到 My %{name} iPhone"
        );
        assert_eq!(
            Language::En.count(1, "history.item_one", "history.item_other"),
            "1 item"
        );
        assert_eq!(
            Language::En.count(2, "history.item_one", "history.item_other"),
            "2 items"
        );
        assert_eq!(
            Language::ZhCn.count(2, "history.item_one", "history.item_other"),
            "2 项"
        );
    }

    /// `key: {locale: text}` parsed from the generated `_version: 2` catalog.
    fn catalog() -> Vec<(String, Vec<(String, String)>)> {
        let mut entries: Vec<(String, Vec<(String, String)>)> = Vec::new();
        for line in include_str!("../locales/app.yml").lines() {
            if line.starts_with('#') || line.trim().is_empty() || line.starts_with("_version") {
                continue;
            }
            if let Some(key) = line.strip_suffix(':').filter(|key| !key.starts_with(' ')) {
                entries.push((key.to_owned(), Vec::new()));
            } else if let Some((locale, text)) = line.trim().split_once(": ") {
                let text = text.trim_matches('"').to_owned();
                entries
                    .last_mut()
                    .expect("a locale line before its key")
                    .1
                    .push((locale.to_owned(), text));
            }
        }
        entries
    }

    #[test]
    fn catalog_covers_every_locale_and_preserves_placeholders() {
        let entries = catalog();
        assert!(entries.len() > 100, "catalog looks truncated");
        let mut seen = std::collections::HashSet::new();
        for (key, locales) in &entries {
            assert!(seen.insert(key), "duplicate catalog key: {key}");
            let text = |wanted: &str| {
                locales
                    .iter()
                    .find(|(locale, _)| locale == wanted)
                    .unwrap_or_else(|| panic!("{key} has no {wanted} entry"))
                    .1
                    .clone()
            };
            let (en, zh) = (text("en"), text("zh-CN"));
            assert!(!zh.is_empty());
            for placeholder in ["%{name}", "%{count}"] {
                assert_eq!(
                    en.matches(placeholder).count(),
                    zh.matches(placeholder).count(),
                    "{key}"
                );
            }
        }
    }

    /// The interface sources, so a lookup can only reach a key that exists.
    const INTERFACE: &[&str] = &[
        include_str!("views/home.rs"),
        include_str!("views/panels.rs"),
        include_str!("views/about.rs"),
        include_str!("views/history.rs"),
        include_str!("views/logs.rs"),
        include_str!("views/nearby.rs"),
        include_str!("views/updates.rs"),
    ];

    /// Every key the interface asks for by name, whichever lookup it uses.
    fn keys_in(source: &str) -> Vec<&str> {
        let mut keys = Vec::new();
        for opening in ["t!(\"", ".text(\"", ".named(\""] {
            for (at, matched) in source.match_indices(opening) {
                // `format!("…")` ends in the same three characters as `t!("`.
                if opening.starts_with('t')
                    && source[..at]
                        .chars()
                        .next_back()
                        .is_some_and(|char| char.is_alphanumeric() || char == '_')
                {
                    continue;
                }
                keys.push(source[at + matched.len()..].split('"').next().unwrap());
            }
        }
        keys
    }

    #[test]
    fn no_interface_string_still_carries_a_catalog_placeholder() {
        // A key left behind by the move to rust-i18n reaches the interface as
        // itself, so `{name}` renders literally instead of the device alias.
        for source in INTERFACE {
            for placeholder in ["{name}", "{count}"] {
                assert!(
                    !source.contains(placeholder),
                    "{placeholder} is a catalog placeholder, not interface text"
                );
            }
        }
    }

    #[test]
    fn every_key_used_by_the_interface_is_translated() {
        let known: std::collections::HashSet<String> =
            catalog().into_iter().map(|(key, _)| key).collect();
        for source in [
            include_str!("views/home.rs"),
            include_str!("views/panels.rs"),
            include_str!("views/about.rs"),
            include_str!("views/history.rs"),
            include_str!("views/logs.rs"),
            include_str!("views/nearby.rs"),
            include_str!("views/updates.rs"),
        ] {
            for key in keys_in(source) {
                assert!(known.contains(key), "missing translation: {key}");
            }
        }
    }
}
