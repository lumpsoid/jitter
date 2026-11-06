//! Sets `JITTER_VERSION` for `jitter_core::version`: the package version and
//! the 7-character commit hash, `0.1.0 (2391c1d)`. A build without git (a
//! source tarball) gets the bare version, unless `JITTER_COMMIT` names the
//! commit, as the AppImage container build does.

use std::path::Path;
use std::process::Command;

fn main() {
    let version = env!("CARGO_PKG_VERSION");
    println!("cargo:rerun-if-env-changed=JITTER_COMMIT");
    let commit = std::env::var("JITTER_COMMIT")
        .ok()
        .filter(|commit| !commit.is_empty())
        .or_else(git_commit);
    let full = match commit {
        Some(commit) => format!("{version} ({commit})"),
        None => version.to_owned(),
    };
    println!("cargo:rustc-env=JITTER_VERSION={full}");
}

/// The short hash of `HEAD`, rebuilding when `HEAD` moves. Only paths that
/// exist are watched: a missing one would make cargo rebuild every time.
fn git_commit() -> Option<String> {
    for (arg, file) in [
        ("--git-dir", "HEAD"),
        ("--git-common-dir", "refs"),
        ("--git-common-dir", "packed-refs"),
    ] {
        if let Some(dir) = git(&["rev-parse", "--path-format=absolute", arg]) {
            let path = Path::new(&dir).join(file);
            if path.exists() {
                println!("cargo:rerun-if-changed={}", path.display());
            }
        }
    }
    git(&["rev-parse", "--short=7", "HEAD"])
}

fn git(args: &[&str]) -> Option<String> {
    let output = Command::new("git").args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8(output.stdout).ok()?;
    let text = text.trim();
    (!text.is_empty()).then(|| text.to_owned())
}
