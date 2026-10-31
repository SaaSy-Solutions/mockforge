use super::path_policy::has_windows_path_syntax;
use super::reader::open_confined_file;
use super::*;
use std::fs;
#[cfg(target_os = "linux")]
use std::path::Path;
use std::path::PathBuf;
use tempfile::TempDir;

fn fixture() -> (TempDir, PathBuf) {
    let dir = TempDir::new().expect("create temp dir");
    let root = dir.path().join("specs");
    fs::create_dir_all(root.join("fixtures")).expect("create fixtures dir");
    fs::write(root.join("fixtures/user.json"), r#"{"id":1}"#).expect("write fixture");
    fs::write(dir.path().join("secret.txt"), "top-secret").expect("write secret");
    (dir, root)
}

#[cfg(unix)]
#[test]
fn replacing_resolved_file_with_an_escaping_symlink_is_rejected() {
    let (temp, root) = fixture();
    let root = root.canonicalize().unwrap();
    let allowed = root.join("allowed.txt");
    fs::write(&allowed, "allowed fixture").unwrap();
    let resolved = allowed.canonicalize().unwrap();
    let outside = temp.path().join("outside.txt");
    fs::write(&outside, "outside fixture").unwrap();
    fs::remove_file(&allowed).unwrap();
    std::os::unix::fs::symlink(outside, allowed).unwrap();
    assert!(open_confined_file(&root, &resolved).is_err());
}

#[cfg(unix)]
#[test]
fn replacing_resolved_parent_with_an_escaping_symlink_is_rejected() {
    let (temp, root) = fixture();
    let root = root.canonicalize().unwrap();
    let parent = root.join("parent");
    fs::create_dir(&parent).unwrap();
    let allowed = parent.join("file.txt");
    fs::write(&allowed, "allowed fixture").unwrap();
    let resolved = allowed.canonicalize().unwrap();
    let outside = temp.path().join("outside");
    fs::create_dir(&outside).unwrap();
    fs::write(outside.join("file.txt"), "outside fixture").unwrap();
    fs::rename(&parent, root.join("previous-parent")).unwrap();
    std::os::unix::fs::symlink(outside, parent).unwrap();
    assert!(open_confined_file(&root, &resolved).is_err());
}

#[cfg(unix)]
#[test]
fn replacing_resolved_root_with_an_escaping_symlink_is_rejected() {
    let (temp, root) = fixture();
    let root = root.canonicalize().unwrap();
    let allowed = root.join("allowed.txt");
    fs::write(&allowed, "allowed fixture").unwrap();
    let resolved = allowed.canonicalize().unwrap();
    let outside = temp.path().join("outside");
    fs::create_dir(&outside).unwrap();
    fs::write(outside.join("allowed.txt"), "outside fixture").unwrap();
    fs::rename(&root, temp.path().join("previous-root")).unwrap();
    std::os::unix::fs::symlink(outside, &root).unwrap();
    assert!(open_confined_file(&root, &resolved).is_err());
}

#[cfg(unix)]
#[test]
fn fifo_is_rejected_without_waiting_for_a_writer() {
    let (_temp, root) = fixture();
    let fifo = root.join("fixture.pipe");
    assert!(std::process::Command::new("mkfifo").arg(&fifo).status().unwrap().success());
    let (sender, receiver) = std::sync::mpsc::channel();
    let worker = std::thread::spawn(move || {
        let _ = sender.send(read_file_in_root(&root, "fixture.pipe", DEFAULT_MAX_BYTES));
    });
    let result = receiver
        .recv_timeout(std::time::Duration::from_secs(2))
        .expect("non-regular files must be refused without blocking");
    assert_eq!(result, Err(TemplateFsError::NotFound));
    worker.join().unwrap();
}

#[test]
fn legitimate_relative_file_is_read() {
    let (_dir, root) = fixture();
    let out = read_file_in_root(&root, "fixtures/user.json", DEFAULT_MAX_BYTES);
    assert_eq!(out, Ok(r#"{"id":1}"#.to_string()));
    let out = read_file_in_root(&root, "./fixtures/user.json", DEFAULT_MAX_BYTES);
    assert_eq!(out, Ok(r#"{"id":1}"#.to_string()));
}

#[cfg(unix)]
#[test]
fn absolute_path_inside_root_is_read() {
    let (_dir, root) = fixture();
    let abs = root.canonicalize().expect("canon").join("fixtures/user.json");
    let out = read_file_in_root(&root, &abs.to_string_lossy(), DEFAULT_MAX_BYTES);
    assert_eq!(out, Ok(r#"{"id":1}"#.to_string()));
}

#[test]
fn dot_dot_traversal_is_rejected() {
    let (_dir, root) = fixture();
    for p in [
        "../secret.txt",
        "fixtures/../../secret.txt",
        "fixtures/../user.json",
    ] {
        assert_eq!(
            read_file_in_root(&root, p, DEFAULT_MAX_BYTES),
            Err(TemplateFsError::Rejected("parent-directory traversal")),
            "{p}"
        );
    }
}

#[cfg(unix)]
#[test]
fn absolute_path_outside_root_is_rejected() {
    let (dir, root) = fixture();
    let secret = dir.path().canonicalize().expect("canon").join("secret.txt");
    assert_eq!(
        read_file_in_root(&root, &secret.to_string_lossy(), DEFAULT_MAX_BYTES),
        Err(TemplateFsError::OutsideRoot)
    );
    assert_eq!(
        read_file_in_root(&root, "/nonexistent/file.txt", DEFAULT_MAX_BYTES),
        Err(TemplateFsError::OutsideRoot)
    );
}

#[cfg(unix)]
#[test]
fn symlink_escaping_root_is_rejected() {
    let (dir, root) = fixture();
    std::os::unix::fs::symlink(dir.path().join("secret.txt"), root.join("link.txt"))
        .expect("symlink file");
    std::os::unix::fs::symlink(dir.path(), root.join("up")).expect("symlink dir");
    assert_eq!(
        read_file_in_root(&root, "link.txt", DEFAULT_MAX_BYTES),
        Err(TemplateFsError::OutsideRoot)
    );
    assert_eq!(
        read_file_in_root(&root, "up/secret.txt", DEFAULT_MAX_BYTES),
        Err(TemplateFsError::OutsideRoot)
    );
}

#[cfg(unix)]
#[test]
fn symlink_inside_root_is_allowed() {
    let (_dir, root) = fixture();
    std::os::unix::fs::symlink(root.join("fixtures/user.json"), root.join("alias.json"))
        .expect("symlink");
    assert_eq!(
        read_file_in_root(&root, "alias.json", DEFAULT_MAX_BYTES),
        Ok(r#"{"id":1}"#.to_string())
    );
}

#[cfg(unix)]
#[test]
fn system_paths_are_denied() {
    let (_dir, root) = fixture();
    for p in [
        "/etc/passwd",
        "/etc/shadow",
        "/proc/self/environ",
        "/proc/1/cmdline",
        "/sys/x",
    ] {
        assert_eq!(
            read_file_in_root(&root, p, DEFAULT_MAX_BYTES),
            Err(TemplateFsError::Denied),
            "{p}"
        );
    }
}

#[cfg(target_os = "linux")]
#[test]
fn system_paths_denied_even_when_root_contains_them() {
    // A misconfigured root of `/proc/self` still cannot leak the environment.
    let root = Path::new("/proc/self");
    assert_eq!(read_file_in_root(root, "environ", 4096), Err(TemplateFsError::Denied));
    // And `/` is never a usable root.
    assert_eq!(
        read_file_in_root(Path::new("/"), "etc/hostname", 4096),
        Err(TemplateFsError::InvalidRoot)
    );
}

#[test]
fn env_like_files_are_denied() {
    let (_dir, root) = fixture();
    for name in [".env", ".env.production", "prod.env", ".envrc", "environ"] {
        fs::write(root.join(name), "SECRET=1").expect("write env file");
        assert_eq!(
            read_file_in_root(&root, name, DEFAULT_MAX_BYTES),
            Err(TemplateFsError::Denied),
            "{name}"
        );
    }
}

#[cfg(unix)]
#[test]
fn symlink_to_env_file_inside_root_is_denied() {
    let (_dir, root) = fixture();
    fs::write(root.join(".env"), "SECRET=1").expect("write env");
    std::os::unix::fs::symlink(root.join(".env"), root.join("innocent.json")).expect("link");
    assert_eq!(
        read_file_in_root(&root, "innocent.json", DEFAULT_MAX_BYTES),
        Err(TemplateFsError::Denied)
    );
}

#[test]
fn percent_encoded_variants_are_rejected() {
    let (_dir, root) = fixture();
    for p in [
        "%2e%2e/secret.txt",
        "..%2fsecret.txt",
        "%2Fetc%2Fpasswd",
        "fixtures%2fuser.json",
    ] {
        assert_eq!(
            read_file_in_root(&root, p, DEFAULT_MAX_BYTES),
            Err(TemplateFsError::Rejected("percent-encoding")),
            "{p}"
        );
    }
}

#[test]
fn windows_style_variants_are_rejected() {
    let (_dir, root) = fixture();
    for p in [
        r"..\secret.txt",
        r"fixtures\user.json",
        r"C:\Windows\win.ini",
        "C:/Windows/win.ini",
        r"\\server\share\file",
    ] {
        assert_eq!(
            read_file_in_root(&root, p, DEFAULT_MAX_BYTES),
            Err(TemplateFsError::Rejected("Windows-style path")),
            "{p}"
        );
    }
}

#[test]
fn windows_alternate_stream_syntax_is_rejected_before_opening() {
    for path in [
        "fixtures/user.json:private",
        ".env::$DATA",
        "fixtures/.env:private",
    ] {
        assert!(has_windows_path_syntax(path, true), "{path}");
        // Colons in POSIX filenames are ordinary bytes, not stream selectors.
        assert!(!has_windows_path_syntax(path, false), "{path}");
    }
}

#[cfg(windows)]
#[test]
fn windows_tokens_use_portable_relative_paths() {
    let (_dir, root) = fixture();
    let absolute = root.canonicalize().unwrap().join("fixtures/user.json");
    for path in [absolute.to_string_lossy().as_ref(), ".env::$DATA"] {
        assert_eq!(
            read_file_in_root(&root, path, DEFAULT_MAX_BYTES),
            Err(TemplateFsError::Rejected("Windows-style path"))
        );
    }
    assert_eq!(
        read_file_in_root(&root, "fixtures/user.json", DEFAULT_MAX_BYTES),
        Ok(r#"{"id":1}"#.to_string())
    );
}

#[test]
fn nul_and_empty_are_rejected() {
    let (_dir, root) = fixture();
    assert_eq!(
        read_file_in_root(&root, "fixtures/user.json\0.png", DEFAULT_MAX_BYTES),
        Err(TemplateFsError::Rejected("NUL byte"))
    );
    assert_eq!(
        read_file_in_root(&root, "  ", DEFAULT_MAX_BYTES),
        Err(TemplateFsError::EmptyPath)
    );
}

#[test]
fn oversized_file_is_rejected() {
    let (_dir, root) = fixture();
    fs::write(root.join("big.txt"), vec![b'a'; 2048]).expect("write big");
    assert_eq!(read_file_in_root(&root, "big.txt", 1024), Err(TemplateFsError::TooLarge(1024)));
    assert_eq!(read_file_in_root(&root, "big.txt", 2048).map(|s| s.len()), Ok(2048));
}

#[test]
fn directories_and_missing_files_are_not_found() {
    let (_dir, root) = fixture();
    assert_eq!(
        read_file_in_root(&root, "fixtures", DEFAULT_MAX_BYTES),
        Err(TemplateFsError::NotFound)
    );
    assert_eq!(
        read_file_in_root(&root, "missing.json", DEFAULT_MAX_BYTES),
        Err(TemplateFsError::NotFound)
    );
}

#[test]
fn missing_root_is_invalid() {
    let (dir, _root) = fixture();
    assert_eq!(
        read_file_in_root(&dir.path().join("nope"), "a.json", DEFAULT_MAX_BYTES),
        Err(TemplateFsError::InvalidRoot)
    );
}

#[test]
fn root_precedence_disables_hosted_unless_explicit() {
    let explicit = Some(PathBuf::from("/srv/fixtures"));
    let registered = Some(PathBuf::from("/home/me/specs"));
    // Explicit root wins everywhere, including hosted mocks.
    assert_eq!(resolve_root(explicit.clone(), true, registered.clone()), explicit);
    assert_eq!(resolve_root(explicit.clone(), false, registered.clone()), explicit);
    // Hosted without an explicit root: disabled, even if a spec dir was registered.
    assert_eq!(resolve_root(None, true, registered.clone()), None);
    // Self-hosted: the spec directory.
    assert_eq!(resolve_root(None, false, registered.clone()), registered);
    // Nothing configured: disabled.
    assert_eq!(resolve_root(None, false, None), None);
}
