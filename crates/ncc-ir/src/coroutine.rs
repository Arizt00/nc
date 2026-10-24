use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoroutineKindIR {
    Future,
    Generator,
    Stream,
    AsyncStream,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoroutineFrameStorageIR {
    HeapOwned,
    CallerOwned,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoroutineSuspensionKindIR {
    Await,
    Yield,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoroutineFunctionRoleIR {
    Constructor,
    Resume,
    Drop,
    Cancel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoroutineStateActionKindIR {
    Execute,
    Branch,
    Await,
    Yield,
    Complete,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoroutineStateIR {
    pub state: usize,
    pub action: CoroutineStateActionKindIR,
    pub successors: Vec<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoroutineSuspensionPointIR {
    pub state: usize,
    pub kind: CoroutineSuspensionKindIR,
    pub source_order: usize,
    pub resume_state: usize,
    pub cancellation_checkpoint: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoroutineFrameSlotIR {
    pub symbol: String,
    pub ty: String,
    pub offset_bytes: usize,
    pub size_bytes: usize,
    pub align_bytes: usize,
    pub mem_type: Option<ScalarMemTypeIR>,
    pub live_across_states: Vec<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoroutinePlanIR {
    pub kind: CoroutineKindIR,
    pub payload_type: String,
    pub storage: CoroutineFrameStorageIR,
    pub frame_size_bytes: usize,
    pub frame_align_bytes: usize,
    pub header_size_bytes: usize,
    pub descriptor_size_bytes: usize,
    pub state_offset_bytes: usize,
    pub flags_offset_bytes: usize,
    pub payload_offset_bytes: usize,
    pub payload_size_bytes: usize,
    pub payload_align_bytes: usize,
    pub payload_mem_type: Option<ScalarMemTypeIR>,
    pub entry_state: usize,
    pub state_count: usize,
    pub suspensions: Vec<CoroutineSuspensionPointIR>,
    pub states: Vec<CoroutineStateIR>,
    pub frame_slots: Vec<CoroutineFrameSlotIR>,
    pub constructor_symbol: String,
    pub resume_symbol: String,
    pub drop_symbol: String,
    pub cancel_symbol: Option<String>,
    pub requires_scheduler: bool,
    pub hidden_allocation: bool,
}

#[derive(Default)]
struct CoroutineScan {
    order: usize,
    definitions: HashMap<String, usize>,
    uses: HashMap<String, Vec<usize>>,
    suspensions: Vec<(usize, CoroutineSuspensionKindIR)>,
    requires_scheduler: bool,
    cleanup_depth: usize,
    cleanup_symbols: HashSet<String>,
}

fn coroutine_call_requires_scheduler(name: &str) -> bool {
    matches!(
        name.rsplit('.').next().unwrap_or(name),
        "spawn" | "sleep" | "asyncRun" | "schedulerSubmit" | "taskSpawn"
    )
}

pub(crate) trait CoroutineParamDecl {
    fn coroutine_param_name(&self) -> &str;
}

impl CoroutineParamDecl for ncc_ast::Param {
    fn coroutine_param_name(&self) -> &str {
        &self.name
    }
}

impl CoroutineParamDecl for ncc_ast::ClosureParam {
    fn coroutine_param_name(&self) -> &str {
        &self.name
    }
}

impl CoroutineScan {
    fn next_order(&mut self) -> usize {
        self.order = self.order.saturating_add(1);
        self.order
    }

    fn define(&mut self, name: &str, order: usize) {
        self.definitions.entry(name.to_string()).or_insert(order);
    }

    fn use_symbol(&mut self, name: &str, order: usize) {
        self.uses.entry(name.to_string()).or_default().push(order);
        if self.cleanup_depth > 0 {
            self.cleanup_symbols.insert(name.to_string());
        }
    }

    fn scan_expr(&mut self, expr: &Expr, order: usize) {
        match expr {
            Expr::StructLiteral { fields, .. } => {
                for (_, value) in fields {
                    self.scan_expr(value, order);
                }
            }
            Expr::VectorLiteral(items) | Expr::ArrayLiteral(items) => {
                for item in items {
                    self.scan_expr(item, order);
                }
            }
            Expr::Index { target, index } => {
                self.scan_expr(target, order);
                self.scan_expr(index, order);
            }
            Expr::Closure(_) => {
                // A closure owns a separate frame and is analysed when it is lowered.
            }
            Expr::Identifier(name) => self.use_symbol(name, order),
            Expr::ContextRef(ctx) => {
                if let Some(root) = ctx.path.first() {
                    self.use_symbol(root, order);
                }
            }
            Expr::MethodCall { receiver, args, .. } => {
                self.scan_expr(receiver, order);
                for arg in args {
                    self.scan_expr(arg, order);
                }
            }
            Expr::QualifiedCall { path, args } => {
                for arg in args {
                    self.scan_expr(arg, order);
                }
                if path
                    .last()
                    .is_some_and(|name| name.eq_ignore_ascii_case("await"))
                {
                    self.suspensions
                        .push((order, CoroutineSuspensionKindIR::Await));
                } else if path
                    .last()
                    .is_some_and(|name| name.eq_ignore_ascii_case("yield"))
                {
                    self.suspensions
                        .push((order, CoroutineSuspensionKindIR::Yield));
                }
                if path
                    .last()
                    .is_some_and(|name| coroutine_call_requires_scheduler(name))
                {
                    self.requires_scheduler = true;
                }
            }
            Expr::StrictCheckExpr { target, value, .. } => {
                if let Some(root) = target.path.first() {
                    self.use_symbol(root, order);
                }
                self.scan_expr(value, order);
            }
            Expr::PostIncrement { target }
            | Expr::FieldPostIncrement { target }
            | Expr::PointerPostIncrementRead { target } => {
                if let Some(root) = target.path.first() {
                    self.use_symbol(root, order);
                }
            }
            Expr::Unary { expr, .. } => self.scan_expr(expr, order),
            Expr::Binary { left, right, .. } => {
                self.scan_expr(left, order);
                self.scan_expr(right, order);
            }
            Expr::Call { callee, args } => {
                for arg in args {
                    self.scan_expr(arg, order);
                }
                let base = callee.split('<').next().unwrap_or(callee);
                if base.eq_ignore_ascii_case("await") {
                    self.suspensions
                        .push((order, CoroutineSuspensionKindIR::Await));
                } else if base.eq_ignore_ascii_case("yield") {
                    self.suspensions
                        .push((order, CoroutineSuspensionKindIR::Yield));
                }
                if coroutine_call_requires_scheduler(base) {
                    self.requires_scheduler = true;
                }
            }
            Expr::IfElse {
                condition,
                then_expr,
                else_expr,
            } => {
                self.scan_expr(condition, order);
                self.scan_expr(then_expr, order);
                self.scan_expr(else_expr, order);
            }
            Expr::Bool(_) | Expr::Number(_) | Expr::Char(_) | Expr::String(_) | Expr::Dev(_) => {}
        }
    }

    fn scan_block(&mut self, stmts: &[Stmt]) {
        for stmt in stmts {
            self.scan_stmt(stmt);
        }
    }

    fn scan_stmt(&mut self, stmt: &Stmt) {
        let order = self.next_order();
        match stmt {
            Stmt::Return(expr) | Stmt::Expr(expr) => self.scan_expr(expr, order),
            Stmt::Projection { target, value, .. } | Stmt::StrictCheck { target, value, .. } => {
                self.scan_expr(value, order);
                match target {
                    AssignTarget::Context(ctx) => {
                        if let Some(root) = ctx.path.first() {
                            self.define(root, order);
                        }
                    }
                    AssignTarget::Mem { addr } => self.scan_expr(addr, order),
                }
            }
            Stmt::If {
                condition,
                then_block,
                else_block,
            } => {
                self.scan_expr(condition, order);
                self.scan_block(then_block);
                self.scan_block(else_block);
            }
            Stmt::Unless { condition, body }
            | Stmt::While { condition, body }
            | Stmt::RepeatUntil { condition, body } => {
                self.scan_expr(condition, order);
                self.scan_block(body);
            }
            Stmt::DoWhile { body, condition } => {
                self.scan_block(body);
                self.scan_expr(condition, order);
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
                    self.scan_expr(condition, order);
                }
                self.scan_block(body);
                if let Some(step) = step {
                    self.scan_stmt(step);
                }
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
            }
            | Stmt::EachDoWhile {
                item,
                iterable,
                body,
                ..
            } => {
                self.scan_expr(iterable, order);
                self.define(item, order);
                self.scan_block(body);
                if let Stmt::EachDoWhile { condition, .. } = stmt {
                    self.scan_expr(condition, order);
                }
            }
            Stmt::ForRange {
                item,
                start,
                end,
                step,
                body,
                ..
            } => {
                self.scan_expr(start, order);
                self.scan_expr(end, order);
                if let Some(step) = step {
                    self.scan_expr(step, order);
                }
                self.define(item, order);
                self.scan_block(body);
            }
            Stmt::Loop { body } => self.scan_block(body),
            Stmt::Defer { body } => {
                self.cleanup_depth = self.cleanup_depth.saturating_add(1);
                self.scan_block(body);
                self.cleanup_depth = self.cleanup_depth.saturating_sub(1);
            }
            Stmt::RepeatCount { count, body } => {
                self.scan_expr(count, order);
                self.scan_block(body);
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
                self.scan_expr(value, order);
                for (case, block) in cases {
                    self.scan_expr(case, order);
                    self.scan_block(block);
                }
                self.scan_block(default);
            }
            Stmt::PatternMatch { value, arms } => {
                self.scan_expr(value, order);
                for arm in arms {
                    if let Some(guard) = &arm.guard {
                        self.scan_expr(guard, order);
                    }
                    self.scan_block(&arm.body);
                }
            }
            Stmt::GuardElse {
                condition,
                else_block,
            } => {
                self.scan_expr(condition, order);
                self.scan_block(else_block);
            }
            Stmt::Select { branches, default } => {
                for (condition, block) in branches {
                    self.scan_expr(condition, order);
                    self.scan_block(block);
                }
                self.scan_block(default);
            }
            Stmt::Print { value, .. } => self.scan_expr(value, order),
            Stmt::Destructure { args, .. } | Stmt::Constructure { args, .. } => {
                for arg in args {
                    self.scan_expr(arg, order);
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

fn coroutine_type_payload(ty: &TypeName) -> Option<(CoroutineKindIR, String)> {
    let TypeName::Generic { base, args } = ty else {
        return None;
    };
    let payload = args.first().map(type_name_to_string)?;
    let kind = if base.eq_ignore_ascii_case("Future") {
        CoroutineKindIR::Future
    } else if base.eq_ignore_ascii_case("Generator") {
        CoroutineKindIR::Generator
    } else if base.eq_ignore_ascii_case("Stream") {
        CoroutineKindIR::Stream
    } else if base.eq_ignore_ascii_case("AsyncStream") {
        CoroutineKindIR::AsyncStream
    } else {
        return None;
    };
    Some((kind, payload))
}

fn coroutine_symbol_fragment(name: &str) -> String {
    name.chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect()
}

pub(crate) fn build_coroutine_plan<P: CoroutineParamDecl>(
    container_context: &Option<Context>,
    function_name: &str,
    is_async: bool,
    declared_return: &TypeName,
    params: &[P],
    body: &[Stmt],
    symbol_types: &HashMap<String, String>,
    nonlocal_symbol_types: &HashMap<String, String>,
    backend_profile: &BackendLoweringProfileIR,
    source_contract: &SourceExecutionContract,
) -> Option<CoroutinePlanIR> {
    let mut scan = CoroutineScan::default();
    for param in params {
        scan.define(param.coroutine_param_name(), 0);
    }
    scan.scan_block(body);

    let has_yield = scan
        .suspensions
        .iter()
        .any(|(_, kind)| *kind == CoroutineSuspensionKindIR::Yield);
    let declared = coroutine_type_payload(declared_return);
    let (kind, payload_type) = if is_async && has_yield {
        let payload = declared
            .as_ref()
            .map(|(_, payload)| payload.clone())
            .unwrap_or_else(|| type_name_to_string(declared_return));
        (CoroutineKindIR::AsyncStream, payload)
    } else if is_async {
        (
            CoroutineKindIR::Future,
            type_name_to_string(declared_return),
        )
    } else if let Some(declared) = declared {
        declared
    } else if has_yield {
        (CoroutineKindIR::Generator, "unknown".into())
    } else {
        return None;
    };

    let pointer_bytes = backend_profile.target_spec.pointer_width.div_ceil(8).max(1);
    let mut frame_align = pointer_bytes.max(4);
    // Descriptor: resume, frame, drop and cancel. Frame: state, flags and payload.
    let descriptor_size = 4 * pointer_bytes;
    let state_offset = 0;
    let flags_offset = pointer_bytes;
    let payload_lowering =
        backend_type_lowering_for_type_with_pointer_width(&payload_type, pointer_bytes);
    let payload_align = payload_lowering
        .align_bytes
        .max(1)
        .min(frame_align.max(pointer_bytes));
    let payload_offset = align_up(2 * pointer_bytes, payload_align);
    let payload_size = payload_lowering.size_bytes.max(pointer_bytes);
    let header_size = align_up(payload_offset + payload_size, frame_align);
    let mut cursor = header_size;
    let mut frame_slots = Vec::new();

    let param_names = params
        .iter()
        .map(|param| param.coroutine_param_name().to_string())
        .collect::<HashSet<_>>();

    let mut symbols = scan.definitions.keys().cloned().collect::<Vec<_>>();
    symbols.sort();
    for symbol in symbols {
        if nonlocal_symbol_types.contains_key(&symbol) && !param_names.contains(&symbol) {
            continue;
        }
        let definition = scan.definitions.get(&symbol).copied().unwrap_or(0);
        let uses = scan.uses.get(&symbol).cloned().unwrap_or_default();
        let live_across_states = scan
            .suspensions
            .iter()
            .enumerate()
            .filter_map(|(index, (order, kind))| {
                let needed_after = uses.iter().any(|use_order| *use_order > *order);
                let needed_to_poll = *kind == CoroutineSuspensionKindIR::Await
                    && uses.iter().any(|use_order| *use_order >= *order);
                (definition <= *order && (needed_after || needed_to_poll)).then_some(index + 1)
            })
            .collect::<Vec<_>>();
        let mut live_across_states = live_across_states;
        if param_names.contains(&symbol) || scan.cleanup_symbols.contains(&symbol) {
            live_across_states = (0..=scan.suspensions.len()).collect();
        }
        if live_across_states.is_empty() {
            continue;
        }
        let ty = symbol_types
            .get(&symbol)
            .cloned()
            .unwrap_or_else(|| "unknown".into());
        let lowering = backend_type_lowering_for_type_with_pointer_width(&ty, pointer_bytes);
        let align = lowering.align_bytes.max(1);
        frame_align = frame_align.max(align);
        cursor = align_up(cursor, align);
        frame_slots.push(CoroutineFrameSlotIR {
            symbol,
            ty,
            offset_bytes: cursor,
            size_bytes: lowering.size_bytes.max(pointer_bytes),
            align_bytes: align,
            mem_type: lowering.scalar_mem_type,
            live_across_states,
        });
        cursor = cursor.saturating_add(lowering.size_bytes.max(pointer_bytes));
    }

    let suspensions = scan
        .suspensions
        .iter()
        .enumerate()
        .map(
            |(index, (source_order, suspension_kind))| CoroutineSuspensionPointIR {
                state: index + 1,
                kind: *suspension_kind,
                source_order: *source_order,
                resume_state: index + 2,
                cancellation_checkpoint: is_async
                    || *suspension_kind == CoroutineSuspensionKindIR::Await,
            },
        )
        .collect::<Vec<_>>();

    let hosted = matches!(
        backend_profile.target_spec.runtime_model,
        BackendRuntimeModelIR::Hosted
    );
    let kernel_mode = matches!(container_context, Some(Context::Kernel))
        || matches!(
            backend_profile.target_spec.code_model,
            BackendCodeModelIR::Kernel
        )
        || matches!(
            backend_profile.target_spec.runtime_model,
            BackendRuntimeModelIR::Freestanding | BackendRuntimeModelIR::Sovereign
        );
    let storage = if hosted
        && !kernel_mode
        && source_contract.hidden_work.hidden_allocation
        && !source_contract.is_manual_raw()
    {
        CoroutineFrameStorageIR::HeapOwned
    } else {
        CoroutineFrameStorageIR::CallerOwned
    };
    let base = coroutine_symbol_fragment(function_name);
    let cancellable = is_async || kind == CoroutineKindIR::AsyncStream;

    Some(CoroutinePlanIR {
        kind,
        payload_type,
        storage,
        frame_size_bytes: align_up(cursor, frame_align),
        frame_align_bytes: frame_align,
        header_size_bytes: header_size,
        descriptor_size_bytes: descriptor_size,
        state_offset_bytes: state_offset,
        flags_offset_bytes: flags_offset,
        payload_offset_bytes: payload_offset,
        payload_size_bytes: payload_size,
        payload_align_bytes: payload_align,
        payload_mem_type: payload_lowering.scalar_mem_type,
        entry_state: 0,
        state_count: suspensions.len() + 2,
        suspensions,
        states: Vec::new(),
        frame_slots,
        constructor_symbol: format!("__ncc_coroutine_{base}_new"),
        resume_symbol: format!("__ncc_coroutine_{base}_resume"),
        drop_symbol: format!("__ncc_coroutine_{base}_drop"),
        cancel_symbol: cancellable.then(|| format!("__ncc_coroutine_{base}_cancel")),
        requires_scheduler: scan.requires_scheduler,
        hidden_allocation: storage == CoroutineFrameStorageIR::HeapOwned,
    })
}

pub(crate) fn coroutine_storage_layout(
    ty: &str,
    container_name: &str,
    struct_layouts: &HashMap<String, LlvmStructLayout>,
    pointer_bytes: usize,
) -> (usize, usize, Option<ScalarMemTypeIR>) {
    let compact = normalize_backend_type_name(ty);
    if let Some(payload) = backend_atomic_payload_type(&compact) {
        return coroutine_storage_layout(&payload, container_name, struct_layouts, pointer_bytes);
    }
    if let Some(layout) = struct_layouts
        .get(&compact)
        .or_else(|| struct_layouts.get(&format!("{container_name}.{compact}")))
    {
        return (layout.size_bytes.max(1), layout.align_bytes.max(1), None);
    }
    if let Some((element, len)) = llvm_fixed_array_parts_from_nc_type(&compact) {
        let (element_size, element_align, _) =
            coroutine_storage_layout(&element, container_name, struct_layouts, pointer_bytes);
        return (
            element_size.saturating_mul(len.max(1)),
            element_align.max(1),
            None,
        );
    }
    let lowering = backend_type_lowering_for_type_with_pointer_width(&compact, pointer_bytes);
    (
        lowering.size_bytes.max(1),
        lowering.align_bytes.max(1),
        lowering.scalar_mem_type,
    )
}

fn coroutine_relayout_plan(
    plan: &mut CoroutinePlanIR,
    container_name: &str,
    struct_layouts: &HashMap<String, LlvmStructLayout>,
    pointer_bytes: usize,
) {
    let pointer_bytes = pointer_bytes.max(1);
    plan.descriptor_size_bytes = 4 * pointer_bytes;
    plan.state_offset_bytes = 0;
    plan.flags_offset_bytes = pointer_bytes;

    let (payload_size, payload_align, payload_mem_type) = coroutine_storage_layout(
        &plan.payload_type,
        container_name,
        struct_layouts,
        pointer_bytes,
    );
    plan.payload_size_bytes = payload_size;
    plan.payload_align_bytes = payload_align;
    plan.payload_mem_type = payload_mem_type;
    plan.payload_offset_bytes = align_up(2 * pointer_bytes, payload_align);
    plan.frame_align_bytes = pointer_bytes.max(4).max(payload_align);
    plan.header_size_bytes = align_up(
        plan.payload_offset_bytes.saturating_add(payload_size),
        plan.frame_align_bytes,
    );

    let mut cursor = plan.header_size_bytes;
    for slot in &mut plan.frame_slots {
        let (size, align, mem_type) =
            coroutine_storage_layout(&slot.ty, container_name, struct_layouts, pointer_bytes);
        cursor = align_up(cursor, align);
        slot.offset_bytes = cursor;
        slot.size_bytes = size;
        slot.align_bytes = align;
        slot.mem_type = mem_type;
        plan.frame_align_bytes = plan.frame_align_bytes.max(align);
        cursor = cursor.saturating_add(size);
    }
    plan.frame_size_bytes = align_up(cursor, plan.frame_align_bytes);
}

#[derive(Debug, Clone)]
enum CoroutineNodeAction {
    Execute {
        body: Vec<StatementIR>,
        next: usize,
    },
    Branch {
        condition: ScalarExprIR,
        then_state: usize,
        else_state: usize,
    },
    Await {
        future: ScalarExprIR,
        target: String,
        target_type: String,
        next: usize,
    },
    Yield {
        value: ScalarExprIR,
        next: usize,
    },
    Complete {
        value: ScalarExprIR,
        store_payload: bool,
    },
}

#[derive(Debug, Clone)]
struct CoroutineNode {
    state: usize,
    action: CoroutineNodeAction,
}

#[derive(Debug, Clone)]
struct CoroutineCleanup {
    active_symbol: String,
    body: Vec<StatementIR>,
}

#[derive(Debug, Clone)]
struct CoroutineLoopTarget {
    label: Option<String>,
    break_state: usize,
    continue_state: usize,
}

struct CoroutineGraphBuilder {
    nodes: Vec<CoroutineNode>,
    generated_types: HashMap<String, String>,
    next_temp: usize,
    cleanups: Vec<CoroutineCleanup>,
    payload_type: String,
}

impl CoroutineGraphBuilder {
    fn new(payload_type: &str) -> Self {
        Self {
            nodes: Vec::new(),
            generated_types: HashMap::new(),
            next_temp: 0,
            cleanups: Vec::new(),
            payload_type: payload_type.to_string(),
        }
    }

    fn add(&mut self, action: CoroutineNodeAction) -> usize {
        let state = self.nodes.len();
        self.nodes.push(CoroutineNode { state, action });
        state
    }

    fn placeholder(&mut self) -> usize {
        self.add(CoroutineNodeAction::Complete {
            value: ScalarExprIR::Number("0".into()),
            store_payload: false,
        })
    }

    fn replace(&mut self, state: usize, action: CoroutineNodeAction) {
        self.nodes[state].action = action;
    }

    fn temp(&mut self, prefix: &str, ty: &str) -> String {
        let name = format!("__ncc_coroutine_{prefix}_{}", self.next_temp);
        self.next_temp += 1;
        self.generated_types.insert(name.clone(), ty.to_string());
        name
    }

    fn resolve_loop_target(
        loops: &[CoroutineLoopTarget],
        label: Option<&str>,
        is_break: bool,
    ) -> Option<usize> {
        loops
            .iter()
            .rev()
            .find(|target| label.is_none() || target.label.as_deref() == label)
            .map(|target| {
                if is_break {
                    target.break_state
                } else {
                    target.continue_state
                }
            })
    }

    fn build_sequence(
        &mut self,
        statements: &[StatementIR],
        continuation: usize,
        loops: &[CoroutineLoopTarget],
    ) -> usize {
        let mut next = continuation;
        for statement in statements.iter().rev() {
            next = self.build_statement(statement, next, loops);
        }
        next
    }

    fn build_statement(
        &mut self,
        statement: &StatementIR,
        continuation: usize,
        loops: &[CoroutineLoopTarget],
    ) -> usize {
        match statement {
            StatementIR::ScalarProjection {
                target,
                ty,
                value:
                    ScalarExprIR::Unary {
                        op: ScalarUnaryOpIR::TryPropagate,
                        expr,
                    },
                ..
            } => {
                let result_type = self.payload_type.clone();
                let result = self.temp("try_result", &result_type);
                let result_value = ScalarExprIR::Symbol(result.clone());
                let complete = self.add(CoroutineNodeAction::Complete {
                    value: result_value.clone(),
                    store_payload: true,
                });
                let success = self.add(CoroutineNodeAction::Execute {
                    body: vec![StatementIR::ScalarProjection {
                        target: target.clone(),
                        ty: ty.clone(),
                        raw: "coroutine.try.payload".into(),
                        value: ScalarExprIR::Binary {
                            left: Box::new(result_value.clone()),
                            op: ScalarBinaryOpIR::ShiftRight,
                            right: Box::new(ScalarExprIR::Number("1".into())),
                        },
                        storage: None,
                    }],
                    next: continuation,
                });
                let branch = self.add(CoroutineNodeAction::Branch {
                    condition: ScalarExprIR::Binary {
                        left: Box::new(ScalarExprIR::Binary {
                            left: Box::new(result_value),
                            op: ScalarBinaryOpIR::BitAnd,
                            right: Box::new(ScalarExprIR::Number("1".into())),
                        }),
                        op: ScalarBinaryOpIR::EqEq,
                        right: Box::new(ScalarExprIR::Number("1".into())),
                    },
                    then_state: success,
                    else_state: complete,
                });
                self.add(CoroutineNodeAction::Execute {
                    body: vec![StatementIR::ScalarProjection {
                        target: result,
                        ty: result_type,
                        raw: "coroutine.try.materialize".into(),
                        value: expr.as_ref().clone(),
                        storage: None,
                    }],
                    next: branch,
                })
            }
            StatementIR::ScalarProjection {
                target, ty, value, ..
            } if coroutine_intrinsic_arg(value, "await").is_some() => {
                let future = coroutine_intrinsic_arg(value, "await")
                    .expect("guarded await")
                    .clone();
                self.add(CoroutineNodeAction::Await {
                    future,
                    target: target.clone(),
                    target_type: ty.clone(),
                    next: continuation,
                })
            }
            StatementIR::ScalarReturn { value, .. }
                if coroutine_intrinsic_arg(value, "await").is_some() =>
            {
                let target_type = self.payload_type.clone();
                let target = self.temp("await_return", &target_type);
                let complete = self.add(CoroutineNodeAction::Complete {
                    value: ScalarExprIR::Symbol(target.clone()),
                    store_payload: true,
                });
                self.add(CoroutineNodeAction::Await {
                    future: coroutine_intrinsic_arg(value, "await")
                        .expect("guarded await")
                        .clone(),
                    target,
                    target_type,
                    next: complete,
                })
            }
            StatementIR::ScalarReturn { value, .. } => {
                let payload = if matches!(value, ScalarExprIR::Symbol(_)) {
                    value.clone()
                } else {
                    let payload_type = self.payload_type.clone();
                    ScalarExprIR::Symbol(self.temp("return_payload", &payload_type))
                };
                let complete = self.add(CoroutineNodeAction::Complete {
                    value: payload.clone(),
                    store_payload: true,
                });
                if let ScalarExprIR::Symbol(target) = payload {
                    if !matches!(value, ScalarExprIR::Symbol(_)) {
                        return self.add(CoroutineNodeAction::Execute {
                            body: vec![StatementIR::ScalarProjection {
                                target,
                                ty: self.payload_type.clone(),
                                raw: "coroutine.payload.materialize".into(),
                                value: value.clone(),
                                storage: None,
                            }],
                            next: complete,
                        });
                    }
                }
                complete
            }
            StatementIR::ScalarExpr { value, .. }
                if coroutine_intrinsic_arg(value, "yield").is_some() =>
            {
                let yielded = coroutine_intrinsic_arg(value, "yield").expect("guarded yield");
                let payload = if matches!(yielded, ScalarExprIR::Symbol(_)) {
                    yielded.clone()
                } else {
                    let payload_type = self.payload_type.clone();
                    ScalarExprIR::Symbol(self.temp("yield_payload", &payload_type))
                };
                let suspend = self.add(CoroutineNodeAction::Yield {
                    value: payload.clone(),
                    next: continuation,
                });
                if let ScalarExprIR::Symbol(target) = payload {
                    if !matches!(yielded, ScalarExprIR::Symbol(_)) {
                        return self.add(CoroutineNodeAction::Execute {
                            body: vec![StatementIR::ScalarProjection {
                                target,
                                ty: self.payload_type.clone(),
                                raw: "coroutine.payload.materialize".into(),
                                value: yielded.clone(),
                                storage: None,
                            }],
                            next: suspend,
                        });
                    }
                }
                suspend
            }
            StatementIR::If {
                condition: Some(condition),
                then_body,
                else_body,
                ..
            } => {
                let then_state = self.build_sequence(then_body, continuation, loops);
                let else_state = self.build_sequence(else_body, continuation, loops);
                self.add(CoroutineNodeAction::Branch {
                    condition: condition.clone(),
                    then_state,
                    else_state,
                })
            }
            StatementIR::While {
                condition: Some(condition),
                body,
                ..
            } => {
                let condition_state = self.placeholder();
                let mut nested = loops.to_vec();
                nested.push(CoroutineLoopTarget {
                    label: None,
                    break_state: continuation,
                    continue_state: condition_state,
                });
                let body_state = self.build_sequence(body, condition_state, &nested);
                self.replace(
                    condition_state,
                    CoroutineNodeAction::Branch {
                        condition: condition.clone(),
                        then_state: body_state,
                        else_state: continuation,
                    },
                );
                condition_state
            }
            StatementIR::DoWhile {
                condition: Some(condition),
                body,
                ..
            } => {
                let condition_state = self.placeholder();
                let mut nested = loops.to_vec();
                nested.push(CoroutineLoopTarget {
                    label: None,
                    break_state: continuation,
                    continue_state: condition_state,
                });
                let body_state = self.build_sequence(body, condition_state, &nested);
                self.replace(
                    condition_state,
                    CoroutineNodeAction::Branch {
                        condition: condition.clone(),
                        then_state: body_state,
                        else_state: continuation,
                    },
                );
                body_state
            }
            StatementIR::Loop { body } => {
                let head = self.placeholder();
                let mut nested = loops.to_vec();
                nested.push(CoroutineLoopTarget {
                    label: None,
                    break_state: continuation,
                    continue_state: head,
                });
                let body_state = self.build_sequence(body, head, &nested);
                self.replace(
                    head,
                    CoroutineNodeAction::Execute {
                        body: Vec::new(),
                        next: body_state,
                    },
                );
                body_state
            }
            StatementIR::RepeatUntil {
                condition: Some(condition),
                body,
                ..
            } => {
                let check = self.placeholder();
                let mut nested = loops.to_vec();
                nested.push(CoroutineLoopTarget {
                    label: None,
                    break_state: continuation,
                    continue_state: check,
                });
                let body_state = self.build_sequence(body, check, &nested);
                self.replace(
                    check,
                    CoroutineNodeAction::Branch {
                        condition: condition.clone(),
                        then_state: continuation,
                        else_state: body_state,
                    },
                );
                body_state
            }
            StatementIR::ForClassic {
                init,
                condition,
                step,
                body,
                ..
            } => {
                let check = self.placeholder();
                let step_state = step
                    .as_deref()
                    .map(|step| self.build_statement(step, check, loops))
                    .unwrap_or(check);
                let mut nested = loops.to_vec();
                nested.push(CoroutineLoopTarget {
                    label: None,
                    break_state: continuation,
                    continue_state: step_state,
                });
                let body_state = self.build_sequence(body, step_state, &nested);
                self.replace(
                    check,
                    CoroutineNodeAction::Branch {
                        condition: condition.clone().unwrap_or(ScalarExprIR::Bool(true)),
                        then_state: body_state,
                        else_state: continuation,
                    },
                );
                init.as_deref()
                    .map(|init| self.build_statement(init, check, loops))
                    .unwrap_or(check)
            }
            StatementIR::ForRange {
                item,
                start: Some(start),
                end: Some(end),
                inclusive,
                step,
                body,
            } => {
                let item_ty = "int".to_string();
                self.generated_types
                    .entry(item.clone())
                    .or_insert_with(|| item_ty.clone());
                let step_expr = step
                    .clone()
                    .unwrap_or_else(|| ScalarExprIR::Number("1".into()));
                let check = self.placeholder();
                let increment = StatementIR::ScalarProjection {
                    target: item.clone(),
                    ty: item_ty.clone(),
                    raw: "coroutine.for_range.step".into(),
                    value: ScalarExprIR::Binary {
                        left: Box::new(ScalarExprIR::Symbol(item.clone())),
                        op: ScalarBinaryOpIR::Plus,
                        right: Box::new(step_expr.clone()),
                    },
                    storage: None,
                };
                let step_state = self.build_statement(&increment, check, loops);
                let mut nested = loops.to_vec();
                nested.push(CoroutineLoopTarget {
                    label: None,
                    break_state: continuation,
                    continue_state: step_state,
                });
                let body_state = self.build_sequence(body, step_state, &nested);
                let positive = ScalarExprIR::Binary {
                    left: Box::new(step_expr.clone()),
                    op: ScalarBinaryOpIR::GreaterEq,
                    right: Box::new(ScalarExprIR::Number("0".into())),
                };
                let forward = ScalarExprIR::Binary {
                    left: Box::new(ScalarExprIR::Symbol(item.clone())),
                    op: if *inclusive {
                        ScalarBinaryOpIR::LessEq
                    } else {
                        ScalarBinaryOpIR::Less
                    },
                    right: Box::new(end.clone()),
                };
                let backward = ScalarExprIR::Binary {
                    left: Box::new(ScalarExprIR::Symbol(item.clone())),
                    op: if *inclusive {
                        ScalarBinaryOpIR::GreaterEq
                    } else {
                        ScalarBinaryOpIR::Greater
                    },
                    right: Box::new(end.clone()),
                };
                let condition = ScalarExprIR::IfElse {
                    condition: Box::new(positive),
                    then_expr: Box::new(forward),
                    else_expr: Box::new(backward),
                };
                self.replace(
                    check,
                    CoroutineNodeAction::Branch {
                        condition,
                        then_state: body_state,
                        else_state: continuation,
                    },
                );
                self.add(CoroutineNodeAction::Execute {
                    body: vec![StatementIR::ScalarProjection {
                        target: item.clone(),
                        ty: item_ty,
                        raw: "coroutine.for_range.init".into(),
                        value: start.clone(),
                        storage: None,
                    }],
                    next: check,
                })
            }
            StatementIR::RepeatCount {
                count: Some(count),
                body,
                ..
            } => {
                let counter = self.temp("repeat", "int");
                let check = self.placeholder();
                let increment = StatementIR::ScalarProjection {
                    target: counter.clone(),
                    ty: "int".into(),
                    raw: "coroutine.repeat.step".into(),
                    value: ScalarExprIR::Binary {
                        left: Box::new(ScalarExprIR::Symbol(counter.clone())),
                        op: ScalarBinaryOpIR::Plus,
                        right: Box::new(ScalarExprIR::Number("1".into())),
                    },
                    storage: None,
                };
                let step = self.build_statement(&increment, check, loops);
                let mut nested = loops.to_vec();
                nested.push(CoroutineLoopTarget {
                    label: None,
                    break_state: continuation,
                    continue_state: step,
                });
                let body_state = self.build_sequence(body, step, &nested);
                self.replace(
                    check,
                    CoroutineNodeAction::Branch {
                        condition: ScalarExprIR::Binary {
                            left: Box::new(ScalarExprIR::Symbol(counter.clone())),
                            op: ScalarBinaryOpIR::Less,
                            right: Box::new(count.clone()),
                        },
                        then_state: body_state,
                        else_state: continuation,
                    },
                );
                self.add(CoroutineNodeAction::Execute {
                    body: vec![StatementIR::ScalarProjection {
                        target: counter,
                        ty: "int".into(),
                        raw: "coroutine.repeat.init".into(),
                        value: ScalarExprIR::Number("0".into()),
                        storage: None,
                    }],
                    next: check,
                })
            }
            StatementIR::Defer { body } => {
                let active_symbol = self.temp("defer_active", "bool");
                self.cleanups.push(CoroutineCleanup {
                    active_symbol: active_symbol.clone(),
                    body: body.clone(),
                });
                self.add(CoroutineNodeAction::Execute {
                    body: vec![StatementIR::ScalarProjection {
                        target: active_symbol,
                        ty: "bool".into(),
                        raw: "coroutine.defer.activate".into(),
                        value: ScalarExprIR::Bool(true),
                        storage: None,
                    }],
                    next: continuation,
                })
            }
            StatementIR::Break { label } => {
                Self::resolve_loop_target(loops, label.as_deref(), true).unwrap_or(continuation)
            }
            StatementIR::Continue { label } => {
                Self::resolve_loop_target(loops, label.as_deref(), false).unwrap_or(continuation)
            }
            StatementIR::Labeled { label, stmt } => {
                let mut labeled_loops = loops.to_vec();
                if let Some(last) = labeled_loops.last_mut() {
                    last.label = Some(label.clone());
                }
                self.build_statement(stmt, continuation, &labeled_loops)
            }
            StatementIR::Match {
                value: Some(value),
                cases,
                default,
                ..
            } => {
                let match_value = self.temp("match", "int");
                let mut next_case = self.build_sequence(default, continuation, loops);
                for (case, body) in cases.iter().rev() {
                    let body_state = self.build_sequence(body, continuation, loops);
                    next_case = self.add(CoroutineNodeAction::Branch {
                        condition: ScalarExprIR::Binary {
                            left: Box::new(ScalarExprIR::Symbol(match_value.clone())),
                            op: ScalarBinaryOpIR::EqEq,
                            right: Box::new(case.clone()),
                        },
                        then_state: body_state,
                        else_state: next_case,
                    });
                }
                self.add(CoroutineNodeAction::Execute {
                    body: vec![StatementIR::ScalarProjection {
                        target: match_value,
                        ty: "int".into(),
                        raw: "coroutine.match.value".into(),
                        value: value.clone(),
                        storage: None,
                    }],
                    next: next_case,
                })
            }
            _ => self.add(CoroutineNodeAction::Execute {
                body: vec![statement.clone()],
                next: continuation,
            }),
        }
    }
}

fn coroutine_intrinsic_arg<'a>(expr: &'a ScalarExprIR, name: &str) -> Option<&'a ScalarExprIR> {
    let ScalarExprIR::Call { target, args } = expr else {
        return None;
    };
    (target
        .rsplit('.')
        .next()
        .is_some_and(|target| target.eq_ignore_ascii_case(name))
        && args.len() == 1)
        .then(|| &args[0])
}

pub(crate) fn coroutine_mem_width(word_bytes: usize) -> ScalarMemWidthIR {
    match word_bytes {
        1 => ScalarMemWidthIR::Byte,
        2 => ScalarMemWidthIR::Word,
        4 => ScalarMemWidthIR::Dword,
        _ => ScalarMemWidthIR::Qword,
    }
}

fn coroutine_mem_type(word_bytes: usize) -> ScalarMemTypeIR {
    ScalarMemTypeIR {
        width: coroutine_mem_width(word_bytes),
        extension: ScalarMemExtensionIR::Zero,
    }
}

fn coroutine_frame_addr(offset: usize) -> ScalarExprIR {
    ScalarExprIR::PtrOffset {
        ptr: Box::new(ScalarExprIR::Symbol("__ncc_coroutine_frame".into())),
        index: Box::new(ScalarExprIR::Number(offset.to_string())),
        elem_size: 1,
    }
}

fn coroutine_expr_statement(raw: &str, value: ScalarExprIR) -> StatementIR {
    StatementIR::ScalarExpr {
        raw: raw.into(),
        value,
    }
}

fn coroutine_store_frame(
    offset: usize,
    value: ScalarExprIR,
    mem_type: ScalarMemTypeIR,
) -> StatementIR {
    coroutine_expr_statement(
        "coroutine.frame.store",
        ScalarExprIR::PtrWrite {
            ptr: Box::new(coroutine_frame_addr(offset)),
            value: Box::new(value),
            mem_type,
            volatile: false,
        },
    )
}

fn coroutine_load_frame(offset: usize, mem_type: ScalarMemTypeIR) -> ScalarExprIR {
    ScalarExprIR::PtrRead {
        ptr: Box::new(coroutine_frame_addr(offset)),
        mem_type,
        volatile: false,
    }
}

fn coroutine_state_store(plan: &CoroutinePlanIR, state: usize, word_bytes: usize) -> StatementIR {
    coroutine_store_frame(
        plan.state_offset_bytes,
        ScalarExprIR::Number(state.to_string()),
        coroutine_mem_type(word_bytes),
    )
}

fn coroutine_transition(
    plan: &CoroutinePlanIR,
    state: usize,
    word_bytes: usize,
) -> Vec<StatementIR> {
    vec![
        coroutine_state_store(plan, state, word_bytes),
        StatementIR::Continue { label: None },
    ]
}

fn coroutine_frame_restore_statements(plan: &CoroutinePlanIR) -> Vec<StatementIR> {
    let mut out = Vec::new();
    for slot in &plan.frame_slots {
        if let Some(mem_type) = slot.mem_type {
            out.push(StatementIR::ScalarProjection {
                target: slot.symbol.clone(),
                ty: slot.ty.clone(),
                raw: "coroutine.frame.restore".into(),
                value: coroutine_load_frame(slot.offset_bytes, mem_type),
                storage: None,
            });
        } else {
            out.push(coroutine_expr_statement(
                "coroutine.frame.restore.aggregate",
                ScalarExprIR::Call {
                    target: format!("__ncc_coroutine_restore_aggregate:{}", slot.size_bytes),
                    args: vec![
                        ScalarExprIR::AddressOf {
                            target: slot.symbol.clone(),
                        },
                        coroutine_frame_addr(slot.offset_bytes),
                    ],
                },
            ));
        }
    }
    out
}

fn coroutine_frame_spill_statements(plan: &CoroutinePlanIR) -> Vec<StatementIR> {
    let mut out = Vec::new();
    for slot in &plan.frame_slots {
        if let Some(mem_type) = slot.mem_type {
            out.push(coroutine_store_frame(
                slot.offset_bytes,
                ScalarExprIR::Symbol(slot.symbol.clone()),
                mem_type,
            ));
        } else {
            out.push(coroutine_expr_statement(
                "coroutine.frame.spill.aggregate",
                ScalarExprIR::Call {
                    target: format!("__ncc_coroutine_spill_aggregate:{}", slot.size_bytes),
                    args: vec![
                        coroutine_frame_addr(slot.offset_bytes),
                        ScalarExprIR::AddressOf {
                            target: slot.symbol.clone(),
                        },
                    ],
                },
            ));
        }
    }
    out
}

fn coroutine_payload_store(plan: &CoroutinePlanIR, value: ScalarExprIR) -> Vec<StatementIR> {
    if let Some(mem_type) = plan.payload_mem_type {
        return vec![coroutine_store_frame(
            plan.payload_offset_bytes,
            value,
            mem_type,
        )];
    }
    let ScalarExprIR::Symbol(symbol) = value else {
        unreachable!("coroutine graph materializes every aggregate payload")
    };
    vec![coroutine_expr_statement(
        "coroutine.payload.aggregate.store",
        ScalarExprIR::Call {
            target: format!(
                "__ncc_coroutine_spill_aggregate:{}",
                plan.payload_size_bytes
            ),
            args: vec![
                coroutine_frame_addr(plan.payload_offset_bytes),
                ScalarExprIR::AddressOf { target: symbol },
            ],
        },
    )]
}

fn coroutine_cleanup_statements(
    plan: &CoroutinePlanIR,
    cleanups: &[CoroutineCleanup],
) -> Vec<StatementIR> {
    let mut out = Vec::new();
    for cleanup in cleanups {
        let Some(slot) = plan
            .frame_slots
            .iter()
            .find(|slot| slot.symbol == cleanup.active_symbol)
        else {
            continue;
        };
        let mem_type = slot.mem_type.unwrap_or(ScalarMemTypeIR {
            width: ScalarMemWidthIR::Byte,
            extension: ScalarMemExtensionIR::Zero,
        });
        let mut body = vec![
            StatementIR::ScalarProjection {
                target: cleanup.active_symbol.clone(),
                ty: "bool".into(),
                raw: "coroutine.defer.disarm".into(),
                value: ScalarExprIR::Bool(false),
                storage: None,
            },
            coroutine_store_frame(slot.offset_bytes, ScalarExprIR::Bool(false), mem_type),
        ];
        body.extend(cleanup.body.clone());
        out.push(StatementIR::If {
            condition_raw: "coroutine.defer.active".into(),
            condition: Some(ScalarExprIR::Symbol(cleanup.active_symbol.clone())),
            then_body: body,
            else_body: Vec::new(),
        });
    }
    out
}

fn coroutine_append_generated_slots(
    plan: &mut CoroutinePlanIR,
    generated_types: &HashMap<String, String>,
    container_name: &str,
    struct_layouts: &HashMap<String, LlvmStructLayout>,
    pointer_bytes: usize,
) {
    let mut cursor = plan.frame_size_bytes;
    for (symbol, ty) in generated_types {
        if plan.frame_slots.iter().any(|slot| slot.symbol == *symbol) {
            continue;
        }
        let (size, align, mem_type) =
            coroutine_storage_layout(ty, container_name, struct_layouts, pointer_bytes);
        cursor = align_up(cursor, align);
        plan.frame_slots.push(CoroutineFrameSlotIR {
            symbol: symbol.clone(),
            ty: ty.clone(),
            offset_bytes: cursor,
            size_bytes: size,
            align_bytes: align,
            mem_type,
            live_across_states: (0..plan.state_count).collect(),
        });
        cursor = cursor.saturating_add(size);
    }
    plan.frame_slots.sort_by_key(|slot| slot.offset_bytes);
    plan.frame_size_bytes = align_up(cursor, plan.frame_align_bytes);
}

fn coroutine_dispatch_body(
    plan: &CoroutinePlanIR,
    nodes: &[CoroutineNode],
    cleanups: &[CoroutineCleanup],
    _entry_state: usize,
    word_bytes: usize,
) -> Vec<StatementIR> {
    const STATUS_PENDING: &str = "0";
    const STATUS_YIELDED: &str = "1";
    const STATUS_COMPLETE: &str = "2";
    const STATUS_CANCELLED: &str = "3";

    let word_type = coroutine_mem_type(word_bytes);
    let mut body = coroutine_frame_restore_statements(plan);
    body.push(StatementIR::If {
        condition_raw: "coroutine.cancelled".into(),
        condition: Some(ScalarExprIR::Call {
            target: "__ncc_coroutine_is_cancelled".into(),
            args: vec![ScalarExprIR::Symbol("__ncc_coroutine_frame".into())],
        }),
        then_body: vec![StatementIR::ScalarReturn {
            raw: "coroutine.cancelled".into(),
            value: ScalarExprIR::Number(STATUS_CANCELLED.into()),
        }],
        else_body: Vec::new(),
    });
    let state_symbol = "__ncc_coroutine_state".to_string();
    let mut dispatch = Vec::new();
    dispatch.push(StatementIR::ScalarProjection {
        target: state_symbol.clone(),
        ty: "usize".into(),
        raw: "coroutine.state.load".into(),
        value: coroutine_load_frame(plan.state_offset_bytes, word_type),
        storage: None,
    });
    for node in nodes {
        let mut state_body = match &node.action {
            CoroutineNodeAction::Execute { body, next } => {
                let mut out = body.clone();
                out.extend(coroutine_transition(plan, *next, word_bytes));
                out
            }
            CoroutineNodeAction::Branch {
                condition,
                then_state,
                else_state,
            } => vec![StatementIR::If {
                condition_raw: "coroutine.branch".into(),
                condition: Some(condition.clone()),
                then_body: coroutine_transition(plan, *then_state, word_bytes),
                else_body: coroutine_transition(plan, *else_state, word_bytes),
            }],
            CoroutineNodeAction::Yield { value, next } => {
                let mut out = coroutine_payload_store(plan, value.clone());
                out.extend(coroutine_frame_spill_statements(plan));
                out.push(coroutine_state_store(plan, *next, word_bytes));
                out.push(StatementIR::ScalarReturn {
                    raw: "coroutine.yield".into(),
                    value: ScalarExprIR::Number(STATUS_YIELDED.into()),
                });
                out
            }
            CoroutineNodeAction::Await {
                future,
                target,
                target_type,
                next,
            } => {
                let status = format!("__ncc_await_status_{}", node.state);
                let completed = ScalarExprIR::Binary {
                    left: Box::new(ScalarExprIR::Symbol(status.clone())),
                    op: ScalarBinaryOpIR::EqEq,
                    right: Box::new(ScalarExprIR::Number(STATUS_COMPLETE.into())),
                };
                let cancelled = ScalarExprIR::Binary {
                    left: Box::new(ScalarExprIR::Symbol(status.clone())),
                    op: ScalarBinaryOpIR::EqEq,
                    right: Box::new(ScalarExprIR::Number(STATUS_CANCELLED.into())),
                };
                let mut complete_body = vec![StatementIR::ScalarProjection {
                    target: target.clone(),
                    ty: target_type.clone(),
                    raw: "coroutine.await.payload".into(),
                    value: ScalarExprIR::Call {
                        target: "__ncc_coroutine_payload".into(),
                        args: vec![future.clone()],
                    },
                    storage: None,
                }];
                complete_body.extend(coroutine_transition(plan, *next, word_bytes));
                let mut pending_body = coroutine_frame_spill_statements(plan);
                pending_body.push(StatementIR::ScalarReturn {
                    raw: "coroutine.await.pending".into(),
                    value: ScalarExprIR::Number(STATUS_PENDING.into()),
                });
                vec![
                    StatementIR::ScalarProjection {
                        target: status,
                        ty: "int".into(),
                        raw: "coroutine.await.resume".into(),
                        value: ScalarExprIR::Call {
                            target: "__ncc_coroutine_resume".into(),
                            args: vec![future.clone()],
                        },
                        storage: None,
                    },
                    StatementIR::If {
                        condition_raw: "coroutine.await.complete".into(),
                        condition: Some(completed),
                        then_body: complete_body,
                        else_body: vec![StatementIR::If {
                            condition_raw: "coroutine.await.cancelled".into(),
                            condition: Some(cancelled),
                            then_body: vec![StatementIR::ScalarReturn {
                                raw: "coroutine.await.cancelled".into(),
                                value: ScalarExprIR::Number(STATUS_CANCELLED.into()),
                            }],
                            else_body: pending_body,
                        }],
                    },
                ]
            }
            CoroutineNodeAction::Complete {
                value,
                store_payload,
            } => {
                let mut out = if *store_payload {
                    coroutine_payload_store(plan, value.clone())
                } else {
                    Vec::new()
                };
                out.extend(coroutine_cleanup_statements(plan, cleanups));
                out.push(coroutine_state_store(plan, plan.state_count, word_bytes));
                out.push(StatementIR::ScalarReturn {
                    raw: "coroutine.complete".into(),
                    value: ScalarExprIR::Number(STATUS_COMPLETE.into()),
                });
                out
            }
        };
        dispatch.push(StatementIR::If {
            condition_raw: format!("coroutine.state.{}", node.state),
            condition: Some(ScalarExprIR::Binary {
                left: Box::new(ScalarExprIR::Symbol(state_symbol.clone())),
                op: ScalarBinaryOpIR::EqEq,
                right: Box::new(ScalarExprIR::Number(node.state.to_string())),
            }),
            then_body: std::mem::take(&mut state_body),
            else_body: Vec::new(),
        });
    }
    dispatch.push(StatementIR::ScalarReturn {
        raw: "coroutine.invalid_state".into(),
        value: ScalarExprIR::Number(STATUS_CANCELLED.into()),
    });
    body.push(StatementIR::Loop { body: dispatch });
    body
}

fn coroutine_direct_call_link(target: &str, params: &[FunctionParamIR]) -> FunctionCallLinkIR {
    FunctionCallLinkIR {
        surface: FunctionCallSurfaceIR::Unqualified,
        kind: FunctionCallLinkKindIR::Local,
        target: target.to_string(),
        arg_count: params.len(),
        args: params
            .iter()
            .map(|param| FunctionCallArgIR {
                pass_class: param.pass_class.clone(),
                source_type: Some(param.ty.clone()),
                source: FunctionCallArgSourceIR::Symbol(param.name.clone()),
                context_ref_kind: None,
            })
            .collect(),
        target_mode: None,
        requires_bridge: false,
    }
}

fn coroutine_synthetic_abi(
    source: &FunctionAbiIR,
    return_class: AbiReturnClassIR,
) -> FunctionAbiIR {
    let mut abi = source.clone();
    abi.tag.return_class = return_class;
    abi.frame_class = AbiFrameClassIR::NonLeaf;
    abi
}

fn coroutine_function(
    name: String,
    params: Vec<FunctionParamIR>,
    return_type: String,
    mut symbol_types: HashMap<String, String>,
    abi: FunctionAbiIR,
    call_links: Vec<FunctionCallLinkIR>,
    body: Vec<StatementIR>,
    plan: &CoroutinePlanIR,
    role: CoroutineFunctionRoleIR,
    backend_profile: &BackendLoweringProfileIR,
) -> FunctionIR {
    for param in &params {
        symbol_types.insert(param.name.clone(), param.ty.clone());
    }
    let backend_plan =
        build_function_backend_plan(&abi, &params, &return_type, &call_links, backend_profile);
    FunctionIR {
        name,
        visibility: FunctionVisibilityIR::Private,
        effects: Vec::new(),
        params,
        return_type,
        symbol_types,
        abi,
        backend_plan,
        call_links,
        body,
        heap_effects: Vec::new(),
        mem_region_effects: Vec::new(),
        pointer_contract: FunctionPointerContractIR::default(),
        coroutine: Some(plan.clone()),
        coroutine_role: Some(role),
        closure_key: None,
        closure_captures: Vec::new(),
        closure_storage_layout: None,
        closures: Vec::new(),
    }
}

pub(crate) fn materialize_coroutine_function(
    function: &mut FunctionIR,
    container_name: &str,
    struct_layouts: &HashMap<String, LlvmStructLayout>,
    backend_profile: &BackendLoweringProfileIR,
) {
    let Some(mut plan) = function.coroutine.clone() else {
        return;
    };
    let pointer_bytes = backend_profile.target_spec.data_layout.pointer_bytes.max(1);
    coroutine_relayout_plan(&mut plan, container_name, struct_layouts, pointer_bytes);
    let original_body = function.body.clone();
    let original_params = function.params.clone();
    let original_symbol_types = function.symbol_types.clone();
    let original_call_links = function.call_links.clone();
    let original_closure_captures = function.closure_captures.clone();
    let mut graph = CoroutineGraphBuilder::new(&plan.payload_type);
    let complete = graph.add(CoroutineNodeAction::Complete {
        value: ScalarExprIR::Number("0".into()),
        store_payload: false,
    });
    let entry_state = graph.build_sequence(&original_body, complete, &[]);
    let mut persistent_generated_types = graph.generated_types.clone();
    if function.closure_key.is_some() {
        persistent_generated_types.insert("__closure_env".into(), "ptr<byte>".into());
        for capture in &original_closure_captures {
            persistent_generated_types.insert(capture.name.clone(), capture.ty.clone());
        }
    }
    coroutine_append_generated_slots(
        &mut plan,
        &persistent_generated_types,
        container_name,
        struct_layouts,
        pointer_bytes,
    );
    plan.state_count = graph.nodes.len().saturating_add(1);
    plan.entry_state = entry_state;
    plan.states = graph
        .nodes
        .iter()
        .map(|node| {
            let (action, successors) = match &node.action {
                CoroutineNodeAction::Execute { next, .. } => {
                    (CoroutineStateActionKindIR::Execute, vec![*next])
                }
                CoroutineNodeAction::Branch {
                    then_state,
                    else_state,
                    ..
                } => (
                    CoroutineStateActionKindIR::Branch,
                    vec![*then_state, *else_state],
                ),
                CoroutineNodeAction::Await { next, .. } => {
                    (CoroutineStateActionKindIR::Await, vec![*next])
                }
                CoroutineNodeAction::Yield { next, .. } => {
                    (CoroutineStateActionKindIR::Yield, vec![*next])
                }
                CoroutineNodeAction::Complete { .. } => {
                    (CoroutineStateActionKindIR::Complete, Vec::new())
                }
            };
            CoroutineStateIR {
                state: node.state,
                action,
                successors,
            }
        })
        .collect();

    let mut resume_types = original_symbol_types.clone();
    resume_types.extend(graph.generated_types.clone());
    resume_types.insert("__ncc_coroutine_state".into(), "usize".into());
    for node in &graph.nodes {
        if matches!(node.action, CoroutineNodeAction::Await { .. }) {
            resume_types.insert(format!("__ncc_await_status_{}", node.state), "int".into());
        }
    }
    let frame_param = FunctionParamIR {
        name: "__ncc_coroutine_frame".into(),
        ty: "ptr<byte>".into(),
        pass_class: AbiPassClassIR::PointerWord,
    };
    let resume_abi = coroutine_synthetic_abi(&function.abi, AbiReturnClassIR::Scalar);
    let resume = coroutine_function(
        plan.resume_symbol.clone(),
        vec![frame_param],
        "int".into(),
        resume_types,
        resume_abi,
        original_call_links,
        coroutine_dispatch_body(
            &plan,
            &graph.nodes,
            &graph.cleanups,
            entry_state,
            backend_profile.target_spec.data_layout.word_bytes.max(1),
        ),
        &plan,
        CoroutineFunctionRoleIR::Resume,
        backend_profile,
    );

    let descriptor_param = FunctionParamIR {
        name: "__ncc_coroutine_descriptor".into(),
        ty: "ptr<byte>".into(),
        pass_class: AbiPassClassIR::PointerWord,
    };
    let bool_abi = coroutine_synthetic_abi(&function.abi, AbiReturnClassIR::Scalar);
    let mut drop_types = plan
        .frame_slots
        .iter()
        .map(|slot| (slot.symbol.clone(), slot.ty.clone()))
        .collect::<HashMap<_, _>>();
    drop_types.insert("__ncc_coroutine_frame".into(), "ptr<byte>".into());
    let mut drop_body = vec![StatementIR::ScalarProjection {
        target: "__ncc_coroutine_frame".into(),
        ty: "ptr<byte>".into(),
        raw: "coroutine.drop.frame".into(),
        value: ScalarExprIR::PtrRead {
            ptr: Box::new(ScalarExprIR::PtrOffset {
                ptr: Box::new(ScalarExprIR::Symbol("__ncc_coroutine_descriptor".into())),
                index: Box::new(ScalarExprIR::Number(pointer_bytes.to_string())),
                elem_size: 1,
            }),
            mem_type: coroutine_mem_type(pointer_bytes),
            volatile: false,
        },
        storage: None,
    }];
    drop_body.extend(coroutine_frame_restore_statements(&plan));
    drop_body.extend(coroutine_cleanup_statements(&plan, &graph.cleanups));
    drop_body.extend([
        coroutine_expr_statement(
            "coroutine.drop",
            ScalarExprIR::Call {
                target: "__ncc_coroutine_drop".into(),
                args: vec![ScalarExprIR::Symbol("__ncc_coroutine_descriptor".into())],
            },
        ),
        StatementIR::ScalarReturn {
            raw: "coroutine.drop.complete".into(),
            value: ScalarExprIR::Bool(true),
        },
    ]);
    let drop_function = coroutine_function(
        plan.drop_symbol.clone(),
        vec![descriptor_param.clone()],
        "bool".into(),
        drop_types,
        bool_abi.clone(),
        Vec::new(),
        drop_body,
        &plan,
        CoroutineFunctionRoleIR::Drop,
        backend_profile,
    );
    let cancel_function = plan.cancel_symbol.as_ref().map(|symbol| {
        coroutine_function(
            symbol.clone(),
            vec![descriptor_param.clone()],
            "bool".into(),
            HashMap::new(),
            bool_abi,
            Vec::new(),
            vec![
                coroutine_expr_statement(
                    "coroutine.cancel",
                    ScalarExprIR::Call {
                        target: "__ncc_coroutine_cancel".into(),
                        args: vec![ScalarExprIR::Symbol("__ncc_coroutine_descriptor".into())],
                    },
                ),
                StatementIR::ScalarReturn {
                    raw: "coroutine.cancel.complete".into(),
                    value: ScalarExprIR::Bool(true),
                },
            ],
            &plan,
            CoroutineFunctionRoleIR::Cancel,
            backend_profile,
        )
    });

    let mut constructor_params = original_params.clone();
    if plan.storage == CoroutineFrameStorageIR::CallerOwned {
        constructor_params.insert(
            0,
            FunctionParamIR {
                name: "__ncc_coroutine_storage".into(),
                ty: "ptr<byte>".into(),
                pass_class: AbiPassClassIR::PointerWord,
            },
        );
    }
    let constructor_abi = coroutine_synthetic_abi(&function.abi, AbiReturnClassIR::PointerWord);
    let mut constructor = coroutine_function(
        plan.constructor_symbol.clone(),
        constructor_params.clone(),
        function.return_type.clone(),
        original_symbol_types.clone(),
        constructor_abi,
        Vec::new(),
        vec![StatementIR::ScalarReturn {
            raw: "coroutine.new".into(),
            value: ScalarExprIR::Call {
                target: "__ncc_coroutine_new".into(),
                args: constructor_params
                    .iter()
                    .map(|param| ScalarExprIR::Symbol(param.name.clone()))
                    .collect(),
            },
        }],
        &plan,
        CoroutineFunctionRoleIR::Constructor,
        backend_profile,
    );
    constructor.closure_captures = original_closure_captures;

    if plan.storage == CoroutineFrameStorageIR::CallerOwned {
        function.params.insert(
            0,
            FunctionParamIR {
                name: "__ncc_coroutine_storage".into(),
                ty: "ptr<byte>".into(),
                pass_class: AbiPassClassIR::PointerWord,
            },
        );
        function
            .symbol_types
            .insert("__ncc_coroutine_storage".into(), "ptr<byte>".into());
    }
    let wrapper_link = coroutine_direct_call_link(&plan.constructor_symbol, &function.params);
    function.call_links = vec![wrapper_link.clone()];
    function.backend_plan = build_function_backend_plan(
        &function.abi,
        &function.params,
        &function.return_type,
        &function.call_links,
        backend_profile,
    );
    function.body = vec![StatementIR::ScalarReturn {
        raw: "coroutine.constructor.wrapper".into(),
        value: ScalarExprIR::Call {
            target: plan.constructor_symbol.clone(),
            args: function
                .params
                .iter()
                .map(|param| ScalarExprIR::Symbol(param.name.clone()))
                .collect(),
        },
    }];
    function.coroutine = Some(plan.clone());
    function.closures.push(constructor);
    function.closures.push(resume);
    function.closures.push(drop_function);
    if let Some(cancel) = cancel_function {
        function.closures.push(cancel);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_manual(source: &str, label: &str) -> Program {
        let tokens = ncc_lexer::lex(source)
            .unwrap_or_else(|error| panic!("{label} fixture must lex: {error}"));
        let program = ncc_parser::parse(tokens)
            .unwrap_or_else(|error| panic!("{label} fixture must parse: {error}"));
        ncc_sema::analyze(&program)
            .unwrap_or_else(|error| panic!("{label} fixture must pass sema: {error}"));
        program
    }

    #[test]
    fn manual_hosted_coroutine_is_caller_owned_and_materially_tagged() {
        let source = r#"
manualCoroutine {
    fn:
        async answer(value: int) -> int {
            public:
                return @value + 1;
        }
}
"#;
        let tokens = ncc_lexer::lex(source).expect("manual coroutine fixture must lex");
        let program = ncc_parser::parse(tokens).expect("manual coroutine fixture must parse");
        ncc_sema::analyze(&program).expect("manual coroutine fixture must pass sema");
        let ir = lower_with_backend_profile(program, BackendLoweringProfileIR::nc_sysv64());
        assert_eq!(
            ir.source_contract.mode,
            ncc_ast::SourceExecutionMode::ManualRaw
        );
        let function = ir
            .containers
            .iter()
            .find_map(|container| match container {
                ContainerIR::Regular(container) => container.functions.first(),
                _ => None,
            })
            .expect("manual async function");
        let plan = function.coroutine.as_ref().expect("manual coroutine plan");
        assert_eq!(plan.storage, CoroutineFrameStorageIR::CallerOwned);
        assert!(!plan.hidden_allocation);

        let llvm = mct_emit_llvm_module(&ir);
        assert!(!llvm.llvm_ir.contains("call ptr @malloc"));
        assert!(!llvm.llvm_ir.contains("section \".nc.mode\""));
        assert!(!llvm.llvm_ir.contains("__ncc_source_execution_contract"));
        let backend = emit_backend(&ir);
        let object = emit_object(&backend);
        assert!(object
            .sections
            .iter()
            .all(|section| section.name != ".nc.mode"));
        assert!(object
            .symbols
            .iter()
            .all(|symbol| symbol.name != "__ncc_source_execution_contract"));
    }

    #[test]
    fn async_plan_has_material_frame_states_and_live_slots() {
        let source = r#"
asyncPlan {
    fn:
        async produce(value: int) -> int {
            public:
                return @value;
        }

        async work(seed: int) -> int
        effects(async.await)
        {
            public:
                @frame !> p<byte> : alloc(512);
                @future >> Future<int> : asyncAt(@frame, produce(seed));
                @value >> int : await future;
                @result >> int : @value + @seed;
                free(@frame);
                return @result;
        }
}
"#;
        let program = parse_manual(source, "async plan");
        let ir = lower_with_backend_profile(program, BackendLoweringProfileIR::nc_sysv64());
        let function = ir
            .containers
            .iter()
            .find_map(|container| match container {
                ContainerIR::Regular(container) => container
                    .functions
                    .iter()
                    .find(|function| function.name == "work"),
                _ => None,
            })
            .expect("async function must lower");
        let plan = function.coroutine.as_ref().expect("coroutine plan");
        assert_eq!(plan.kind, CoroutineKindIR::Future);
        assert_eq!(plan.suspensions.len(), 1);
        assert_eq!(plan.suspensions[0].kind, CoroutineSuspensionKindIR::Await);
        assert!(plan.state_count >= 5);
        assert!(!plan.states.is_empty());
        assert!(plan.payload_offset_bytes >= 2 * ir.target_spec.data_layout.word_bytes);
        assert!(plan.frame_size_bytes >= plan.header_size_bytes);
        assert!(plan.frame_slots.iter().any(|slot| slot.symbol == "seed"));
        assert!(!plan.requires_scheduler);
        assert_eq!(plan.storage, CoroutineFrameStorageIR::CallerOwned);
        assert!(!plan.hidden_allocation);
        assert!(plan.cancel_symbol.is_some());
        assert!(function.closures.iter().any(|function| {
            function.coroutine_role == Some(CoroutineFunctionRoleIR::Constructor)
        }));
        assert!(function
            .closures
            .iter()
            .any(|function| { function.coroutine_role == Some(CoroutineFunctionRoleIR::Resume) }));
        assert!(function
            .closures
            .iter()
            .any(|function| { function.coroutine_role == Some(CoroutineFunctionRoleIR::Drop) }));
        assert!(function
            .closures
            .iter()
            .any(|function| { function.coroutine_role == Some(CoroutineFunctionRoleIR::Cancel) }));

        let module = mct_emit_llvm_module(&ir);
        assert!(
            module.diagnostics.is_empty(),
            "diagnostics={:#?}\n{}",
            module.diagnostics,
            module.llvm_ir
        );
        assert_eq!(
            module.llvm_ir.matches("call ptr @malloc").count(),
            1,
            "the only frame allocation must be the source-level alloc: {}",
            module.llvm_ir
        );
        assert!(module.llvm_ir.contains("call void @free(ptr"));
        assert!(
            module.llvm_ir.contains(" = call i") && module.llvm_ir.contains("(ptr "),
            "{}",
            module.llvm_ir
        );
        assert!(module.llvm_ir.contains("load atomic"));
        assert!(module.llvm_ir.contains("store atomic"));

        let backend = emit_backend(&ir);
        let generated = backend
            .units
            .iter()
            .filter_map(|unit| match unit {
                BackendUnitIR::Function(function)
                    if function.symbol.contains("__ncc_coroutine_work_") =>
                {
                    Some(function)
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(generated.len(), 4, "{generated:#?}");
        assert!(
            generated.iter().all(|function| {
                function.lowering_coverage.route == BackendFunctionLoweringRouteIR::ScalarRegStack
            }),
            "{generated:#?}"
        );
        assert!(generated.iter().any(|function| {
            function.ops.iter().any(|op| match op {
                BackendFuncOpIR::ScalarOps { ops } => ops
                    .iter()
                    .any(|op| matches!(op, BackendScalarOpIR::CallIndirect { .. })),
                _ => false,
            })
        }));
        let object = emit_object(&backend);
        let elf = emit_object_elf64(&object)
            .expect("coroutine machine units must serialize as a real ELF64 object");
        assert_eq!(&elf[..4], b"\x7fELF");
        let elf_strings = String::from_utf8_lossy(&elf);
        assert!(elf_strings.contains("__ncc_coroutine_work_new"));
        assert!(elf_strings.contains("__ncc_coroutine_work_resume"));
        assert!(elf_strings.contains("__ncc_coroutine_work_drop"));
        assert!(elf_strings.contains("__ncc_coroutine_work_cancel"));

        if std::process::Command::new("clang")
            .arg("--version")
            .output()
            .is_ok()
        {
            let harness = format!(
                r#"
define i32 @main() {{
entry:
  %storage = alloca [{} x i8], align {}
  %handle = call ptr @asyncPlan_work(ptr %storage, i64 40)
  %resume = load ptr, ptr %handle, align 8
  %frame_slot = getelementptr i8, ptr %handle, i64 8
  %frame = load ptr, ptr %frame_slot, align 8
  %status = call i64 %resume(ptr %frame)
  %status_ok = icmp eq i64 %status, 2
  %payload_ptr = getelementptr i8, ptr %frame, i64 {}
  %payload = load i64, ptr %payload_ptr, align 8
  %payload_ok = icmp eq i64 %payload, 80
  %ok = and i1 %status_ok, %payload_ok
  %drop_slot = getelementptr i8, ptr %handle, i64 16
  %drop = load ptr, ptr %drop_slot, align 8
  %dropped = call i1 %drop(ptr %handle)
  %cancel_storage = alloca [{} x i8], align {}
  %cancel_handle = call ptr @asyncPlan_work(ptr %cancel_storage, i64 1)
  %cancel_slot = getelementptr i8, ptr %cancel_handle, i64 24
  %cancel = load ptr, ptr %cancel_slot, align 8
  %cancelled = call i1 %cancel(ptr %cancel_handle)
  %cancel_resume = load ptr, ptr %cancel_handle, align 8
  %cancel_frame_slot = getelementptr i8, ptr %cancel_handle, i64 8
  %cancel_frame = load ptr, ptr %cancel_frame_slot, align 8
  %cancel_status = call i64 %cancel_resume(ptr %cancel_frame)
  %cancel_ok = icmp eq i64 %cancel_status, 3
  %all_ok = and i1 %ok, %cancel_ok
  %cancel_drop_slot = getelementptr i8, ptr %cancel_handle, i64 16
  %cancel_drop = load ptr, ptr %cancel_drop_slot, align 8
  %cancel_dropped = call i1 %cancel_drop(ptr %cancel_handle)
  %ret = select i1 %all_ok, i32 0, i32 1
  ret i32 %ret
}}
"#,
                plan.descriptor_size_bytes + plan.frame_size_bytes,
                plan.frame_align_bytes,
                plan.payload_offset_bytes,
                plan.descriptor_size_bytes + plan.frame_size_bytes,
                plan.frame_align_bytes
            );
            let mut llvm = module.llvm_ir.clone();
            let insert = llvm
                .find("attributes #")
                .or_else(|| llvm.find("!llvm.module.flags"))
                .unwrap_or(llvm.len());
            llvm.insert_str(insert, &harness);
            let nonce = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock")
                .as_nanos();
            let stem = format!("ncc_coroutine_e2e_{}_{}", std::process::id(), nonce);
            let llvm_path = std::env::temp_dir().join(format!("{stem}.ll"));
            let exe_path = std::env::temp_dir().join(stem);
            std::fs::write(&llvm_path, llvm).expect("coroutine LLVM fixture must be writable");
            let clang = std::process::Command::new("clang")
                .arg("-x")
                .arg("ir")
                .arg(&llvm_path)
                .arg("-o")
                .arg(&exe_path)
                .output()
                .expect("clang coroutine link");
            assert!(
                clang.status.success(),
                "clang must link coroutine state machine:\n{}",
                String::from_utf8_lossy(&clang.stderr)
            );
            let run = std::process::Command::new(&exe_path)
                .output()
                .expect("linked coroutine executable must run");
            let _ = std::fs::remove_file(&llvm_path);
            let _ = std::fs::remove_file(&exe_path);
            assert_eq!(
                run.status.code(),
                Some(0),
                "stderr={}",
                String::from_utf8_lossy(&run.stderr)
            );
        }
    }

    #[test]
    fn generator_plan_uses_yield_states_without_scheduler() {
        let source = r#"
generatorPlan {
    fn:
        values() -> Generator<int> {
            public:
                @seed >> int : 40;
                yield @seed;
                @answer >> int : @seed + 2;
                yield @answer;
        }
}
"#;
        let program = parse_manual(source, "generator plan");
        let ir = lower_with_backend_profile(program, BackendLoweringProfileIR::nc_sovereign());
        let function = ir
            .containers
            .iter()
            .find_map(|container| match container {
                ContainerIR::Regular(container) => container.functions.first(),
                _ => None,
            })
            .expect("generator function must lower");
        let plan = function.coroutine.as_ref().expect("generator plan");
        assert_eq!(plan.kind, CoroutineKindIR::Generator);
        assert_eq!(plan.suspensions.len(), 2);
        assert!(plan
            .suspensions
            .iter()
            .all(|point| point.kind == CoroutineSuspensionKindIR::Yield));
        assert_eq!(plan.storage, CoroutineFrameStorageIR::CallerOwned);
        assert!(!plan.requires_scheduler);
        assert!(!plan.hidden_allocation);
        assert!(plan.cancel_symbol.is_none());
        let constructor = function
            .closures
            .iter()
            .find(|function| function.coroutine_role == Some(CoroutineFunctionRoleIR::Constructor))
            .expect("caller-owned constructor");
        assert_eq!(constructor.params[0].name, "__ncc_coroutine_storage");
        let module = mct_emit_llvm_module(&ir);
        assert!(module.diagnostics.is_empty(), "{:#?}", module.diagnostics);
        assert!(!module.llvm_ir.contains("call ptr @malloc"));
    }

    #[test]
    fn aggregate_payload_and_frame_slots_use_resolved_binary_layout() {
        let source = r#"
aggregateCoroutine {
    type:
        Packet {
            public:
                tag: int;
                score: int;
                flags: int;
        }

    fn:
        async echo(packet: Packet) -> Packet {
            public:
                return @packet;
        }
}
"#;
        let program = parse_manual(source, "aggregate coroutine");
        let ir = lower_with_backend_profile(program, BackendLoweringProfileIR::nc_sysv64());
        let function = ir
            .containers
            .iter()
            .find_map(|container| match container {
                ContainerIR::Regular(container) => container.functions.first(),
                _ => None,
            })
            .expect("aggregate coroutine function must lower");
        let plan = function
            .coroutine
            .as_ref()
            .expect("aggregate coroutine plan");
        assert_eq!(plan.payload_size_bytes, 12);
        assert_eq!(plan.payload_align_bytes, 4);
        assert_eq!(plan.payload_mem_type, None);
        let packet = plan
            .frame_slots
            .iter()
            .find(|slot| slot.symbol == "packet")
            .expect("aggregate parameter must persist in the frame");
        assert_eq!(packet.size_bytes, 12);
        assert_eq!(packet.align_bytes, 4);
        assert_eq!(packet.mem_type, None);

        let module = mct_emit_llvm_module(&ir);
        assert!(
            module.diagnostics.is_empty(),
            "diagnostics={:#?}\n{}",
            module.diagnostics,
            module.llvm_ir
        );
        assert!(module
            .llvm_ir
            .contains("%struct.aggregateCoroutine_Packet = type { i32, i32, i32 }"));
        assert!(module.llvm_ir.contains("i64 12, i1 false"));

        if std::process::Command::new("clang")
            .arg("--version")
            .output()
            .is_ok()
        {
            let harness = format!(
                r#"
define i32 @main() {{
entry:
  %storage = alloca [{} x i8], align {}
  %packet0 = insertvalue %struct.aggregateCoroutine_Packet undef, i32 11, 0
  %packet1 = insertvalue %struct.aggregateCoroutine_Packet %packet0, i32 22, 1
  %packet2 = insertvalue %struct.aggregateCoroutine_Packet %packet1, i32 33, 2
  %handle = call ptr @aggregateCoroutine_echo(ptr %storage, %struct.aggregateCoroutine_Packet %packet2)
  %resume = load ptr, ptr %handle, align 8
  %frame_slot = getelementptr i8, ptr %handle, i64 8
  %frame = load ptr, ptr %frame_slot, align 8
  %status = call i64 %resume(ptr %frame)
  %payload_ptr = getelementptr i8, ptr %frame, i64 {}
  %payload = load %struct.aggregateCoroutine_Packet, ptr %payload_ptr, align 4
  %tag = extractvalue %struct.aggregateCoroutine_Packet %payload, 0
  %score = extractvalue %struct.aggregateCoroutine_Packet %payload, 1
  %flags = extractvalue %struct.aggregateCoroutine_Packet %payload, 2
  %status_ok = icmp eq i64 %status, 2
  %tag_ok = icmp eq i32 %tag, 11
  %score_ok = icmp eq i32 %score, 22
  %flags_ok = icmp eq i32 %flags, 33
  %fields01 = and i1 %tag_ok, %score_ok
  %fields = and i1 %fields01, %flags_ok
  %ok = and i1 %status_ok, %fields
  %drop_slot = getelementptr i8, ptr %handle, i64 16
  %drop = load ptr, ptr %drop_slot, align 8
  %dropped = call i1 %drop(ptr %handle)
  %result = select i1 %ok, i32 0, i32 1
  ret i32 %result
}}
"#,
                plan.descriptor_size_bytes + plan.frame_size_bytes,
                plan.frame_align_bytes,
                plan.payload_offset_bytes
            );
            let mut llvm = module.llvm_ir.clone();
            let insert = llvm
                .find("attributes #")
                .or_else(|| llvm.find("!llvm.module.flags"))
                .unwrap_or(llvm.len());
            llvm.insert_str(insert, &harness);
            let nonce = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock")
                .as_nanos();
            let stem = format!("ncc_coroutine_aggregate_{}_{}", std::process::id(), nonce);
            let llvm_path = std::env::temp_dir().join(format!("{stem}.ll"));
            let exe_path = std::env::temp_dir().join(stem);
            std::fs::write(&llvm_path, llvm)
                .expect("aggregate coroutine LLVM fixture must be writable");
            let clang = std::process::Command::new("clang")
                .arg("-x")
                .arg("ir")
                .arg(&llvm_path)
                .arg("-o")
                .arg(&exe_path)
                .output()
                .expect("clang aggregate coroutine link");
            assert!(
                clang.status.success(),
                "clang must link aggregate coroutine state machine:\n{}",
                String::from_utf8_lossy(&clang.stderr)
            );
            let run = std::process::Command::new(&exe_path)
                .output()
                .expect("linked aggregate coroutine executable must run");
            let _ = std::fs::remove_file(&llvm_path);
            let _ = std::fs::remove_file(&exe_path);
            assert_eq!(
                run.status.code(),
                Some(0),
                "stderr={}",
                String::from_utf8_lossy(&run.stderr)
            );
        }
    }

    #[test]
    fn coroutine_defer_runs_on_complete_or_drop_but_not_on_suspend() {
        let source = r#"
cleanupCoroutine {
    stc:
        cleaned: int;

    fn:
        async values() -> AsyncStream<int> {
            public:
                defer {
                    @cleaned >> int : @cleaned + 1;
                }
                yield 7;
        }
}
"#;
        let program = parse_manual(source, "coroutine defer");
        let ir = lower_with_backend_profile(program, BackendLoweringProfileIR::nc_sysv64());
        let function = ir
            .containers
            .iter()
            .find_map(|container| match container {
                ContainerIR::Regular(container) => container.functions.first(),
                _ => None,
            })
            .expect("deferred generator must lower");
        let plan = function
            .coroutine
            .as_ref()
            .expect("deferred generator plan");
        assert!(plan.cancel_symbol.is_some());
        assert!(plan
            .frame_slots
            .iter()
            .any(|slot| slot.symbol.starts_with("__ncc_coroutine_defer_active_")));
        let module = mct_emit_llvm_module(&ir);
        assert!(
            module.diagnostics.is_empty(),
            "diagnostics={:#?}\n{}",
            module.diagnostics,
            module.llvm_ir
        );
        let backend = emit_backend(&ir);
        let generated = backend
            .units
            .iter()
            .filter_map(|unit| match unit {
                BackendUnitIR::Function(function)
                    if function.symbol.contains("__ncc_coroutine_values_") =>
                {
                    Some(function)
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(generated.len(), 4, "{generated:#?}");
        assert!(
            generated.iter().all(|function| {
                function.lowering_coverage.route == BackendFunctionLoweringRouteIR::ScalarRegStack
            }),
            "{generated:#?}"
        );

        if std::process::Command::new("clang")
            .arg("--version")
            .output()
            .is_ok()
        {
            let harness = r#"
define i32 @main() {
entry:
  %early_storage = alloca [COROUTINE_STORAGE_BYTES x i8], align COROUTINE_STORAGE_ALIGN
  %early = call ptr @cleanupCoroutine_values(ptr %early_storage)
  %early_resume = load ptr, ptr %early, align 8
  %early_frame_slot = getelementptr i8, ptr %early, i64 8
  %early_frame = load ptr, ptr %early_frame_slot, align 8
  %yielded = call i64 %early_resume(ptr %early_frame)
  %before_drop = load i64, ptr @__ncc_stc_cleanupCoroutine_cleaned, align 8
  %cancel_slot = getelementptr i8, ptr %early, i64 24
  %cancel = load ptr, ptr %cancel_slot, align 8
  %cancelled = call i1 %cancel(ptr %early)
  %cancel_status = call i64 %early_resume(ptr %early_frame)
  %early_drop_slot = getelementptr i8, ptr %early, i64 16
  %early_drop = load ptr, ptr %early_drop_slot, align 8
  %early_dropped = call i1 %early_drop(ptr %early)
  %after_early_drop = load i64, ptr @__ncc_stc_cleanupCoroutine_cleaned, align 8

  %complete_storage = alloca [COROUTINE_STORAGE_BYTES x i8], align COROUTINE_STORAGE_ALIGN
  %complete_handle = call ptr @cleanupCoroutine_values(ptr %complete_storage)
  %complete_resume = load ptr, ptr %complete_handle, align 8
  %complete_frame_slot = getelementptr i8, ptr %complete_handle, i64 8
  %complete_frame = load ptr, ptr %complete_frame_slot, align 8
  %yielded_again = call i64 %complete_resume(ptr %complete_frame)
  %completed = call i64 %complete_resume(ptr %complete_frame)
  %after_complete = load i64, ptr @__ncc_stc_cleanupCoroutine_cleaned, align 8
  %complete_drop_slot = getelementptr i8, ptr %complete_handle, i64 16
  %complete_drop = load ptr, ptr %complete_drop_slot, align 8
  %complete_dropped = call i1 %complete_drop(ptr %complete_handle)
  %after_final_drop = load i64, ptr @__ncc_stc_cleanupCoroutine_cleaned, align 8

  %yield_ok = icmp eq i64 %yielded, 1
  %cancel_ok = icmp eq i64 %cancel_status, 3
  %suspend_clean = icmp eq i64 %before_drop, 0
  %early_clean = icmp eq i64 %after_early_drop, 1
  %yield_again_ok = icmp eq i64 %yielded_again, 1
  %complete_ok = icmp eq i64 %completed, 2
  %completion_clean = icmp eq i64 %after_complete, 2
  %no_double_clean = icmp eq i64 %after_final_drop, 2
  %fail0 = select i1 %yield_ok, i32 0, i32 1
  %fail1 = select i1 %suspend_clean, i32 0, i32 2
  %fail2 = select i1 %early_clean, i32 0, i32 4
  %fail3 = select i1 %yield_again_ok, i32 0, i32 8
  %fail4 = select i1 %complete_ok, i32 0, i32 16
  %fail5 = select i1 %completion_clean, i32 0, i32 32
  %fail6 = select i1 %no_double_clean, i32 0, i32 64
  %fail7 = select i1 %cancel_ok, i32 0, i32 128
  %mask01 = or i32 %fail0, %fail1
  %mask02 = or i32 %fail2, %fail3
  %mask03 = or i32 %fail4, %fail5
  %mask04 = or i32 %mask01, %mask02
  %mask05 = or i32 %mask03, %fail6
  %mask06 = or i32 %mask04, %mask05
  %result = or i32 %mask06, %fail7
  ret i32 %result
}
"#
            .replace(
                "COROUTINE_STORAGE_BYTES",
                &(plan.descriptor_size_bytes + plan.frame_size_bytes).to_string(),
            )
            .replace(
                "COROUTINE_STORAGE_ALIGN",
                &plan.frame_align_bytes.to_string(),
            );
            let mut llvm = module.llvm_ir.clone();
            let insert = llvm
                .find("attributes #")
                .or_else(|| llvm.find("!llvm.module.flags"))
                .unwrap_or(llvm.len());
            llvm.insert_str(insert, &harness);
            let nonce = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock")
                .as_nanos();
            let stem = format!("ncc_coroutine_defer_{}_{}", std::process::id(), nonce);
            let llvm_path = std::env::temp_dir().join(format!("{stem}.ll"));
            let exe_path = std::env::temp_dir().join(stem);
            std::fs::write(&llvm_path, llvm)
                .expect("coroutine defer LLVM fixture must be writable");
            let clang = std::process::Command::new("clang")
                .arg("-x")
                .arg("ir")
                .arg(&llvm_path)
                .arg("-o")
                .arg(&exe_path)
                .output()
                .expect("clang coroutine defer link");
            assert!(
                clang.status.success(),
                "clang must link coroutine defer state machine:\n{}",
                String::from_utf8_lossy(&clang.stderr)
            );
            let run = std::process::Command::new(&exe_path)
                .output()
                .expect("linked coroutine defer executable must run");
            let _ = std::fs::remove_file(&llvm_path);
            let _ = std::fs::remove_file(&exe_path);
            assert_eq!(
                run.status.code(),
                Some(0),
                "stderr={}",
                String::from_utf8_lossy(&run.stderr)
            );
        }
    }

    #[test]
    fn high_level_coroutine_descriptor_methods_execute_end_to_end() {
        let source = r#"
coroutineMethods {
    fn:
        async values() -> AsyncStream<int> {
            public:
                yield 7;
        }

        run() -> bool {
            public:
                @frame !> p<byte> : alloc(512);
                @stream >> AsyncStream<int> : asyncAt(@frame, values());
                @status >> int : @stream.resume();
                @item >> int : @stream.payload();
                @cancelled >> bool : @stream.cancel();
                @closed >> bool : @stream.drop();
                @ok >> bool : @status == 1 && @item == 7 && @cancelled && @closed;
                free(@frame);
                return @ok;
        }
}
"#;
        let program = parse_manual(source, "coroutine methods");
        let ir = lower_with_backend_profile(program, BackendLoweringProfileIR::nc_sysv64());
        let run = ir
            .containers
            .iter()
            .find_map(|container| match container {
                ContainerIR::Regular(container) => container
                    .functions
                    .iter()
                    .find(|function| function.name == "run"),
                _ => None,
            })
            .expect("coroutine method consumer must lower");
        let debug = format!("{:#?}", run.body);
        assert!(debug.contains("__ncc_coroutine_resume"));
        assert!(debug.contains("__ncc_coroutine_payload"));
        assert!(debug.contains("__ncc_coroutine_invoke_cancel"));
        assert!(debug.contains("__ncc_coroutine_invoke_drop"));

        let module = mct_emit_llvm_module(&ir);
        assert!(
            module.diagnostics.is_empty(),
            "diagnostics={:#?}\n{}",
            module.diagnostics,
            module.llvm_ir
        );
        let backend = emit_backend(&ir);
        let run_backend = backend
            .units
            .iter()
            .find_map(|unit| match unit {
                BackendUnitIR::Function(function) if function.symbol == "coroutineMethods.run" => {
                    Some(function)
                }
                _ => None,
            })
            .expect("native coroutine method consumer");
        assert_eq!(
            run_backend.lowering_coverage.route,
            BackendFunctionLoweringRouteIR::ScalarRegStack,
            "{run_backend:#?}"
        );

        if std::process::Command::new("clang")
            .arg("--version")
            .output()
            .is_ok()
        {
            let harness = r#"
define i32 @main() {
entry:
  %ok = call i1 @coroutineMethods_run()
  %result = select i1 %ok, i32 0, i32 1
  ret i32 %result
}
"#;
            let mut llvm = module.llvm_ir.clone();
            let insert = llvm
                .find("attributes #")
                .or_else(|| llvm.find("!llvm.module.flags"))
                .unwrap_or(llvm.len());
            llvm.insert_str(insert, harness);
            let nonce = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock")
                .as_nanos();
            let stem = format!("ncc_coroutine_methods_{}_{}", std::process::id(), nonce);
            let llvm_path = std::env::temp_dir().join(format!("{stem}.ll"));
            let exe_path = std::env::temp_dir().join(stem);
            std::fs::write(&llvm_path, llvm)
                .expect("coroutine methods LLVM fixture must be writable");
            let clang = std::process::Command::new("clang")
                .arg("-x")
                .arg("ir")
                .arg(&llvm_path)
                .arg("-o")
                .arg(&exe_path)
                .output()
                .expect("clang coroutine methods link");
            assert!(
                clang.status.success(),
                "clang must link coroutine method consumer:\n{}",
                String::from_utf8_lossy(&clang.stderr)
            );
            let run = std::process::Command::new(&exe_path)
                .output()
                .expect("linked coroutine method executable must run");
            let _ = std::fs::remove_file(&llvm_path);
            let _ = std::fs::remove_file(&exe_path);
            assert_eq!(
                run.status.code(),
                Some(0),
                "stderr={}",
                String::from_utf8_lossy(&run.stderr)
            );
        }
    }

    #[test]
    fn hosted_generator_yield_resume_executes_as_native_state_transitions() {
        let source = r#"
generatorNative {
    fn:
        values(seed: int) -> Generator<int> {
            public:
                yield @seed;
                yield @seed + 2;
        }
}
"#;
        let program = parse_manual(source, "generator");
        let ir = lower_with_backend_profile(program, BackendLoweringProfileIR::nc_sysv64());
        let function = ir
            .containers
            .iter()
            .find_map(|container| match container {
                ContainerIR::Regular(container) => container.functions.first(),
                _ => None,
            })
            .expect("generator function");
        let plan = function.coroutine.as_ref().expect("generator plan");
        assert_eq!(plan.storage, CoroutineFrameStorageIR::CallerOwned);
        assert_eq!(
            plan.states
                .iter()
                .filter(|state| state.action == CoroutineStateActionKindIR::Yield)
                .count(),
            2
        );
        let module = mct_emit_llvm_module(&ir);
        assert!(module.diagnostics.is_empty(), "{:#?}", module.diagnostics);
        if std::process::Command::new("clang")
            .arg("--version")
            .output()
            .is_err()
        {
            return;
        }
        let harness = format!(
            r#"
define i32 @main() {{
entry:
  %storage = alloca [{} x i8], align {}
  %handle = call ptr @generatorNative_values(ptr %storage, i64 40)
  %resume = load ptr, ptr %handle, align 8
  %frame_slot = getelementptr i8, ptr %handle, i64 8
  %frame = load ptr, ptr %frame_slot, align 8
  %s1 = call i64 %resume(ptr %frame)
  %payload_ptr = getelementptr i8, ptr %frame, i64 {}
  %v1 = load i64, ptr %payload_ptr, align 8
  %s2 = call i64 %resume(ptr %frame)
  %v2 = load i64, ptr %payload_ptr, align 8
  %s3 = call i64 %resume(ptr %frame)
  %a = icmp eq i64 %s1, 1
  %b = icmp eq i64 %v1, 40
  %c = icmp eq i64 %s2, 1
  %d = icmp eq i64 %v2, 42
  %e = icmp eq i64 %s3, 2
  %ab = and i1 %a, %b
  %cd = and i1 %c, %d
  %abcd = and i1 %ab, %cd
  %ok = and i1 %abcd, %e
  %drop_slot = getelementptr i8, ptr %handle, i64 16
  %drop = load ptr, ptr %drop_slot, align 8
  %dropped = call i1 %drop(ptr %handle)
  %ret = select i1 %ok, i32 0, i32 1
  ret i32 %ret
}}
"#,
            plan.descriptor_size_bytes + plan.frame_size_bytes,
            plan.frame_align_bytes,
            plan.payload_offset_bytes
        );
        let mut llvm = module.llvm_ir.clone();
        let insert = llvm
            .find("attributes #")
            .or_else(|| llvm.find("!llvm.module.flags"))
            .unwrap_or(llvm.len());
        llvm.insert_str(insert, &harness);
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock")
            .as_nanos();
        let stem = format!("ncc_generator_e2e_{}_{}", std::process::id(), nonce);
        let llvm_path = std::env::temp_dir().join(format!("{stem}.ll"));
        let exe_path = std::env::temp_dir().join(stem);
        std::fs::write(&llvm_path, llvm).expect("generator LLVM fixture must be writable");
        let clang = std::process::Command::new("clang")
            .arg("-x")
            .arg("ir")
            .arg(&llvm_path)
            .arg("-o")
            .arg(&exe_path)
            .output()
            .expect("clang generator link");
        assert!(
            clang.status.success(),
            "clang must link generator:\n{}",
            String::from_utf8_lossy(&clang.stderr)
        );
        let run = std::process::Command::new(&exe_path)
            .output()
            .expect("linked generator must run");
        let _ = std::fs::remove_file(&llvm_path);
        let _ = std::fs::remove_file(&exe_path);
        assert_eq!(
            run.status.code(),
            Some(0),
            "stderr={}\n{}",
            String::from_utf8_lossy(&run.stderr),
            module.llvm_ir
        );
    }

    #[test]
    fn aggregate_literal_payload_is_materialized_copied_and_executed() {
        let source = r#"
aggregateCoroutineNative {
    fn:
        values() -> Generator<array<int, 2>> {
            public:
                yield [40, 2];
        }
}
"#;
        let program = parse_manual(source, "aggregate coroutine");
        let ir = lower_with_backend_profile(program, BackendLoweringProfileIR::nc_sysv64());
        let function = ir
            .containers
            .iter()
            .find_map(|container| match container {
                ContainerIR::Regular(container) => container.functions.first(),
                _ => None,
            })
            .expect("aggregate coroutine must lower");
        let plan = function
            .coroutine
            .as_ref()
            .expect("aggregate coroutine plan");
        assert_eq!(plan.payload_size_bytes, 8);
        assert_eq!(plan.payload_align_bytes, 4);
        let debug = format!("{ir:#?}");
        assert!(debug.contains("coroutine.payload.materialize"), "{debug}");
        assert!(!debug.contains("aggregate.unsupported"), "{debug}");

        let module = mct_emit_llvm_module(&ir);
        assert!(
            module.diagnostics.is_empty(),
            "diagnostics={:#?}\n{}",
            module.diagnostics,
            module.llvm_ir
        );
        let backend = emit_backend(&ir);
        let resume = backend
            .units
            .iter()
            .find_map(|unit| match unit {
                BackendUnitIR::Function(function)
                    if function.symbol == plan.resume_symbol
                        || function
                            .symbol
                            .ends_with(&format!(".{}", plan.resume_symbol)) =>
                {
                    Some(function)
                }
                _ => None,
            })
            .expect("aggregate coroutine resume function");
        assert_eq!(
            resume.lowering_coverage.route,
            BackendFunctionLoweringRouteIR::ScalarRegStack,
            "{resume:#?}"
        );
        let object = emit_object(&backend);
        let elf = emit_object_elf64(&object).expect("aggregate coroutine object must be ELF64");
        assert_eq!(&elf[..4], b"\x7fELF");

        if std::process::Command::new("clang")
            .arg("--version")
            .output()
            .is_err()
        {
            return;
        }
        let harness = format!(
            r#"
define i32 @main() {{
entry:
  %storage = alloca [{} x i8], align {}
  %handle = call ptr @aggregateCoroutineNative_values(ptr %storage)
  %resume = load ptr, ptr %handle, align 8
  %frame_slot = getelementptr i8, ptr %handle, i64 8
  %frame = load ptr, ptr %frame_slot, align 8
  %status = call i64 %resume(ptr %frame)
  %payload = getelementptr i8, ptr %frame, i64 {}
  %second_ptr = getelementptr i8, ptr %payload, i64 4
  %first = load i32, ptr %payload, align 4
  %second = load i32, ptr %second_ptr, align 4
  %a = icmp eq i64 %status, 1
  %b = icmp eq i32 %first, 40
  %c = icmp eq i32 %second, 2
  %ab = and i1 %a, %b
  %ok = and i1 %ab, %c
  %drop_slot = getelementptr i8, ptr %handle, i64 16
  %drop = load ptr, ptr %drop_slot, align 8
  %dropped = call i1 %drop(ptr %handle)
  %ret = select i1 %ok, i32 0, i32 1
  ret i32 %ret
}}
"#,
            plan.descriptor_size_bytes + plan.frame_size_bytes,
            plan.frame_align_bytes,
            plan.payload_offset_bytes
        );
        let mut llvm = module.llvm_ir.clone();
        let insert = llvm
            .find("attributes #")
            .or_else(|| llvm.find("!llvm.module.flags"))
            .unwrap_or(llvm.len());
        llvm.insert_str(insert, &harness);
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock")
            .as_nanos();
        let stem = format!("ncc_aggregate_coroutine_{}_{}", std::process::id(), nonce);
        let llvm_path = std::env::temp_dir().join(format!("{stem}.ll"));
        let exe_path = std::env::temp_dir().join(stem);
        std::fs::write(&llvm_path, llvm)
            .expect("aggregate coroutine LLVM fixture must be writable");
        let clang = std::process::Command::new("clang")
            .arg("-x")
            .arg("ir")
            .arg(&llvm_path)
            .arg("-o")
            .arg(&exe_path)
            .output()
            .expect("clang aggregate coroutine link");
        assert!(
            clang.status.success(),
            "clang must link aggregate coroutine:\n{}",
            String::from_utf8_lossy(&clang.stderr)
        );
        let run = std::process::Command::new(&exe_path)
            .output()
            .expect("linked aggregate coroutine must run");
        let _ = std::fs::remove_file(&llvm_path);
        let _ = std::fs::remove_file(&exe_path);
        assert_eq!(
            run.status.code(),
            Some(0),
            "stderr={}\n{}",
            String::from_utf8_lossy(&run.stderr),
            module.llvm_ir
        );
    }

    #[test]
    fn async_closure_capture_and_blocking_await_execute() {
        let source = r#"
asyncClosureNative {
    fn:
        run() -> bool
        effects(async.await)
        {
            public:
                @base >> int : 40;
                @closureFrame !> p<byte> : alloc(512);
                @futureFrame !> p<byte> : alloc(512);
                @plus >> fn(int) -> Future<int> : closureAt(@closureFrame, async |value: int| -> int
                captures(@base)
                {
                    return @value + @base;
                });
                @pending >> Future<int> : asyncAt(@futureFrame, plus(2));
                @answer >> int : await pending;
                @ok >> bool : @answer == 42;
                free(@futureFrame);
                free(@closureFrame);
                return @ok;
        }
}
"#;
        let program = parse_manual(source, "async closure");
        let ir = lower_with_backend_profile(program, BackendLoweringProfileIR::nc_sysv64());
        let module = mct_emit_llvm_module(&ir);
        assert!(
            module.diagnostics.is_empty(),
            "diagnostics={:#?}\n{}",
            module.diagnostics,
            module.llvm_ir
        );
        assert!(module.llvm_ir.contains("switch i"), "{}", module.llvm_ir);
        assert!(module.llvm_ir.contains("__ncc_coroutine_"));
        let backend = emit_backend(&ir);
        let run = backend
            .units
            .iter()
            .find_map(|unit| match unit {
                BackendUnitIR::Function(function)
                    if function.symbol == "asyncClosureNative.run" =>
                {
                    Some(function)
                }
                _ => None,
            })
            .expect("run backend function");
        assert_eq!(
            run.lowering_coverage.route,
            BackendFunctionLoweringRouteIR::ScalarRegStack,
            "{run:#?}"
        );
        if std::process::Command::new("clang")
            .arg("--version")
            .output()
            .is_err()
        {
            return;
        }
        let harness = r#"
define i32 @main() {
entry:
  %ok = call i1 @asyncClosureNative_run()
  %ret = select i1 %ok, i32 0, i32 1
  ret i32 %ret
}
"#;
        let mut llvm = module.llvm_ir.clone();
        let insert = llvm
            .find("attributes #")
            .or_else(|| llvm.find("!llvm.module.flags"))
            .unwrap_or(llvm.len());
        llvm.insert_str(insert, harness);
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock")
            .as_nanos();
        let stem = format!("ncc_async_closure_e2e_{}_{}", std::process::id(), nonce);
        let llvm_path = std::env::temp_dir().join(format!("{stem}.ll"));
        let exe_path = std::env::temp_dir().join(stem);
        std::fs::write(&llvm_path, llvm).expect("async closure LLVM fixture must be writable");
        let clang = std::process::Command::new("clang")
            .arg("-x")
            .arg("ir")
            .arg(&llvm_path)
            .arg("-o")
            .arg(&exe_path)
            .output()
            .expect("clang async closure link");
        assert!(
            clang.status.success(),
            "clang must link async closure:\n{}",
            String::from_utf8_lossy(&clang.stderr)
        );
        let run = std::process::Command::new(&exe_path)
            .output()
            .expect("linked async closure must run");
        let _ = std::fs::remove_file(&llvm_path);
        let _ = std::fs::remove_file(&exe_path);
        assert_eq!(run.status.code(), Some(0));
    }

    #[test]
    fn canonical_runtime_await_import_uses_native_coroutine_lowering() {
        let source = r#"
use::stoud::collections::<result>;
use nC.runtime.async;

canonicalAwait[@runtime] {
    stc:
        cleaned >> int = 0;

    fn:
        source() -> Result<int, int> {
            public:
                return err(7);
        }

        async answer() -> Result<int, int> {
            public:
                defer {
                    cleaned++;
                }
                value >> int = source()?;
                return ok(value);
        }

        run() -> bool
        effects(async.await)
        {
            public:
                cleaned = 0;
                frame !> p<byte> : alloc(512);
                pending >> Future<Result<int, int>> = asyncAt(frame, answer());
                value >> Result<int, int> = await pending;
                ok >> bool = isErr(value) && cleaned == 1;
                free(frame);
                return ok;
        }
}
"#;
        let program = parse_manual(source, "canonical await");
        let ir = lower_with_backend_profile(program, BackendLoweringProfileIR::nc_sysv64());
        let answer_ir = ir
            .containers
            .iter()
            .find_map(|container| match container {
                ContainerIR::Regular(container) => container
                    .functions
                    .iter()
                    .find(|function| function.name == "answer"),
                ContainerIR::Root(container) => container
                    .functions
                    .iter()
                    .find(|function| function.name == "answer"),
                _ => None,
            })
            .expect("answer IR function");
        let resume_ir = answer_ir
            .closures
            .iter()
            .find(|function| function.coroutine_role == Some(CoroutineFunctionRoleIR::Resume))
            .expect("answer resume IR function");
        assert!(format!("{:#?}", resume_ir.body).contains("coroutine.try.materialize"));

        let module = mct_emit_llvm_module(&ir);
        assert_eq!(module.rejected_function_count, 0, "{:#?}", module.functions);
        assert!(module.llvm_ir.contains("switch i"), "{}", module.llvm_ir);
        assert!(!module.llvm_ir.contains("@nC_runtime_async_await"));

        let backend = emit_backend(&ir);
        let run = backend
            .units
            .iter()
            .find_map(|unit| match unit {
                BackendUnitIR::Function(function) if function.symbol == "canonicalAwait.run" => {
                    Some(function)
                }
                _ => None,
            })
            .expect("run backend function");
        assert_eq!(
            run.lowering_coverage.route,
            BackendFunctionLoweringRouteIR::ScalarRegStack
        );
        assert!(
            run.lowering_coverage.is_complete(),
            "{:#?}",
            run.lowering_coverage
        );
        let resume = backend
            .units
            .iter()
            .find_map(|unit| match unit {
                BackendUnitIR::Function(function)
                    if function.symbol == "canonicalAwait.__ncc_coroutine_answer_resume" =>
                {
                    Some(function)
                }
                _ => None,
            })
            .expect("answer resume backend function");
        assert_eq!(
            resume.lowering_coverage.route,
            BackendFunctionLoweringRouteIR::ScalarRegStack,
            "{resume:#?}"
        );
        assert!(
            resume.lowering_coverage.is_complete(),
            "{:#?}",
            resume.lowering_coverage
        );
        let incomplete = backend
            .units
            .iter()
            .filter_map(|unit| match unit {
                BackendUnitIR::Function(function)
                    if function.container == "canonicalAwait"
                        && !function.lowering_coverage.is_complete() =>
                {
                    Some(function.symbol.as_str())
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        assert!(
            incomplete.is_empty(),
            "incomplete native units: {incomplete:#?}"
        );
        let object = emit_object(&backend);
        let elf = emit_object_elf64(&object)
            .expect("canonical await machine units must serialize as ELF64");
        assert_eq!(&elf[..4], b"\x7fELF");
        assert!(String::from_utf8_lossy(&elf).contains("__ncc_coroutine_answer_resume"));
    }

    #[test]
    fn coroutine_descriptor_and_lifecycle_are_layout_correct_across_declared_abis() {
        let source = r#"
coroutineAbiMatrix {
    fn:
        async work(value: int) -> int {
            public:
                return @value + 1;
        }

    fa:
        return true;
}
"#;
        let profiles = vec![
            ("x86_64-linux-sysv", BackendLoweringProfileIR::nc_sysv64()),
            ("x86_64-windows", BackendLoweringProfileIR::nc_win64()),
            (
                "x86_64-darwin",
                BackendLoweringProfileIR::nc_x86_64_darwin(),
            ),
            ("aarch64-aapcs", BackendLoweringProfileIR::nc_aapcs64()),
            (
                "aarch64-linux",
                BackendLoweringProfileIR::nc_aarch64_linux(),
            ),
            (
                "aarch64-windows",
                BackendLoweringProfileIR::nc_aarch64_windows(),
            ),
            ("wasm32-portable", BackendLoweringProfileIR::nc_portable()),
            ("x86_64-sovereign", BackendLoweringProfileIR::nc_sovereign()),
        ];

        for (label, profile) in profiles {
            let program = parse_manual(source, "ABI matrix");
            let ir = lower_with_backend_profile(program, profile.clone());
            let function = ir
                .containers
                .iter()
                .find_map(|container| match container {
                    ContainerIR::Regular(container) => container
                        .functions
                        .iter()
                        .find(|function| function.name == "work"),
                    _ => None,
                })
                .unwrap_or_else(|| panic!("{label}: missing work function"));
            let plan = function
                .coroutine
                .as_ref()
                .unwrap_or_else(|| panic!("{label}: missing coroutine plan"));
            let pointer_bytes = profile.target_spec.data_layout.pointer_bytes;
            assert_eq!(plan.descriptor_size_bytes, pointer_bytes * 4, "{label}");
            let generated = function
                .closures
                .iter()
                .filter(|child| child.coroutine_role.is_some())
                .collect::<Vec<_>>();
            assert_eq!(generated.len(), 4, "{label}: {generated:#?}");
            assert!(generated.iter().any(|child| {
                child.coroutine_role == Some(CoroutineFunctionRoleIR::Constructor)
            }));
            assert!(generated
                .iter()
                .any(|child| { child.coroutine_role == Some(CoroutineFunctionRoleIR::Resume) }));
            assert!(generated
                .iter()
                .any(|child| { child.coroutine_role == Some(CoroutineFunctionRoleIR::Drop) }));
            assert!(generated
                .iter()
                .any(|child| { child.coroutine_role == Some(CoroutineFunctionRoleIR::Cancel) }));

            assert_eq!(
                plan.storage,
                CoroutineFrameStorageIR::CallerOwned,
                "{label}"
            );
            assert!(!plan.hidden_allocation, "{label}");
            assert!(!plan.requires_scheduler, "{label}");
            assert!(generated.iter().all(|child| {
                !format!("{:#?}", child.body).contains("malloc")
                    && !format!("{:#?}", child.body).contains("free")
            }));
            let constructor = generated
                .iter()
                .find(|child| child.coroutine_role == Some(CoroutineFunctionRoleIR::Constructor))
                .expect("caller-owned constructor");
            assert_eq!(
                constructor.params.first().map(|param| param.name.as_str()),
                Some("__ncc_coroutine_storage"),
                "{label}"
            );
        }
    }
}
