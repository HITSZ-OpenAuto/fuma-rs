use crate::constants::should_include_file;
use crate::models::{FileNode, NodeType, WorktreeData};
use std::collections::HashMap;

/// Format Unix timestamp to YYYY-MM-DD format
fn format_timestamp(unix_ts: i64) -> String {
    use std::time::UNIX_EPOCH;
    let duration = std::time::Duration::from_secs(unix_ts as u64);
    let datetime = UNIX_EPOCH + duration;
    let datetime = chrono::DateTime::<chrono::Utc>::from(datetime);
    datetime.format("%Y-%m-%d").to_string()
}

/// Generate download URL for a file in the repository
fn generate_download_url(repo: &str, path: &str) -> String {
    // Only encode parts, not the path separators
    let parts: Vec<String> = path
        .split('/')
        .map(|p| urlencoding::encode(p).into_owned())
        .collect();
    let encoded_path = parts.join("/");
    format!(
        "https://gh.hoa.moe/github.com/HITSZ-OpenAuto/{}/raw/main/{}",
        repo, encoded_path
    )
}

/// Build nested file tree from flat worktree data
pub fn build_file_tree(flat_data: &WorktreeData, repo_name: &str) -> Vec<FileNode> {
    #[derive(Debug)]
    struct TreeBuilder {
        children: HashMap<String, TreeBuilder>,
        is_file: bool,
        url: Option<String>,
        size: Option<u64>,
        date: Option<String>,
    }

    impl TreeBuilder {
        fn new() -> Self {
            Self {
                children: HashMap::new(),
                is_file: false,
                url: None,
                size: None,
                date: None,
            }
        }

        fn into_node(self, name: String) -> FileNode {
            let mut children: Vec<FileNode> = self
                .children
                .into_iter()
                .map(|(child_name, builder)| builder.into_node(child_name))
                .collect();

            // Sort: folders first, then by name
            children.sort_by(|a, b| match (&a.node_type, &b.node_type) {
                (NodeType::Folder, NodeType::File) => std::cmp::Ordering::Less,
                (NodeType::File, NodeType::Folder) => std::cmp::Ordering::Greater,
                _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
            });

            FileNode {
                name,
                node_type: if self.is_file {
                    NodeType::File
                } else {
                    NodeType::Folder
                },
                children,
                url: self.url,
                size: self.size,
                date: self.date,
            }
        }
    }

    let mut root = TreeBuilder::new();

    // Build tree from flat paths
    for (path, meta) in flat_data.0.iter() {
        if !should_include_file(path) {
            continue;
        }

        let parts: Vec<&str> = path.split('/').collect();
        let mut current = &mut root;

        for (i, &part) in parts.iter().enumerate() {
            let is_last = i == parts.len() - 1;
            current = current
                .children
                .entry(part.to_string())
                .or_insert_with(TreeBuilder::new);

            if is_last {
                current.is_file = true;
                current.url = Some(generate_download_url(repo_name, path));
                current.size = meta.size;
                current.date = meta.time.map(format_timestamp);
            }
        }
    }

    // Convert to sorted node list
    let mut result: Vec<FileNode> = root
        .children
        .into_iter()
        .map(|(name, builder)| builder.into_node(name))
        .collect();

    result.sort_by(|a, b| match (&a.node_type, &b.node_type) {
        (NodeType::Folder, NodeType::File) => std::cmp::Ordering::Less,
        (NodeType::File, NodeType::Folder) => std::cmp::Ordering::Greater,
        _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
    });

    result
}

/// Convert file tree to JSX string for Fumadocs Files component
pub fn tree_to_jsx(nodes: &[FileNode], indent_level: usize) -> String {
    let indent = "  ".repeat(indent_level);
    let mut result = Vec::new();

    for node in nodes {
        match node.node_type {
            NodeType::Folder => {
                result.push(format!("{}<Folder name=\"{}\">", indent, node.name));
                result.push(tree_to_jsx(&node.children, indent_level + 1));
                result.push(format!("{}</Folder>", indent));
            }
            NodeType::File => {
                let mut props = vec![format!("name=\"{}\"", node.name)];
                if let Some(ref url) = node.url {
                    props.push(format!("url=\"{}\"", url));
                }
                if let Some(ref date) = node.date {
                    props.push(format!("date=\"{}\"", date));
                }
                // Skip size if it's 0 or None
                if let Some(size) = node.size {
                    if size > 0 {
                        props.push(format!("size={{{}}}", size));
                    }
                }
                result.push(format!("{}<File {} />", indent, props.join(" ")));
            }
        }
    }

    result.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::FileMetadata;

    #[test]
    fn test_build_simple_tree() {
        let mut data = HashMap::new();
        data.insert(
            "file1.txt".to_string(),
            FileMetadata {
                size: Some(100),
                time: Some(1640000000),
            },
        );
        data.insert(
            "folder/file2.txt".to_string(),
            FileMetadata {
                size: Some(200),
                time: Some(1640000000),
            },
        );

        let worktree = WorktreeData(data);
        let tree = build_file_tree(&worktree, "test-repo");

        assert_eq!(tree.len(), 2); // file1.txt and folder
        assert!(tree.iter().any(|n| n.name == "file1.txt"));
        assert!(tree.iter().any(|n| n.name == "folder"));
    }

    #[test]
    fn test_build_nested_tree() {
        let mut data = HashMap::new();
        data.insert(
            "docs/notes/lecture1.pdf".to_string(),
            FileMetadata {
                size: Some(1024),
                time: Some(1640000000),
            },
        );
        data.insert(
            "docs/notes/lecture2.pdf".to_string(),
            FileMetadata {
                size: Some(2048),
                time: Some(1640000000),
            },
        );
        data.insert(
            "docs/assignments/hw1.pdf".to_string(),
            FileMetadata {
                size: Some(512),
                time: Some(1640000000),
            },
        );

        let worktree = WorktreeData(data);
        let tree = build_file_tree(&worktree, "test-repo");

        assert_eq!(tree.len(), 1); // Only docs folder at root
        let docs_folder = &tree[0];
        assert_eq!(docs_folder.name, "docs");
        assert_eq!(docs_folder.node_type, NodeType::Folder);
        assert_eq!(docs_folder.children.len(), 2); // notes and assignments
    }

    #[test]
    fn test_tree_sorting() {
        let mut data = HashMap::new();
        data.insert(
            "z_file.txt".to_string(),
            FileMetadata {
                size: Some(100),
                time: None,
            },
        );
        data.insert(
            "a_folder/file.txt".to_string(),
            FileMetadata {
                size: Some(100),
                time: None,
            },
        );
        data.insert(
            "b_file.txt".to_string(),
            FileMetadata {
                size: Some(100),
                time: None,
            },
        );

        let worktree = WorktreeData(data);
        let tree = build_file_tree(&worktree, "test-repo");

        // Folders should come before files
        assert_eq!(tree[0].name, "a_folder");
        assert_eq!(tree[0].node_type, NodeType::Folder);
        assert_eq!(tree[1].name, "b_file.txt");
        assert_eq!(tree[2].name, "z_file.txt");
    }

    #[test]
    fn test_exclusion_rules() {
        let mut data = HashMap::new();
        data.insert(
            "README.md".to_string(),
            FileMetadata {
                size: Some(100),
                time: None,
            },
        );
        data.insert(
            "valid.txt".to_string(),
            FileMetadata {
                size: Some(100),
                time: None,
            },
        );
        data.insert(
            ".github/workflow.yml".to_string(),
            FileMetadata {
                size: Some(100),
                time: None,
            },
        );

        let worktree = WorktreeData(data);
        let tree = build_file_tree(&worktree, "test-repo");

        // Only valid.txt should remain
        assert_eq!(tree.len(), 1);
        assert_eq!(tree[0].name, "valid.txt");
    }

    #[test]
    fn test_generate_download_url() {
        let url = generate_download_url("TEST101", "slides/lecture1.pdf");
        assert_eq!(
            url,
            "https://gh.hoa.moe/github.com/HITSZ-OpenAuto/TEST101/raw/main/slides/lecture1.pdf"
        );
    }

    #[test]
    fn test_generate_download_url_with_spaces() {
        let url = generate_download_url("COURSE", "folder/file name.pdf");
        assert!(url.contains("file%20name.pdf"));
    }

    #[test]
    fn test_generate_download_url_with_chinese() {
        let url = generate_download_url("COURSE", "作业/题目.pdf");
        assert!(url.contains("%E4%BD%9C%E4%B8%9A")); // Encoded Chinese
    }

    #[test]
    fn test_format_timestamp() {
        let formatted = format_timestamp(1640000000);
        assert_eq!(formatted, "2021-12-20");
    }

    #[test]
    fn test_tree_to_jsx_simple() {
        let nodes = vec![FileNode {
            name: "test.pdf".to_string(),
            node_type: NodeType::File,
            children: vec![],
            url: Some("https://example.com/test.pdf".to_string()),
            size: Some(1024),
            date: Some("2021-12-20".to_string()),
        }];

        let jsx = tree_to_jsx(&nodes, 1);
        assert!(jsx.contains("<File"));
        assert!(jsx.contains("name=\"test.pdf\""));
        assert!(jsx.contains("url=\"https://example.com/test.pdf\""));
        assert!(jsx.contains("date=\"2021-12-20\""));
        assert!(jsx.contains("size={1024}"));
    }

    #[test]
    fn test_tree_to_jsx_folder() {
        let nodes = vec![FileNode {
            name: "docs".to_string(),
            node_type: NodeType::Folder,
            children: vec![FileNode {
                name: "file.txt".to_string(),
                node_type: NodeType::File,
                children: vec![],
                url: Some("https://example.com/file.txt".to_string()),
                size: Some(100),
                date: None,
            }],
            url: None,
            size: None,
            date: None,
        }];

        let jsx = tree_to_jsx(&nodes, 1);
        assert!(jsx.contains("<Folder name=\"docs\">"));
        assert!(jsx.contains("</Folder>"));
        assert!(jsx.contains("<File name=\"file.txt\""));
    }

    #[test]
    fn test_tree_to_jsx_zero_size_excluded() {
        let nodes = vec![FileNode {
            name: "empty.txt".to_string(),
            node_type: NodeType::File,
            children: vec![],
            url: Some("https://example.com/empty.txt".to_string()),
            size: Some(0),
            date: None,
        }];

        let jsx = tree_to_jsx(&nodes, 1);
        // Size should be excluded if 0
        assert!(!jsx.contains("size="));
    }

    #[test]
    fn test_tree_to_jsx_indentation() {
        let nodes = vec![FileNode {
            name: "folder".to_string(),
            node_type: NodeType::Folder,
            children: vec![FileNode {
                name: "nested".to_string(),
                node_type: NodeType::Folder,
                children: vec![FileNode {
                    name: "file.txt".to_string(),
                    node_type: NodeType::File,
                    children: vec![],
                    url: Some("https://example.com/file.txt".to_string()),
                    size: Some(100),
                    date: None,
                }],
                url: None,
                size: None,
                date: None,
            }],
            url: None,
            size: None,
            date: None,
        }];

        let jsx = tree_to_jsx(&nodes, 1);
        // Check proper indentation
        assert!(jsx.contains("  <Folder name=\"folder\">"));
        assert!(jsx.contains("    <Folder name=\"nested\">"));
        assert!(jsx.contains("      <File name=\"file.txt\""));
    }

    #[test]
    fn test_tree_to_jsx_empty() {
        let nodes: Vec<FileNode> = vec![];
        let jsx = tree_to_jsx(&nodes, 1);
        assert_eq!(jsx, "");
    }
}
