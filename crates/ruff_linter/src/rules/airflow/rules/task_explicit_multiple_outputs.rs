use ruff_macros::{ViolationMetadata, derive_message_formats};
use ruff_python_ast::helpers::{ReturnStatementVisitor, map_callable};
use ruff_python_ast::visitor::Visitor;
use ruff_python_ast::{self as ast, Expr, ExprAttribute, StmtFunctionDef};
use ruff_python_semantic::{Modules, SemanticModel};
use ruff_text_size::Ranged;

use crate::checkers::ast::Checker;
use crate::fix::edits::add_argument;
use crate::{Edit, Fix, FixAvailability, Violation};

/// ## What it does
/// Checks for `@task`-decorated functions that appear to return a `Mapping`
/// (a dict literal, a dict comprehension, or an annotated return type in the
/// `Mapping` family) without explicitly specifying the `multiple_outputs`
/// keyword argument.
///
/// ## Why is this bad?
/// Airflow infers `multiple_outputs` from the function's return type
/// annotation: if the annotation is a `Mapping` subclass, the returned
/// mapping's keys are pushed as separate `XCom` entries; otherwise the entire
/// return value is stored as a single `XCom`. This implicit coupling between
/// type annotations and runtime `XCom` semantics is surprising to Dag authors
/// and fragile to future changes in the inference logic.
///
/// Passing `multiple_outputs` explicitly makes the intent clear and decouples
/// the `XCom` shape from the annotation.
///
/// ## Example
/// ```python
/// from airflow.sdk import task
///
///
/// @task
/// def my_task() -> dict:
///     return {"x": 1}
/// ```
///
/// Use instead:
/// ```python
/// from airflow.sdk import task
///
///
/// @task(multiple_outputs=True)
/// def my_task() -> dict:
///     return {"x": 1}
/// ```
///
/// ## Fix safety
/// The fix is always unsafe: it pins `multiple_outputs` to the value Airflow
/// would infer today. If a future Airflow release changes the inference
/// rules, the explicit value may end up disagreeing with what the implicit
/// behavior would have been.
#[derive(ViolationMetadata)]
#[violation_metadata(preview_since = "0.15.13")]
pub(crate) struct AirflowTaskMissingMultipleOutputs {
    inferred: bool,
}

impl Violation for AirflowTaskMissingMultipleOutputs {
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::Always;

    #[derive_message_formats]
    fn message(&self) -> String {
        "`multiple_outputs` should be explicitly specified on `@task` returning a `Mapping`"
            .to_string()
    }

    fn fix_title(&self) -> Option<String> {
        Some(format!(
            "Add `multiple_outputs={}`",
            if self.inferred { "True" } else { "False" }
        ))
    }
}

/// AIR202
pub(crate) fn task_explicit_multiple_outputs(checker: &Checker, function_def: &StmtFunctionDef) {
    if !checker.semantic().seen_module(Modules::AIRFLOW) {
        return;
    }

    let semantic = checker.semantic();

    // Find the `@task` (or `@task.<variant>`) decorator, if any.
    let Some(task_decorator) = function_def
        .decorator_list
        .iter()
        .find(|decorator| is_task_decorator(&decorator.expression, semantic))
    else {
        return;
    };

    // If the decorator already specifies `multiple_outputs`, we're done.
    if let Expr::Call(call) = &task_decorator.expression {
        if call.arguments.find_keyword("multiple_outputs").is_some() {
            return;
        }
    }

    // Determine whether the function "looks like" it returns a mapping.
    let annotation_is_mapping = function_def
        .returns
        .as_deref()
        .is_some_and(|annotation| is_mapping_annotation(annotation, semantic));
    let body_returns_mapping = body_returns_mapping(&function_def.body);

    if !annotation_is_mapping && !body_returns_mapping {
        return;
    }

    // Airflow's runtime inference: `multiple_outputs=True` iff the return
    // annotation is a `Mapping` subclass; otherwise `False`.
    let inferred = annotation_is_mapping;
    let argument = format!(
        "multiple_outputs={}",
        if inferred { "True" } else { "False" }
    );

    let edit = match &task_decorator.expression {
        Expr::Call(call) => add_argument(&argument, &call.arguments, checker.tokens()),
        other => Edit::insertion(format!("({argument})"), other.range().end()),
    };

    checker
        .report_diagnostic(
            AirflowTaskMissingMultipleOutputs { inferred },
            task_decorator.range(),
        )
        .set_fix(Fix::unsafe_edit(edit));
}

/// Returns `true` if the given decorator expression is `@task` or
/// `@task.<variant>` (for `airflow.decorators.task` or `airflow.sdk.task`),
/// including the called form `@task(...)` / `@task.<variant>(...)`.
fn is_task_decorator(expr: &Expr, semantic: &SemanticModel) -> bool {
    let inner = map_callable(expr);

    if semantic
        .resolve_qualified_name(inner)
        .is_some_and(|qn| matches!(qn.segments(), ["airflow", "decorators" | "sdk", "task"]))
    {
        return true;
    }

    if let Expr::Attribute(ExprAttribute { value, .. }) = inner {
        return semantic
            .resolve_qualified_name(value)
            .is_some_and(|qn| matches!(qn.segments(), ["airflow", "decorators" | "sdk", "task"]));
    }

    false
}

/// Returns `true` if the annotation expression refers (by name) to a
/// `Mapping`-family type.
fn is_mapping_annotation(expr: &Expr, semantic: &SemanticModel) -> bool {
    // For subscripted annotations like `dict[str, int]`, resolve the base.
    let target = if let Expr::Subscript(ast::ExprSubscript { value, .. }) = expr {
        value.as_ref()
    } else {
        expr
    };

    if semantic.match_builtin_expr(target, "dict") {
        return true;
    }

    semantic.resolve_qualified_name(target).is_some_and(|qn| {
        matches!(
            qn.segments(),
            [
                "typing" | "typing_extensions",
                "Dict" | "Mapping" | "MutableMapping" | "OrderedDict"
            ] | ["collections", "abc", "Mapping" | "MutableMapping"]
                | ["collections", "OrderedDict"]
        )
    })
}

/// Returns `true` if any `return` in the function body returns a dict literal
/// or dict comprehension.
fn body_returns_mapping(body: &[ast::Stmt]) -> bool {
    let mut visitor = ReturnStatementVisitor::default();
    visitor.visit_body(body);
    visitor.returns.iter().any(|ret| {
        ret.value
            .as_deref()
            .is_some_and(|value| matches!(value, Expr::Dict(_) | Expr::DictComp(_)))
    })
}
