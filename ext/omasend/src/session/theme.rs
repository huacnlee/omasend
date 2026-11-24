//! The theme mode the user picked: "system", "light" or "dark". An Omarchy
//! installation always follows its system theme.

/// Whether this is an Omarchy installation, whose theme the app follows.
pub(crate) fn omarchy() -> bool {
    cfg!(target_os = "linux")
        && std::fs::read_to_string("/etc/os-release").is_ok_and(|s| omarchy_os_release(&s))
}

/// The saved mode, "system" when none (or on Omarchy).
pub(crate) fn load() -> &'static str {
    if omarchy() {
        return "system";
    }
    parse(
        super::preferences::read("theme")
            .as_deref()
            .unwrap_or_default(),
    )
}

/// Saves `mode`; anything but "light" and "dark" is "system".
pub(crate) fn save(mode: &str) -> anyhow::Result<()> {
    super::preferences::write("theme", parse(mode))
}

fn parse(value: &str) -> &'static str {
    match value.trim() {
        "light" => "light",
        "dark" => "dark",
        _ => "system",
    }
}

fn omarchy_os_release(contents: &str) -> bool {
    contents.lines().any(|line| {
        line.split_once('=').is_some_and(|(key, value)| {
            key.trim() == "ID" && value.trim().trim_matches(['\'', '"']) == "omarchy"
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modes_parse_to_system_unless_light_or_dark() {
        assert_eq!(parse("light"), "light");
        assert_eq!(parse(" dark\n"), "dark");
        assert_eq!(parse("solarized"), "system");
        assert!(omarchy_os_release("NAME=Omarchy\nID=\"omarchy\"\n"));
        assert!(!omarchy_os_release("ID=arch\nID_LIKE=omarchy\n"));
    }
}
