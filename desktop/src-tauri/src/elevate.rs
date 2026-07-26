//! Running one command with administrator rights, on macOS, via the system
//! authorisation dialog.
//!
//! Spiral Slim never collects a password. `do shell script … with
//! administrator privileges` hands that entirely to macOS, which shows its
//! own dialog naming the app. Spiral Slim only ever sees whether the command
//! ran.
//!
//! The command is built by quoting each argument twice — once for `/bin/sh`,
//! which `do shell script` runs the string through, and once for the
//! AppleScript string literal that carries it. Both quoters are tested below
//! because a mistake in either is a command-injection bug in a privileged
//! path.

use std::path::Path;
use std::process::{Command, Output};

use crate::error::{SlimError, SlimResult};

/// Wrap a value in single quotes for `/bin/sh`, escaping embedded quotes.
pub fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', r"'\''"))
}

/// Wrap a value in double quotes for an AppleScript string literal.
pub fn applescript_quote(value: &str) -> String {
    let escaped = value.replace('\\', r"\\").replace('"', "\\\"");
    format!("\"{escaped}\"")
}

/// Build the AppleScript that asks macOS to run `argv` as an administrator.
pub fn build_admin_script(argv: &[String], prompt: &str) -> String {
    let command = argv
        .iter()
        .map(|part| shell_quote(part))
        .collect::<Vec<_>>()
        .join(" ");
    format!(
        "do shell script {} with prompt {} with administrator privileges",
        applescript_quote(&command),
        applescript_quote(prompt),
    )
}

/// True when osascript failed because the person dismissed the dialog.
/// That is a decision, not a fault, and must not be reported as an error.
pub fn is_user_cancelled(stderr: &str) -> bool {
    stderr.contains("User cancelled") || stderr.contains("User canceled") || stderr.contains("-128")
}

#[cfg(target_os = "macos")]
pub fn run_privileged(argv: &[String], prompt: &str) -> SlimResult<Output> {
    let script = build_admin_script(argv, prompt);
    Command::new("/usr/bin/osascript")
        .arg("-e")
        .arg(&script)
        .output()
        .map_err(|error| {
            SlimError::new(
                "Could not ask macOS for permission",
                error.to_string(),
                "Reopen Spiral Slim. If it keeps failing, run the SlimBrave Neo \
                 script from Terminal with sudo instead.",
            )
        })
}

#[cfg(not(target_os = "macos"))]
pub fn run_privileged(_argv: &[String], _prompt: &str) -> SlimResult<Output> {
    Err(SlimError::new(
        "Not supported on this platform",
        "Spiral Slim applies Brave policies on macOS only.".to_string(),
        "Run the SlimBrave Neo script for your platform directly.",
    ))
}

/// Reject anything that is not a plain identifier before it reaches a
/// privileged command line. Channel ids come from detection, but this is the
/// last place to be sure.
pub fn is_safe_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// Control ids are dotted (`vendor.ai`), so they are not plain identifiers.
/// They only ever reach the read-only engine, never a privileged command
/// line, but they are still validated before being put on any argv.
pub fn is_safe_control_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 96
        && !value.starts_with('.')
        && !value.ends_with('.')
        && !value.contains("..")
        && value
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '.')
}

/// Absolute paths only, and no interior NUL. Paths are ones Spiral Slim
/// built, so this is a guard against a bug, not against the user.
pub fn is_safe_path(path: &Path) -> bool {
    path.is_absolute() && !path.to_string_lossy().contains('\0')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shell_quoting_neutralises_an_embedded_quote() {
        assert_eq!(shell_quote("plain"), "'plain'");
        assert_eq!(shell_quote("it's"), r"'it'\''s'");
    }

    #[test]
    fn shell_quoting_neutralises_command_substitution() {
        let quoted = shell_quote("$(rm -rf /)");
        assert_eq!(quoted, "'$(rm -rf /)'");
        // Nothing escapes the single quotes, so the shell cannot expand it.
        assert!(!quoted[1..quoted.len() - 1].contains('\''));
    }

    #[test]
    fn a_quote_break_attempt_stays_inside_one_argument() {
        // The classic escape: close the quote, run something, reopen.
        let quoted = shell_quote("'; touch /tmp/pwned; '");
        assert_eq!(quoted, r"''\''; touch /tmp/pwned; '\'''");
    }

    #[test]
    fn applescript_quoting_escapes_backslashes_and_quotes() {
        assert_eq!(applescript_quote(r#"a"b"#), r#""a\"b""#);
        assert_eq!(applescript_quote(r"a\b"), r#""a\\b""#);
    }

    #[test]
    fn applescript_quoting_escapes_the_backslash_before_the_quote() {
        // Escaping in the wrong order would turn \" into \\" and end the
        // literal early.
        assert_eq!(applescript_quote(r#"a\"b"#), r#""a\\\"b""#);
    }

    #[test]
    fn the_admin_script_quotes_every_argument() {
        let argv = vec![
            "/usr/bin/python3".to_string(),
            "/Apps/My Tools/slimbrave-mac.py".to_string(),
            "--apply-plan".to_string(),
            "/tmp/plan.json".to_string(),
        ];
        let script = build_admin_script(&argv, "Spiral Slim");
        assert!(script.contains("'/Apps/My Tools/slimbrave-mac.py'"));
        assert!(script.starts_with("do shell script \""));
        assert!(script.ends_with("with administrator privileges"));
    }

    #[test]
    fn a_hostile_path_cannot_break_out_of_the_script() {
        let argv = vec![
            "/usr/bin/python3".to_string(),
            r#"/tmp/a" & (do shell script "id") & ""#.to_string(),
        ];
        let script = build_admin_script(&argv, "Spiral Slim");
        // The inner double quotes are escaped, so the AppleScript literal
        // is never terminated early.
        assert!(script.contains(r#"\""#));
        assert_eq!(script.matches("do shell script").count(), 2);
        // ...and the second occurrence is inert text inside the literal.
        let body = script
            .strip_prefix("do shell script ")
            .expect("known prefix");
        assert!(body.starts_with('"'));
    }

    #[test]
    fn identifiers_are_limited_to_channel_shaped_values() {
        assert!(is_safe_identifier("stable"));
        assert!(is_safe_identifier("nightly-2"));
        assert!(!is_safe_identifier(""));
        assert!(!is_safe_identifier("stable;rm"));
        assert!(!is_safe_identifier("Stable"));
        assert!(!is_safe_identifier("../etc"));
        assert!(!is_safe_identifier(&"a".repeat(65)));
    }

    #[test]
    fn control_ids_may_be_dotted_but_not_traversal() {
        assert!(is_safe_control_id("vendor.ai"));
        assert!(is_safe_control_id("permissions.notifications.default"));
        assert!(is_safe_control_id("security.downloads.malicious"));
        assert!(!is_safe_control_id(""));
        assert!(!is_safe_control_id("../etc/passwd"));
        assert!(!is_safe_control_id("a..b"));
        assert!(!is_safe_control_id(".hidden"));
        assert!(!is_safe_control_id("trailing."));
        assert!(!is_safe_control_id("Vendor.AI"));
        assert!(!is_safe_control_id("vendor.ai;rm"));
    }

    #[test]
    fn relative_paths_are_rejected() {
        assert!(is_safe_path(Path::new("/tmp/plan.json")));
        assert!(!is_safe_path(Path::new("plan.json")));
    }

    #[test]
    fn a_dismissed_dialog_is_recognised() {
        assert!(is_user_cancelled("execution error: User cancelled. (-128)"));
        assert!(is_user_cancelled("User canceled."));
        assert!(!is_user_cancelled("sudo: a password is required"));
    }
}
