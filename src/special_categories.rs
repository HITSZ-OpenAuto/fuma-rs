//! Special category generation for cross-specialty and general-knowledge courses.
//!
//! These courses are not tied to specific years or majors, so they need
//! separate handling from the regular training plan-based courses.

use crate::error::Result;
use crate::models::{CourseMetadata, Frontmatter, GradingItem, HourDistributionMeta, WorktreeData};
use crate::tree::{build_file_tree, tree_to_jsx};
use std::collections::HashSet;
use std::fs;
use std::path::Path;

/// Special category definition
pub struct SpecialCategory {
    pub id: &'static str,
    pub title: &'static str,
    pub repos: &'static [&'static str],
}

/// Define the special categories and their repositories
pub const SPECIAL_CATEGORIES: &[SpecialCategory] = &[
    SpecialCategory {
        id: "cross-specialty",
        title: "跨专业选修",
        repos: &[
            "CrossSpecialty",  // Overview/Index
            "CHEM1012",        // 大学化学 III
            "COMP3043",        // 深度学习体系结构
            "ECON2005F",       // 经济学原理
            "SPST1004",        // 普通天文学
        ],
    },
    SpecialCategory {
        id: "general-knowledge",
        title: "文理通识与 MOOC",
        repos: &[
            "GeneralKnowledge", // Overview/Index
            "MOOC",             // MOOC overview
            "SEIN1040",         // 中国科技史话
            "WOCD1008",         // 日语 I
            "WRIT0001",         // 写作与沟通
        ],
    },
];

/// Load grades summary for special categories from repos
fn load_grades_summary_from_repo(repo_dir: &Path, repo_id: &str) -> Option<Vec<(String, String)>> {
    let grades_path = repo_dir.join(format!("{}.grades.json", repo_id));
    
    if !grades_path.exists() {
        return None;
    }

    match fs::read_to_string(&grades_path) {
        Ok(content) => {
            serde_json::from_str(&content).unwrap_or(None)
        }
        Err(_) => None,
    }
}

/// Build YAML frontmatter for a special category page
fn build_frontmatter(title: &str, repo_id: &str, grades: Option<Vec<(String, String)>>) -> String {
    // Default hour distribution (all zeros for special categories)
    let hour_distribution = HourDistributionMeta {
        theory: 0,
        lab: 0,
        practice: 0,
        exercise: 0,
        computer: 0,
        tutoring: 0,
    };

    // Convert grades to grading scheme
    let grading_scheme: Vec<GradingItem> = grades
        .map(|g| {
            g.into_iter()
                .filter_map(|(name, percent_str)| {
                    let percent = percent_str
                        .trim_end_matches('%')
                        .parse::<u32>()
                        .unwrap_or(0);
                    
                    (percent > 0).then(|| GradingItem { name, percent })
                })
                .collect()
        })
        .unwrap_or_default();

    let frontmatter = Frontmatter {
        title: title.to_string(),
        description: String::new(),
        course: CourseMetadata {
            credit: 0,
            assessment_method: String::new(),
            course_nature: String::new(),
            hour_distribution,
            grading_scheme,
        },
    };

    frontmatter.to_yaml()
}

/// Extract title from MDX content (first line starting with "# ")
fn extract_title_from_mdx(content: &str) -> String {
    content
        .lines()
        .find(|line| line.starts_with("# "))
        .map(|line| line.trim_start_matches("# ").trim().to_string())
        .unwrap_or_else(|| "Untitled".to_string())
}

/// Generate pages for all special categories
pub async fn generate_special_category_pages(
    repos_dir: &Path,
    docs_dir: &Path,
    repos_set: &HashSet<String>,
) -> Result<()> {
    for category in SPECIAL_CATEGORIES {
        let category_dir = docs_dir.join(category.id);
        fs::create_dir_all(&category_dir)?;

        let mut category_pages: Vec<(String, String)> = Vec::new(); // (slug, title)

        for repo_id in category.repos {
            // Skip if repos_set is not empty and this repo is not in it
            if !repos_set.is_empty() && !repos_set.contains(*repo_id) {
                continue;
            }

            let mdx_path = repos_dir.join(format!("{}.mdx", repo_id));
            let json_path = repos_dir.join(format!("{}.json", repo_id));

            if !mdx_path.exists() {
                eprintln!("Warning: MDX file not found for {}: {:?}", repo_id, mdx_path);
                continue;
            }

            // Read README content
            let readme_content = fs::read_to_string(&mdx_path)?;
            
            // Extract title from first heading
            let title = extract_title_from_mdx(&readme_content);
            
            // Read content (skip first line which is the title)
            let content_lines: Vec<&str> = readme_content.lines().skip(1).collect();
            let content = content_lines.join("\n");

            // Load grades if available
            let grades = load_grades_summary_from_repo(repos_dir, repo_id);

            // Generate file tree from worktree.json
            let filetree_content = if json_path.exists() {
                let json_content = fs::read_to_string(&json_path)?;
                let worktree: WorktreeData = serde_json::from_str(&json_content)?;
                let tree = build_file_tree(&worktree, repo_id);
                let jsx = tree_to_jsx(&tree, 1);
                format!(
                    "\n\n## 资源下载\n\n<Files url=\"https://open.osa.moe/openauto/{}\"\u003e\n{}\n</Files\u003e",
                    repo_id, jsx
                )
            } else {
                String::new()
            };

            // Build frontmatter
            let frontmatter = build_frontmatter(&title, repo_id, grades);

            // Write course page
            let page_content = format!(
                "{}\n\n<CourseInfo />\n\n{}{}",
                frontmatter, content, filetree_content
            );
            
            let page_path = category_dir.join(format!("{}.mdx", repo_id));
            fs::write(&page_path, &page_content)?;
            
            category_pages.push((repo_id.to_string(), title));
        }

        // Write category meta.json
        let pages: Vec<String> = std::iter::once("...".to_string())
            .chain(category_pages.iter().map(|(slug, _)| slug.clone()))
            .collect();

        let category_meta = serde_json::json!({
            "title": category.title,
            "root": true,
            "defaultOpen": true,
            "pages": pages,
        });
        fs::write(
            category_dir.join("meta.json"),
            serde_json::to_string_pretty(&category_meta)?,
        )?;

        // Generate category index page
        let mut index_content = vec![
            "---".to_string(),
            format!("title: {}", category.title),
            "---".to_string(),
            "".to_string(),
            "<Cards>".to_string(),
        ];

        for (slug, title) in &category_pages {
            index_content.push(format!(
                "  <Card title=\"{}\" href=\"/docs/{}/{}\" />",
                title, category.id, slug
            ));
        }
        index_content.push("</Cards>".to_string());

        fs::write(category_dir.join("index.mdx"), index_content.join("\n"))?;

        println!(
            "Generated {} pages for category '{}'",
            category_pages.len(),
            category.id
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_title_from_mdx() {
        let content = "# Hello World\n\nSome content here.";
        assert_eq!(extract_title_from_mdx(content), "Hello World");

        let content_no_title = "Some content without title.";
        assert_eq!(extract_title_from_mdx(content_no_title), "Untitled");

        let content_with_whitespace = "#   Title With Spaces   \nContent.";
        assert_eq!(extract_title_from_mdx(content_with_whitespace), "Title With Spaces");
    }
}
