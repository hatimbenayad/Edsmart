pub mod checker;
pub mod rules;

pub use checker::{lint_project, LintReport};
pub use rules::{LintIssue, Severity};
