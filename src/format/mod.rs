//! MDX/Markdown Formatting Module
//!
//! This module provides two approaches to formatting MDX files:
//! 1. **Regex-based** (`regex.rs`) - Fast string replacement using regular expressions
//! 2. **AST-based** (`ast.rs`) - Structure-aware processing using Abstract Syntax Tree (default)
//!
//! ## Main Transformations
//!
//! The formatter performs the following operations:
//!
//! ### 1. Remove Elements
//! - HTML comments (`<!-- ... -->`)
//! - Shield.io badge images
//!
//! ### 2. Fix HTML
//! - Self-closing tags: `<br>` → `<br />`, `<hr>` → `<hr />`
//! - Malformed table tags
//! - Style attributes to JSX: `style="color:red"` → `style={{color: "red"}}`
//!
//! ### 3. Convert Hugo Shortcodes
//! - `{{% details title="..." %}}` → `<Accordion title="...">`
//! - Wrap consecutive Accordions in `<Accordions>` container
//!
//! ### 4. Escape Math Syntax
//! - Escape curly braces in LaTeX: `${x}$` → `$\{x\}$`
//!
//! ## Usage
//!
//! ```rust
//! use crate::format::format_mdx_file;
//!
//! let content = "<!-- comment -->\nText <br> here";
//! let formatted = format_mdx_file(&content);
//! ```

pub mod ast;
pub mod regex;

// Export AST formatter as the default
pub use ast::format_all_mdx_files;