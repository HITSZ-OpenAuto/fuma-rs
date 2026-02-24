use crate::constants::{get_semester_title_by_folder, parse_semester_folders, SEMESTER_MAPPING};
use crate::error::Result;
use crate::models::{
    Course, CourseMetadata, Frontmatter, GradeDetail, GradingItem, HourDistributionMeta, Plan,
    SharedCategory, WorktreeData,
};
use crate::tree::{build_file_tree, tree_to_jsx};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

/// Build YAML frontmatter for a course page using serde_yaml
fn build_frontmatter(title: &str, course: &Course) -> String {
    let credit = course.credit.map(|c| c as u32).unwrap_or(0);
    let assessment_method = course
        .assessment_method
        .as_deref()
        .unwrap_or("")
        .to_string();
    let course_nature = course.course_nature.as_deref().unwrap_or("").to_string();

    let hour_distribution = if let Some(ref h) = course.hours {
        HourDistributionMeta {
            theory: h.theory.unwrap_or(0),
            lab: h.lab.unwrap_or(0),
            practice: h.practice.unwrap_or(0),
            exercise: h.exercise.unwrap_or(0),
            computer: h.computer.unwrap_or(0),
            tutoring: h.tutoring.unwrap_or(0),
        }
    } else {
        HourDistributionMeta {
            theory: 0,
            lab: 0,
            practice: 0,
            exercise: 0,
            computer: 0,
            tutoring: 0,
        }
    };

    let grading_scheme = if let Some(ref details) = course.grade_details {
        details
            .iter()
            .filter_map(|detail| {
                let percent = if let Some(ref percent_str) = detail.percent {
                    percent_str
                        .trim_end_matches('%')
                        .parse::<u32>()
                        .unwrap_or(0)
                } else {
                    0
                };

                (percent > 0).then(|| GradingItem {
                    name: detail.name.clone(),
                    percent,
                })
            })
            .collect()
    } else {
        Vec::new()
    };

    let frontmatter = Frontmatter {
        title: title.to_string(),
        description: String::new(),
        course: CourseMetadata {
            credit,
            assessment_method,
            course_nature,
            hour_distribution,
            grading_scheme,
        },
    };

    frontmatter.to_yaml()
}

fn title_from_mdx(mdx_content: &str, fallback: &str) -> String {
    let lines: Vec<&str> = mdx_content.lines().collect();
    for line in lines.iter().take(5) {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed == "---" {
            continue;
        }
        let raw = if let Some(t) = trimmed.strip_prefix("title:") {
            t.trim().trim_matches('"').trim_matches('\'').to_string()
        } else {
            trimmed.to_string()
        };
        let raw = raw.trim_start_matches("# ").trim();
        return if let Some(rest) = raw.split_once(" - ") {
            rest.1.trim().to_string()
        } else {
            raw.to_string()
        };
    }
    fallback.to_string()
}

fn minimal_course(repo_id: &str, name: &str, grade_details: Option<Vec<GradeDetail>>) -> Course {
    Course {
        repo_id: repo_id.to_string(),
        name: name.to_string(),
        credit: None,
        assessment_method: None,
        course_nature: None,
        recommended_semester: None,
        hours: None,
        grade_details,
    }
}

/// Generate all course pages and index pages
pub async fn generate_course_pages(
    plans: &[Plan],
    shared_categories: &[SharedCategory],
    no_course_info_repo_ids: &HashSet<String>,
    grades_summary: &HashMap<String, HashMap<String, Vec<GradeDetail>>>,
    repos_dir: &Path,
    docs_dir: &Path,
    repos_set: &HashSet<String>,
) -> Result<()> {
    let mut years: HashSet<String> = HashSet::new();
    let mut majors_by_year: HashMap<String, Vec<(String, String)>> = HashMap::new();

    for plan in plans {
        years.insert(plan.year.clone());

        majors_by_year
            .entry(plan.year.clone())
            .or_default()
            .push((plan.major_code.clone(), plan.major_name.clone()));

        let major_dir = docs_dir.join(&plan.year).join(&plan.major_code);
        fs::create_dir_all(&major_dir)?;

        // Track courses by semester for this major
        let mut courses_by_semester: HashMap<String, Vec<(String, String)>> = HashMap::new();

        // Process each course
        for course in &plan.courses {
            // Only process courses that exist in repos_list (if repos_list.txt exists)
            if !repos_set.is_empty() && !repos_set.contains(&course.repo_id) {
                continue;
            }

            let mdx_path = repos_dir.join(format!("{}.mdx", course.repo_id));
            let json_path = repos_dir.join(format!("{}.json", course.repo_id));

            if !mdx_path.exists() {
                continue;
            }

            // Read README content (skip first 2 lines which are title)
            let readme_content = fs::read_to_string(&mdx_path)?;
            let content_lines: Vec<&str> = readme_content.lines().skip(2).collect();
            let content = content_lines.join("\n");

            // Determine target directories based on semester (supports multi-semester values)
            let semester_folders = course
                .recommended_semester
                .as_deref()
                .map(parse_semester_folders)
                .unwrap_or_default();

            let mut target_dirs = Vec::new();
            if semester_folders.is_empty() {
                target_dirs.push(major_dir.clone());
            } else {
                for (folder, _title) in semester_folders {
                    let sem_dir = major_dir.join(folder);
                    fs::create_dir_all(&sem_dir)?;
                    courses_by_semester
                        .entry(folder.to_string())
                        .or_default()
                        .push((course.repo_id.clone(), course.name.clone()));
                    target_dirs.push(sem_dir);
                }
            }

            // Generate file tree from worktree.json
            let filetree_content = if json_path.exists() {
                let json_content = fs::read_to_string(&json_path)?;
                let worktree: WorktreeData = serde_json::from_str(&json_content)?;
                let tree = build_file_tree(&worktree, &course.repo_id);
                let jsx = tree_to_jsx(&tree, 1);
                format!(
                    "\n\n## 资源下载\n\n<Files url=\"https://open.osa.moe/openauto/{}\">\n{}\n</Files>",
                    course.repo_id, jsx
                )
            } else {
                String::new()
            };

            // Build frontmatter
            let frontmatter = build_frontmatter(&course.name, course);

            // Write course page
            let page_content = format!(
                "{}\n\n<CourseInfo />\n\n{}{}",
                frontmatter, content, filetree_content
            );
            for target_dir in target_dirs {
                fs::write(
                    target_dir.join(format!("{}.mdx", course.repo_id)),
                    &page_content,
                )?;
            }
        }

        // Keep semester pages and navigation in semantic order
        let ordered_semester_folders: Vec<String> = SEMESTER_MAPPING
            .iter()
            .filter_map(|(_, folder, _)| {
                courses_by_semester
                    .contains_key(*folder)
                    .then_some((*folder).to_string())
            })
            .collect();

        // Generate semester index pages
        for folder in &ordered_semester_folders {
            let courses = courses_by_semester.get(folder).cloned().unwrap_or_default();
            let sem_dir = major_dir.join(folder);
            let sem_title = get_semester_title_by_folder(folder).unwrap_or(folder.as_str());

            let mut cards = vec![
                "---".to_string(),
                format!("title: {}", sem_title),
                "---".to_string(),
                "".to_string(),
                "<Cards>".to_string(),
            ];

            for (slug, name) in &courses {
                cards.push(format!(
                    "  <Card title=\"{}\" href=\"/docs/{}/{}/{}/{}\" />",
                    name, plan.year, plan.major_code, folder, slug
                ));
            }
            cards.push("</Cards>".to_string());

            fs::write(sem_dir.join("index.mdx"), cards.join("\n"))?;
        }

        // Shared categories
        let mut category_pages: Vec<String> = Vec::new();
        for cat in shared_categories {
            let cat_dir = major_dir.join(&cat.id);
            fs::create_dir_all(&cat_dir)?;

            let mut category_courses: Vec<(String, String)> = Vec::new();

            for repo_id in &cat.repo_ids {
                if !repos_set.is_empty() && !repos_set.contains(repo_id) {
                    continue;
                }

                let mdx_path = repos_dir.join(format!("{}.mdx", repo_id));
                let json_path = repos_dir.join(format!("{}.json", repo_id));

                if !mdx_path.exists() {
                    continue;
                }

                let readme_content = fs::read_to_string(&mdx_path)?;
                let title = title_from_mdx(&readme_content, repo_id);
                category_courses.push((repo_id.clone(), title.clone()));

                let content_lines: Vec<&str> = readme_content.lines().skip(2).collect();
                let content = content_lines.join("\n");

                let filetree_content = if json_path.exists() {
                    let json_content = fs::read_to_string(&json_path)?;
                    let worktree: WorktreeData = serde_json::from_str(&json_content)?;
                    let tree = build_file_tree(&worktree, repo_id);
                    let jsx = tree_to_jsx(&tree, 1);
                    format!(
                        "\n\n## 资源下载\n\n<Files url=\"https://open.osa.moe/openauto/{}\">\n{}\n</Files>",
                        repo_id, jsx
                    )
                } else {
                    String::new()
                };

                let grade_details = grades_summary
                    .get(repo_id)
                    .and_then(|m| m.get("default"))
                    .cloned();
                let course = minimal_course(repo_id, &title, grade_details);
                let frontmatter = build_frontmatter(&title, &course);
                let use_course_info = !no_course_info_repo_ids.contains(repo_id);
                let page_content = if use_course_info {
                    format!("{}\n\n<CourseInfo />\n\n{}{}", frontmatter, content, filetree_content)
                } else {
                    format!("{}\n\n{}{}", frontmatter, content, filetree_content)
                };
                fs::write(cat_dir.join(format!("{}.mdx", repo_id)), &page_content)?;
            }

            if !category_courses.is_empty() {
                category_pages.push(cat.id.clone());

                let mut cards = vec![
                    "---".to_string(),
                    format!("title: {}", cat.title),
                    "---".to_string(),
                    "".to_string(),
                    "<Cards>".to_string(),
                ];
                for (slug, name) in &category_courses {
                    cards.push(format!(
                        "  <Card title=\"{}\" href=\"/docs/{}/{}/{}/{}\" />",
                        name, plan.year, plan.major_code, cat.id, slug
                    ));
                }
                cards.push("</Cards>".to_string());
                fs::write(cat_dir.join("index.mdx"), cards.join("\n"))?;
            }
        }

        // Write major metadata
        let pages: Vec<String> = std::iter::once("...".to_string())
            .chain(ordered_semester_folders.iter().cloned())
            .chain(category_pages.iter().cloned())
            .collect();

        let major_meta = serde_json::json!({
            "title": plan.major_name,
            "root": true,
            "defaultOpen": true,
            "pages": pages,
        });
        fs::write(
            major_dir.join("meta.json"),
            serde_json::to_string_pretty(&major_meta)?,
        )?;

        // Generate major index page with semester cards
        let mut major_index = vec![
            "---".to_string(),
            "title: 目录".to_string(),
            "---".to_string(),
            "".to_string(),
            "<Cards>".to_string(),
        ];

        for folder in &ordered_semester_folders {
            let title = get_semester_title_by_folder(folder).unwrap_or(folder.as_str());
            major_index.push(format!(
                "  <Card title=\"{}\" href=\"/docs/{}/{}/{}\" />",
                title, plan.year, plan.major_code, folder
            ));
        }
        for cat in shared_categories {
            if category_pages.contains(&cat.id) {
                major_index.push(format!(
                    "  <Card title=\"{}\" href=\"/docs/{}/{}/{}\" />",
                    cat.title, plan.year, plan.major_code, cat.id
                ));
            }
        }
        major_index.push("</Cards>".to_string());

        fs::write(major_dir.join("index.mdx"), major_index.join("\n"))?;
    }

    // Generate year index pages in sorted order
    let mut year_list: Vec<String> = years.into_iter().collect();
    year_list.sort();
    for year in &year_list {
        let year_dir = docs_dir.join(year);
        let year_meta = serde_json::json!({"title": year});
        fs::write(
            year_dir.join("meta.json"),
            serde_json::to_string_pretty(&year_meta)?,
        )?;

        // Generate year index with major cards
        if let Some(majors) = majors_by_year.get(year) {
            let mut year_index = vec![
                "---".to_string(),
                "title: 目录".to_string(),
                "---".to_string(),
                "".to_string(),
                "<Cards>".to_string(),
            ];

            for (code, name) in majors {
                year_index.push(format!(
                    "  <Card title=\"{}\" href=\"/docs/{}/{}\" />",
                    name, year, code
                ));
            }
            year_index.push("</Cards>".to_string());

            fs::write(year_dir.join("index.mdx"), year_index.join("\n"))?;
        }
    }

    Ok(())
}
