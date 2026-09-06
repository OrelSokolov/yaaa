//! Environment for the shells spawned in terminal tabs.
//!
//! Everything built here is scoped to the PTY child process only — the
//! application's own environment is never modified. A GUI launch
//! (Finder/Dock) carries no locale, and without one the shell falls back
//! to the C locale, where zle reads input byte-by-byte: Cyrillic letters
//! whose UTF-8 tail byte lands in 0x80..0x9F (р..я) render as
//! `<0080>`-style junk, and line editing smears the halves across the
//! redraw. Like Terminal.app, we seed a UTF-8 locale when none is
//! inherited.
//!
//! The locale must actually exist on the system: `setlocale` silently
//! falls back to C for unknown names (e.g. the system preference
//! `ru_KZ` composes to a non-existent `ru_KZ.UTF-8`), so candidates are
//! validated against `locale -a` before use.

use std::collections::HashMap;

/// Builds the PTY environment from the current process environment.
pub fn build() -> HashMap<String, String> {
    build_with(read_process_env, system_utf8_locale)
}

/// Pure core: terminal defaults plus locale seeding, parameterized by
/// environment access so it can be tested without touching the process.
fn build_with(
    getenv: impl Fn(&str) -> Option<String>,
    fallback_locale: impl Fn() -> String,
) -> HashMap<String, String> {
    let mut env = HashMap::new();
    env.insert("TERM".to_string(), "xterm-256color".to_string());
    env.insert("COLORTERM".to_string(), "truecolor".to_string());

    let locale_unset =
        ["LC_ALL", "LC_CTYPE", "LANG"].iter().all(|k| getenv(k).is_none());
    if locale_unset {
        env.insert("LANG".to_string(), fallback_locale());
    }

    env
}

fn read_process_env(key: &str) -> Option<String> {
    std::env::var(key).ok().filter(|value| !value.is_empty())
}

/// Best-effort system UTF-8 locale, validated against the installed ones.
fn system_utf8_locale() -> String {
    pick_utf8_locale(preferred_locale().as_deref(), &available_utf8_locales())
}

/// Pure selection logic over the installed locale list.
fn pick_utf8_locale(preferred: Option<&str>, available: &[String]) -> String {
    if let Some(preferred) = preferred {
        let exact = format!("{preferred}.UTF-8");
        if let Some(hit) =
            available.iter().find(|l| l.eq_ignore_ascii_case(&exact))
        {
            return hit.clone();
        }
        // Region mismatch (ru_KZ -> ru_RU.UTF-8): keep the language if any
        // region of it is installed.
        let lang = preferred.split('_').next().unwrap_or(preferred);
        let prefix = format!("{lang}_");
        if let Some(hit) = available
            .iter()
            .find(|l| l.starts_with(&prefix) && is_utf8_spelling(l))
        {
            return hit.clone();
        }
    }

    let system_default = if cfg!(target_os = "macos") {
        "en_US.UTF-8"
    } else {
        "C.UTF-8"
    };
    if available
        .iter()
        .any(|l| l.eq_ignore_ascii_case(system_default))
    {
        return system_default.to_string();
    }
    if let Some(first) =
        available.iter().find(|l| is_utf8_spelling(l))
    {
        return first.clone();
    }
    system_default.to_string()
}

fn is_utf8_spelling(locale: &str) -> bool {
    let l = locale.to_ascii_lowercase();
    l.ends_with(".utf-8") || l.ends_with(".utf8")
}

/// Installed UTF-8 locales, in `locale -a` spelling (that spelling is the
/// valid one to pass on).
fn available_utf8_locales() -> Vec<String> {
    std::process::Command::new("locale")
        .arg("-a")
        .output()
        .ok()
        .map_or_else(Vec::new, |output| {
            String::from_utf8_lossy(&output.stdout)
                .lines()
                .filter(|l| {
                    let l = l.to_ascii_lowercase();
                    l.ends_with(".utf-8") || l.ends_with(".utf8")
                })
                .map(|l| l.trim().to_string())
                .filter(|l| !l.is_empty())
                .collect()
        })
}

/// The system-preferred locale, without the codeset suffix (e.g. `ru_KZ`).
fn preferred_locale() -> Option<String> {
    #[cfg(target_os = "macos")]
    {
        apple_locale()
    }
    #[cfg(not(target_os = "macos"))]
    {
        None
    }
}

#[cfg(target_os = "macos")]
fn apple_locale() -> Option<String> {
    let output = std::process::Command::new("defaults")
        .args(["read", "-g", "AppleLocale"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let locale =
        String::from_utf8_lossy(&output.stdout).trim().replace('-', "_");
    (!locale.is_empty()).then_some(locale)
}

#[cfg(test)]
mod tests {
    use super::{build_with, pick_utf8_locale};
    use std::collections::HashMap;

    fn build(env: &[(&str, &str)]) -> HashMap<String, String> {
        let env: HashMap<String, String> = env
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        build_with(
            move |key| env.get(key).cloned().filter(|v| !v.is_empty()),
            || "test.UTF-8".to_string(),
        )
    }

    #[test]
    fn no_inherited_locale_seeds_lang() {
        let env = build(&[]);
        assert_eq!(env.get("LANG").map(String::as_str), Some("test.UTF-8"));
        assert_eq!(env.get("TERM").map(String::as_str), Some("xterm-256color"));
        assert_eq!(env.get("COLORTERM").map(String::as_str), Some("truecolor"));
    }

    #[test]
    fn inherited_lang_is_not_overridden() {
        // The inherited value reaches the shell via normal process-env
        // inheritance; the map must not touch it.
        let env = build(&[("LANG", "ru_RU.UTF-8")]);
        assert!(env.get("LANG").is_none());
    }

    #[test]
    fn any_locale_var_suppresses_seeding() {
        for key in ["LC_ALL", "LC_CTYPE", "LANG"] {
            let env = build(&[(key, "de_DE.UTF-8")]);
            assert!(
                env.get("LANG").is_none() || key == "LANG",
                "{key} should suppress LANG seeding"
            );
        }
    }

    #[test]
    fn empty_locale_values_count_as_unset() {
        let env = build(&[
            ("LC_ALL", ""),
            ("LC_CTYPE", ""),
            ("LANG", ""),
        ]);
        assert_eq!(env.get("LANG").map(String::as_str), Some("test.UTF-8"));
    }

    #[test]
    fn exact_locale_match_wins() {
        let available =
            ["ru_RU.UTF-8", "en_US.UTF-8"].map(String::from);
        assert_eq!(
            pick_utf8_locale(Some("ru_RU"), &available),
            "ru_RU.UTF-8"
        );
    }

    #[test]
    fn region_mismatch_falls_back_to_same_language() {
        // ru_KZ.UTF-8 does not exist; ru_RU.UTF-8 must be picked instead.
        // The non-UTF-8 entry must never win even if present.
        let available = [
            "en_US.UTF-8",
            "ru_RU.ISO8859-5",
            "ru_RU.UTF-8",
        ]
        .map(String::from);
        assert_eq!(
            pick_utf8_locale(Some("ru_KZ"), &available),
            "ru_RU.UTF-8"
        );
    }

    #[test]
    fn unknown_language_falls_back_to_system_default() {
        let expected = default_locale_name();
        let available = ["de_DE.UTF-8", "en_US.UTF-8"].map(String::from);
        assert_eq!(pick_utf8_locale(Some("xx_XX"), &available), expected);
    }

    #[test]
    fn empty_list_falls_back_to_default() {
        let expected = default_locale_name();
        assert_eq!(pick_utf8_locale(None, &[]), expected);
        assert_eq!(pick_utf8_locale(Some("ru_KZ"), &[]), expected);
    }

    fn default_locale_name() -> &'static str {
        if cfg!(target_os = "macos") {
            "en_US.UTF-8"
        } else {
            "C.UTF-8"
        }
    }
}
