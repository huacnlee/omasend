use omasend::{
    clipboard::{ClipboardKind, read_offer, select_mime},
    model::SendItem,
};

#[test]
fn binary_clipboard_is_not_misread_as_text() {
    assert_eq!(
        select_mime(&["text/plain", "image/png"]).unwrap(),
        ClipboardKind::Image("image/png".into())
    );
    assert_eq!(
        select_mime(&["text/plain", "video/mp4"]).unwrap(),
        ClipboardKind::Video("video/mp4".into())
    );
    assert_eq!(
        select_mime(&["image/png", "text/uri-list"]).unwrap(),
        ClipboardKind::Files
    );
}

#[test]
fn file_uris_resolve_original_files_without_materializing_copies() {
    let root = tempfile::tempdir().unwrap();
    let file = root.path().join("photo with space.png");
    std::fs::write(&file, b"png").unwrap();
    let uri = url::Url::from_file_path(&file).unwrap();
    let items = read_offer(
        ClipboardKind::Files,
        format!("# copied files\r\n{uri}\r\n").as_bytes(),
        root.path(),
    )
    .unwrap();
    assert!(matches!(&items[0], SendItem::File(path) if path == &file));
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 1);
}

#[test]
fn remote_file_uri_and_empty_binary_clipboard_are_rejected() {
    let root = tempfile::tempdir().unwrap();
    assert!(
        read_offer(
            ClipboardKind::Files,
            b"file://remote/etc/passwd",
            root.path()
        )
        .is_err()
    );
    assert!(
        read_offer(
            ClipboardKind::Files,
            b"https://example.com/photo.png",
            root.path()
        )
        .is_err()
    );
    assert!(read_offer(ClipboardKind::Image("image/png".into()), b"", root.path()).is_err());
}

#[test]
fn temporary_binary_survives_while_upload_owns_it_and_cleans_up_afterward() {
    let root = tempfile::tempdir().unwrap();
    let items = read_offer(
        ClipboardKind::Video("video/mp4".into()),
        b"\0\0\0\x18ftypmp42\xff",
        root.path(),
    )
    .unwrap();
    let uploads = items[0].uploads().unwrap();
    let path = items[0].path().unwrap().to_owned();
    assert_eq!(std::fs::read(&path).unwrap(), b"\0\0\0\x18ftypmp42\xff");
    assert_eq!(uploads[0].offer.mime, "video/mp4");
    drop(items);
    assert!(path.exists());
    drop(uploads);
    assert!(!path.exists());
}

#[test]
fn text_preserves_newlines_and_folders_keep_relative_paths() {
    let root = tempfile::tempdir().unwrap();
    let text = read_offer(
        ClipboardKind::Text("text/plain".into()),
        b"hello\n\n",
        root.path(),
    )
    .unwrap();
    assert!(matches!(&text[0], SendItem::Text(value) if value == "hello\n\n"));
    std::fs::create_dir_all(root.path().join("src")).unwrap();
    std::fs::write(root.path().join("a.png"), b"a").unwrap();
    std::fs::write(root.path().join("src/main.rs"), b"fn main() {}").unwrap();
    let files = SendItem::File(root.path().into()).uploads().unwrap();
    let names: Vec<_> = files.iter().map(|file| file.offer.name.as_str()).collect();
    assert_eq!(names, vec!["a.png", "src/main.rs"]);
}

#[cfg(unix)]
#[test]
fn folder_symlinks_cannot_silently_send_outside_content() {
    let root = tempfile::tempdir().unwrap();
    std::os::unix::fs::symlink("/etc/passwd", root.path().join("secret")).unwrap();
    assert!(SendItem::File(root.path().into()).uploads().is_err());
}
