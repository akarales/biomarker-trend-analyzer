//! Architecture rule: the drift engine is pure — serde is its only
//! dependency, and no source file touches the clock, the filesystem, the
//! network or the environment. Callers pass timestamps and "now" in.

use std::path::Path;

const FORBIDDEN: &[&str] = &[
    "std::time",
    "SystemTime",
    "Instant::now",
    "chrono",
    "std::fs",
    "std::net",
    "std::env",
    "std::process",
    "tokio",
];

#[test]
fn only_depends_on_serde() {
    let manifest =
        std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"))
            .expect("manifest readable");
    let deps: Vec<&str> = manifest
        .split("[dependencies]")
        .nth(1)
        .unwrap_or("")
        .lines()
        .take_while(|l| !l.starts_with('['))
        .filter_map(|l| l.split(['=', '.']).next())
        .map(str::trim)
        .filter(|name| !name.is_empty() && !name.starts_with('#'))
        .collect();
    assert_eq!(deps, ["serde"]);
}

fn rust_files(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("dir readable") {
        let path = entry.expect("dir entry").path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

#[test]
fn sources_do_no_io_and_read_no_clock() {
    let mut files = Vec::new();
    rust_files(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("src"),
        &mut files,
    );
    assert!(files.len() > 5, "walks submodules too");
    let mut offenders = Vec::new();
    for path in files {
        let text = std::fs::read_to_string(&path).expect("source readable");
        for needle in FORBIDDEN {
            if text.contains(needle) {
                offenders.push(format!("{}: {needle}", path.display()));
            }
        }
    }
    assert!(offenders.is_empty(), "impure drift engine: {offenders:?}");
}
