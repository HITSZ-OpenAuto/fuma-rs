use serde::{Deserialize, Serialize};

// ============================================================================
// TOML Data Models (for reading from hoa-majors data)
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct TomlPlan {
    pub info: PlanInfo,
    pub courses: Vec<TomlCourse>,
}

#[derive(Debug, Deserialize)]
pub struct PlanInfo {
    pub year: String,
    pub major_code: String,
    pub major_name: String,
    #[serde(rename = "plan_ID")]
    pub _plan_id: String,
}

#[derive(Debug, Deserialize)]
pub struct TomlCourse {
    pub course_code: String,
    pub course_name: String,
    pub credit: Option<f64>,
    pub assessment_method: Option<String>,
    pub course_nature: Option<String>,
    pub recommended_year_semester: Option<String>,
    pub hours: Option<HourDistribution>,
    pub grade_details: Option<Vec<GradeDetail>>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct GradeDetail {
    pub name: String,
    pub percent: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct HourDistribution {
    pub theory: Option<u32>,
    pub lab: Option<u32>,
    pub practice: Option<u32>,
    pub exercise: Option<u32>,
    pub computer: Option<u32>,
    pub tutoring: Option<u32>,
}

// ============================================================================
// Runtime Data Models
// ============================================================================

#[derive(Debug, Clone)]
pub struct Plan {
    pub year: String,
    pub major_code: String,
    pub major_name: String,
    pub courses: Vec<Course>,
}

#[derive(Debug, Clone)]
pub struct Course {
    pub code: String,
    pub name: String,
    pub credit: Option<f64>,
    pub assessment_method: Option<String>,
    pub course_nature: Option<String>,
    pub recommended_semester: Option<String>,
    pub hours: Option<HourDistribution>,
    pub grade_details: Option<Vec<GradeDetail>>,
}

// ============================================================================
// Worktree JSON Models
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct WorktreeData(pub std::collections::HashMap<String, FileMetadata>);

#[derive(Debug, Deserialize)]
pub struct FileMetadata {
    pub size: Option<u64>,
    pub time: Option<i64>,
}

// ============================================================================
// File Tree Models
// ============================================================================

#[derive(Debug, Clone)]
pub struct FileNode {
    pub name: String,
    pub node_type: NodeType,
    pub children: Vec<FileNode>,
    pub url: Option<String>,
    pub size: Option<u64>,
    pub date: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum NodeType {
    Folder,
    File,
}

// ============================================================================
// Frontmatter Models (for YAML serialization)
// ============================================================================

#[derive(Debug, Serialize)]
pub struct Frontmatter {
    pub title: String,
    pub description: String,
    pub course: CourseMetadata,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CourseMetadata {
    pub credit: u32,
    pub assessment_method: String,
    pub course_nature: String,
    pub hour_distribution: HourDistributionMeta,
    pub grading_scheme: Vec<GradingItem>,
}

#[derive(Debug, Serialize)]
pub struct HourDistributionMeta {
    pub theory: u32,
    pub lab: u32,
    pub practice: u32,
    pub exercise: u32,
    pub computer: u32,
    pub tutoring: u32,
}

#[derive(Debug, Serialize)]
pub struct GradingItem {
    pub name: String,
    pub percent: u32,
}

impl Frontmatter {
    /// Convert frontmatter to YAML string
    pub fn to_yaml(&self) -> String {
        // Use serde_yaml to serialize, but customize for better formatting
        match serde_yaml::to_string(self) {
            Ok(yaml) => format!("---\n{}---", yaml),
            Err(_) => {
                // Fallback to empty frontmatter
                "---\ntitle: ''\ndescription: ''\n---".to_string()
            }
        }
    }
}
