//! File names for exports: `sprite.png`, or `sprite_0001.png`, `sprite_0002.png`, …

use std::path::{Path, PathBuf};

use super::NamingOptions;

/// Keeps only characters that are safe in Windows file names.
pub fn sanitize_stem(stem: &str) -> String {
    let cleaned: String = stem
        .chars()
        .map(|c| {
            if c.is_control() || r#"<>:"/\|?*"#.contains(c) {
                '_'
            } else {
                c
            }
        })
        .collect();
    let trimmed = cleaned.trim().trim_end_matches('.');
    if trimmed.is_empty() {
        "sprite".to_owned()
    } else {
        trimmed.chars().take(100).collect()
    }
}

/// The base name to use: the configured one, or the project name.
pub fn base_name(naming: &NamingOptions, project_name: &str) -> String {
    let raw = if naming.base_name.trim().is_empty() {
        project_name
    } else {
        naming.base_name.as_str()
    };
    sanitize_stem(raw)
}

/// Frame name without extension: `base` for a single unnumbered file, else
/// `base_0001` (1-based, zero-padded to `digits`).
pub fn frame_stem(base: &str, number: usize, digits: u8) -> String {
    format!(
        "{base}_{number:0width$}",
        width = usize::from(digits.clamp(1, 8))
    )
}

/// File names for `count` files.
pub fn file_names(
    naming: &NamingOptions,
    base: &str,
    count: usize,
    extension: &str,
) -> Vec<String> {
    if count == 1 && !naming.number_single {
        return vec![format!("{base}.{extension}")];
    }
    (1..=count)
        .map(|n| format!("{}.{extension}", frame_stem(base, n, naming.digits)))
        .collect()
}

/// Full paths in `dir`.
pub fn paths_in(dir: &Path, names: &[String]) -> Vec<PathBuf> {
    names.iter().map(|n| dir.join(n)).collect()
}

/// Paths that already exist (would be overwritten).
pub fn existing(paths: &[PathBuf]) -> Vec<PathBuf> {
    paths.iter().filter(|p| p.exists()).cloned().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_file_is_not_numbered_unless_asked() {
        let mut n = NamingOptions::default();
        assert_eq!(file_names(&n, "hero", 1, "png"), vec!["hero.png"]);
        n.number_single = true;
        assert_eq!(file_names(&n, "hero", 1, "png"), vec!["hero_0001.png"]);
    }

    #[test]
    fn sequences_use_padding() {
        let n = NamingOptions {
            digits: 3,
            ..NamingOptions::default()
        };
        assert_eq!(
            file_names(&n, "walk", 3, "jpg"),
            vec!["walk_001.jpg", "walk_002.jpg", "walk_003.jpg"]
        );
    }

    #[test]
    fn names_are_sanitized() {
        assert_eq!(sanitize_stem("a/b:c*"), "a_b_c_");
        assert_eq!(sanitize_stem("  ..  "), "sprite");
        let n = NamingOptions::default();
        assert_eq!(base_name(&n, "My Project"), "My Project");
        let custom = NamingOptions {
            base_name: "run?".into(),
            ..n
        };
        assert_eq!(base_name(&custom, "x"), "run_");
    }
}
