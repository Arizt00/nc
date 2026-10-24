use crate::{AssignTarget, Context, Expr, Pattern, Program, Section, Stmt, UnaryOp};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const SOURCE_EXECUTION_SCHEMA: &str = "nc.source.execution.v2";
pub const SOURCE_EXPANSION_SCHEMA: &str = "nc.source.expansion.v2";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceExecutionMode {
    ManualRaw,
}

impl SourceExecutionMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ManualRaw => "manual_raw",
        }
    }

    pub fn parse_profile(profile: &str) -> Option<Self> {
        (!profile.trim().is_empty()).then_some(Self::ManualRaw)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceModeOrigin {
    DefaultManual,
    PackageProfile,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceCheckPolicy {
    pub bounds: bool,
    pub overflow: bool,
    pub null_dereference: bool,
    pub pointer_provenance: bool,
    pub lifetime: bool,
    pub aliasing: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceHiddenWorkPolicy {
    pub implicit_cleanup: bool,
    pub hidden_allocation: bool,
    pub implicit_scheduler: bool,
    pub hidden_dynamic_dispatch: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceExplicitEffectPolicy {
    pub memory_requires_source_operation: bool,
    pub allocation_requires_source_call: bool,
    pub release_requires_source_call: bool,
    pub io_requires_source_call: bool,
    pub process_requires_source_call: bool,
    pub thread_requires_source_call: bool,
    pub synchronization_requires_source_call: bool,
    pub scheduling_requires_source_call: bool,
    pub recovery_requires_source_construct: bool,
    pub dynamic_dispatch_requires_source_construct: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceExecutionContract {
    pub schema: &'static str,
    pub mode: SourceExecutionMode,
    pub origin: SourceModeOrigin,
    pub compact_source_is_official: bool,
    pub verbose_form_is_internal_only: bool,
    pub checks: SourceCheckPolicy,
    pub hidden_work: SourceHiddenWorkPolicy,
    pub explicit_effects: SourceExplicitEffectPolicy,
    pub raw_pointer_ub_is_permitted: bool,
    pub deterministic_expansion: bool,
    pub expansion_is_inspectable: bool,
    pub kernel_forbids_hosted_runtime: bool,
    pub kernel_forbids_implicit_heap: bool,
    pub kernel_forbids_hidden_dispatch: bool,
    pub kernel_constraints_active: bool,
}

impl SourceExecutionContract {
    pub fn for_mode(mode: SourceExecutionMode, origin: SourceModeOrigin) -> Self {
        Self {
            schema: SOURCE_EXECUTION_SCHEMA,
            mode,
            origin,
            compact_source_is_official: true,
            verbose_form_is_internal_only: true,
            checks: SourceCheckPolicy {
                bounds: false,
                overflow: false,
                null_dereference: false,
                pointer_provenance: false,
                lifetime: false,
                aliasing: false,
            },
            hidden_work: SourceHiddenWorkPolicy {
                implicit_cleanup: false,
                hidden_allocation: false,
                implicit_scheduler: false,
                hidden_dynamic_dispatch: false,
            },
            explicit_effects: SourceExplicitEffectPolicy {
                memory_requires_source_operation: true,
                allocation_requires_source_call: true,
                release_requires_source_call: true,
                io_requires_source_call: true,
                process_requires_source_call: true,
                thread_requires_source_call: true,
                synchronization_requires_source_call: true,
                scheduling_requires_source_call: true,
                recovery_requires_source_construct: true,
                dynamic_dispatch_requires_source_construct: true,
            },
            raw_pointer_ub_is_permitted: true,
            deterministic_expansion: true,
            expansion_is_inspectable: true,
            kernel_forbids_hosted_runtime: true,
            kernel_forbids_implicit_heap: true,
            kernel_forbids_hidden_dispatch: true,
            kernel_constraints_active: false,
        }
    }

    pub fn manual_raw() -> Self {
        Self::for_mode(
            SourceExecutionMode::ManualRaw,
            SourceModeOrigin::DefaultManual,
        )
    }

    pub const fn is_manual_raw(&self) -> bool {
        matches!(self.mode, SourceExecutionMode::ManualRaw)
    }

    pub const fn is_fully_manual(&self) -> bool {
        self.is_manual_raw()
            && !self.checks.bounds
            && !self.checks.overflow
            && !self.checks.null_dereference
            && !self.checks.pointer_provenance
            && !self.checks.lifetime
            && !self.checks.aliasing
            && !self.hidden_work.implicit_cleanup
            && !self.hidden_work.hidden_allocation
            && !self.hidden_work.implicit_scheduler
            && !self.hidden_work.hidden_dynamic_dispatch
            && self.explicit_effects.memory_requires_source_operation
            && self.explicit_effects.allocation_requires_source_call
            && self.explicit_effects.release_requires_source_call
            && self.explicit_effects.io_requires_source_call
            && self.explicit_effects.process_requires_source_call
            && self.explicit_effects.thread_requires_source_call
            && self.explicit_effects.synchronization_requires_source_call
            && self.explicit_effects.scheduling_requires_source_call
            && self.explicit_effects.recovery_requires_source_construct
            && self
                .explicit_effects
                .dynamic_dispatch_requires_source_construct
    }

    pub fn require_fully_manual(&self) -> Result<(), String> {
        self.is_fully_manual().then_some(()).ok_or_else(|| {
            "E_SOURCE_MANUAL_CONTRACT_001: source contract would inject compiler-owned runtime work"
                .to_string()
        })
    }

    pub fn constrained_for_kernel(mut self) -> Self {
        self.hidden_work.implicit_cleanup = false;
        self.hidden_work.hidden_allocation = false;
        self.hidden_work.implicit_scheduler = false;
        self.hidden_work.hidden_dynamic_dispatch = false;
        self.kernel_constraints_active = true;
        self
    }

    pub fn material_signature(&self) -> String {
        format!(
            "{};mode={};bounds={};overflow={};lifetime={};hidden_alloc={};implicit_scheduler={};explicit_memory={};explicit_io={};explicit_threads={};kernel_constraints={}",
            self.schema,
            self.mode.as_str(),
            self.checks.bounds,
            self.checks.overflow,
            self.checks.lifetime,
            self.hidden_work.hidden_allocation,
            self.hidden_work.implicit_scheduler,
            self.explicit_effects.memory_requires_source_operation,
            self.explicit_effects.io_requires_source_call,
            self.explicit_effects.thread_requires_source_call,
            self.kernel_constraints_active,
        )
    }
}

impl Default for SourceExecutionContract {
    fn default() -> Self {
        Self::manual_raw()
    }
}

pub fn resolve_source_execution_contract(
    program: &Program,
) -> Result<SourceExecutionContract, String> {
    // Package profiles are an open build/platform namespace. They never select a
    // second source execution semantics: every profile lowers through ManualRaw.
    let explicit = program
        .packages
        .iter()
        .any(|package| package.profile.is_some());

    Ok(if explicit {
        SourceExecutionContract::for_mode(
            SourceExecutionMode::ManualRaw,
            SourceModeOrigin::PackageProfile,
        )
    } else {
        SourceExecutionContract::manual_raw()
    })
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceExpansionRow {
    pub family: String,
    pub occurrences: usize,
    pub manual_lowering: String,
    pub explicit_in_source: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceExpansionReport {
    pub schema: &'static str,
    pub mode: SourceExecutionMode,
    pub deterministic: bool,
    pub convertible_to_manual_ir: bool,
    pub compiler_owned_runtime_effects: usize,
    pub rows: Vec<SourceExpansionRow>,
    pub total_occurrences: usize,
}

impl SourceExpansionReport {
    pub fn empty(mode: SourceExecutionMode) -> Self {
        Self {
            schema: SOURCE_EXPANSION_SCHEMA,
            mode,
            deterministic: true,
            convertible_to_manual_ir: true,
            compiler_owned_runtime_effects: 0,
            rows: Vec::new(),
            total_occurrences: 0,
        }
    }
}

impl Default for SourceExpansionReport {
    fn default() -> Self {
        Self::empty(SourceExecutionMode::ManualRaw)
    }
}

#[derive(Default)]
struct ExpansionCounts(BTreeMap<&'static str, usize>);

impl ExpansionCounts {
    fn add(&mut self, family: &'static str) {
        *self.0.entry(family).or_default() += 1;
    }

    fn scan_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::StructLiteral { fields, .. } => {
                self.add("aggregate_literal_storage");
                for (_, value) in fields {
                    self.scan_expr(value);
                }
            }
            Expr::VectorLiteral(items) | Expr::ArrayLiteral(items) => {
                self.add("aggregate_literal_storage");
                for item in items {
                    self.scan_expr(item);
                }
            }
            Expr::Index { target, index } => {
                self.scan_expr(target);
                self.scan_expr(index);
            }
            Expr::Closure(closure) => {
                self.add("closure_environment");
                if closure.is_async {
                    self.add("coroutine_state_machine");
                }
                for stmt in &closure.body {
                    self.scan_stmt(stmt);
                }
            }
            Expr::MethodCall {
                receiver,
                method,
                args,
            } => {
                self.scan_call(method);
                self.scan_expr(receiver);
                for arg in args {
                    self.scan_expr(arg);
                }
            }
            Expr::QualifiedCall { path, args } => {
                self.scan_call(path.last().map(String::as_str).unwrap_or_default());
                for arg in args {
                    self.scan_expr(arg);
                }
            }
            Expr::StrictCheckExpr { value, .. } => self.scan_expr(value),
            Expr::Unary { op, expr } => {
                if matches!(op, UnaryOp::TryPropagate) {
                    self.add("error_propagation");
                }
                self.scan_expr(expr);
            }
            Expr::Binary { left, right, .. } => {
                self.scan_expr(left);
                self.scan_expr(right);
            }
            Expr::Call { callee, args } => {
                self.scan_call(callee);
                for arg in args {
                    self.scan_expr(arg);
                }
            }
            Expr::IfElse {
                condition,
                then_expr,
                else_expr,
            } => {
                self.scan_expr(condition);
                self.scan_expr(then_expr);
                self.scan_expr(else_expr);
            }
            Expr::Bool(_)
            | Expr::Number(_)
            | Expr::Char(_)
            | Expr::String(_)
            | Expr::Identifier(_)
            | Expr::ContextRef(_)
            | Expr::Dev(_)
            | Expr::PostIncrement { .. }
            | Expr::FieldPostIncrement { .. }
            | Expr::PointerPostIncrementRead { .. } => {}
        }
    }

    fn scan_call(&mut self, callee: &str) {
        let leaf = callee
            .split('<')
            .next()
            .unwrap_or(callee)
            .rsplit(['.', ':'])
            .find(|part| !part.is_empty())
            .unwrap_or(callee)
            .to_ascii_lowercase();
        match leaf.as_str() {
            "await" | "yield" => self.add("coroutine_state_machine"),
            "spawn" | "asyncrun" | "schedulersubmit" | "taskspawn" => self.add("scheduler_request"),
            "alloc"
            | "calloc"
            | "realloc"
            | "free"
            | "regioncreate"
            | "listcreate"
            | "listcreatewithcapacity"
            | "listclone"
            | "queuecreate"
            | "queuecreatewithcapacity"
            | "vectorcreate"
            | "vectorcreatewithcapacity"
            | "dinarraycreate"
            | "dinarraycreatewithcapacity"
            | "listadd"
            | "listpush"
            | "listinsert"
            | "listset"
            | "listremove"
            | "listclear"
            | "listreserve"
            | "listshrinktofit"
            | "listdestroy"
            | "queuepush"
            | "queuepop"
            | "queueset"
            | "queueclear"
            | "queuereserve"
            | "queuedestroy"
            | "vectoradd"
            | "vectorpush"
            | "vectorpop"
            | "vectorinsert"
            | "vectorset"
            | "vectorremove"
            | "vectorclear"
            | "vectorreserve"
            | "vectorshrinktofit"
            | "vectordestroy"
            | "dinarrayadd"
            | "dinarraypush"
            | "dinarraypop"
            | "dinarrayinsert"
            | "dinarrayset"
            | "dinarrayremove"
            | "dinarrayclear"
            | "dinarrayreserve"
            | "dinarrayshrinktofit"
            | "dinarraydestroy"
            | "push"
            | "pop"
            | "insert"
            | "remove"
            | "reserve"
            | "shrinktofit" => self.add("explicit_memory_operation"),
            _ => {}
        }
    }

    fn scan_assign_target(&mut self, target: &AssignTarget) {
        if let AssignTarget::Mem { addr } = target {
            self.scan_expr(addr);
        }
    }

    fn scan_pattern(&mut self, pattern: &Pattern) {
        match pattern {
            Pattern::VariantPattern { items, .. } | Pattern::Array { items, .. } => {
                for item in items {
                    self.scan_pattern(item);
                }
            }
            Pattern::Struct { fields, .. } => {
                for field in fields {
                    self.scan_pattern(&field.pattern);
                }
            }
            Pattern::Range { start, end, .. } => {
                self.scan_expr(start);
                self.scan_expr(end);
            }
            Pattern::Literal(value) => self.scan_expr(value),
            Pattern::Wildcard | Pattern::Variant { .. } | Pattern::Binding(_) => {}
        }
    }

    fn scan_block(&mut self, body: &[Stmt]) {
        for stmt in body {
            self.scan_stmt(stmt);
        }
    }

    fn scan_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Return(expr) | Stmt::Print { value: expr, .. } | Stmt::Expr(expr) => {
                self.scan_expr(expr)
            }
            Stmt::Projection { target, value, .. } | Stmt::StrictCheck { target, value, .. } => {
                self.scan_assign_target(target);
                self.scan_expr(value);
            }
            Stmt::If {
                condition,
                then_block,
                else_block,
            } => {
                self.scan_expr(condition);
                self.scan_block(then_block);
                self.scan_block(else_block);
            }
            Stmt::Unless { condition, body }
            | Stmt::While { condition, body }
            | Stmt::RepeatUntil { condition, body } => {
                self.scan_expr(condition);
                self.scan_block(body);
            }
            Stmt::DoWhile { body, condition } => {
                self.scan_block(body);
                self.scan_expr(condition);
            }
            Stmt::ForClassic {
                init,
                condition,
                step,
                body,
            } => {
                if let Some(init) = init {
                    self.scan_stmt(init);
                }
                if let Some(condition) = condition {
                    self.scan_expr(condition);
                }
                if let Some(step) = step {
                    self.scan_stmt(step);
                }
                self.scan_block(body);
            }
            Stmt::ForIn { iterable, body, .. } | Stmt::SameIn { iterable, body, .. } => {
                self.add("iterator_state_machine");
                self.scan_expr(iterable);
                self.scan_block(body);
            }
            Stmt::ForRange {
                start,
                end,
                step,
                body,
                ..
            } => {
                self.scan_expr(start);
                self.scan_expr(end);
                if let Some(step) = step {
                    self.scan_expr(step);
                }
                self.scan_block(body);
            }
            Stmt::Loop { body } => self.scan_block(body),
            Stmt::RepeatCount { count, body } => {
                self.scan_expr(count);
                self.scan_block(body);
            }
            Stmt::EachDoWhile {
                iterable,
                body,
                condition,
                ..
            } => {
                self.add("iterator_state_machine");
                self.scan_expr(iterable);
                self.scan_block(body);
                self.scan_expr(condition);
            }
            Stmt::Labeled { stmt, .. } => self.scan_stmt(stmt),
            Stmt::Switch {
                value,
                cases,
                default,
            }
            | Stmt::Match {
                value,
                cases,
                default,
            } => {
                self.scan_expr(value);
                for (case, body) in cases {
                    self.scan_expr(case);
                    self.scan_block(body);
                }
                self.scan_block(default);
            }
            Stmt::PatternMatch { value, arms } => {
                self.add("pattern_decision_tree");
                self.scan_expr(value);
                for arm in arms {
                    self.scan_pattern(&arm.pattern);
                    if let Some(guard) = &arm.guard {
                        self.scan_expr(guard);
                    }
                    self.scan_block(&arm.body);
                }
            }
            Stmt::Defer { body } => {
                self.add("deferred_cleanup");
                self.scan_block(body);
            }
            Stmt::GuardElse {
                condition,
                else_block,
            } => {
                self.scan_expr(condition);
                self.scan_block(else_block);
            }
            Stmt::Select { branches, default } => {
                for (condition, body) in branches {
                    self.scan_expr(condition);
                    self.scan_block(body);
                }
                self.scan_block(default);
            }
            Stmt::Destructure { args, .. } | Stmt::Constructure { args, .. } => {
                for arg in args {
                    self.scan_expr(arg);
                }
            }
            Stmt::Dev(_)
            | Stmt::Break { .. }
            | Stmt::Continue { .. }
            | Stmt::MainRefFn { .. }
            | Stmt::FunctionRelation { .. } => {}
        }
    }
}

fn manual_lowering_for(family: &str) -> &'static str {
    match family {
        "closure_environment" => "explicit code pointer plus caller-owned environment layout",
        "coroutine_state_machine" => {
            "explicit caller-owned frame plus new/resume/drop/cancel control functions"
        }
        "deferred_cleanup" => "cleanup blocks cloned onto every terminating control-flow edge",
        "error_propagation" => "tag test, payload extraction and explicit early return",
        "iterator_state_machine" => "explicit cursor, bounds/end test and next transition",
        "pattern_decision_tree" => "ordered tag/range/field tests and explicit branch targets",
        "scheduler_request" => "explicit scheduler/runtime call present in source",
        "explicit_memory_operation" => {
            "explicit allocator, collection storage or release operation present in source"
        }
        "aggregate_literal_storage" => {
            "explicit backing bytes plus the declared aggregate descriptor"
        }
        "static_monomorphization" => {
            "one concrete function/type body per resolved type/const tuple"
        }
        _ => "explicit typed control-flow and storage operations",
    }
}

pub fn source_expansion_report(
    program: &Program,
    contract: &SourceExecutionContract,
) -> SourceExpansionReport {
    let mut counts = ExpansionCounts::default();
    for container in &program.containers {
        for section in &container.sections {
            match section {
                Section::Fn(functions)
                | Section::MoveFn(functions)
                | Section::ConstFn(functions) => {
                    for function in functions {
                        if function.is_async {
                            counts.add("coroutine_state_machine");
                        }
                        if !function.generic_params.is_empty() {
                            counts.add("static_monomorphization");
                        }
                        counts.scan_block(&function.public_body);
                    }
                }
                Section::Probe(body)
                | Section::Structure(body)
                | Section::Final(body)
                | Section::Bind(body)
                | Section::Raw(body)
                | Section::Binary(body)
                | Section::MainEntry { body, .. } => counts.scan_block(body),
                _ => {}
            }
        }
    }

    let rows = counts
        .0
        .into_iter()
        .map(|(family, occurrences)| SourceExpansionRow {
            family: family.to_string(),
            occurrences,
            manual_lowering: manual_lowering_for(family).to_string(),
            explicit_in_source: true,
        })
        .collect::<Vec<_>>();
    let total_occurrences = rows.iter().map(|row| row.occurrences).sum();
    SourceExpansionReport {
        schema: SOURCE_EXPANSION_SCHEMA,
        mode: contract.mode,
        deterministic: true,
        convertible_to_manual_ir: true,
        compiler_owned_runtime_effects: 0,
        rows,
        total_occurrences,
    }
}

pub fn program_uses_legacy_kernel_context(program: &Program) -> bool {
    program
        .containers
        .iter()
        .any(|container| matches!(container.context, Some(Context::Kernel)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{PackageDecl, Program};

    fn program_with_profile(profile: Option<&str>) -> Program {
        Program {
            modules: vec![],
            packages: profile
                .map(|profile| PackageDecl {
                    name: "modeprobe".into(),
                    version: None,
                    profile: Some(profile.into()),
                    modules: vec![],
                })
                .into_iter()
                .collect(),
            uses: vec![],
            containers: vec![],
        }
    }

    #[test]
    fn manual_raw_is_the_default_and_disables_hidden_work() {
        let contract = resolve_source_execution_contract(&program_with_profile(None)).unwrap();
        assert_eq!(contract.mode, SourceExecutionMode::ManualRaw);
        assert!(!contract.checks.bounds);
        assert!(!contract.checks.overflow);
        assert!(!contract.checks.lifetime);
        assert!(!contract.hidden_work.hidden_allocation);
        assert!(!contract.hidden_work.implicit_scheduler);
        assert!(contract.raw_pointer_ub_is_permitted);
        assert!(contract.is_fully_manual());
        contract.require_fully_manual().unwrap();
        assert!(contract.explicit_effects.memory_requires_source_operation);
        assert!(contract.explicit_effects.io_requires_source_call);
        assert!(contract.explicit_effects.thread_requires_source_call);
    }

    #[test]
    fn any_hidden_runtime_work_breaks_the_manual_contract() {
        let mut contract = SourceExecutionContract::manual_raw();
        contract.hidden_work.implicit_scheduler = true;
        assert!(!contract.is_fully_manual());
        assert!(contract
            .require_fully_manual()
            .expect_err("hidden scheduler must violate manual authority")
            .contains("E_SOURCE_MANUAL_CONTRACT_001"));
    }

    #[test]
    fn explicit_raw_profile_selects_the_only_execution_contract() {
        let contract =
            resolve_source_execution_contract(&program_with_profile(Some("raw"))).unwrap();
        assert_eq!(contract.mode, SourceExecutionMode::ManualRaw);
        assert_eq!(contract.origin, SourceModeOrigin::PackageProfile);
        assert!(!contract.checks.bounds);
        assert!(!contract.hidden_work.hidden_allocation);
    }

    #[test]
    fn kernel_constraints_preserve_the_raw_no_hidden_work_contract() {
        let contract = resolve_source_execution_contract(&program_with_profile(Some("manual")))
            .unwrap()
            .constrained_for_kernel();
        assert_eq!(contract.mode, SourceExecutionMode::ManualRaw);
        assert!(!contract.checks.bounds);
        assert!(contract.kernel_constraints_active);
        assert!(!contract.hidden_work.implicit_cleanup);
        assert!(!contract.hidden_work.hidden_allocation);
        assert!(!contract.hidden_work.implicit_scheduler);
        assert!(!contract.hidden_work.hidden_dynamic_dispatch);
    }

    #[test]
    fn arbitrary_package_profiles_keep_manual_raw_execution() {
        let contract =
            resolve_source_execution_contract(&program_with_profile(Some("vendor_fast"))).unwrap();
        assert_eq!(contract.mode, SourceExecutionMode::ManualRaw);
        assert_eq!(contract.origin, SourceModeOrigin::PackageProfile);
        assert!(!contract.checks.bounds);
        assert!(!contract.hidden_work.hidden_allocation);
    }

    #[test]
    fn every_non_empty_profile_name_maps_to_manual_raw() {
        for profile in ["manual", "raw", "kernel", "release_fast", "vendor_debug"] {
            assert_eq!(
                SourceExecutionMode::parse_profile(profile),
                Some(SourceExecutionMode::ManualRaw),
                "profile '{profile}' must remain open metadata"
            );
        }
        assert_eq!(SourceExecutionMode::parse_profile("   "), None);
    }

    #[test]
    fn expansion_scan_includes_memory_targets_and_explicit_runtime_work() {
        let mut counts = ExpansionCounts::default();
        counts.scan_stmt(&Stmt::Projection {
            target: AssignTarget::Mem {
                addr: Box::new(Expr::Call {
                    callee: "alloc".into(),
                    args: vec![Expr::Number("64".into())],
                }),
            },
            ty: crate::TypeName::Simple("int".into()),
            value: Expr::Call {
                callee: "schedulerSubmit".into(),
                args: Vec::new(),
            },
        });
        counts.scan_expr(&Expr::MethodCall {
            receiver: Box::new(Expr::ArrayLiteral(vec![Expr::Number("1".into())])),
            method: "push".into(),
            args: vec![Expr::Number("2".into())],
        });
        assert_eq!(counts.0.get("explicit_memory_operation"), Some(&2));
        assert_eq!(counts.0.get("aggregate_literal_storage"), Some(&1));
        assert_eq!(counts.0.get("scheduler_request"), Some(&1));
    }
}
