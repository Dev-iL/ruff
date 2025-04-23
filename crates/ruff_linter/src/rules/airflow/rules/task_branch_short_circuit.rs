use ruff_diagnostics::{Diagnostic, Violation};
use ruff_macros::{derive_message_formats, ViolationMetadata};
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_python_semantic::SemanticModel;
use ruff_text_size::Ranged;

use crate::checkers::ast::Checker;

/// ## What it does
/// Checks for `task.branch` with at least two `return`s and suggests replacing it with `task.short_circuit`.
///
/// ## Why is this bad?
/// Using `task.short_circuit` is simpler and more readable than using `task.branch` with multiple `return` statements.
///
/// ## Example
/// ```python
/// @task.branch
/// def my_task():
///     if condition1:
///         return []
///     if condition2:
///         return ["my_downstream_task"]
///     return []
///
/// my_task >> my_downstream_task
/// ```
///
/// Use instead:
/// ```python
/// @task.short_circuit
/// def my_task():
///     if condition2:
///         return True
///     return False
///
/// my_task >> my_downstream_task
/// ```
#[derive(ViolationMetadata)]
pub(crate) struct TaskBranchShortCircuit;

impl Violation for TaskBranchShortCircuit {
    #[derive_message_formats]
    fn message(&self) -> String {
        "Replace `task.branch` with `task.short_circuit`".to_string()
    }
}

/// Check for `task.branch` with at least two `return`s and suggest replacing it with `task.short_circuit`.
pub(crate) fn task_branch_short_circuit(checker: &Checker, stmt: &Stmt) {
    if let Stmt::FunctionDef(ast::StmtFunctionDef { body, .. }) = stmt {
        let mut return_count = 0;
        let mut non_empty_return_count = 0;

        for stmt in body {
            if let Stmt::Return(ast::StmtReturn { value, .. }) = stmt {
                return_count += 1;
                if let Some(Expr::List(ast::ExprList { elts, .. })) = value.as_deref() {
                    if !elts.is_empty() {
                        non_empty_return_count += 1;
                    }
                }
            }
        }

        if return_count >= 2 && non_empty_return_count == 1 {
            checker.report_diagnostic(Diagnostic::new(TaskBranchShortCircuit, stmt.range()));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ruff_python_ast::source_code::SourceCode;
    use ruff_python_ast::source_code::SourceCodeBuilder;
    use ruff_python_ast::source_code::SourceCodeKind;
    use ruff_python_ast::source_code::SourceCodeRange;
    use ruff_python_ast::source_code::SourceCodeText;
}
