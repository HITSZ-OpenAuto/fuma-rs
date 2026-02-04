/// Semester mapping from Chinese names to folder names and display titles
pub const SEMESTER_MAPPING: &[(&str, &str, &str)] = &[
    ("第一学年秋季", "fresh-autumn", "大一·秋"),
    ("第一学年春季", "fresh-spring", "大一·春"),
    ("第二学年秋季", "sophomore-autumn", "大二·秋"),
    ("第二学年春季", "sophomore-spring", "大二·春"),
    ("第三学年秋季", "junior-autumn", "大三·秋"),
    ("第三学年春季", "junior-spring", "大三·春"),
    ("第四学年秋季", "senior-autumn", "大四·秋"),
    ("第四学年春季", "senior-spring", "大四·春"),
];

/// Get semester folder and title from Chinese semester name
pub fn get_semester_folder(recommended: &str) -> Option<(&'static str, &'static str)> {
    SEMESTER_MAPPING
        .iter()
        .find(|&&(key, _, _)| key == recommended)
        .map(|&(_, folder, title)| (folder, title))
}

// ============================================================================
// File Exclusion Rules
// ============================================================================

/// Files to exclude from the file tree
pub const EXCLUDED_PATTERNS: &[&str] = &[".gitkeep", "README.md", "LICENSE", "tag.txt"];

/// File extensions to exclude
pub const EXCLUDED_EXTENSIONS: &[&str] = &[".toml"];

/// Directory prefixes to exclude
pub const EXCLUDED_PREFIXES: &[&str] = &[".github/"];

/// Check if a file path should be included in the file tree
pub fn should_include_file(path: &str) -> bool {
    let filename = path.split('/').next_back().unwrap_or("");

    // Check exact matches
    if EXCLUDED_PATTERNS.contains(&filename) {
        return false;
    }

    // Check extensions
    if EXCLUDED_EXTENSIONS
        .iter()
        .any(|ext| filename.ends_with(ext))
    {
        return false;
    }

    // Check prefixes
    if EXCLUDED_PREFIXES
        .iter()
        .any(|prefix| path.starts_with(prefix))
    {
        return false;
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_semester_folder_valid() {
        let result = get_semester_folder("第一学年秋季");
        assert_eq!(result, Some(("fresh-autumn", "大一·秋")));

        let result = get_semester_folder("第二学年春季");
        assert_eq!(result, Some(("sophomore-spring", "大二·春")));

        let result = get_semester_folder("第四学年春季");
        assert_eq!(result, Some(("senior-spring", "大四·春")));
    }

    #[test]
    fn test_get_semester_folder_invalid() {
        let result = get_semester_folder("第五学年秋季");
        assert_eq!(result, None);

        let result = get_semester_folder("invalid");
        assert_eq!(result, None);

        let result = get_semester_folder("");
        assert_eq!(result, None);
    }

    #[test]
    fn test_should_include_file_excluded_patterns() {
        assert!(!should_include_file(".gitkeep"));
        assert!(!should_include_file("README.md"));
        assert!(!should_include_file("LICENSE"));
        assert!(!should_include_file("tag.txt"));
        assert!(!should_include_file("folder/.gitkeep"));
        assert!(!should_include_file("docs/README.md"));
    }

    #[test]
    fn test_should_include_file_excluded_extensions() {
        assert!(!should_include_file("config.toml"));
        assert!(!should_include_file("folder/settings.toml"));
        assert!(!should_include_file("path/to/file.toml"));
    }

    #[test]
    fn test_should_include_file_excluded_prefixes() {
        assert!(!should_include_file(".github/workflows/ci.yml"));
        assert!(!should_include_file(".github/ISSUE_TEMPLATE.md"));
    }

    #[test]
    fn test_should_include_file_valid_files() {
        assert!(should_include_file("notes.pdf"));
        assert!(should_include_file("lecture.pptx"));
        assert!(should_include_file("folder/document.docx"));
        assert!(should_include_file("path/to/file.txt"));
        assert!(should_include_file("code.py"));
        assert!(should_include_file("assignment.md"));
    }

    #[test]
    fn test_should_include_file_edge_cases() {
        assert!(should_include_file("readme.txt")); // Not exactly README.md
        assert!(should_include_file("my.toml.txt")); // Doesn't end with .toml
        assert!(should_include_file("github/file.txt")); // Not .github prefix
        assert!(!should_include_file(".github/file.txt")); // Is .github prefix
    }

    #[test]
    fn test_semester_mapping_complete() {
        // Ensure all 8 semesters are mapped
        assert_eq!(SEMESTER_MAPPING.len(), 8);

        // Check uniqueness of folders
        let mut folders = std::collections::HashSet::new();
        for (_, folder, _) in SEMESTER_MAPPING {
            assert!(folders.insert(folder), "Duplicate folder: {}", folder);
        }

        // Check uniqueness of titles
        let mut titles = std::collections::HashSet::new();
        for (_, _, title) in SEMESTER_MAPPING {
            assert!(titles.insert(title), "Duplicate title: {}", title);
        }
    }
}
