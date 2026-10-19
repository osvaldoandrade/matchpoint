use std::io::Write;

#[test]
fn uninstall_removes_only_manifest_paths() {
    let root = tempfile::tempdir().unwrap();
    let tree = root.path().join("usr/local/pgm-data/bin");
    std::fs::create_dir_all(&tree).unwrap();
    let bin = tree.join("pgm-data");
    std::fs::write(&bin, b"bin").unwrap();
    let foreign = root.path().join("usr/local/bin/other");
    std::fs::create_dir_all(foreign.parent().unwrap()).unwrap();
    std::fs::write(&foreign, b"keep").unwrap();
    let data = root.path().join("data");
    std::fs::create_dir_all(&data).unwrap();
    std::fs::write(data.join("job.sqlite"), b"db").unwrap();
    let manifest = root.path().join("manifest.txt");
    let mut file = std::fs::File::create(&manifest).unwrap();
    writeln!(file, "/usr/local/pgm-data/bin/pgm-data").unwrap();
    let script = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/packaging/macos/scripts/uninstall.sh"
    );
    let status = std::process::Command::new("bash")
        .arg(script)
        .env("PGM_UNINSTALL_ROOT", root.path())
        .env("PGM_UNINSTALL_MANIFEST", &manifest)
        .status()
        .unwrap();
    assert!(status.success());
    assert!(!bin.exists());
    assert_eq!(std::fs::read(&foreign).unwrap(), b"keep");
    assert!(data.join("job.sqlite").exists());
}

#[test]
fn preinstall_refuses_a_foreign_binary() {
    let root = tempfile::tempdir().unwrap();
    let link_parent = root.path().join("usr/local/bin");
    std::fs::create_dir_all(&link_parent).unwrap();
    std::fs::write(link_parent.join("pgm-data"), b"other").unwrap();
    let script = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/packaging/macos/scripts/preinstall"
    );
    // The script checks the real /usr/local path. This test documents that the
    // script source refuses a non-symlink. Execute a copy with the path rewritten
    // into the temp root.
    let text = std::fs::read_to_string(script).unwrap().replace(
        "/usr/local",
        &root.path().join("usr/local").to_string_lossy(),
    );
    let rewritten = root.path().join("preinstall");
    std::fs::write(&rewritten, text).unwrap();
    let status = std::process::Command::new("bash")
        .arg(&rewritten)
        .status()
        .unwrap();
    assert!(!status.success());
}
