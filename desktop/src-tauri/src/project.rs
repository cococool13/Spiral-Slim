//! Locating the SlimBrave Neo source and a Python to run it with.
//!
//! Spiral Slim ships no policy logic of its own; it drives the scripts that
//! already exist. Both halves of that — where the scripts are, and which
//! interpreter runs them — are resolved once, explicitly, and fail with an
//! actionable message rather than a silent fallback.

use std::path::{Path, PathBuf};

use crate::error::{SlimError, SlimResult};

/// Scripts the project root must contain before we will run anything from it.
const REQUIRED: [&str; 2] = ["slimbrave-mac.py", "browser_collection.py"];

/// Checked in order. The scripts are stdlib-only, so the system interpreter
/// on macOS is enough and is preferred: an app launched from Finder has a
/// minimal PATH and should not depend on the user's shell setup.
const PYTHON_CANDIDATES: [&str; 3] = [
    "/usr/bin/python3",
    "/opt/homebrew/bin/python3",
    "/usr/local/bin/python3",
];

#[derive(Debug, Clone)]
pub struct Project {
    pub root: PathBuf,
    pub python: PathBuf,
}

fn is_project_root(candidate: &Path) -> bool {
    REQUIRED.iter().all(|name| candidate.join(name).is_file())
}

/// Candidate roots, most explicit first.
///
/// `resource_dir` is where the bundled copy lands. The compile-time checkout
/// path is a **debug-only** fallback, and that restriction is the point: it
/// exists so `tauri dev` works without bundling, but if a release build could
/// also reach it, a bundle that shipped its resources incorrectly would still
/// run perfectly on the machine that built it and fail for every other
/// person. A packaging bug has to be visible here or it is invisible until a
/// user hits it.
pub fn candidate_roots(resource_dir: Option<PathBuf>, env_override: Option<PathBuf>) -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Some(path) = env_override {
        roots.push(path);
    }
    if let Some(dir) = resource_dir {
        roots.push(dir.join("slimbrave"));
        roots.push(dir);
    }
    #[cfg(debug_assertions)]
    {
        // src-tauri -> desktop -> slim
        let checkout = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .map(Path::to_path_buf);
        if let Some(path) = checkout {
            roots.push(path);
        }
    }
    roots
}

pub fn resolve_root(candidates: &[PathBuf]) -> SlimResult<PathBuf> {
    candidates
        .iter()
        .find(|candidate| is_project_root(candidate))
        .cloned()
        .ok_or_else(|| {
            SlimError::new(
                "SlimBrave Neo source not found",
                format!(
                    "Spiral Slim looked for {} in {} location(s) and found neither.",
                    REQUIRED.join(" and "),
                    candidates.len()
                ),
                "Set SPIRAL_SLIM_PROJECT_DIR to the apps/slim folder and reopen \
                 Spiral Slim.",
            )
        })
}

pub fn resolve_python(env_override: Option<PathBuf>) -> SlimResult<PathBuf> {
    if let Some(path) = env_override {
        if path.is_file() {
            return Ok(path);
        }
        return Err(SlimError::new(
            "Python not found",
            format!("SPIRAL_SLIM_PYTHON points at {}, which is not a file.", path.display()),
            "Correct SPIRAL_SLIM_PYTHON, or unset it to use the system Python 3.",
        ));
    }
    for candidate in PYTHON_CANDIDATES {
        let path = PathBuf::from(candidate);
        if path.is_file() {
            return Ok(path);
        }
    }
    Err(SlimError::new(
        "Python 3 not found",
        "Spiral Slim needs Python 3 to run the SlimBrave Neo scripts and could \
         not find it in any standard location."
            .to_string(),
        "Install the Xcode command line tools with `xcode-select --install`, \
         then reopen Spiral Slim.",
    ))
}

impl Project {
    pub fn locate(resource_dir: Option<PathBuf>) -> SlimResult<Self> {
        let env_root = std::env::var_os("SPIRAL_SLIM_PROJECT_DIR").map(PathBuf::from);
        let env_python = std::env::var_os("SPIRAL_SLIM_PYTHON").map(PathBuf::from);
        let candidates = candidate_roots(resource_dir, env_root);
        Ok(Self {
            root: resolve_root(&candidates)?,
            python: resolve_python(env_python)?,
        })
    }

    pub fn script(&self, name: &str) -> PathBuf {
        self.root.join(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_directory_without_the_scripts_is_not_a_project_root() {
        let dir = tempfile::tempdir().unwrap();
        assert!(!is_project_root(dir.path()));
    }

    #[test]
    fn a_directory_with_both_scripts_is_a_project_root() {
        let dir = tempfile::tempdir().unwrap();
        for name in REQUIRED {
            std::fs::write(dir.path().join(name), "").unwrap();
        }
        assert!(is_project_root(dir.path()));
    }

    #[test]
    fn a_partial_checkout_is_rejected() {
        // Only one of the two scripts present: refuse rather than half-work.
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("slimbrave-mac.py"), "").unwrap();
        assert!(!is_project_root(dir.path()));
    }

    #[test]
    fn the_env_override_is_tried_before_anything_else() {
        let roots = candidate_roots(
            Some(PathBuf::from("/resources")),
            Some(PathBuf::from("/explicit")),
        );
        assert_eq!(roots.first(), Some(&PathBuf::from("/explicit")));
    }

    /// A release build must depend on its own bundled resources and nothing
    /// else. If this ever regresses, a mis-packaged `.app` passes every test
    /// on the build machine and fails on every other one.
    #[test]
    #[cfg(not(debug_assertions))]
    fn a_release_build_never_falls_back_to_the_build_machine_checkout() {
        assert!(candidate_roots(None, None).is_empty());
    }

    #[test]
    #[cfg(debug_assertions)]
    fn a_debug_build_falls_back_to_the_checkout_so_tauri_dev_works() {
        let roots = candidate_roots(None, None);
        assert_eq!(roots.len(), 1);
        assert!(is_project_root(&roots[0]), "the checkout should be usable");
    }

    #[test]
    fn resolving_a_root_reports_where_it_looked() {
        let error = resolve_root(&[PathBuf::from("/nowhere")]).unwrap_err();
        assert!(error.next_step.contains("SPIRAL_SLIM_PROJECT_DIR"));
        assert!(!error.detail.is_empty());
    }

    #[test]
    fn a_bad_python_override_is_reported_rather_than_ignored() {
        let error = resolve_python(Some(PathBuf::from("/nowhere/python3"))).unwrap_err();
        assert!(error.detail.contains("/nowhere/python3"));
    }
}
