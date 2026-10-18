//! Atomic preferences beside the existing application configuration.
use omasend::i18n::Language;
use std::path::Path;

pub(super) fn read(key: &str) -> Option<String> {
    dirs::config_dir().and_then(|dir| std::fs::read_to_string(dir.join("omasend").join(key)).ok())
}

pub(super) fn write(key: &str, value: &str) -> anyhow::Result<()> {
    if let Some(directory) = dirs::config_dir() {
        write_at(&directory.join("omasend").join(key), value)?;
    }
    Ok(())
}

pub(super) fn load_language() -> Language {
    read("language")
        .map(|value| Language::from_locale(value.trim()))
        .unwrap_or_else(Language::system)
}

pub(super) fn save_language(language: Language) -> anyhow::Result<()> {
    write("language", language.code())
}

fn write_at(path: &Path, value: &str) -> anyhow::Result<()> {
    use std::io::Write;
    let parent = path.parent().expect("preference has a directory");
    std::fs::create_dir_all(parent)?;
    let mut file = tempfile::NamedTempFile::new_in(parent)?;
    file.write_all(value.as_bytes())?;
    file.persist(path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preferences_replace_atomically_and_preserve_language_codes() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("config/language");
        for language in [Language::ZhCn, Language::En] {
            write_at(&path, language.code()).unwrap();
            let stored = std::fs::read_to_string(&path).unwrap();
            assert_eq!(Language::from_locale(&stored), language);
        }
    }
}
