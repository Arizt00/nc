use ncc_ast::{AssignTarget, Expr, FunctionDecl, MainEntryKind, Program, Section, Stmt};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
struct AllocationFact {
    bytes: Option<usize>,
    live: bool,
    generation: u64,
}

#[derive(Debug, Clone)]
struct PointerFact {
    allocation: usize,
    offset_bytes: Option<i64>,
    elem_type: Option<String>,
    generation: u64,
}

#[derive(Debug, Clone)]
struct RegionFact {
    pointer: PointerFact,
    elem_type: String,
    len: Option<usize>,
}

#[derive(Debug, Clone)]
enum MemoryValue {
    Unknown,
    Number(i64),
    Pointer(PointerFact),
    Region(RegionFact),
}

#[derive(Debug, Clone, Default)]
struct MemoryFacts {
    allocations: Vec<AllocationFact>,
    pointers: HashMap<String, PointerFact>,
    regions: HashMap<String, RegionFact>,
}

struct NativeMemoryValidator<'a> {
    functions: HashMap<String, &'a FunctionDecl>,
    facts: MemoryFacts,
    active_calls: HashSet<String>,
}

pub(crate) fn validate_program(
    program: &Program,
    explicit_entry: Option<&str>,
) -> Result<(), String> {
    let mut functions = HashMap::new();
    for container in &program.containers {
        for section in &container.sections {
            if let Section::Fn(items) | Section::MoveFn(items) | Section::ConstFn(items) = section {
                for function in items {
                    functions.insert(function.name.to_ascii_lowercase(), function);
                }
            }
        }
    }

    let mut validator = NativeMemoryValidator {
        functions,
        facts: MemoryFacts::default(),
        active_calls: HashSet::new(),
    };

    if let Some(entry) = explicit_entry {
        let entry = entry
            .rsplit(['.', ':'])
            .find(|part| !part.is_empty())
            .unwrap_or(entry);
        validator.call_function(entry, &[])?;
        return Ok(());
    }

    for container in &program.containers {
        for section in &container.sections {
            let Section::MainEntry { kind, body } = section else {
                continue;
            };
            match kind {
                MainEntryKind::RefFn(reference) | MainEntryKind::SkkFn(reference) => {
                    if let Some(entry) = reference.path.last() {
                        validator.call_function(entry, &[])?;
                        return Ok(());
                    }
                }
                MainEntryKind::ToMain => {
                    if !body.is_empty() {
                        validator.process_block(body)?;
                    } else {
                        validator.call_function("main", &[])?;
                    }
                    return Ok(());
                }
                MainEntryKind::ToMainProfile(_) => {}
            }
        }
    }
    Ok(())
}

impl NativeMemoryValidator<'_> {
    fn call_function(&mut self, name: &str, args: &[MemoryValue]) -> Result<MemoryValue, String> {
        let key = name.to_ascii_lowercase();
        let Some(function) = self.functions.get(&key).copied() else {
            return Ok(MemoryValue::Unknown);
        };
        if !self.active_calls.insert(key.clone()) {
            return Ok(MemoryValue::Unknown);
        }

        let mut saved = Vec::new();
        for (index, parameter) in function.params.iter().enumerate() {
            let old_pointer = self.facts.pointers.remove(&parameter.name);
            let old_region = self.facts.regions.remove(&parameter.name);
            saved.push((parameter.name.clone(), old_pointer, old_region));
            self.assign_symbol(
                &parameter.name,
                args.get(index).cloned().unwrap_or(MemoryValue::Unknown),
            );
        }

        let result = self.process_block(&function.public_body);
        for (name, pointer, region) in saved {
            self.facts.pointers.remove(&name);
            self.facts.regions.remove(&name);
            if let Some(pointer) = pointer {
                self.facts.pointers.insert(name.clone(), pointer);
            }
            if let Some(region) = region {
                self.facts.regions.insert(name, region);
            }
        }
        self.active_calls.remove(&key);
        result
    }

    fn process_block(&mut self, statements: &[Stmt]) -> Result<MemoryValue, String> {
        for statement in statements {
            if let Some(value) = self.process_stmt(statement)? {
                return Ok(value);
            }
        }
        Ok(MemoryValue::Unknown)
    }

    fn process_stmt(&mut self, statement: &Stmt) -> Result<Option<MemoryValue>, String> {
        match statement {
            Stmt::Projection { target, value, .. } | Stmt::StrictCheck { target, value, .. } => {
                let value = self.eval_expr(value)?;
                if let Some(symbol) = assignment_symbol(target) {
                    self.assign_symbol(symbol, value);
                }
            }
            Stmt::Expr(expr) | Stmt::Print { value: expr, .. } => {
                self.eval_expr(expr)?;
            }
            Stmt::Return(expr) => return Ok(Some(self.eval_expr(expr)?)),
            Stmt::If {
                condition,
                then_block,
                else_block,
            } => match const_bool(condition) {
                Some(true) => {
                    let value = self.process_block(then_block)?;
                    if !matches!(value, MemoryValue::Unknown) {
                        return Ok(Some(value));
                    }
                }
                Some(false) => {
                    let value = self.process_block(else_block)?;
                    if !matches!(value, MemoryValue::Unknown) {
                        return Ok(Some(value));
                    }
                }
                None => {
                    self.eval_expr(condition)?;
                }
            },
            Stmt::GuardElse {
                condition,
                else_block,
            } => match const_bool(condition) {
                Some(false) => {
                    let value = self.process_block(else_block)?;
                    if !matches!(value, MemoryValue::Unknown) {
                        return Ok(Some(value));
                    }
                }
                _ => {
                    self.eval_expr(condition)?;
                }
            },
            Stmt::Defer { body } => {
                // Deferred bodies execute on every normal function exit. Their
                // memory effects are checked without changing straight-line facts.
                let mut deferred = NativeMemoryValidator {
                    functions: self.functions.clone(),
                    facts: self.facts.clone(),
                    active_calls: self.active_calls.clone(),
                };
                deferred.process_block(body)?;
            }
            Stmt::Labeled { stmt, .. } => {
                if let Some(value) = self.process_stmt(stmt)? {
                    return Ok(Some(value));
                }
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
                let selector = self.eval_expr(value)?;
                if let MemoryValue::Number(selector) = selector {
                    let mut selected = None;
                    for (case, body) in cases {
                        if matches!(self.eval_expr(case)?, MemoryValue::Number(value) if value == selector)
                        {
                            selected = Some(body.as_slice());
                            break;
                        }
                    }
                    let value = self.process_block(selected.unwrap_or(default))?;
                    if !matches!(value, MemoryValue::Unknown) {
                        return Ok(Some(value));
                    }
                }
            }
            Stmt::PatternMatch { value, .. }
            | Stmt::Unless {
                condition: value, ..
            }
            | Stmt::While {
                condition: value, ..
            } => {
                self.eval_expr(value)?;
            }
            Stmt::Select { branches, .. } => {
                for (condition, _) in branches {
                    self.eval_expr(condition)?;
                }
            }
            Stmt::DoWhile {
                body, condition, ..
            }
            | Stmt::RepeatUntil {
                body, condition, ..
            } if body.is_empty() => {
                self.eval_expr(condition)?;
            }
            Stmt::ForIn { iterable, .. }
            | Stmt::SameIn { iterable, .. }
            | Stmt::EachDoWhile { iterable, .. } => {
                self.eval_expr(iterable)?;
            }
            Stmt::ForRange {
                start, end, step, ..
            } => {
                self.eval_expr(start)?;
                self.eval_expr(end)?;
                if let Some(step) = step {
                    self.eval_expr(step)?;
                }
            }
            Stmt::RepeatCount { count, .. } => {
                self.eval_expr(count)?;
            }
            Stmt::ForClassic {
                init,
                condition,
                step,
                ..
            } => {
                if let Some(init) = init {
                    self.process_stmt(init)?;
                }
                if let Some(condition) = condition {
                    self.eval_expr(condition)?;
                }
                if let Some(step) = step {
                    self.process_stmt(step)?;
                }
            }
            Stmt::Constructure { args, .. } | Stmt::Destructure { args, .. } => {
                for arg in args {
                    self.eval_expr(arg)?;
                }
            }
            _ => {}
        }
        Ok(None)
    }

    fn eval_expr(&mut self, expr: &Expr) -> Result<MemoryValue, String> {
        match expr {
            Expr::Number(value) => Ok(value
                .replace('_', "")
                .parse::<i64>()
                .map(MemoryValue::Number)
                .unwrap_or(MemoryValue::Unknown)),
            Expr::Identifier(name) => Ok(self.symbol_value(name)),
            Expr::ContextRef(reference) => Ok(reference
                .path
                .last()
                .map(|name| self.symbol_value(name))
                .unwrap_or(MemoryValue::Unknown)),
            Expr::Call { callee, args } => self.eval_call(callee, args),
            Expr::QualifiedCall { path, args } => {
                self.eval_call(path.last().map(String::as_str).unwrap_or(""), args)
            }
            Expr::MethodCall {
                receiver,
                method,
                args,
            } => {
                let mut values = vec![self.eval_expr(receiver)?];
                for arg in args {
                    values.push(self.eval_expr(arg)?);
                }
                self.eval_call_values(method, values)
            }
            Expr::StrictCheckExpr { value, .. } => self.eval_expr(value),
            Expr::Binary { left, op, right } => {
                let left = self.eval_expr(left)?;
                let right = self.eval_expr(right)?;
                if matches!(op, ncc_ast::BinaryOp::Plus | ncc_ast::BinaryOp::Minus) {
                    match (left, right) {
                        (MemoryValue::Pointer(mut pointer), MemoryValue::Number(delta)) => {
                            let stride = pointer
                                .elem_type
                                .as_deref()
                                .and_then(layout_size)
                                .unwrap_or(1) as i64;
                            let delta = if matches!(op, ncc_ast::BinaryOp::Minus) {
                                delta.saturating_neg()
                            } else {
                                delta
                            };
                            pointer.offset_bytes = pointer
                                .offset_bytes
                                .map(|offset| offset.saturating_add(delta.saturating_mul(stride)));
                            return Ok(MemoryValue::Pointer(pointer));
                        }
                        (MemoryValue::Number(left), MemoryValue::Number(right)) => {
                            return Ok(MemoryValue::Number(
                                if matches!(op, ncc_ast::BinaryOp::Plus) {
                                    left.saturating_add(right)
                                } else {
                                    left.saturating_sub(right)
                                },
                            ));
                        }
                        _ => {}
                    }
                }
                Ok(MemoryValue::Unknown)
            }
            Expr::Unary { expr, .. } => self.eval_expr(expr),
            Expr::IfElse {
                condition,
                then_expr,
                else_expr,
            } => match const_bool(condition) {
                Some(true) => self.eval_expr(then_expr),
                Some(false) => self.eval_expr(else_expr),
                None => {
                    self.eval_expr(condition)?;
                    Ok(MemoryValue::Unknown)
                }
            },
            Expr::Index { target, index } => {
                self.eval_expr(target)?;
                self.eval_expr(index)?;
                Ok(MemoryValue::Unknown)
            }
            Expr::ArrayLiteral(items) | Expr::VectorLiteral(items) => {
                for item in items {
                    self.eval_expr(item)?;
                }
                Ok(MemoryValue::Unknown)
            }
            _ => Ok(MemoryValue::Unknown),
        }
    }

    fn eval_call(&mut self, callee: &str, args: &[Expr]) -> Result<MemoryValue, String> {
        let mut values = Vec::with_capacity(args.len());
        for arg in args {
            values.push(self.eval_expr(arg)?);
        }
        self.eval_call_values(callee, values)
    }

    fn eval_call_values(
        &mut self,
        callee: &str,
        args: Vec<MemoryValue>,
    ) -> Result<MemoryValue, String> {
        let (name, generic) = split_call_name(callee);
        match name.as_str() {
            "alloc" | "heapalloc" | "malloc" => {
                let bytes = args
                    .first()
                    .and_then(memory_number)
                    .and_then(non_negative_usize);
                Ok(MemoryValue::Pointer(self.allocate(bytes, generic)))
            }
            "calloc" => {
                let count = args
                    .first()
                    .and_then(memory_number)
                    .and_then(non_negative_usize);
                let size = args
                    .get(1)
                    .and_then(memory_number)
                    .and_then(non_negative_usize);
                let bytes = count.zip(size).and_then(|(a, b)| a.checked_mul(b));
                Ok(MemoryValue::Pointer(self.allocate(bytes, generic)))
            }
            "realloc" | "heaprealloc" => {
                let old = args.first().and_then(memory_pointer).cloned();
                if let Some(old) = old {
                    self.require_live(&old, "realloc")?;
                    if let Some(allocation) = self.facts.allocations.get_mut(old.allocation) {
                        allocation.live = false;
                    }
                }
                let bytes = args
                    .get(1)
                    .and_then(memory_number)
                    .and_then(non_negative_usize);
                Ok(MemoryValue::Pointer(self.allocate(bytes, generic)))
            }
            "free" | "heapfree" => {
                if let Some(pointer) = args.first().and_then(memory_pointer) {
                    self.require_live(pointer, "free")?;
                    if let Some(allocation) = self.facts.allocations.get_mut(pointer.allocation) {
                        allocation.live = false;
                    }
                }
                Ok(MemoryValue::Unknown)
            }
            "regionfromptr" => self.create_region(&args, generic),
            "regionptrat" => self.region_pointer_at(&args, generic),
            "ptrread" | "volatileread" => {
                if let Some(pointer) = args.first().and_then(memory_pointer) {
                    self.require_live(pointer, &name)?;
                    self.require_pointer_type(pointer, generic.as_deref(), &name)?;
                }
                Ok(MemoryValue::Unknown)
            }
            "ptrwrite" | "volatilewrite" => {
                if let Some(pointer) = args.first().and_then(memory_pointer) {
                    self.require_live(pointer, &name)?;
                    self.require_pointer_type(pointer, generic.as_deref(), &name)?;
                }
                Ok(MemoryValue::Unknown)
            }
            "ptroffset" => {
                let Some(mut pointer) = args.first().and_then(memory_pointer).cloned() else {
                    return Ok(MemoryValue::Unknown);
                };
                let Some(index) = args.get(1).and_then(memory_number) else {
                    pointer.offset_bytes = None;
                    return Ok(MemoryValue::Pointer(pointer));
                };
                let stride = pointer
                    .elem_type
                    .as_deref()
                    .and_then(layout_size)
                    .unwrap_or(1) as i64;
                pointer.offset_bytes = pointer
                    .offset_bytes
                    .map(|offset| offset.saturating_add(index.saturating_mul(stride)));
                Ok(MemoryValue::Pointer(pointer))
            }
            "ptrcast" => {
                let Some(mut pointer) = args.first().and_then(memory_pointer).cloned() else {
                    return Ok(MemoryValue::Unknown);
                };
                if let Some(generic) = generic {
                    pointer.elem_type = Some(normalize_type(&generic));
                }
                Ok(MemoryValue::Pointer(pointer))
            }
            _ => self.call_function(&name, &args),
        }
    }

    fn create_region(
        &mut self,
        args: &[MemoryValue],
        generic: Option<String>,
    ) -> Result<MemoryValue, String> {
        let Some(pointer) = args.first().and_then(memory_pointer).cloned() else {
            return Ok(MemoryValue::Unknown);
        };
        self.require_live(&pointer, "regionFromPtr")?;
        let elem_type = generic
            .map(|value| normalize_type(&value))
            .or_else(|| pointer.elem_type.clone())
            .unwrap_or_else(|| "byte".into());
        let len = args
            .get(1)
            .and_then(memory_number)
            .and_then(non_negative_usize);
        if matches!(len, Some(0)) {
            return Err("E_REGION_BOUNDS_001: regionFromPtr requires len > 0".into());
        }
        if let (Some(len), Some(stride), Some(offset), Some(allocation)) = (
            len,
            layout_size(&elem_type),
            pointer.offset_bytes,
            self.facts.allocations.get(pointer.allocation),
        ) {
            if let Some(bytes) = allocation.bytes {
                let view_bytes = len.checked_mul(stride);
                let end = usize::try_from(offset)
                    .ok()
                    .zip(view_bytes)
                    .and_then(|(offset, view)| offset.checked_add(view));
                if end.is_none_or(|end| end > bytes) {
                    return Err("E_REGION_BOUNDS_001: regionFromPtr view leaves heap block".into());
                }
            }
        }
        Ok(MemoryValue::Region(RegionFact {
            pointer,
            elem_type,
            len,
        }))
    }

    fn region_pointer_at(
        &self,
        args: &[MemoryValue],
        generic: Option<String>,
    ) -> Result<MemoryValue, String> {
        let Some(region) = args.first().and_then(memory_region) else {
            return Ok(MemoryValue::Unknown);
        };
        self.require_live(&region.pointer, "regionPtrAt")?;
        let requested = generic
            .map(|value| normalize_type(&value))
            .unwrap_or_else(|| region.elem_type.clone());
        if requested != normalize_type(&region.elem_type) {
            return Err(format!(
                "E_PTR_TYPE_002: regionPtrAt expected {}, found {}",
                region.elem_type, requested
            ));
        }
        if let (Some(index), Some(len)) = (args.get(1).and_then(memory_number), region.len) {
            if index < 0 || usize::try_from(index).map_or(true, |index| index >= len) {
                return Err("E_REGION_BOUNDS_001: regionPtrAt index leaves region".into());
            }
        }
        let mut pointer = region.pointer.clone();
        pointer.elem_type = Some(requested.clone());
        if let (Some(offset), Some(index), Some(stride)) = (
            pointer.offset_bytes,
            args.get(1).and_then(memory_number),
            layout_size(&requested),
        ) {
            pointer.offset_bytes = Some(offset.saturating_add(index.saturating_mul(stride as i64)));
        } else {
            pointer.offset_bytes = None;
        }
        Ok(MemoryValue::Pointer(pointer))
    }

    fn allocate(&mut self, bytes: Option<usize>, elem_type: Option<String>) -> PointerFact {
        let allocation = self.facts.allocations.len();
        self.facts.allocations.push(AllocationFact {
            bytes,
            live: true,
            generation: 1,
        });
        PointerFact {
            allocation,
            offset_bytes: Some(0),
            elem_type: elem_type.map(|value| normalize_type(&value)),
            generation: 1,
        }
    }

    fn require_live(&self, pointer: &PointerFact, operation: &str) -> Result<(), String> {
        let Some(allocation) = self.facts.allocations.get(pointer.allocation) else {
            return Ok(());
        };
        if !allocation.live || allocation.generation != pointer.generation {
            return Err(format!(
                "E_PTR_STALE_SITE_001: {operation} cannot use a stale pointer generation"
            ));
        }
        Ok(())
    }

    fn require_pointer_type(
        &self,
        pointer: &PointerFact,
        requested: Option<&str>,
        operation: &str,
    ) -> Result<(), String> {
        let (Some(actual), Some(requested)) = (pointer.elem_type.as_deref(), requested) else {
            return Ok(());
        };
        let requested = normalize_type(requested);
        if normalize_type(actual) != requested {
            return Err(format!(
                "E_PTR_TYPE_002: {operation} expected {}, found {}",
                actual, requested
            ));
        }
        Ok(())
    }

    fn symbol_value(&self, name: &str) -> MemoryValue {
        self.facts
            .pointers
            .get(name)
            .cloned()
            .map(MemoryValue::Pointer)
            .or_else(|| {
                self.facts
                    .regions
                    .get(name)
                    .cloned()
                    .map(MemoryValue::Region)
            })
            .unwrap_or(MemoryValue::Unknown)
    }

    fn assign_symbol(&mut self, symbol: &str, value: MemoryValue) {
        self.facts.pointers.remove(symbol);
        self.facts.regions.remove(symbol);
        match value {
            MemoryValue::Pointer(pointer) => {
                self.facts.pointers.insert(symbol.to_string(), pointer);
            }
            MemoryValue::Region(region) => {
                self.facts.regions.insert(symbol.to_string(), region);
            }
            MemoryValue::Unknown | MemoryValue::Number(_) => {}
        }
    }
}

fn assignment_symbol(target: &AssignTarget) -> Option<&str> {
    match target {
        AssignTarget::Context(reference) => reference.path.last().map(String::as_str),
        AssignTarget::Mem { .. } => None,
    }
}

fn split_call_name(raw: &str) -> (String, Option<String>) {
    let leaf = raw
        .rsplit(['.', ':'])
        .find(|part| !part.is_empty())
        .unwrap_or(raw);
    let (name, generic) = match leaf.find('<') {
        Some(open) if leaf.ends_with('>') => (
            &leaf[..open],
            Some(leaf[open + 1..leaf.len() - 1].to_string()),
        ),
        _ => (leaf, None),
    };
    (name.to_ascii_lowercase(), generic)
}

fn normalize_type(value: &str) -> String {
    value
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .flat_map(char::to_lowercase)
        .collect()
}

fn layout_size(value: &str) -> Option<usize> {
    match normalize_type(value).as_str() {
        "bool" | "byte" | "int8" | "uint8" => Some(1),
        "int16" | "uint16" | "word" => Some(2),
        "char" | "int32" | "uint32" | "dword" | "ft32" | "int" | "uint" => Some(4),
        "int64" | "uint64" | "qword" | "ft64" | "ft" | "addr" => Some(8),
        value if value.starts_with("ptr<") => Some(8),
        _ => None,
    }
}

fn memory_number(value: &MemoryValue) -> Option<i64> {
    match value {
        MemoryValue::Number(value) => Some(*value),
        _ => None,
    }
}

fn memory_pointer(value: &MemoryValue) -> Option<&PointerFact> {
    match value {
        MemoryValue::Pointer(value) => Some(value),
        _ => None,
    }
}

fn memory_region(value: &MemoryValue) -> Option<&RegionFact> {
    match value {
        MemoryValue::Region(value) => Some(value),
        _ => None,
    }
}

fn non_negative_usize(value: i64) -> Option<usize> {
    usize::try_from(value).ok()
}

fn const_bool(expr: &Expr) -> Option<bool> {
    match expr {
        Expr::Bool(value) => Some(*value),
        _ => None,
    }
}
