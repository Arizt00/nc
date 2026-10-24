use super::*;
use std::collections::{HashMap, HashSet, VecDeque};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ZeroCostMaterializationReport {
    pub monomorphized_functions: Vec<String>,
    pub statically_dispatched_methods: Vec<String>,
    pub templates_removed: usize,
    pub hidden_dynamic_dispatches: usize,
    pub kernel_hidden_allocations: usize,
    pub kernel_hidden_scheduler_calls: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct GenericInstanceKey {
    container: String,
    function: String,
    arguments: Vec<String>,
}

#[derive(Debug, Clone)]
struct PendingFunction {
    container: String,
    function: FunctionDecl,
    report_name: String,
    is_static_dispatch: bool,
}

struct Materializer<'a> {
    source: &'a Program,
    kernel_profile: bool,
    generic_symbols: HashMap<GenericInstanceKey, String>,
    static_symbols: HashMap<(String, String, String), String>,
    pending: VecDeque<PendingFunction>,
    emitted_symbols: HashSet<String>,
    generated: HashMap<String, Vec<FunctionDecl>>,
    report: ZeroCostMaterializationReport,
}

pub fn materialize_zero_cost_program(
    program: &Program,
    kernel_profile: bool,
) -> Result<(Program, ZeroCostMaterializationReport), String> {
    analyze(program)?;
    materialize_zero_cost_program_after_sema(program, kernel_profile)
}

pub fn materialize_zero_cost_program_after_sema(
    program: &Program,
    kernel_profile: bool,
) -> Result<(Program, ZeroCostMaterializationReport), String> {
    refresh_high_phase4_indexes(program)?;
    refresh_function_indexes(program);
    let mut materializer = Materializer::new(program, kernel_profile);
    materializer.seed_static_impl_methods()?;
    let mut output = program.clone();

    for container in &mut output.containers {
        let base_scope = materializer.container_scope(container)?;
        for section in &mut container.sections {
            match section {
                Section::Fn(functions) | Section::MoveFn(functions) => {
                    let mut concrete = Vec::with_capacity(functions.len());
                    for mut function in std::mem::take(functions) {
                        if function.generic_params.is_empty() {
                            materializer.rewrite_function(
                                &container.name,
                                &container.context,
                                &base_scope,
                                &mut function,
                            )?;
                            concrete.push(function);
                        } else {
                            materializer.report.templates_removed += 1;
                        }
                    }
                    *functions = concrete;
                }
                Section::MainEntry { body, .. } => {
                    let mut scope = base_scope.clone();
                    materializer.rewrite_block(
                        &container.name,
                        &container.context,
                        body,
                        &mut scope,
                    )?;
                }
                Section::Final(body)
                | Section::Probe(body)
                | Section::Structure(body)
                | Section::Binary(body)
                | Section::Bind(body)
                | Section::Raw(body) => {
                    let mut scope = base_scope.clone();
                    materializer.rewrite_block(
                        &container.name,
                        &container.context,
                        body,
                        &mut scope,
                    )?;
                }
                _ => {}
            }
        }
    }

    materializer.drain_pending()?;
    for container in &mut output.containers {
        if let Some(functions) = materializer.generated.remove(&container.name) {
            if !functions.is_empty() {
                container.sections.push(Section::MoveFn(functions));
            }
        }
    }

    if !materializer.generated.is_empty() {
        let mut owners = materializer.generated.keys().cloned().collect::<Vec<_>>();
        owners.sort();
        return Err(format!(
            "E_HIGH_MONO_OWNER_001: materialized symbols reference missing containers: {}",
            owners.join(", ")
        ));
    }

    materializer.verify_closed(&output)?;
    materializer.report.monomorphized_functions.sort();
    materializer.report.monomorphized_functions.dedup();
    materializer.report.statically_dispatched_methods.sort();
    materializer.report.statically_dispatched_methods.dedup();
    Ok((output, materializer.report))
}

impl<'a> Materializer<'a> {
    fn new(source: &'a Program, kernel_profile: bool) -> Self {
        Self {
            source,
            kernel_profile,
            generic_symbols: HashMap::new(),
            static_symbols: HashMap::new(),
            pending: VecDeque::new(),
            emitted_symbols: HashSet::new(),
            generated: HashMap::new(),
            report: ZeroCostMaterializationReport::default(),
        }
    }

    fn container_scope(&self, container: &Container) -> Result<HashMap<String, SemaType>, String> {
        let aliases = collect_local_type_aliases(container);
        let mut scope = HashMap::new();
        for section in &container.sections {
            match section {
                Section::Stc(decls) => {
                    for decl in decls {
                        let StcDecl::Typed { name, ty, .. } = decl;
                        scope.insert(name.clone(), lower_type(ty));
                    }
                }
                Section::Local(local) => {
                    for decl in &local.decls {
                        if let Some((name, ty, _, _)) = local_decl_value_parts(decl) {
                            let ty = resolve_local_type_aliases(ty, &aliases)?;
                            scope.insert(name.clone(), lower_type(&ty));
                        }
                    }
                }
                Section::ConstEval(decls) | Section::Comptime(decls) => {
                    for decl in decls {
                        scope.insert(decl.name.clone(), lower_type(&decl.ty));
                    }
                }
                _ => {}
            }
        }
        Ok(scope)
    }

    fn seed_static_impl_methods(&mut self) -> Result<(), String> {
        for container in &self.source.containers {
            for section in &container.sections {
                let Section::ImplBlocks(impls) = section else {
                    continue;
                };
                for implementation in impls {
                    let surface = type_name_base(&implementation.trait_ref)
                        .ok_or("E_HIGH_IMPL_UNKNOWN_TRAIT_001: impl target must name a trait")?;
                    let concrete = render_type_name(&implementation.for_type);
                    for method in &implementation.methods {
                        if method.generic_params.is_empty() {
                            let symbol = static_dispatch_symbol(surface, &concrete, &method.name);
                            self.static_symbols.insert(
                                (
                                    surface.to_ascii_lowercase(),
                                    concrete.clone(),
                                    method.name.clone(),
                                ),
                                symbol.clone(),
                            );
                            self.enqueue(
                                &container.name,
                                renamed_function(method, symbol.clone()),
                                symbol,
                                true,
                            );
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn enqueue(
        &mut self,
        container: &str,
        function: FunctionDecl,
        report_name: String,
        is_static_dispatch: bool,
    ) {
        if self.emitted_symbols.insert(function.name.clone()) {
            self.pending.push_back(PendingFunction {
                container: container.to_string(),
                function,
                report_name,
                is_static_dispatch,
            });
        }
    }

    fn drain_pending(&mut self) -> Result<(), String> {
        while let Some(mut pending) = self.pending.pop_front() {
            let container = self
                .source
                .containers
                .iter()
                .find(|container| container.name == pending.container)
                .ok_or_else(|| {
                    format!(
                        "E_HIGH_MONO_OWNER_001: container '{}' does not exist",
                        pending.container
                    )
                })?;
            let scope = self.container_scope(container)?;
            self.rewrite_function(
                &container.name,
                &container.context,
                &scope,
                &mut pending.function,
            )?;
            if pending.is_static_dispatch {
                self.report
                    .statically_dispatched_methods
                    .push(pending.report_name);
            } else {
                self.report
                    .monomorphized_functions
                    .push(pending.report_name);
            }
            self.generated
                .entry(pending.container)
                .or_default()
                .push(pending.function);
        }
        Ok(())
    }

    fn rewrite_function(
        &mut self,
        container: &str,
        context: &Option<Context>,
        base_scope: &HashMap<String, SemaType>,
        function: &mut FunctionDecl,
    ) -> Result<(), String> {
        let mut scope = base_scope.clone();
        for param in &function.params {
            scope.insert(param.name.clone(), lower_type(&param.ty));
        }
        self.rewrite_block(container, context, &mut function.public_body, &mut scope)
    }

    fn rewrite_block(
        &mut self,
        container: &str,
        context: &Option<Context>,
        body: &mut [Stmt],
        scope: &mut HashMap<String, SemaType>,
    ) -> Result<(), String> {
        for stmt in body {
            self.rewrite_stmt(container, context, stmt, scope)?;
        }
        Ok(())
    }

    fn rewrite_stmt(
        &mut self,
        container: &str,
        context: &Option<Context>,
        stmt: &mut Stmt,
        scope: &mut HashMap<String, SemaType>,
    ) -> Result<(), String> {
        match stmt {
            Stmt::Return(expr) | Stmt::Expr(expr) | Stmt::Print { value: expr, .. } => {
                self.rewrite_expr(container, context, expr, scope)
            }
            Stmt::Projection { target, ty, value } | Stmt::StrictCheck { target, ty, value } => {
                self.rewrite_assign_target(container, context, target, scope)?;
                self.rewrite_expr(container, context, value, scope)?;
                if let AssignTarget::Context(reference) = target {
                    if reference.path.len() == 1 {
                        scope.insert(reference.path[0].clone(), lower_type(ty));
                    }
                }
                Ok(())
            }
            Stmt::If {
                condition,
                then_block,
                else_block,
            } => {
                self.rewrite_expr(container, context, condition, scope)?;
                self.rewrite_block(container, context, then_block, &mut scope.clone())?;
                self.rewrite_block(container, context, else_block, &mut scope.clone())
            }
            Stmt::Unless { condition, body }
            | Stmt::While { condition, body }
            | Stmt::RepeatUntil { condition, body } => {
                self.rewrite_expr(container, context, condition, scope)?;
                self.rewrite_block(container, context, body, &mut scope.clone())
            }
            Stmt::DoWhile { body, condition } => {
                self.rewrite_block(container, context, body, &mut scope.clone())?;
                self.rewrite_expr(container, context, condition, scope)
            }
            Stmt::ForClassic {
                init,
                condition,
                step,
                body,
            } => {
                let mut loop_scope = scope.clone();
                if let Some(init) = init {
                    self.rewrite_stmt(container, context, init, &mut loop_scope)?;
                }
                if let Some(condition) = condition {
                    self.rewrite_expr(container, context, condition, &loop_scope)?;
                }
                self.rewrite_block(container, context, body, &mut loop_scope)?;
                if let Some(step) = step {
                    self.rewrite_stmt(container, context, step, &mut loop_scope)?;
                }
                Ok(())
            }
            Stmt::ForIn {
                item,
                iterable,
                body,
            }
            | Stmt::SameIn {
                item,
                iterable,
                body,
            } => {
                let iterable_type =
                    infer_expr_type(self.source, container, iterable, scope, context)?;
                self.rewrite_expr(container, context, iterable, scope)?;
                let mut loop_scope = scope.clone();
                loop_scope.insert(item.clone(), closure_iter_item_type(iterable_type)?);
                self.rewrite_block(container, context, body, &mut loop_scope)
            }
            Stmt::ForRange {
                item,
                start,
                end,
                step,
                body,
                ..
            } => {
                self.rewrite_expr(container, context, start, scope)?;
                self.rewrite_expr(container, context, end, scope)?;
                if let Some(step) = step {
                    self.rewrite_expr(container, context, step, scope)?;
                }
                let mut loop_scope = scope.clone();
                loop_scope.insert(item.clone(), SemaType::Simple("int".into()));
                self.rewrite_block(container, context, body, &mut loop_scope)
            }
            Stmt::Loop { body } | Stmt::Defer { body } => {
                self.rewrite_block(container, context, body, &mut scope.clone())
            }
            Stmt::RepeatCount { count, body } => {
                self.rewrite_expr(container, context, count, scope)?;
                self.rewrite_block(container, context, body, &mut scope.clone())
            }
            Stmt::EachDoWhile {
                item,
                iterable,
                body,
                condition,
            } => {
                let iterable_type =
                    infer_expr_type(self.source, container, iterable, scope, context)?;
                self.rewrite_expr(container, context, iterable, scope)?;
                let mut loop_scope = scope.clone();
                loop_scope.insert(item.clone(), closure_iter_item_type(iterable_type)?);
                self.rewrite_block(container, context, body, &mut loop_scope)?;
                self.rewrite_expr(container, context, condition, &loop_scope)
            }
            Stmt::Labeled { stmt, .. } => {
                self.rewrite_stmt(container, context, stmt, &mut scope.clone())
            }
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
                self.rewrite_expr(container, context, value, scope)?;
                for (case, body) in cases {
                    self.rewrite_expr(container, context, case, scope)?;
                    self.rewrite_block(container, context, body, &mut scope.clone())?;
                }
                self.rewrite_block(container, context, default, &mut scope.clone())
            }
            Stmt::PatternMatch { value, arms } => {
                let value_type = infer_expr_type(self.source, container, value, scope, context)?;
                self.rewrite_expr(container, context, value, scope)?;
                let enums = collect_high_enums(self.source)?;
                let enum_info = enums
                    .values()
                    .find(|info| high_enum_type_matches(&info.name, &value_type));
                for arm in arms {
                    let mut arm_scope = scope.clone();
                    add_pattern_bindings(&arm.pattern, &value_type, enum_info, &mut arm_scope);
                    if let Some(guard) = &mut arm.guard {
                        self.rewrite_expr(container, context, guard, &arm_scope)?;
                    }
                    self.rewrite_block(container, context, &mut arm.body, &mut arm_scope)?;
                }
                Ok(())
            }
            Stmt::GuardElse {
                condition,
                else_block,
            } => {
                self.rewrite_expr(container, context, condition, scope)?;
                self.rewrite_block(container, context, else_block, &mut scope.clone())
            }
            Stmt::Select { branches, default } => {
                for (condition, body) in branches {
                    self.rewrite_expr(container, context, condition, scope)?;
                    self.rewrite_block(container, context, body, &mut scope.clone())?;
                }
                self.rewrite_block(container, context, default, &mut scope.clone())
            }
            Stmt::Destructure { args, .. } | Stmt::Constructure { args, .. } => {
                for arg in args {
                    self.rewrite_expr(container, context, arg, scope)?;
                }
                Ok(())
            }
            Stmt::Break { .. }
            | Stmt::Continue { .. }
            | Stmt::Dev(_)
            | Stmt::MainRefFn { .. }
            | Stmt::FunctionRelation { .. } => Ok(()),
        }
    }

    fn rewrite_assign_target(
        &mut self,
        container: &str,
        context: &Option<Context>,
        target: &mut AssignTarget,
        scope: &HashMap<String, SemaType>,
    ) -> Result<(), String> {
        if let AssignTarget::Mem { addr } = target {
            self.rewrite_expr(container, context, addr, scope)?;
        }
        Ok(())
    }

    fn rewrite_expr(
        &mut self,
        container: &str,
        context: &Option<Context>,
        expr: &mut Expr,
        scope: &HashMap<String, SemaType>,
    ) -> Result<(), String> {
        match expr {
            Expr::StructLiteral { fields, .. } => {
                for (_, value) in fields {
                    self.rewrite_expr(container, context, value, scope)?;
                }
            }
            Expr::VectorLiteral(items) | Expr::ArrayLiteral(items) => {
                for item in items {
                    self.rewrite_expr(container, context, item, scope)?;
                }
            }
            Expr::Index { target, index } => {
                self.rewrite_expr(container, context, target, scope)?;
                self.rewrite_expr(container, context, index, scope)?;
            }
            Expr::Closure(closure) => {
                let mut closure_scope = scope.clone();
                for param in &closure.params {
                    closure_scope.insert(param.name.clone(), lower_type(&param.ty));
                }
                self.rewrite_block(container, context, &mut closure.body, &mut closure_scope)?;
            }
            Expr::MethodCall {
                receiver,
                method,
                args,
            } => {
                let mut call_args = vec![(**receiver).clone()];
                call_args.extend(args.iter().cloned());
                let replacement =
                    self.resolve_call(container, context, method, &call_args, scope)?;
                self.rewrite_expr(container, context, receiver, scope)?;
                for arg in args {
                    self.rewrite_expr(container, context, arg, scope)?;
                }
                if let Some(symbol) = replacement {
                    *method = symbol;
                }
            }
            Expr::QualifiedCall { path, args } => {
                let callee = path.last().cloned().unwrap_or_default();
                let replacement = self.resolve_call(container, context, &callee, args, scope)?;
                for arg in args {
                    self.rewrite_expr(container, context, arg, scope)?;
                }
                if let (Some(symbol), Some(last)) = (replacement, path.last_mut()) {
                    *last = symbol;
                }
            }
            Expr::StrictCheckExpr { value, .. } | Expr::Unary { expr: value, .. } => {
                self.rewrite_expr(container, context, value, scope)?;
            }
            Expr::Binary { left, right, .. } => {
                self.rewrite_expr(container, context, left, scope)?;
                self.rewrite_expr(container, context, right, scope)?;
            }
            Expr::Call { callee, args } => {
                let replacement = self.resolve_call(container, context, callee, args, scope)?;
                for arg in args {
                    self.rewrite_expr(container, context, arg, scope)?;
                }
                if let Some(symbol) = replacement {
                    *callee = symbol;
                }
            }
            Expr::IfElse {
                condition,
                then_expr,
                else_expr,
            } => {
                self.rewrite_expr(container, context, condition, scope)?;
                self.rewrite_expr(container, context, then_expr, scope)?;
                self.rewrite_expr(container, context, else_expr, scope)?;
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
        Ok(())
    }

    fn resolve_call(
        &mut self,
        container: &str,
        context: &Option<Context>,
        callee: &str,
        args: &[Expr],
        scope: &HashMap<String, SemaType>,
    ) -> Result<Option<String>, String> {
        let arg_types = args
            .iter()
            .map(|arg| infer_expr_type(self.source, container, arg, scope, context))
            .collect::<Result<Vec<_>, _>>()?;
        let signature = FN_SIGS.with(|cell| {
            let index = cell.borrow();
            find_function_signature(
                self.source,
                container,
                context,
                &self.source.uses,
                callee,
                &arg_types,
                &index,
            )
        });
        let Ok(signature) = signature else {
            return Ok(None);
        };

        if !signature.type_params.is_empty() {
            if find_function_decl_by_signature(self.source, &signature).is_none()
                && signature_is_external_generic(&signature)
            {
                return Ok(None);
            }
            return self.request_generic_instance(callee, &signature, &arg_types);
        }

        if find_function_decl_by_signature(self.source, &signature).is_none() {
            if let Some(symbol) = self.static_dispatch_target(&signature) {
                return Ok(Some(symbol));
            }
        }
        Ok(None)
    }

    fn request_generic_instance(
        &mut self,
        callee: &str,
        signature: &FnSig,
        arg_types: &[SemaType],
    ) -> Result<Option<String>, String> {
        let (source, _) =
            find_function_decl_by_signature(self.source, signature).ok_or_else(|| {
                format!(
                    "E_HIGH_MONO_SOURCE_001: generic source '{}.{}' was not found",
                    signature.container, signature.name
                )
            })?;
        let params = source
            .generic_params
            .iter()
            .cloned()
            .collect::<HashSet<_>>();
        let mut bindings = HashMap::new();
        let (_, explicit) = split_generic_callee(callee);
        for (name, concrete) in source.generic_params.iter().zip(explicit.iter()) {
            bind_high_generic_param(name, concrete, &mut bindings)?;
        }
        for (expected, actual) in source.params.iter().zip(arg_types.iter()) {
            bind_high_generic_type(&lower_type(&expected.ty), actual, &params, &mut bindings)?;
        }
        for name in &source.generic_params {
            if !bindings.contains_key(name) {
                return Err(format!(
                    "E_HIGH_MONO_BINDING_001: generic parameter '{}' has no material value",
                    name
                ));
            }
        }
        validate_resolved_high_bounds(&source.generic_bounds, &bindings)?;
        let arguments = source
            .generic_params
            .iter()
            .map(|name| render_sema_type(bindings.get(name).expect("binding was checked")))
            .collect::<Vec<_>>();
        let key = GenericInstanceKey {
            container: signature.container.clone(),
            function: signature.name.clone(),
            arguments: arguments.clone(),
        };
        if let Some(symbol) = self.generic_symbols.get(&key) {
            return Ok(Some(symbol.clone()));
        }
        let symbol = generic_instance_symbol(&key);
        self.generic_symbols.insert(key, symbol.clone());
        let mut function = source.clone();
        substitute_function_types(&mut function, &bindings, &source.generic_bounds)?;
        function.name = symbol.clone();
        function.generic_params.clear();
        function.generic_bounds.clear();
        self.enqueue(
            &signature.container,
            function,
            format!("{}<{}> => {}", signature.name, arguments.join(","), symbol),
            false,
        );
        Ok(Some(symbol))
    }

    fn static_dispatch_target(&self, signature: &FnSig) -> Option<String> {
        HIGH_IMPLS.with(|cell| {
            let implementations = cell.borrow();
            let mut matches = implementations.iter().filter(|implementation| {
                implementation.methods.iter().any(|method| {
                    method.name == signature.name
                        && method.params == signature.params
                        && method.return_type == signature.return_type
                })
            });
            let implementation = matches.next()?;
            if matches.next().is_some() {
                return None;
            }
            let key = (
                implementation.surface_name.to_ascii_lowercase(),
                render_sema_type(&implementation.for_type),
                signature.name.clone(),
            );
            self.static_symbols.get(&key).cloned()
        })
    }

    fn verify_closed(&mut self, program: &Program) -> Result<(), String> {
        for container in &program.containers {
            for section in &container.sections {
                if let Section::Fn(functions) | Section::MoveFn(functions) = section {
                    for function in functions {
                        if !function.generic_params.is_empty() {
                            return Err(format!(
                                "E_HIGH_MONO_TEMPLATE_LEAK_001: template '{}.{}' reached backend input",
                                container.name, function.name
                            ));
                        }
                        verify_no_generic_type_leak(function)?;
                        if self.kernel_profile {
                            let mut dynamic = 0usize;
                            let mut allocations = 0usize;
                            let mut scheduler = 0usize;
                            classify_kernel_hidden_work(
                                &function.public_body,
                                &mut dynamic,
                                &mut allocations,
                                &mut scheduler,
                            );
                            self.report.hidden_dynamic_dispatches += dynamic;
                            self.report.kernel_hidden_allocations += allocations;
                            self.report.kernel_hidden_scheduler_calls += scheduler;
                        }
                    }
                }
            }
        }
        if self.kernel_profile
            && (self.report.hidden_dynamic_dispatches > 0
                || self.report.kernel_hidden_allocations > 0
                || self.report.kernel_hidden_scheduler_calls > 0)
        {
            return Err(format!(
                "E_KERNEL_ZERO_COST_001: kernel lowering contains hidden work (dynamic_dispatch={}, allocation={}, scheduler={})",
                self.report.hidden_dynamic_dispatches,
                self.report.kernel_hidden_allocations,
                self.report.kernel_hidden_scheduler_calls
            ));
        }
        Ok(())
    }
}

fn signature_is_external_generic(signature: &FnSig) -> bool {
    let container = signature.container.to_ascii_lowercase();
    container.starts_with("runtime.")
        || container.starts_with("nc.runtime.")
        || container.starts_with("__runtime_")
}

fn renamed_function(function: &FunctionDecl, name: String) -> FunctionDecl {
    let mut function = function.clone();
    function.name = name;
    function
}

fn stable_hash(text: &str) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in text.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn symbol_part(text: &str) -> String {
    let mut out = text
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
        .collect::<String>();
    while out.contains("__") {
        out = out.replace("__", "_");
    }
    out.trim_matches('_').to_string()
}

fn generic_instance_symbol(key: &GenericInstanceKey) -> String {
    let identity = format!(
        "{}::{}<{}>",
        key.container,
        key.function,
        key.arguments.join(",")
    );
    format!(
        "__nc_mono_{}_{}_{:016x}",
        symbol_part(&key.container),
        symbol_part(&key.function),
        stable_hash(&identity)
    )
}

fn static_dispatch_symbol(surface: &str, concrete: &str, method: &str) -> String {
    let identity = format!("{surface}::{concrete}::{method}");
    format!(
        "__nc_impl_{}_{}_{}_{:016x}",
        symbol_part(surface),
        symbol_part(concrete),
        symbol_part(method),
        stable_hash(&identity)
    )
}

fn sema_type_to_type_name(ty: &SemaType) -> Result<TypeName, String> {
    Ok(match ty {
        SemaType::Simple(name) => TypeName::Simple(name.clone()),
        SemaType::Vector(inner) => TypeName::Vector(Box::new(sema_type_to_type_name(inner)?)),
        SemaType::Array(inner) => TypeName::Array(Box::new(sema_type_to_type_name(inner)?)),
        SemaType::Union(items) => TypeName::Union(
            items
                .iter()
                .map(sema_type_to_type_name)
                .collect::<Result<Vec<_>, _>>()?,
        ),
        SemaType::Generic { base, args } => TypeName::Generic {
            base: base.clone(),
            args: args
                .iter()
                .map(sema_type_to_type_name)
                .collect::<Result<Vec<_>, _>>()?,
        },
        SemaType::Unknown => {
            return Err("E_HIGH_MONO_UNKNOWN_TYPE_001: unknown type cannot be materialized".into())
        }
    })
}

fn substitute_type(
    ty: &TypeName,
    bindings: &HashMap<String, SemaType>,
    bounds: &[GenericBound],
) -> Result<TypeName, String> {
    let params = bindings.keys().cloned().collect::<HashSet<_>>();
    sema_type_to_type_name(&substitute_high_generic_type(
        &lower_type(ty),
        &params,
        bindings,
        bounds,
    )?)
}

fn substitute_function_types(
    function: &mut FunctionDecl,
    bindings: &HashMap<String, SemaType>,
    bounds: &[GenericBound],
) -> Result<(), String> {
    for param in &mut function.params {
        param.ty = substitute_type(&param.ty, bindings, bounds)?;
    }
    function.return_type = substitute_type(&function.return_type, bindings, bounds)?;
    substitute_stmt_types(&mut function.public_body, bindings, bounds)
}

fn substitute_stmt_types(
    body: &mut [Stmt],
    bindings: &HashMap<String, SemaType>,
    bounds: &[GenericBound],
) -> Result<(), String> {
    for stmt in body {
        match stmt {
            Stmt::Projection { ty, value, .. } | Stmt::StrictCheck { ty, value, .. } => {
                *ty = substitute_type(ty, bindings, bounds)?;
                substitute_expr_types(value, bindings, bounds)?;
            }
            Stmt::Return(expr) | Stmt::Expr(expr) | Stmt::Print { value: expr, .. } => {
                substitute_expr_types(expr, bindings, bounds)?;
            }
            Stmt::If {
                condition,
                then_block,
                else_block,
            } => {
                substitute_expr_types(condition, bindings, bounds)?;
                substitute_stmt_types(then_block, bindings, bounds)?;
                substitute_stmt_types(else_block, bindings, bounds)?;
            }
            Stmt::Unless { condition, body }
            | Stmt::While { condition, body }
            | Stmt::RepeatUntil { condition, body } => {
                substitute_expr_types(condition, bindings, bounds)?;
                substitute_stmt_types(body, bindings, bounds)?;
            }
            Stmt::DoWhile { body, condition } => {
                substitute_stmt_types(body, bindings, bounds)?;
                substitute_expr_types(condition, bindings, bounds)?;
            }
            Stmt::ForClassic {
                init,
                condition,
                step,
                body,
            } => {
                if let Some(init) = init {
                    substitute_stmt_types(std::slice::from_mut(init), bindings, bounds)?;
                }
                if let Some(condition) = condition {
                    substitute_expr_types(condition, bindings, bounds)?;
                }
                if let Some(step) = step {
                    substitute_stmt_types(std::slice::from_mut(step), bindings, bounds)?;
                }
                substitute_stmt_types(body, bindings, bounds)?;
            }
            Stmt::ForIn { iterable, body, .. } | Stmt::SameIn { iterable, body, .. } => {
                substitute_expr_types(iterable, bindings, bounds)?;
                substitute_stmt_types(body, bindings, bounds)?;
            }
            Stmt::ForRange {
                start,
                end,
                step,
                body,
                ..
            } => {
                substitute_expr_types(start, bindings, bounds)?;
                substitute_expr_types(end, bindings, bounds)?;
                if let Some(step) = step {
                    substitute_expr_types(step, bindings, bounds)?;
                }
                substitute_stmt_types(body, bindings, bounds)?;
            }
            Stmt::Loop { body } | Stmt::Defer { body } => {
                substitute_stmt_types(body, bindings, bounds)?;
            }
            Stmt::RepeatCount { count, body } => {
                substitute_expr_types(count, bindings, bounds)?;
                substitute_stmt_types(body, bindings, bounds)?;
            }
            Stmt::EachDoWhile {
                iterable,
                body,
                condition,
                ..
            } => {
                substitute_expr_types(iterable, bindings, bounds)?;
                substitute_stmt_types(body, bindings, bounds)?;
                substitute_expr_types(condition, bindings, bounds)?;
            }
            Stmt::Labeled { stmt, .. } => {
                substitute_stmt_types(std::slice::from_mut(stmt), bindings, bounds)?;
            }
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
                substitute_expr_types(value, bindings, bounds)?;
                for (case, body) in cases {
                    substitute_expr_types(case, bindings, bounds)?;
                    substitute_stmt_types(body, bindings, bounds)?;
                }
                substitute_stmt_types(default, bindings, bounds)?;
            }
            Stmt::PatternMatch { value, arms } => {
                substitute_expr_types(value, bindings, bounds)?;
                for arm in arms {
                    if let Some(guard) = &mut arm.guard {
                        substitute_expr_types(guard, bindings, bounds)?;
                    }
                    substitute_stmt_types(&mut arm.body, bindings, bounds)?;
                }
            }
            Stmt::GuardElse {
                condition,
                else_block,
            } => {
                substitute_expr_types(condition, bindings, bounds)?;
                substitute_stmt_types(else_block, bindings, bounds)?;
            }
            Stmt::Select { branches, default } => {
                for (condition, body) in branches {
                    substitute_expr_types(condition, bindings, bounds)?;
                    substitute_stmt_types(body, bindings, bounds)?;
                }
                substitute_stmt_types(default, bindings, bounds)?;
            }
            Stmt::Destructure { args, .. } | Stmt::Constructure { args, .. } => {
                for arg in args {
                    substitute_expr_types(arg, bindings, bounds)?;
                }
            }
            Stmt::Break { .. }
            | Stmt::Continue { .. }
            | Stmt::Dev(_)
            | Stmt::MainRefFn { .. }
            | Stmt::FunctionRelation { .. } => {}
        }
    }
    Ok(())
}

fn substitute_expr_types(
    expr: &mut Expr,
    bindings: &HashMap<String, SemaType>,
    bounds: &[GenericBound],
) -> Result<(), String> {
    let const_symbol = match expr {
        Expr::Identifier(name) => Some(name.as_str()),
        Expr::ContextRef(reference) if reference.path.len() == 1 => {
            Some(reference.path[0].as_str())
        }
        _ => None,
    };
    if let Some(name) = const_symbol {
        if const_generic_param_type(bounds, name).is_some() {
            if let Some(SemaType::Simple(value)) = bindings.get(name) {
                *expr = Expr::Number(value.clone());
                return Ok(());
            }
        }
    }
    match expr {
        Expr::StructLiteral { fields, .. } => {
            for (_, value) in fields {
                substitute_expr_types(value, bindings, bounds)?;
            }
        }
        Expr::VectorLiteral(items) | Expr::ArrayLiteral(items) => {
            for item in items {
                substitute_expr_types(item, bindings, bounds)?;
            }
        }
        Expr::Index { target, index } => {
            substitute_expr_types(target, bindings, bounds)?;
            substitute_expr_types(index, bindings, bounds)?;
        }
        Expr::Closure(closure) => {
            for param in &mut closure.params {
                param.ty = substitute_type(&param.ty, bindings, bounds)?;
            }
            closure.return_type = substitute_type(&closure.return_type, bindings, bounds)?;
            substitute_stmt_types(&mut closure.body, bindings, bounds)?;
        }
        Expr::MethodCall { receiver, args, .. } => {
            substitute_expr_types(receiver, bindings, bounds)?;
            for arg in args {
                substitute_expr_types(arg, bindings, bounds)?;
            }
        }
        Expr::QualifiedCall { args, .. } | Expr::Call { args, .. } => {
            for arg in args {
                substitute_expr_types(arg, bindings, bounds)?;
            }
        }
        Expr::StrictCheckExpr { ty, value, .. } => {
            *ty = substitute_type(ty, bindings, bounds)?;
            substitute_expr_types(value, bindings, bounds)?;
        }
        Expr::Unary { expr, .. } => substitute_expr_types(expr, bindings, bounds)?,
        Expr::Binary { left, right, .. } => {
            substitute_expr_types(left, bindings, bounds)?;
            substitute_expr_types(right, bindings, bounds)?;
        }
        Expr::IfElse {
            condition,
            then_expr,
            else_expr,
        } => {
            substitute_expr_types(condition, bindings, bounds)?;
            substitute_expr_types(then_expr, bindings, bounds)?;
            substitute_expr_types(else_expr, bindings, bounds)?;
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
    Ok(())
}

fn add_pattern_bindings(
    pattern: &Pattern,
    value_type: &SemaType,
    enum_info: Option<&HighEnumInfo>,
    scope: &mut HashMap<String, SemaType>,
) {
    match pattern {
        Pattern::Binding(binding) => {
            let name = pattern_binding_name(binding);
            if name != "_" {
                scope.insert(name.to_string(), value_type.clone());
            }
        }
        Pattern::Variant {
            variant_name,
            bindings,
            ..
        } => {
            if let Some(variant) =
                enum_info.and_then(|info| info.variants.get(&variant_name.to_ascii_lowercase()))
            {
                for (binding, (_, payload)) in bindings.iter().zip(variant.payload.iter()) {
                    let name = pattern_binding_name(binding);
                    if name != "_" {
                        scope.insert(
                            name.to_string(),
                            substitute_enum_payload_type(
                                payload,
                                value_type,
                                enum_info.expect("variant requires enum metadata"),
                            ),
                        );
                    }
                }
            }
        }
        Pattern::VariantPattern {
            variant_name,
            items,
            ..
        } => {
            if let Some(variant) =
                enum_info.and_then(|info| info.variants.get(&variant_name.to_ascii_lowercase()))
            {
                for (item, (_, payload)) in items.iter().zip(variant.payload.iter()) {
                    add_pattern_bindings(
                        item,
                        &substitute_enum_payload_type(
                            payload,
                            value_type,
                            enum_info.expect("variant requires enum metadata"),
                        ),
                        None,
                        scope,
                    );
                }
            }
        }
        Pattern::Array { items, rest } => {
            let item_type = closure_iter_item_type(value_type.clone()).unwrap_or(SemaType::Unknown);
            for item in items {
                add_pattern_bindings(item, &item_type, None, scope);
            }
            if let Some(rest) = rest {
                if rest != "_" {
                    scope.insert(rest.clone(), value_type.clone());
                }
            }
        }
        Pattern::Struct { fields, .. } => {
            for field in fields {
                add_pattern_bindings(&field.pattern, &SemaType::Unknown, None, scope);
            }
        }
        Pattern::Wildcard | Pattern::Literal(_) | Pattern::Range { .. } => {}
    }
}

fn verify_no_generic_type_leak(function: &FunctionDecl) -> Result<(), String> {
    let leaked = function.params.iter().any(|param| {
        function
            .generic_params
            .iter()
            .any(|name| type_name_references(&param.ty, name))
    }) || function
        .generic_params
        .iter()
        .any(|name| type_name_references(&function.return_type, name));
    if leaked {
        return Err(format!(
            "E_HIGH_MONO_TYPE_LEAK_001: function '{}' retains an abstract generic type",
            function.name
        ));
    }
    Ok(())
}

fn classify_kernel_hidden_work(
    body: &[Stmt],
    dynamic: &mut usize,
    allocations: &mut usize,
    scheduler: &mut usize,
) {
    for stmt in body {
        classify_stmt_calls(stmt, &mut |callee| {
            let base = callee.split('<').next().unwrap_or(callee);
            if base == "__ncc_dyn_dispatch" {
                *dynamic += 1;
            }
            if matches!(base, "alloc" | "heapAlloc" | "box" | "streamCreate") {
                *allocations += 1;
            }
            if matches!(base, "spawn" | "asyncRun" | "schedulerSubmit" | "taskSpawn") {
                *scheduler += 1;
            }
        });
    }
}

fn classify_stmt_calls(stmt: &Stmt, callback: &mut impl FnMut(&str)) {
    fn visit_expr(expr: &Expr, callback: &mut impl FnMut(&str)) {
        match expr {
            Expr::Call { callee, args } => {
                callback(callee);
                for arg in args {
                    visit_expr(arg, callback);
                }
            }
            Expr::QualifiedCall { path, args } => {
                if let Some(callee) = path.last() {
                    callback(callee);
                }
                for arg in args {
                    visit_expr(arg, callback);
                }
            }
            Expr::MethodCall {
                receiver,
                method,
                args,
            } => {
                callback(method);
                visit_expr(receiver, callback);
                for arg in args {
                    visit_expr(arg, callback);
                }
            }
            Expr::VectorLiteral(items) | Expr::ArrayLiteral(items) => {
                for item in items {
                    visit_expr(item, callback);
                }
            }
            Expr::Index { target, index } => {
                visit_expr(target, callback);
                visit_expr(index, callback);
            }
            Expr::Closure(closure) => {
                for stmt in &closure.body {
                    classify_stmt_calls(stmt, callback);
                }
            }
            Expr::StrictCheckExpr { value, .. } | Expr::Unary { expr: value, .. } => {
                visit_expr(value, callback)
            }
            Expr::Binary { left, right, .. } => {
                visit_expr(left, callback);
                visit_expr(right, callback);
            }
            Expr::IfElse {
                condition,
                then_expr,
                else_expr,
            } => {
                visit_expr(condition, callback);
                visit_expr(then_expr, callback);
                visit_expr(else_expr, callback);
            }
            _ => {}
        }
    }

    match stmt {
        Stmt::Return(expr) | Stmt::Expr(expr) | Stmt::Print { value: expr, .. } => {
            visit_expr(expr, callback)
        }
        Stmt::Projection { value, .. } | Stmt::StrictCheck { value, .. } => {
            visit_expr(value, callback)
        }
        Stmt::If {
            condition,
            then_block,
            else_block,
        } => {
            visit_expr(condition, callback);
            for stmt in then_block.iter().chain(else_block.iter()) {
                classify_stmt_calls(stmt, callback);
            }
        }
        Stmt::Unless { condition, body }
        | Stmt::While { condition, body }
        | Stmt::RepeatUntil { condition, body } => {
            visit_expr(condition, callback);
            for stmt in body {
                classify_stmt_calls(stmt, callback);
            }
        }
        Stmt::DoWhile { body, condition } => {
            for stmt in body {
                classify_stmt_calls(stmt, callback);
            }
            visit_expr(condition, callback);
        }
        Stmt::ForClassic {
            init,
            condition,
            step,
            body,
        } => {
            if let Some(init) = init {
                classify_stmt_calls(init, callback);
            }
            if let Some(condition) = condition {
                visit_expr(condition, callback);
            }
            if let Some(step) = step {
                classify_stmt_calls(step, callback);
            }
            for stmt in body {
                classify_stmt_calls(stmt, callback);
            }
        }
        Stmt::ForIn { iterable, body, .. } | Stmt::SameIn { iterable, body, .. } => {
            visit_expr(iterable, callback);
            for stmt in body {
                classify_stmt_calls(stmt, callback);
            }
        }
        Stmt::ForRange {
            start,
            end,
            step,
            body,
            ..
        } => {
            visit_expr(start, callback);
            visit_expr(end, callback);
            if let Some(step) = step {
                visit_expr(step, callback);
            }
            for stmt in body {
                classify_stmt_calls(stmt, callback);
            }
        }
        Stmt::Loop { body } | Stmt::Defer { body } => {
            for stmt in body {
                classify_stmt_calls(stmt, callback);
            }
        }
        Stmt::RepeatCount { count, body } => {
            visit_expr(count, callback);
            for stmt in body {
                classify_stmt_calls(stmt, callback);
            }
        }
        Stmt::EachDoWhile {
            iterable,
            body,
            condition,
            ..
        } => {
            visit_expr(iterable, callback);
            for stmt in body {
                classify_stmt_calls(stmt, callback);
            }
            visit_expr(condition, callback);
        }
        Stmt::Labeled { stmt, .. } => classify_stmt_calls(stmt, callback),
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
            visit_expr(value, callback);
            for (case, body) in cases {
                visit_expr(case, callback);
                for stmt in body {
                    classify_stmt_calls(stmt, callback);
                }
            }
            for stmt in default {
                classify_stmt_calls(stmt, callback);
            }
        }
        Stmt::PatternMatch { value, arms } => {
            visit_expr(value, callback);
            for arm in arms {
                if let Some(guard) = &arm.guard {
                    visit_expr(guard, callback);
                }
                for stmt in &arm.body {
                    classify_stmt_calls(stmt, callback);
                }
            }
        }
        Stmt::GuardElse {
            condition,
            else_block,
        } => {
            visit_expr(condition, callback);
            for stmt in else_block {
                classify_stmt_calls(stmt, callback);
            }
        }
        Stmt::Select { branches, default } => {
            for (condition, body) in branches {
                visit_expr(condition, callback);
                for stmt in body {
                    classify_stmt_calls(stmt, callback);
                }
            }
            for stmt in default {
                classify_stmt_calls(stmt, callback);
            }
        }
        Stmt::Destructure { args, .. } | Stmt::Constructure { args, .. } => {
            for arg in args {
                visit_expr(arg, callback);
            }
        }
        Stmt::Break { .. }
        | Stmt::Continue { .. }
        | Stmt::Dev(_)
        | Stmt::MainRefFn { .. }
        | Stmt::FunctionRelation { .. } => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_source(source: &str) -> Program {
        let tokens = ncc_lexer::lex(source).expect("fixture must lex");
        ncc_parser::parse(tokens).expect("fixture must parse")
    }

    fn function_names(program: &Program) -> Vec<String> {
        program
            .containers
            .iter()
            .flat_map(|container| container.sections.iter())
            .filter_map(|section| match section {
                Section::Fn(functions) | Section::MoveFn(functions) => Some(functions),
                _ => None,
            })
            .flatten()
            .map(|function| function.name.clone())
            .collect()
    }

    fn calls_in_function(program: &Program, function_name: &str) -> Vec<String> {
        let mut calls = Vec::new();
        for container in &program.containers {
            for section in &container.sections {
                let functions = match section {
                    Section::Fn(functions) | Section::MoveFn(functions) => functions,
                    _ => continue,
                };
                for function in functions {
                    if function.name == function_name {
                        for stmt in &function.public_body {
                            classify_stmt_calls(stmt, &mut |callee| calls.push(callee.to_string()));
                        }
                    }
                }
            }
        }
        calls
    }

    #[test]
    fn inferred_generic_call_becomes_a_concrete_backend_function() {
        let program = parse_source(
            r#"
genericMaterialization {
    fn:
        identity<T>(input: T) -> T {
            public:
                return @input;
        }

        run() -> bool {
            public:
                @value >> int : identity(7);
                return @value == 7;
        }
}
"#,
        );
        let (materialized, report) =
            materialize_zero_cost_program(&program, false).expect("materialization must close");
        let names = function_names(&materialized);
        let symbol = names
            .iter()
            .find(|name| name.starts_with("__nc_mono_genericMaterialization_identity_"))
            .expect("concrete identity symbol");
        assert!(!names.iter().any(|name| name == "identity"));
        assert!(calls_in_function(&materialized, "run").contains(symbol));
        assert_eq!(report.templates_removed, 1);
        assert_eq!(report.monomorphized_functions.len(), 1);
    }

    #[test]
    fn bounded_generic_dispatch_targets_a_material_impl_symbol() {
        let program = parse_source(
            r#"
boundedDispatch {
    interface:
        Show<T> {
        fn show(value: T) -> str;
    }

    impl:
        Show<int> for int {
        fn show(value: int) -> str {
            public:
                return "int";
        }
    }

    fn:
        twice<T: Show<T>>(a: T, b: T) -> str {
            public:
                return strConcat(show(@a), show(@b));
        }

        run() -> bool {
            public:
                return twice(1, 2) == "intint";
        }
}
"#,
        );
        let (materialized, report) =
            materialize_zero_cost_program(&program, false).expect("materialization must close");
        let names = function_names(&materialized);
        let mono = names
            .iter()
            .find(|name| name.starts_with("__nc_mono_boundedDispatch_twice_"))
            .expect("concrete bounded generic symbol");
        let implementation = names
            .iter()
            .find(|name| name.starts_with("__nc_impl_Show_int_show_"))
            .expect("material implementation symbol");
        assert!(calls_in_function(&materialized, mono).contains(implementation));
        assert_eq!(report.hidden_dynamic_dispatches, 0);
        assert_eq!(report.statically_dispatched_methods.len(), 1);
    }

    #[test]
    fn kernel_profile_rejects_hidden_allocation_or_scheduler_work() {
        let program = parse_source(
            r#"
kernelZeroCost {
    fn:
        identity<T>(input: T) -> T {
            public:
                return @input;
        }

        run() -> bool {
            public:
                @value >> int : identity(7);
                return @value == 7;
        }
}
"#,
        );
        let (_, report) =
            materialize_zero_cost_program(&program, true).expect("pure generic is kernel-safe");
        assert_eq!(report.hidden_dynamic_dispatches, 0);
        assert_eq!(report.kernel_hidden_allocations, 0);
        assert_eq!(report.kernel_hidden_scheduler_calls, 0);
    }

    #[test]
    fn kernel_profile_accepts_async_with_caller_owned_frames() {
        let program = parse_source(
            r#"
kernelAsyncGate {
    fn:
        async pending() -> int
        effects(async.await)
        {
            public:
                return 1;
        }

        run() -> bool {
            public:
                return true;
        }
}
"#,
        );
        let (_, report) = materialize_zero_cost_program(&program, true)
            .expect("caller-owned async state machines have no hidden scheduler");
        assert_eq!(report.kernel_hidden_allocations, 0);
        assert_eq!(report.kernel_hidden_scheduler_calls, 0);
    }

    #[test]
    fn const_generic_is_range_checked_substituted_and_materialized() {
        let program = parse_source(
            r#"
constMaterialization {
    fn:
        width<const N: byte>() -> int {
            public:
                return @N;
        }

        run() -> bool {
            public:
                return width<32>() == 32;
        }
}
"#,
        );
        let (materialized, report) =
            materialize_zero_cost_program(&program, false).expect("const generic must close");
        let names = function_names(&materialized);
        let symbol = names
            .iter()
            .find(|name| name.starts_with("__nc_mono_constMaterialization_width_"))
            .expect("const instance symbol");
        assert!(calls_in_function(&materialized, "run").contains(symbol));
        let generated = materialized
            .containers
            .iter()
            .flat_map(|container| container.sections.iter())
            .filter_map(|section| match section {
                Section::MoveFn(functions) => Some(functions),
                _ => None,
            })
            .flatten()
            .find(|function| function.name == *symbol)
            .expect("generated const function");
        assert!(matches!(
            generated.public_body.as_slice(),
            [Stmt::Return(Expr::Number(value))] if value == "32"
        ));
        assert_eq!(report.templates_removed, 1);

        let out_of_range = parse_source(
            r#"
constRangeFailure {
    fn:
        width<const N: byte>() -> int {
            public:
                return @N;
        }

        run() -> bool {
            public:
                return width<300>() == 300;
        }
}
"#,
        );
        let error = analyze(&out_of_range).expect_err("byte const must be range checked");
        assert!(error.contains("E_HIGH_CONST_GENERIC_RANGE_001"), "{error}");
    }

    #[test]
    fn associated_type_projection_is_resolved_inside_material_instance() {
        let program = parse_source(
            r#"
associatedMaterialization {
    trait:
        Source<T> {
            type Item;
            fn get(value: T) -> Item;
        }

    impl:
        Source<int> for int {
            type Item = int;
            fn get(value: int) -> int {
                public:
                    return @value;
            }
        }

    fn:
        read<S: Source<S>>(source: S) -> S.Item {
            public:
                return get(@source);
        }

        run() -> bool {
            public:
                return read(19) == 19;
        }
}
"#,
        );
        let (materialized, report) = materialize_zero_cost_program(&program, false)
            .expect("associated type materialization must close");
        let generated = materialized
            .containers
            .iter()
            .flat_map(|container| container.sections.iter())
            .filter_map(|section| match section {
                Section::MoveFn(functions) => Some(functions),
                _ => None,
            })
            .flatten()
            .find(|function| {
                function
                    .name
                    .starts_with("__nc_mono_associatedMaterialization_read_")
            })
            .expect("associated generic instance");
        assert_eq!(render_type_name(&generated.params[0].ty), "int");
        assert_eq!(render_type_name(&generated.return_type), "int");
        assert!(generated.generic_params.is_empty());
        assert!(generated.generic_bounds.is_empty());
        assert_eq!(report.hidden_dynamic_dispatches, 0);
        assert_eq!(report.statically_dispatched_methods.len(), 1);
    }

    #[test]
    fn imported_runtime_generics_remain_typed_intrinsics() {
        let program = parse_source(
            r#"
use nC.runtime.list;

runtimeGenericBoundary[@runtime] {
    fn:
        run() -> bool {
            public:
                values >> list<int> = listCreate<int>();
                listAdd(values, 7);
                return listGet(values, 0) == 7;
        }
}
"#,
        );
        let (materialized, report) = materialize_zero_cost_program(&program, false)
            .expect("runtime generics have typed external lowering and no local template body");
        let calls = calls_in_function(&materialized, "run");
        assert!(calls.iter().any(|call| call == "listCreate<int>"));
        assert!(calls.iter().any(|call| call == "listAdd"));
        assert_eq!(report.templates_removed, 0);
        assert!(report.monomorphized_functions.is_empty());
    }

    #[test]
    fn materialization_preserves_qualified_const_contracts() {
        let program = parse_source(
            r#"
contractMain {
    contract:
        LinkedReady {
            require linkedWorker.ready();
        }

    fn:
        useValue<T>(value: T) -> T {
            public:
                return @value;
        }

        run() -> int {
            public:
                return useValue<int>(7);
        }

    fa:
        return true;
}

linkedWorker {
    const fn:
        ready() -> bool {
            public:
                return true;
        }

    fa:
        return true;
}
"#,
        );
        let (materialized, report) = materialize_zero_cost_program(&program, false)
            .expect("qualified const contract must remain verifiable during materialization");
        assert!(function_names(&materialized)
            .iter()
            .any(|name| name.starts_with("__nc_mono_contractMain_useValue_")));
        assert_eq!(report.templates_removed, 1);
    }

    #[test]
    fn qualified_const_contract_false_is_enforced() {
        let program = parse_source(
            r#"
contractMain {
    contract:
        LinkedReady {
            require linkedWorker.ready();
        }

    fa:
        return true;
}

linkedWorker {
    const fn:
        ready() -> bool {
            public:
                return false;
        }

    fa:
        return true;
}
"#,
        );
        let error = materialize_zero_cost_program(&program, false)
            .expect_err("a false qualified const contract must be rejected");
        assert!(error.contains("E_HIGH_CONTRACT_REQUIRE_FAIL_001"));
    }
}
