//! Data loading utilities for training plans and course information.
//!
//! This module provides functions to load all training plan data from TOML files
//! and enrich it with grade details from grades_summary.json. By loading all data
//! upfront, we avoid the N+1 query problem that plagued the Python implementation.

use crate::error::{FumaError, Result};
use crate::models::{Course, GradeDetail, Plan, TomlPlan};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

/// Grades summary data structure mapping course codes to grade details per plan variant
type GradesSummary = HashMap<String, HashMap<String, Vec<GradeDetail>>>;

/// Load grades_summary.json if present.
///
/// Returns an empty HashMap if the file doesn't exist or can't be parsed.
fn load_grades_summary(data_dir: &Path) -> GradesSummary {
    let path = data_dir.join("grades_summary.json");

    if !path.exists() {
        return HashMap::new();
    }

    match fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_else(|_| HashMap::new()),
        Err(_) => HashMap::new(),
    }
}

/// Select grade details for a course based on hierarchical matching rules.
///
/// Match priority order:
/// 1. `{year}_{major_code}` or `{year}_{major_name}` (most specific)
/// 2. `{year}_default` (year-specific default)
/// 3. `default` (global default)
///
/// Returns None if no matching grade details are found.
fn select_grade_details(
    grades_summary: &GradesSummary,
    course_code: &str,
    year: &str,
    major_code: &str,
    major_name: &str,
) -> Option<Vec<GradeDetail>> {
    let entry = grades_summary.get(course_code)?;

    // Try year_major keys (both code and name)
    let year_major_keys = vec![
        format!("{}_{}", year, major_code),
        format!("{}_{}", year, major_name),
    ];

    for key in &year_major_keys {
        if let Some(details) = entry.get(key) {
            if !details.is_empty() {
                return Some(details.clone());
            }
        }
    }

    // Try year_default
    let year_default_key = format!("{}_default", year);
    if let Some(details) = entry.get(&year_default_key) {
        if !details.is_empty() {
            return Some(details.clone());
        }
    }

    // Try default
    if let Some(details) = entry.get("default") {
        if !details.is_empty() {
            return Some(details.clone());
        }
    }

    None
}

/// Load all training plans from TOML files with grade details enrichment.
///
/// This function loads all plan data in a single pass, avoiding the N+1 query problem
/// that occurred in the Python implementation where each course required a separate
/// CLI invocation to fetch grade details.
///
/// # Arguments
/// * `data_dir` - Path to the hoa-majors data directory containing plans/ subdirectory
///
/// # Returns
/// * `Ok(Vec<Plan>)` - All loaded and enriched training plans
/// * `Err(FumaError)` - If the plans directory is missing or files can't be read
pub fn load_all_plans(data_dir: &Path) -> Result<Vec<Plan>> {
    let plans_dir = data_dir.join("plans");

    if !plans_dir.exists() {
        return Err(FumaError::MissingDirectory(plans_dir));
    }

    // Load grades summary once for all plans
    let grades_summary = load_grades_summary(data_dir);

    let mut plans = Vec::new();

    for entry in WalkDir::new(&plans_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "toml"))
    {
        let content = fs::read_to_string(entry.path())?;
        let toml_plan: TomlPlan = toml::from_str(&content)?;

        // Enrich courses with grade_details from grades_summary.json
        let courses = toml_plan
            .courses
            .into_iter()
            .map(|c| {
                // Select grade details if not already in TOML
                let grade_details = c.grade_details.or_else(|| {
                    select_grade_details(
                        &grades_summary,
                        &c.course_code,
                        &toml_plan.info.year,
                        &toml_plan.info.major_code,
                        &toml_plan.info.major_name,
                    )
                });

                Course {
                    code: c.course_code,
                    name: c.course_name,
                    credit: c.credit,
                    assessment_method: c.assessment_method,
                    course_nature: c.course_nature,
                    recommended_semester: c.recommended_year_semester,
                    hours: c.hours,
                    grade_details,
                }
            })
            .collect();

        plans.push(Plan {
            year: toml_plan.info.year,
            major_code: toml_plan.info.major_code,
            major_name: toml_plan.info.major_name,
            courses,
        });
    }

    Ok(plans)
}

/// Load repos_list.txt to filter available courses.
///
/// # Returns
/// * Empty HashSet if repos_list.txt doesn't exist (process all courses)
/// * HashSet of repository codes if the file exists
pub fn load_repos_list(repo_root: &Path) -> Result<HashSet<String>> {
    let path = repo_root.join("repos_list.txt");

    if !path.exists() {
        eprintln!("Warning: repos_list.txt not found, will process all available courses");
        return Ok(HashSet::new());
    }

    let content = fs::read_to_string(&path)?;
    Ok(content
        .lines()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect())
}
