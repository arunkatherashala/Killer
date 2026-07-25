use std::collections::{HashMap, HashSet};

use crate::bytecode::{Instruction, Program};
use crate::error::VmError;

/// Array methods that mutate the receiver; compiled as store-back assignment.
const MUTATING_ARRAY_METHODS: &[&str] = &["push", "pop", "shift", "unshift"];

// Helper function to collect variables from a pattern
fn collect_pattern_vars(pattern: &crate::ast::Pattern, vars: &mut HashSet<String>) {
    use crate::ast::Pattern;
    match pattern {
        Pattern::Identifier(name) => {
            vars.insert(name.clone());
        }
        Pattern::Array(patterns) => {
            for p in patterns {
                collect_pattern_vars(p, vars);
            }
        }
        Pattern::Object(pairs) => {
            for (_key, pattern) in pairs {
                collect_pattern_vars(pattern, vars);
            }
        }
    }
}

// Helper function to collect all variable references from statements
fn collect_var_references(stmts: &[crate::ast::Stmt]) -> HashSet<String> {
    let mut vars = HashSet::new();
    for stmt in stmts {
        collect_stmt_vars(stmt, &mut vars);
    }
    vars
}

fn collect_stmt_vars(stmt: &crate::ast::Stmt, vars: &mut HashSet<String>) {
    use crate::ast::Stmt;
    match stmt {
        Stmt::Let { pattern, value } | Stmt::Quality { pattern, value } | Stmt::Assign { pattern, value } => {
            collect_pattern_vars(pattern, vars);
            collect_expr_vars(value, vars);
        }
        Stmt::IndexAssign { object: _, index, value } => {
            collect_expr_vars(index, vars);
            collect_expr_vars(value, vars);
        }
        Stmt::Print(args) => {
            for arg in args {
                collect_expr_vars(arg, vars);
            }
        }
        Stmt::Expr(expr) => {
            collect_expr_vars(expr, vars);
        }
        Stmt::If { condition, then_branch, else_branch } => {
            collect_expr_vars(condition, vars);
            for s in then_branch {
                collect_stmt_vars(s, vars);
            }
            for s in else_branch {
                collect_stmt_vars(s, vars);
            }
        }
        Stmt::While { condition, body } | Stmt::DoWhile { condition, body } => {
            collect_expr_vars(condition, vars);
            for s in body {
                collect_stmt_vars(s, vars);
            }
        }
        Stmt::For { iterable, body, .. } => {
            collect_expr_vars(iterable, vars);
            for s in body {
                collect_stmt_vars(s, vars);
            }
        }
        Stmt::Return(Some(expr)) => {
            collect_expr_vars(expr, vars);
        }
        Stmt::Try { try_body, catch_body, finally_body, .. } => {
            for s in try_body {
                collect_stmt_vars(s, vars);
            }
            for s in catch_body {
                collect_stmt_vars(s, vars);
            }
            for s in finally_body {
                collect_stmt_vars(s, vars);
            }
        }
        Stmt::Throw(expr) => {
            collect_expr_vars(expr, vars);
        }
        Stmt::Switch { expression, cases, default } => {
            collect_expr_vars(expression, vars);
            for (case_expr, case_stmts) in cases {
                collect_expr_vars(case_expr, vars);
                for s in case_stmts {
                    collect_stmt_vars(s, vars);
                }
            }
            for s in default {
                collect_stmt_vars(s, vars);
            }
        }
        Stmt::Match { expression, arms } => {
            collect_expr_vars(expression, vars);
            for arm in arms {
                collect_match_pattern_vars(&arm.pattern, vars);
                if let Some(guard) = &arm.guard {
                    collect_expr_vars(guard, vars);
                }
                for s in &arm.body {
                    collect_stmt_vars(s, vars);
                }
            }
        }
        _ => {}
    }
}

fn collect_match_pattern_vars(pat: &crate::ast::MatchPattern, vars: &mut HashSet<String>) {
    use crate::ast::MatchPattern;
    match pat {
        MatchPattern::Identifier(name) if name != "_" => { vars.insert(name.clone()); }
        MatchPattern::Array(pats) => {
            for p in pats { collect_match_pattern_vars(p, vars); }
        }
        MatchPattern::Object(pairs) => {
            for (_, p) in pairs { collect_match_pattern_vars(p, vars); }
        }
        _ => {}
    }
}

fn collect_expr_vars(expr: &crate::ast::Expr, vars: &mut HashSet<String>) {
    use crate::ast::Expr;
    match expr {
        Expr::Identifier(name) => {
            vars.insert(name.clone());
        }
        Expr::Binary { left, op: _, right } => {
            collect_expr_vars(left, vars);
            collect_expr_vars(right, vars);
        }
        Expr::Ternary { condition, then_expr, else_expr } => {
            collect_expr_vars(condition, vars);
            collect_expr_vars(then_expr, vars);
            collect_expr_vars(else_expr, vars);
        }
        Expr::PrefixInc(name) | Expr::PrefixDec(name) | Expr::PostfixInc(name) | Expr::PostfixDec(name) => {
            vars.insert(name.clone());
        }
        Expr::Call { callee: _, args } => {
            for arg in args {
                collect_expr_vars(arg, vars);
            }
        }
        Expr::BuiltinCall { name: _, args } => {
            for arg in args {
                collect_expr_vars(arg, vars);
            }
        }
        Expr::CallExpr { callee, args } => {
            collect_expr_vars(callee, vars);
            for arg in args {
                collect_expr_vars(arg, vars);
            }
        }
        Expr::MethodCall { object, method: _, args } => {
            collect_expr_vars(object, vars);
            for arg in args {
                collect_expr_vars(arg, vars);
            }
        }
        Expr::Array(elements) => {
            for elem in elements {
                collect_expr_vars(elem, vars);
            }
        }
        Expr::Index { object, index } => {
            collect_expr_vars(object, vars);
            collect_expr_vars(index, vars);
        }
        Expr::Dict(pairs) => {
            for (key, value) in pairs {
                collect_expr_vars(key, vars);
                collect_expr_vars(value, vars);
            }
        }
        Expr::Range { start, end, step } => {
            collect_expr_vars(start, vars);
            collect_expr_vars(end, vars);
            if let Some(s) = step {
                collect_expr_vars(s, vars);
            }
        }
        Expr::New { class_name: _, args } => {
            for arg in args {
                collect_expr_vars(arg, vars);
            }
        }
        Expr::FunctionExpr { params, body } => {
            // Don't collect vars from nested function bodies - they have their own scope
            // But we need to track that this function is defined
            let inner_vars = collect_var_references(body);
            // Only include variables that are NOT parameters
            for var in inner_vars {
                if !params.contains(&var) {
                    vars.insert(var);
                }
            }
        }
        _ => {}
    }
}

#[derive(Debug, Default)]
struct LoopContext {
    loop_start: usize,
    break_jumps: Vec<usize>,
}

#[derive(Debug, Default)]
struct CompilerState {
    instructions: Vec<Instruction>,
    function_arities: HashMap<usize, usize>,
    functions: HashMap<String, FunctionMeta>,
    pending_calls: Vec<PendingCall>,
    pending_spawn_calls: Vec<PendingCall>,  // v2.2: SpawnCallDirect forward refs
    function_counter: usize,
    /// Track top-level variable names so function bodies can reference them
    /// via named Store/Load (outer_vars in CompileContext).
    known_top_level_vars: std::collections::HashSet<String>,
    /// Class metadata: class_name → (parent, [(method_name, params)])
    /// Used to populate Program.classes and method_bytecode.
    class_defs: HashMap<String, (Option<String>, Vec<(String, Vec<String>)>)>,
    /// Method bytecode map: (class_name, method_name) → bytecode_start index.
    method_bytecode: HashMap<(String, String), usize>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct FunctionMeta {
    start: usize,
    arity: usize,
}

#[derive(Debug, Clone)]
struct PendingCall {
    instruction_index: usize,
    function_name: String,
    arg_count: usize,
    line_no: usize,
}

#[derive(Debug, Default)]
struct CompileContext {
    loop_stack: Vec<LoopContext>,
    in_function: bool,
    /// Set while compiling a function body; enables self tail-call (`TailCall`) lowering.
    current_function_name: Option<String>,
    params: HashMap<String, usize>,
    // Compile-time slot table for fast variable access.
    // Maps variable name → integer slot index (u16) for the current function scope.
    // Only plain local variables use slots; special names (arg0, __temp_, etc.) use named Store/Load.
    slot_map: HashMap<String, u16>,
    next_slot: u16,
    /// Names of variables from an enclosing scope (e.g. top-level vars visible to functions).
    /// Assignment to these emits `Store(name)` so the VM updates the outer named scope.
    outer_vars: std::collections::HashSet<String>,
}

pub fn compile_killer_subset(source: &str) -> Result<Program, VmError> {
    let mut state = CompilerState::default();

    // Phase 0: Convert indentation-based syntax → brace-delimited syntax,
    // and strip `--` / `//` comments.  Must run before preprocess_polyglot so that
    // subsequent stages always see `{}`-delimited blocks.
    let indented = preprocess_indentation(source);

    // Pre-process @lang{} blocks BEFORE normalize_lines() splits braces.
    // This preserves embedded language code (Go, Rust, etc.) that uses { }.
    let preprocessed = preprocess_polyglot(&indented);
    let preprocessed = preprocess_ui_sugar(&preprocessed);
    let lines = normalize_lines(&preprocessed);
    let mut cursor = 0usize;
    let mut context = CompileContext::default();
    compile_block(&lines, &mut cursor, &mut state, &mut context, false)?;

    // Auto-invoke `main()` if it was defined and nothing in the top-level code
    // already called it explicitly.  This matches the common pattern of Killer
    // programs that define kfn main() without an explicit call at the bottom.
    if let Some(meta) = state.functions.get("main").cloned() {
        if meta.arity == 0 {
            state.instructions.push(Instruction::Call {
                target: meta.start,
                arg_count: 0,
            });
        }
    }

    patch_pending_calls(&mut state)?;
    state.instructions.push(Instruction::Halt);

    // OPTIMIZATION: Apply bytecode optimization pass.
    // Get the old→new index map so we can remap function_arities keys and
    // Call targets (the optimizer patches Jump/JumpIfFalse/Call automatically,
    // but function_arities is a separate HashMap keyed by old fn-start indices).
    let (optimized, old_to_new) = crate::optimizer::optimize_bytecode_with_map(&state.instructions);
    let n_old = state.instructions.len();

    // Remap function_arities: keys are old fn-start IPs, must become new IPs.
    let function_arities = state.function_arities
        .into_iter()
        .map(|(old_ip, arity)| (old_to_new[old_ip.min(n_old)], arity))
        .collect();

    // Remap method_bytecode indices through the optimization map too
    let method_bytecode = state.method_bytecode
        .into_iter()
        .map(|(key, old_ip)| (key, old_to_new[old_ip.min(n_old)]))
        .collect();

    // Convert class_defs to Program.classes format:
    // class_name → (parent, Vec<(method_name, params, body_stmts)>)
    // We use empty body_stmts since methods are compiled to bytecode already.
    let classes: HashMap<String, (Option<String>, Vec<(String, Vec<String>, Vec<crate::ast::Stmt>)>)> =
        state.class_defs.into_iter().map(|(name, (parent, methods))| {
            let method_list = methods.into_iter().map(|(mname, params)| {
                (mname, params, Vec::new()) // empty AST body — bytecode is in method_bytecode
            }).collect();
            (name, (parent, method_list))
        }).collect();

    Ok(Program {
        instructions: optimized,
        function_arities,
        function_names: std::collections::HashMap::new(),
        method_bytecode,
        classes,
    })
}

/// Compile AST statements to bytecode
pub fn compile_statements(statements: &[crate::ast::Stmt]) -> Result<Program, VmError> {
    let mut state = CompilerState::default();
    let mut context = CompileContext::default();
    let mut classes = std::collections::HashMap::new();

    // First pass: collect all function definitions and class definitions
    for stmt in statements {
        if let crate::ast::Stmt::Function { name, params, body: _, ai_annotations: _ } = stmt {
            let func_arity = params.len();
            // Just record function exists - don't compile yet
            state.functions.insert(name.clone(), FunctionMeta {
                start: 0, // Will be patched in second pass
                arity: func_arity,
            });
        } else if let crate::ast::Stmt::Class { name, extends, methods } = stmt {
            // Collect class definition
            classes.insert(name.clone(), (extends.clone(), methods.clone()));
        }
    }

    // Second pass: compile main code, skipping function definitions
    for stmt in statements {
        match stmt {
            crate::ast::Stmt::Function { .. } => {
                // Skip function definitions in main code
                // They'll be compiled in third pass
            }
            _ => {
                compile_stmt(stmt, &mut state, &mut context)?;
            }
        }
    }

    // Prevent execution from falling through into appended function/method bytecode.
    state.instructions.push(Instruction::Halt);

    // Third pass: compile function bodies at the end
    for stmt in statements {
        if let crate::ast::Stmt::Function { name, params, body, ai_annotations: _ } = stmt {
            let func_start = state.instructions.len();
            let func_arity = params.len();

            // Update function metadata with actual start position
            state.functions.insert(name.clone(), FunctionMeta {
                start: func_start,
                arity: func_arity,
            });
            state.function_arities.insert(func_start, func_arity);

            context.current_function_name = Some(name.clone());

            // Create a new parameter context for this function
            let saved_params = context.params.clone();
            let saved_slot_map = context.slot_map.clone();
            let saved_next_slot = context.next_slot;
            context.params.clear();
            context.slot_map.clear();
            context.next_slot = 0;

            // Register parameters with their index
            for (idx, param) in params.iter().enumerate() {
                context.params.insert(param.clone(), idx);
            }

            // Compile function body
            for (i, stmt_body) in body.iter().enumerate() {
                let is_last = i + 1 == body.len();
                if is_last {
                    compile_tail_stmt_as_return(stmt_body, &mut state, &mut context)?;
                    continue;
                }
                compile_stmt(stmt_body, &mut state, &mut context)?;
            }

            // Add implicit return if needed
            if !matches!(state.instructions.last(), Some(Instruction::Ret)) {
                state.instructions.push(Instruction::ConstNum(0.0));
                state.instructions.push(Instruction::Ret);
            }

            // Restore parameter context and slot map
            context.params = saved_params;
            context.slot_map = saved_slot_map;
            context.next_slot = saved_next_slot;
            context.current_function_name = None;
        }
    }

    // Patch all pending function calls with correct targets
    patch_pending_calls(&mut state)?;
    
    // Calculate where methods will start
    let _method_start_index = state.instructions.len();
    let mut method_bytecode_map: HashMap<(String, String), usize> = HashMap::new();

    // If there are methods, emit JUMP to skip over them
    let skip_methods_index = if !classes.iter().any(|(_, (_, methods))| !methods.is_empty()) {
        None
    } else {
        Some(state.instructions.len())
    };
    
    if skip_methods_index.is_some() {
        state.instructions.push(Instruction::Jump(0)); // Placeholder, will be patched
    }

    // Fourth pass: compile method bodies
    for (class_name, (_, methods)) in &classes {
        for (method_name, params, method_body) in methods {
            let method_start = state.instructions.len();
            method_bytecode_map.insert((class_name.clone(), method_name.clone()), method_start);
            
            // Create a new parameter context for this method
            let saved_params = context.params.clone();
            let saved_slot_map = context.slot_map.clone();
            let saved_next_slot = context.next_slot;
            context.params.clear();
            context.slot_map.clear();
            context.next_slot = 0;
            
            // Register parameters with their index
            for (idx, param) in params.iter().enumerate() {
                context.params.insert(param.clone(), idx);
            }
            
            // Compile method body
            for (i, stmt_method) in method_body.iter().enumerate() {
                let is_last = i + 1 == method_body.len();
                if is_last {
                    compile_tail_stmt_as_return(stmt_method, &mut state, &mut context)?;
                    continue;
                }
                compile_stmt(stmt_method, &mut state, &mut context)?;
            }
            
            // Add implicit return if needed
            if !matches!(state.instructions.last(), Some(Instruction::Ret)) {
                state.instructions.push(Instruction::ConstNum(0.0));
                state.instructions.push(Instruction::Ret);
            }
            
            // Restore parameter context and slot map
            context.params = saved_params;
            context.slot_map = saved_slot_map;
            context.next_slot = saved_next_slot;
        }
    }
    
    // Patch the JUMP to skip methods to point after all methods
    if let Some(idx) = skip_methods_index {
        let target_ip = state.instructions.len();
        if let Instruction::Jump(ref mut target) = &mut state.instructions[idx] {
            *target = target_ip;
        }
    }

    state.instructions.push(Instruction::Halt);
    
    let mut function_names = std::collections::HashMap::new();
    for (name, func_meta) in &state.functions {
        function_names.insert(func_meta.start, name.clone());
    }
    
    Ok(Program {
        instructions: state.instructions,
        function_arities: state.function_arities,
        function_names,
        method_bytecode: method_bytecode_map,
        classes,
    })
}

/// Default Killer pipeline: **line-oriented** compiler ([`compile_killer_subset`]).
/// Used by `run_killer_source`, package import, and MCP. Supports `kfn`, control flow, and many
/// expression forms; the returned [`Program::classes`] map is **always empty** on this path.
#[inline]
pub fn compile_killer_default(source: &str) -> Result<Program, VmError> {
    compile_killer_subset(source)
}

/// **Full AST** pipeline: compile [`crate::ast::Stmt`] trees (built programmatically or from a
/// future token parser). Produces bytecode with **classes**, **methods**, and **function** layout
/// via [`compile_statements`].
pub fn compile_killer_ast(statements: &[crate::ast::Stmt]) -> Result<Program, VmError> {
    compile_statements(statements)
}

/// Indent + polyglot preprocessing (same order as [`compile_killer_subset`]). Exposed for
/// [`crate::stmt_parser::parse_killer_program`].
pub fn preprocess_killer_source(source: &str) -> String {
    let indented = preprocess_indentation(source);
    let preprocessed = preprocess_polyglot(&indented);
    preprocess_ui_sugar(&preprocessed)
}

/// If `callee` is the function currently being compiled, emit [`Instruction::TailCall`] instead of `Call`+`Ret`.
fn try_compile_self_tail_call(
    callee: &str,
    args: &[crate::ast::Expr],
    state: &mut CompilerState,
    context: &mut CompileContext,
) -> Result<bool, VmError> {
    if context.current_function_name.as_deref() != Some(callee) {
        return Ok(false);
    }
    let target_ip = {
        let Some(meta) = state.functions.get(callee) else {
            return Ok(false);
        };
        meta.start
    };
    for arg in args {
        compile_expr(arg, state, context)?;
    }
    state.instructions.push(Instruction::TailCall {
        target: target_ip,
        arg_count: args.len(),
    });
    Ok(true)
}

fn compile_tail_stmt_as_return(
    stmt: &crate::ast::Stmt,
    state: &mut CompilerState,
    context: &mut CompileContext,
) -> Result<(), VmError> {
    use crate::ast::Stmt;

    match stmt {
        Stmt::Return(None) => {
            state.instructions.push(Instruction::ConstNum(0.0));
            state.instructions.push(Instruction::Ret);
        }
        Stmt::Return(Some(expr)) => {
            if let crate::ast::Expr::Call { callee, args } = expr.as_ref() {
                if try_compile_self_tail_call(callee, args, state, context)? {
                    return Ok(());
                }
            }
            compile_expr(expr, state, context)?;
            state.instructions.push(Instruction::Ret);
        }
        Stmt::Expr(expr) => {
            if let crate::ast::Expr::Call { callee, args } = expr {
                if try_compile_self_tail_call(callee, args, state, context)? {
                    return Ok(());
                }
            }
            compile_expr(expr, state, context)?;
            state.instructions.push(Instruction::Ret);
        }
        Stmt::If {
            condition,
            then_branch,
            else_branch,
        } => {
            compile_expr(condition, state, context)?;
            let jump_false_idx = state.instructions.len();
            state.instructions.push(Instruction::JumpIfFalse(usize::MAX));

            compile_branch_tail_as_return(then_branch, state, context)?;

            let jump_end_idx = state.instructions.len();
            state.instructions.push(Instruction::Jump(usize::MAX));
            let else_target = state.instructions.len();
            state.instructions[jump_false_idx] = Instruction::JumpIfFalse(else_target);

            if else_branch.is_empty() {
                state.instructions.push(Instruction::ConstNum(0.0));
                state.instructions.push(Instruction::Ret);
            } else {
                compile_branch_tail_as_return(else_branch, state, context)?;
            }

            let end_target = state.instructions.len();
            state.instructions[jump_end_idx] = Instruction::Jump(end_target);
        }
        _ => {
            compile_stmt(stmt, state, context)?;
            if !matches!(state.instructions.last(), Some(Instruction::Ret)) {
                state.instructions.push(Instruction::ConstNum(0.0));
                state.instructions.push(Instruction::Ret);
            }
        }
    }

    Ok(())
}

fn compile_branch_tail_as_return(
    branch: &[crate::ast::Stmt],
    state: &mut CompilerState,
    context: &mut CompileContext,
) -> Result<(), VmError> {
    if branch.is_empty() {
        state.instructions.push(Instruction::ConstNum(0.0));
        state.instructions.push(Instruction::Ret);
        return Ok(());
    }

    for (i, stmt) in branch.iter().enumerate() {
        let is_last = i + 1 == branch.len();
        if is_last {
            compile_tail_stmt_as_return(stmt, state, context)?;
        } else {
            compile_stmt(stmt, state, context)?;
        }
    }

    Ok(())
}

// Compile pattern-based assignment/destructuring
fn compile_pattern_assignment(
    pattern: &crate::ast::Pattern,
    state: &mut CompilerState,
    context: &mut CompileContext,
    temp_var: Option<&str>,  // Temporary variable holding the value to destructure
) -> Result<(), VmError> {
    use crate::ast::Pattern;
    
    match pattern {
        Pattern::Identifier(name) => {
            // Simple assignment: value is already on stack
            if let Some(idx) = context.params.get(name) {
                state.instructions.push(Instruction::Store(format!("arg{}", idx)));
            } else if name.starts_with("__") {
                // Internal temp variable — keep as named store
                state.instructions.push(Instruction::Store(name.clone()));
            } else {
                // Assign slot on first encounter, reuse on subsequent
                let slot = if let Some(&s) = context.slot_map.get(name) {
                    s
                } else {
                    let s = context.next_slot;
                    context.slot_map.insert(name.clone(), s);
                    context.next_slot = s.saturating_add(1);
                    s
                };
                state.instructions.push(Instruction::StoreSlot(slot));
            }
        }
        Pattern::Array(patterns) => {
            // Destructuring: [a, b, c] = value
            // If temp_var is None, store value in a temp first
            let temp = if let Some(t) = temp_var {
                t.to_string()
            } else {
                let t = format!("__destruct_temp_{}", state.instructions.len());
                state.instructions.push(Instruction::Store(t.clone()));
                t
            };
            
            for (i, elem_pattern) in patterns.iter().enumerate() {
                // Load the array from temp
                state.instructions.push(Instruction::Load(temp.clone()));
                // Push the index
                state.instructions.push(Instruction::ConstNum(i as f64));
                // Get element via IndexRead
                state.instructions.push(Instruction::IndexRead);
                // Recursively assign to the element pattern
                compile_pattern_assignment(elem_pattern, state, context, None)?;
            }
        }
        Pattern::Object(pairs) => {
            // Destructuring: {x, y, z} = value
            // If temp_var is None, store value in a temp first
            let temp = if let Some(t) = temp_var {
                t.to_string()
            } else {
                let t = format!("__destruct_temp_{}", state.instructions.len());
                state.instructions.push(Instruction::Store(t.clone()));
                t
            };
            
            for (key, elem_pattern) in pairs {
                // Load the object from temp
                state.instructions.push(Instruction::Load(temp.clone()));
                // Push the key
                state.instructions.push(Instruction::ConstStr(key.clone()));
                // Get field via IndexRead
                state.instructions.push(Instruction::IndexRead);
                // Recursively assign to the element pattern
                compile_pattern_assignment(elem_pattern, state, context, None)?;
            }
        }
    }
    
    Ok(())
}

fn compile_stmt(
    stmt: &crate::ast::Stmt,
    state: &mut CompilerState,
    context: &mut CompileContext,
) -> Result<(), VmError> {
    use crate::ast::Stmt;

    match stmt {
        Stmt::Let { pattern, value } => {
            compile_expr(value, state, context)?;
            compile_pattern_assignment(pattern, state, context, None)?;
        }
        Stmt::Quality { pattern, value } => {
            compile_expr(value, state, context)?;
            // Emit NewQuality instruction to wrap value in DataQuality
            state.instructions.push(Instruction::NewQuality);
            compile_pattern_assignment(pattern, state, context, None)?;
        }
        Stmt::Assign { pattern, value } => {
            compile_expr(value, state, context)?;
            compile_pattern_assignment(pattern, state, context, None)?;
        }
        Stmt::IndexAssign { object, index, value } => {
            compile_expr(index, state, context)?;
            compile_expr(value, state, context)?;
            state.instructions.push(Instruction::IndexWrite(object.clone()));
        }
        Stmt::Print(args) => {
            // Compile all arguments first
            for arg in args {
                compile_expr(arg, state, context)?;
            }
            // Then emit print with argument count
            state.instructions.push(Instruction::PrintMultiple(args.len()));
        }
        Stmt::Expr(expr) => {
            // Special handling for mutating array methods on identifiers
            // e.g., arr.push(x) should update arr, not just return a value
            if let crate::ast::Expr::MethodCall { object, method, args } = expr {
                if let crate::ast::Expr::Identifier(obj_name) = &**object {
                    // Check if this is a mutating array method
                    if MUTATING_ARRAY_METHODS.contains(&method.as_str()) {
                        // Compile as assignment: obj = obj.method(args)
                        // Load object
                        compile_expr(object, state, context)?;
                        // Load arguments
                        for arg in args {
                            compile_expr(arg, state, context)?;
                        }
                        // Call method
                        state.instructions.push(Instruction::CallMethodDynamic {
                            method_name: method.clone(),
                            arg_count: args.len(),
                        });
                        // Store result back to the object variable
                        state.instructions.push(Instruction::Store(obj_name.clone()));
                        return Ok(());
                    }
                }
            }
            // Default behavior for non-mutating expressions
            compile_expr(expr, state, context)?;
            state.instructions.push(Instruction::Pop);
        }
        Stmt::If { condition, then_branch, else_branch } => {
            compile_expr(condition, state, context)?;
            let jump_false_idx = state.instructions.len();
            state.instructions.push(Instruction::JumpIfFalse(usize::MAX));

            for stmt in then_branch {
                compile_stmt(stmt, state, context)?;
            }

            if !else_branch.is_empty() {
                let jump_end_idx = state.instructions.len();
                state.instructions.push(Instruction::Jump(usize::MAX));
                let else_target = state.instructions.len();
                state.instructions[jump_false_idx] = Instruction::JumpIfFalse(else_target);

                for stmt in else_branch {
                    compile_stmt(stmt, state, context)?;
                }
                let end_target = state.instructions.len();
                state.instructions[jump_end_idx] = Instruction::Jump(end_target);
            } else {
                let end_target = state.instructions.len();
                state.instructions[jump_false_idx] = Instruction::JumpIfFalse(end_target);
            }
        }
        Stmt::While { condition, body } => {
            let loop_start = state.instructions.len();

            compile_expr(condition, state, context)?;
            let jump_false_idx = state.instructions.len();
            state.instructions.push(Instruction::JumpIfFalse(usize::MAX));

            context.loop_stack.push(LoopContext {
                loop_start,
                break_jumps: Vec::new(),
            });

            for stmt in body {
                compile_stmt(stmt, state, context)?;
            }

            state.instructions.push(Instruction::Jump(loop_start));
            let loop_end = state.instructions.len();
            state.instructions[jump_false_idx] = Instruction::JumpIfFalse(loop_end);

            if let Some(loop_ctx) = context.loop_stack.pop() {
                for break_idx in loop_ctx.break_jumps {
                    state.instructions[break_idx] = Instruction::Jump(loop_end);
                }
            }
        }
        Stmt::DoWhile { body, condition } => {
            let loop_start = state.instructions.len();

            context.loop_stack.push(LoopContext {
                loop_start,
                break_jumps: Vec::new(),
            });

            for stmt in body {
                compile_stmt(stmt, state, context)?;
            }

            compile_expr(condition, state, context)?;
            let jump_end_idx = state.instructions.len();
            state.instructions.push(Instruction::JumpIfFalse(usize::MAX));
            state.instructions.push(Instruction::Jump(loop_start));

            let loop_end = state.instructions.len();
            state.instructions[jump_end_idx] = Instruction::JumpIfFalse(loop_end);

            if let Some(loop_ctx) = context.loop_stack.pop() {
                for break_idx in loop_ctx.break_jumps {
                    state.instructions[break_idx] = Instruction::Jump(loop_end);
                }
            }
        }
        Stmt::For {
            variable,
            iterable,
            is_for_of,
            body,
        } => {
            // For now, compile for-in to a while loop over array indices
            // for (x in arr) { ... } becomes:
            //   __arr = iterable
            //   __idx = 0
            //   while (__idx < len(__arr)) { x = __arr[__idx]; ... ; __idx = __idx + 1 }
            let iter_var = format!("__iter_{}", state.instructions.len());
            let idx_var = format!("__idx_{}", state.instructions.len());
            let saved_params = context.params.clone();
            
            // Compile iterable expression (should push array/range onto stack)
            compile_expr(iterable, state, context)?;

            // for...in iterates keys/indexes, for...of iterates values.
            if !*is_for_of {
                state
                    .instructions
                    .push(Instruction::CallBuiltin("iterKeys".to_string(), 1));
            }
            
            // Store the array in a variable
            state.instructions.push(Instruction::Store(iter_var.clone()));
            
            // Initialize loop index to 0
            state.instructions.push(Instruction::ConstNum(0.0));
            state.instructions.push(Instruction::Store(idx_var.clone()));
            
            let loop_start = state.instructions.len();
            
            // Load current index
            state.instructions.push(Instruction::Load(idx_var.clone()));
            // Load the array to compare against
            state.instructions.push(Instruction::Load(iter_var.clone()));
            // Get length of array
            state.instructions.push(Instruction::CallBuiltin("len".to_string(), 1));
            // Compare: __idx < len(__iter)
            state.instructions.push(Instruction::Lt);
            
            let jump_false_idx = state.instructions.len();
            state.instructions.push(Instruction::JumpIfFalse(usize::MAX));
            
            context.loop_stack.push(LoopContext {
                loop_start,
                break_jumps: Vec::new(),
            });
            
            // Get current element: x = __iter[__idx]
            state.instructions.push(Instruction::Load(iter_var.clone()));
            state.instructions.push(Instruction::Load(idx_var.clone()));
            state.instructions.push(Instruction::IndexRead);
            state.instructions.push(Instruction::Store(variable.clone()));
            
            // Execute body
            for stmt in body {
                compile_stmt(stmt, state, context)?;
            }
            
            // Increment index: __idx = __idx + 1
            state.instructions.push(Instruction::Load(idx_var.clone()));
            state.instructions.push(Instruction::ConstNum(1.0));
            state.instructions.push(Instruction::Add);
            state.instructions.push(Instruction::Store(idx_var.clone()));
            
            // Jump back to loop start
            state.instructions.push(Instruction::Jump(loop_start));
            let loop_end = state.instructions.len();
            state.instructions[jump_false_idx] = Instruction::JumpIfFalse(loop_end);
            
            // Patch break jumps
            if let Some(loop_ctx) = context.loop_stack.pop() {
                for break_idx in loop_ctx.break_jumps {
                    state.instructions[break_idx] = Instruction::Jump(loop_end);
                }
            }
            
            context.params = saved_params;
        }
        Stmt::ForC { init, condition, update, body } => {
            // C-style for (init; condition; update) { body }
            // Compiles to:
            //   init
            //   loop_start: if (!condition) goto loop_end
            //   body
            //   update
            //   goto loop_start
            //   loop_end:
            
            let saved_params = context.params.clone();
            
            // Compile init if present
            if let Some(init_stmt) = init {
                compile_stmt(init_stmt, state, context)?;
            }
            
            let loop_start = state.instructions.len();
            
            // Compile condition check and jump if false
            let jump_false_idx = if let Some(cond_expr) = condition {
                compile_expr(cond_expr, state, context)?;
                let idx = state.instructions.len();
                state.instructions.push(Instruction::JumpIfFalse(usize::MAX));
                idx
            } else {
                usize::MAX // No condition, always execute (infinite loop unless break)
            };
            
            context.loop_stack.push(LoopContext {
                loop_start,
                break_jumps: Vec::new(),
            });
            
            // Compile body
            for stmt in body {
                compile_stmt(stmt, state, context)?;
            }
            
            // Compile update expression if present
            if let Some(upd_expr) = update {
                compile_expr(upd_expr, state, context)?;
                // Pop the result off stack since it's just for side effects
                state.instructions.push(Instruction::Pop);
            }
            
            // Jump back to condition check
            state.instructions.push(Instruction::Jump(loop_start));
            let loop_end = state.instructions.len();
            
            // Patch the condition jump and break jumps
            if jump_false_idx != usize::MAX {
                state.instructions[jump_false_idx] = Instruction::JumpIfFalse(loop_end);
            }
            
            if let Some(loop_ctx) = context.loop_stack.pop() {
                for break_idx in loop_ctx.break_jumps {
                    state.instructions[break_idx] = Instruction::Jump(loop_end);
                }
            }
            
            context.params = saved_params;
        }
        Stmt::Function { name: _, params, body: _, ai_annotations: _ } => {
            // Don't compile function body inline - just store metadata
            // Functions will be compiled in the second pass
            let _func_arity = params.len();
            
            // Just record that we've seen a function definition
            // The actual compilation happens later via a two-pass system
            // For now, we'll defer and handle in patch phase
        }
        Stmt::Return(expr_opt) => {
            if let Some(expr) = expr_opt {
                compile_expr(expr, state, context)?;
            }
            state.instructions.push(Instruction::Ret);
        }
        Stmt::Break => {
            if let Some(loop_ctx) = context.loop_stack.last_mut() {
                let break_idx = state.instructions.len();
                loop_ctx.break_jumps.push(break_idx);
                state.instructions.push(Instruction::Jump(usize::MAX));
            } else {
                return Err(VmError::parse_error_simple(
                    "break statement outside of loop".to_string(),
                ));
            }
        }
        Stmt::Continue => {
            if let Some(loop_ctx) = context.loop_stack.last() {
                state.instructions.push(Instruction::Jump(loop_ctx.loop_start));
            } else {
                return Err(VmError::parse_error_simple(
                    "continue statement outside of loop".to_string(),
                ));
            }
        }
        Stmt::Try { try_body, catch_var, catch_body, finally_body } => {
            // Placeholder indices for jumps
            let try_enter_idx = state.instructions.len();
            state.instructions.push(Instruction::TryEnter { catch_target: usize::MAX, finally_target: usize::MAX });
            
            // Compile try body
            for stmt in try_body {
                compile_stmt(stmt, state, context)?;
            }
            
            // Push exit instruction
            state.instructions.push(Instruction::TryExit);
            
            // Jump to skip catch and go to finally/end (if no exception)
            let skip_catch_idx = state.instructions.len();
            state.instructions.push(Instruction::Jump(usize::MAX));
            
            // Store the target addresses for catch and finally
            let catch_target = state.instructions.len();
            let finally_target = if finally_body.is_empty() {
                usize::MAX
            } else {
                0  // Will be set after catch
            };
            
            // Fix try_enter instruction
            if let Instruction::TryEnter { catch_target: ref mut ct, finally_target: ref mut ft } = &mut state.instructions[try_enter_idx] {
                *ct = catch_target;
                *ft = finally_target;
            }
            
            // Compile catch body if present
            if !catch_body.is_empty() {
                state.instructions.push(Instruction::CatchEnter { var_name: Some(catch_var.clone()) });
                for stmt in catch_body {
                    compile_stmt(stmt, state, context)?;
                }
                state.instructions.push(Instruction::TryExit);
            }
            
            // Now set the jump target to after catch block
            let jump_target = state.instructions.len();
            if let Instruction::Jump(ref mut target) = &mut state.instructions[skip_catch_idx] {
                *target = jump_target;
            }
            
            // Compile finally body if present
            if !finally_body.is_empty() {
                let finally_idx = state.instructions.len();
                state.instructions.push(Instruction::FinallyEnter);
                for stmt in finally_body {
                    compile_stmt(stmt, state, context)?;
                }
                state.instructions.push(Instruction::TryExit);
                
                // Fix finally target in try_enter
                if let Instruction::TryEnter { catch_target: _, finally_target: ref mut ft } = &mut state.instructions[try_enter_idx] {
                    *ft = finally_idx;
                }
            }
        }
        Stmt::Throw(expr) => {
            compile_expr(expr, state, context)?;
            state.instructions.push(Instruction::Throw);
        }
        Stmt::Yield(expr) => {
            compile_expr(expr, state, context)?;
            state.instructions.push(Instruction::Yield);
        }
        Stmt::Switch {
            expression,
            cases,
            default,
        } => {
            let mut jump_end_indices = Vec::new();
            let mut next_case_jumps = Vec::new();

            for (case_expr, case_body) in cases {
                compile_expr(expression, state, context)?;
                compile_expr(case_expr, state, context)?;
                state.instructions.push(Instruction::Eq);

                let jump_false_idx = state.instructions.len();
                state.instructions.push(Instruction::JumpIfFalse(usize::MAX));
                next_case_jumps.push(jump_false_idx);

                for stmt in case_body {
                    compile_stmt(stmt, state, context)?;
                }
                let jump_end_idx = state.instructions.len();
                state.instructions.push(Instruction::Jump(usize::MAX));
                jump_end_indices.push(jump_end_idx);

                let next_case_target = state.instructions.len();
                if let Instruction::JumpIfFalse(ref mut t) = state.instructions[jump_false_idx] {
                    *t = next_case_target;
                }
            }

            for stmt in default {
                compile_stmt(stmt, state, context)?;
            }

            let end_target = state.instructions.len();
            for idx in jump_end_indices {
                if let Instruction::Jump(ref mut t) = state.instructions[idx] {
                    *t = end_target;
                }
            }
            let _ = next_case_jumps;
        }
        // ── match expression (pattern-matching) ────────────────────────────
        Stmt::Match { expression, arms } => {
            // Store the scrutinee in a temp variable (slot)
            compile_expr(expression, state, context)?;
            let scrutinee_var = format!("__match_scrutinee_{}", state.instructions.len());
            state.instructions.push(Instruction::Store(scrutinee_var.clone()));

            let mut jump_end_indices = Vec::new();

            for arm in arms {
                // Emit pattern test: push bool onto stack
                compile_match_pattern_test(
                    &arm.pattern,
                    &scrutinee_var,
                    state,
                    context,
                )?;

                // If there's a guard, AND it with the pattern test result
                if let Some(guard) = &arm.guard {
                    let skip_guard = state.instructions.len();
                    state.instructions.push(Instruction::JumpIfFalse(usize::MAX));

                    compile_expr(guard, state, context)?;

                    let past_guard = state.instructions.len();
                    state.instructions.push(Instruction::Jump(usize::MAX)); // jump to bind
                    
                    // If pattern failed, push false and skip guard
                    let fail_target = state.instructions.len();
                    if let Instruction::JumpIfFalse(ref mut t) = state.instructions[skip_guard] {
                        *t = fail_target;
                    }
                    state.instructions.push(Instruction::ConstBool(false));

                    let bind_target = state.instructions.len();
                    if let Instruction::Jump(ref mut t) = state.instructions[past_guard] {
                        *t = bind_target;
                    }
                }

                let jump_false_idx = state.instructions.len();
                state.instructions.push(Instruction::JumpIfFalse(usize::MAX));

                // Pattern matched — bind variables from the pattern
                compile_match_pattern_bind(
                    &arm.pattern,
                    &scrutinee_var,
                    state,
                    context,
                )?;

                // Compile arm body
                for stmt in &arm.body {
                    compile_stmt(stmt, state, context)?;
                }

                let jump_end_idx = state.instructions.len();
                state.instructions.push(Instruction::Jump(usize::MAX));
                jump_end_indices.push(jump_end_idx);

                let next_arm = state.instructions.len();
                if let Instruction::JumpIfFalse(ref mut t) = state.instructions[jump_false_idx] {
                    *t = next_arm;
                }
            }

            let end_target = state.instructions.len();
            for idx in jump_end_indices {
                if let Instruction::Jump(ref mut t) = state.instructions[idx] {
                    *t = end_target;
                }
            }
        }
        Stmt::Class { name, extends, methods: _ } => {
            // Emit DefineClass instruction with parent info
            state.instructions.push(Instruction::DefineClass {
                name: name.clone(),
                parent: extends.clone(),
            });
            
            // TODO: Store method definitions - will be compiled separately
            // For now, just mark the class as defined
        }
        // v2.2: async/await/spawn/import -----------------------------------------
        Stmt::AsyncFunction { name, params, body } => {
            // Async function: compiled like a regular function.
            // The SpawnTask opcode wraps the call in an OS thread at call-site.
            compile_stmt(
                &crate::ast::Stmt::Function {
                    name: name.clone(),
                    params: params.clone(),
                    body: body.clone(),
                    ai_annotations: vec![],
                },
                state, context,
            )?;
        }
        Stmt::Spawn(expr) => {
            // Compile the inner expression (function ref or call).
            // For AST-level Spawn we still use SpawnTask (expression already evaluated).
            // True parallel SpawnCall is emitted by the text compiler's emit_spawn_call().
            compile_expr(expr, state, context)?;
            state.instructions.push(Instruction::SpawnTask);
            // pop the future — fire-and-forget spawn statements discard the handle
            state.instructions.push(Instruction::Pop);
        }
        Stmt::Import { path, .. } => {
            state.instructions.push(Instruction::ImportPkg(path.clone()));
        }
        Stmt::Export(_names) => {
            // Export declarations are handled at module boundary; no bytecode emitted.
        }
    }

    Ok(())
}

// ── Match-pattern helpers ──────────────────────────────────────────────────

/// Emit instructions that push `true`/`false` depending on whether `scrutinee`
/// matches `pattern`.  Does NOT bind variables.
fn compile_match_pattern_test(
    pattern: &crate::ast::MatchPattern,
    scrutinee: &str,
    state: &mut CompilerState,
    context: &mut CompileContext,
) -> Result<(), VmError> {
    use crate::ast::MatchPattern;
    match pattern {
        MatchPattern::Wildcard | MatchPattern::Identifier(_) => {
            // Always matches
            state.instructions.push(Instruction::ConstBool(true));
        }
        MatchPattern::Literal(expr) => {
            state.instructions.push(Instruction::Load(scrutinee.to_string()));
            compile_expr(expr, state, context)?;
            state.instructions.push(Instruction::Eq);
        }
        MatchPattern::Array(pats) => {
            // Check: is array AND len == pats.len(), then check each element
            state.instructions.push(Instruction::Load(scrutinee.to_string()));
            state.instructions.push(Instruction::CallBuiltin("len".to_string(), 1));
            state.instructions.push(Instruction::ConstNum(pats.len() as f64));
            state.instructions.push(Instruction::Eq);

            for (i, sub_pat) in pats.iter().enumerate() {
                if matches!(sub_pat, MatchPattern::Wildcard | MatchPattern::Identifier(_)) {
                    continue; // These always match
                }
                // AND with element test
                let temp = format!("__match_elem_{}_{}", state.instructions.len(), i);
                state.instructions.push(Instruction::Load(scrutinee.to_string()));
                state.instructions.push(Instruction::ConstNum(i as f64));
                state.instructions.push(Instruction::IndexRead);
                state.instructions.push(Instruction::Store(temp.clone()));
                compile_match_pattern_test(sub_pat, &temp, state, context)?;
                state.instructions.push(Instruction::And);
            }
        }
        MatchPattern::Object(pairs) => {
            // For object patterns: just push true (structural matching)
            // Each field-level sub-pattern is tested recursively
            state.instructions.push(Instruction::ConstBool(true));
            for (key, sub_pat) in pairs {
                if matches!(sub_pat, MatchPattern::Wildcard | MatchPattern::Identifier(_)) {
                    continue;
                }
                let temp = format!("__match_field_{}_{}", state.instructions.len(), key);
                state.instructions.push(Instruction::Load(scrutinee.to_string()));
                state.instructions.push(Instruction::ConstStr(key.clone()));
                state.instructions.push(Instruction::IndexRead);
                state.instructions.push(Instruction::Store(temp.clone()));
                compile_match_pattern_test(sub_pat, &temp, state, context)?;
                state.instructions.push(Instruction::And);
            }
        }
    }
    Ok(())
}

/// Emit instructions that bind variables from a matched pattern, assuming the
/// pattern already tested true.
fn compile_match_pattern_bind(
    pattern: &crate::ast::MatchPattern,
    scrutinee: &str,
    state: &mut CompilerState,
    context: &mut CompileContext,
) -> Result<(), VmError> {
    use crate::ast::MatchPattern;
    match pattern {
        MatchPattern::Wildcard | MatchPattern::Literal(_) => { /* no bindings */ }
        MatchPattern::Identifier(name) => {
            if name != "_" {
                state.instructions.push(Instruction::Load(scrutinee.to_string()));
                let slot = if let Some(&s) = context.slot_map.get(name) {
                    s
                } else {
                    let s = context.next_slot;
                    context.slot_map.insert(name.clone(), s);
                    context.next_slot = s.saturating_add(1);
                    s
                };
                state.instructions.push(Instruction::StoreSlot(slot));
            }
        }
        MatchPattern::Array(pats) => {
            for (i, sub_pat) in pats.iter().enumerate() {
                let temp = format!("__match_bind_{}_{}", state.instructions.len(), i);
                state.instructions.push(Instruction::Load(scrutinee.to_string()));
                state.instructions.push(Instruction::ConstNum(i as f64));
                state.instructions.push(Instruction::IndexRead);
                state.instructions.push(Instruction::Store(temp.clone()));
                compile_match_pattern_bind(sub_pat, &temp, state, context)?;
            }
        }
        MatchPattern::Object(pairs) => {
            for (key, sub_pat) in pairs {
                let temp = format!("__match_bind_{}_{}", state.instructions.len(), key);
                state.instructions.push(Instruction::Load(scrutinee.to_string()));
                state.instructions.push(Instruction::ConstStr(key.clone()));
                state.instructions.push(Instruction::IndexRead);
                state.instructions.push(Instruction::Store(temp.clone()));
                compile_match_pattern_bind(sub_pat, &temp, state, context)?;
            }
        }
    }
    Ok(())
}

fn compile_expr(
    expr: &crate::ast::Expr,
    state: &mut CompilerState,
    context: &mut CompileContext,
) -> Result<(), VmError> {
    use crate::ast::{BinaryOp, Expr};

    match expr {
        Expr::Number(n) => {
            state.instructions.push(Instruction::ConstNum(*n));
        }
        Expr::String(s) => {
            state.instructions.push(Instruction::ConstStr(s.clone()));
        }
        Expr::KString(s) => {
            if s.contains('{') {
                compile_kstring(s, 0, state, context)?;
            } else {
                state.instructions.push(Instruction::ConstStr(s.clone()));
            }
        }
        Expr::Bool(b) => {
            state.instructions.push(Instruction::ConstBool(*b));
        }
        Expr::Null => {
            state.instructions.push(Instruction::ConstNull);
        }
        Expr::Identifier(name) => {
            // Check if this is a function parameter - if so, reference it as arg{index}
            if let Some(idx) = context.params.get(name) {
                state.instructions.push(Instruction::Load(format!("arg{}", idx)));
            } else if let Some(&slot) = context.slot_map.get(name) {
                // Fast path: known local variable → integer-indexed slot access
                state.instructions.push(Instruction::LoadSlot(slot));
            } else {
                // Load regular variable (global, builtin, etc.)
                state.instructions.push(Instruction::Load(name.clone()));
            }
        }
        Expr::Binary { left, op, right } => {
            compile_expr(left, state, context)?;
            compile_expr(right, state, context)?;
            match op {
                BinaryOp::Add => state.instructions.push(Instruction::Add),
                BinaryOp::Sub => state.instructions.push(Instruction::Sub),
                BinaryOp::Mul => state.instructions.push(Instruction::Mul),
                BinaryOp::Div => state.instructions.push(Instruction::Div),
                BinaryOp::IntDiv => state.instructions.push(Instruction::IntDiv),
                BinaryOp::Mod => state.instructions.push(Instruction::Mod),
                BinaryOp::Pow => state.instructions.push(Instruction::CallBuiltin("pow".to_string(), 2)),
                BinaryOp::Eq => state.instructions.push(Instruction::Eq),
                BinaryOp::Ne => state.instructions.push(Instruction::Ne),
                BinaryOp::Gt => state.instructions.push(Instruction::Gt),
                BinaryOp::Ge => state.instructions.push(Instruction::Ge),
                BinaryOp::Lt => state.instructions.push(Instruction::Lt),
                BinaryOp::Le => state.instructions.push(Instruction::Le),
                BinaryOp::And => state.instructions.push(Instruction::And),
                BinaryOp::Or => state.instructions.push(Instruction::Or),
            }
        }
        Expr::Ternary {
            condition,
            then_expr,
            else_expr,
        } => {
            compile_expr(condition, state, context)?;
            let jump_false_idx = state.instructions.len();
            state.instructions.push(Instruction::JumpIfFalse(usize::MAX));

            compile_expr(then_expr, state, context)?;
            let jump_end_idx = state.instructions.len();
            state.instructions.push(Instruction::Jump(usize::MAX));

            let else_target = state.instructions.len();
            state.instructions[jump_false_idx] = Instruction::JumpIfFalse(else_target);
            compile_expr(else_expr, state, context)?;

            let end_target = state.instructions.len();
            state.instructions[jump_end_idx] = Instruction::Jump(end_target);
        }
        Expr::PrefixInc(name) => {
            let var = if let Some(idx) = context.params.get(name) {
                format!("arg{}", idx)
            } else {
                name.clone()
            };
            state.instructions.push(Instruction::Load(var.clone()));
            state.instructions.push(Instruction::ConstNum(1.0));
            state.instructions.push(Instruction::Add);
            state.instructions.push(Instruction::Store(var.clone()));
            state.instructions.push(Instruction::Load(var));
        }
        Expr::PrefixDec(name) => {
            let var = if let Some(idx) = context.params.get(name) {
                format!("arg{}", idx)
            } else {
                name.clone()
            };
            state.instructions.push(Instruction::Load(var.clone()));
            state.instructions.push(Instruction::ConstNum(1.0));
            state.instructions.push(Instruction::Sub);
            state.instructions.push(Instruction::Store(var.clone()));
            state.instructions.push(Instruction::Load(var));
        }
        Expr::PostfixInc(name) => {
            let var = if let Some(idx) = context.params.get(name) {
                format!("arg{}", idx)
            } else {
                name.clone()
            };
            state.instructions.push(Instruction::Load(var.clone()));
            state.instructions.push(Instruction::Load(var.clone()));
            state.instructions.push(Instruction::ConstNum(1.0));
            state.instructions.push(Instruction::Add);
            state.instructions.push(Instruction::Store(var));
        }
        Expr::PostfixDec(name) => {
            let var = if let Some(idx) = context.params.get(name) {
                format!("arg{}", idx)
            } else {
                name.clone()
            };
            state.instructions.push(Instruction::Load(var.clone()));
            state.instructions.push(Instruction::Load(var.clone()));
            state.instructions.push(Instruction::ConstNum(1.0));
            state.instructions.push(Instruction::Sub);
            state.instructions.push(Instruction::Store(var));
        }
        Expr::Call { callee, args } => {
            // Check if this is a known named function
            if state.functions.contains_key(callee) {
                // Known named function - compile as before with pending call
                for arg in args {
                    compile_expr(arg, state, context)?;
                }
                let call_idx = state.instructions.len();
                state.pending_calls.push(PendingCall {
                    instruction_index: call_idx,
                    function_name: callee.clone(),
                    arg_count: args.len(),
                    line_no: 0,
                });
                state.instructions.push(Instruction::Call {
                    target: usize::MAX,
                    arg_count: args.len(),
                });
            } else {
                // Unknown function - could be a variable containing a function
                // Load the variable (should be a function value)
                state.instructions.push(Instruction::Load(callee.clone()));
                
                // Compile all arguments
                for arg in args {
                    compile_expr(arg, state, context)?;
                }
                
                // Emit dynamic call instruction
                state.instructions.push(Instruction::CallDynamic {
                    arg_count: args.len(),
                });
            }
        }
        Expr::BuiltinCall { name, args } => {
            for arg in args {
                compile_expr(arg, state, context)?;
            }
            state.instructions.push(Instruction::CallBuiltin(
                name.clone(),
                args.len(),
            ));
        }
        Expr::CallExpr { callee, args } => {
            // Compile the callee expression (should evaluate to a Value::Function)
            compile_expr(callee, state, context)?;
            
            // Compile all arguments
            for arg in args {
                compile_expr(arg, state, context)?;
            }
            
            // Emit dynamic call instruction - pops function from stack and calls it
            state.instructions.push(Instruction::CallDynamic {
                arg_count: args.len(),
            });
        }
        Expr::MethodCall { object, method, args } => {
            // Compile object first
            compile_expr(object, state, context)?;
            // Then compile arguments
            for arg in args {
                compile_expr(arg, state, context)?;
            }
            // Emit dynamic method call instruction
            state.instructions.push(Instruction::CallMethodDynamic {
                method_name: method.clone(),
                arg_count: args.len(),
            });
        }
        Expr::Array(elements) => {
            // Check if any elements are spreads
            let has_spreads = elements.iter().any(|e| matches!(e, Expr::Spread(_)));
            
            if !has_spreads {
                // Simple case: no spreads, use regular BuildArray
                for elem in elements {
                    compile_expr(elem, state, context)?;
                }
                state.instructions.push(Instruction::BuildArray(elements.len()));
            } else {
                // Complex case: spreads present, need to accumulate and concatenate
                // Strategy: accumulate regular elements, concatenate when we hit spreads or end
                let mut accumulated = Vec::new();
                let mut is_first_value = true;
                
                for elem in elements {
                    match elem {
                        Expr::Spread(spread_expr) => {
                            // First, handle accumulated regular values if any
                            if !accumulated.is_empty() {
                                // Compile accumulated elements into an array
                                for e in &accumulated {
                                    compile_expr(e, state, context)?;
                                }
                                state.instructions.push(Instruction::BuildArray(accumulated.len()));
                                accumulated.clear();
                                
                                // Compile the spread expression
                                compile_expr(spread_expr, state, context)?;
                                
                                // Concat the accumulated array with the spread
                                state.instructions.push(Instruction::CallBuiltin("concat".to_string(), 2));
                                
                                // If we already had a prior result, concat again to merge them
                                if !is_first_value {
                                    state.instructions.push(Instruction::CallBuiltin("concat".to_string(), 2));
                                }
                                is_first_value = false;
                            } else if is_first_value {
                                // First element is a spread with no accumulated values
                                compile_expr(spread_expr, state, context)?;
                                is_first_value = false;
                            } else {
                                // We have a prior result on stack, just concat this spread
                                compile_expr(spread_expr, state, context)?;
                                state.instructions.push(Instruction::CallBuiltin("concat".to_string(), 2));
                            }
                        }
                        _ => {
                            // Regular element, accumulate it
                            accumulated.push(elem.clone());
                        }
                    }
                }
                
                // Handle final accumulated values
                if !accumulated.is_empty() {
                    for e in &accumulated {
                        compile_expr(e, state, context)?;
                    }
                    state.instructions.push(Instruction::BuildArray(accumulated.len()));
                    
                    if !is_first_value {
                        // We have a prior result on stack, concat with it
                        state.instructions.push(Instruction::CallBuiltin("concat".to_string(), 2));
                    }
                }
            }
        }
        Expr::Index { object, index } => {
            compile_expr(object, state, context)?;
            compile_expr(index, state, context)?;
            state.instructions.push(Instruction::IndexRead);
        }
        Expr::Dict(pairs) => {
            for (key, value) in pairs {
                compile_expr(key, state, context)?;
                compile_expr(value, state, context)?;
            }
            state.instructions.push(Instruction::BuildDict(pairs.len()));
        }
        Expr::Range { start, end, step } => {
            // Compile range as a call to range() built-in
            compile_expr(start, state, context)?;
            compile_expr(end, state, context)?;
            
            let arg_count = if let Some(step_expr) = step {
                compile_expr(step_expr, state, context)?;
                3
            } else {
                2
            };
            
            state.instructions.push(Instruction::CallBuiltin("range".to_string(), arg_count));
        }
        Expr::New { class_name, args } => {
            // Compile arguments first (for constructor)
            for arg in args {
                compile_expr(arg, state, context)?;
            }
            // Emit NewObject instruction
            state.instructions.push(Instruction::NewObject(class_name.clone()));
        }
        Expr::This => {
            // Load the special "this" variable
            state.instructions.push(Instruction::Load("this".to_string()));
        }
        Expr::FunctionExpr { params, body } => {
            // Compile anonymous function: fn(params) { body }
            // Store it as a function value with a unique ID
            let _fn_id = state.function_counter;
            state.function_counter += 1;
            
            // Find variables referenced in the function body that are NOT parameters
            let referenced_vars = collect_var_references(body);
            let captured_names: Vec<String> = referenced_vars
                .into_iter()
                .filter(|var| !params.contains(var))
                .collect();
            
            // Emit a Jump instruction to skip over the function body
            let skip_jump_idx = state.instructions.len();
            state.instructions.push(Instruction::Jump(usize::MAX));  // Will be patched
            
            // Record where function bytecode starts
            let fn_start_idx = state.instructions.len();
            state.function_arities.insert(fn_start_idx, params.len());
            
            // Setup function context
            let mut fn_context = CompileContext {
                loop_stack: Vec::new(),
                in_function: true,
                current_function_name: None,
                params: params.iter().enumerate().map(|(i, p)| (p.clone(), i)).collect(),
                slot_map: HashMap::new(),
                next_slot: 0,
                outer_vars: std::collections::HashSet::new(),
            };
            
            // Compile function body
            for stmt in body {
                compile_stmt(stmt, state, &mut fn_context)?;
            }
            
            // Add implicit return null if no return statement
            if !matches!(state.instructions.last(), Some(Instruction::Ret)) {
                state.instructions.push(Instruction::ConstNull);
                state.instructions.push(Instruction::Ret);
            }
            
            // Patch the skip jump to jump past the function body
            let after_fn_idx = state.instructions.len();
            state.instructions[skip_jump_idx] = Instruction::Jump(after_fn_idx);
            
            // Now emit the ConstFunc instruction to push function value onto stack
            state.instructions.push(Instruction::ConstFunc {
                params: params.clone(),
                bytecode_start: fn_start_idx,
                captured_names,
            });
        }
        Expr::Spread(_) => {
            // Spread should only appear in array, dict, or function call context
            // It should be handled at those higher levels, not as a standalone expression
            return Err(VmError::parse_error_simple(
                "Spread operator can only be used in array, object, or function call literals".to_string(),
            ));
        }
        Expr::Await(inner) => {
            compile_expr(inner, state, context)?;
            state.instructions.push(Instruction::AwaitTask);
        }
        Expr::Assign { name, value } => {
            // Assignment expression: evaluate value, store in variable, and leave value on stack
            compile_expr(value, state, context)?;
            state.instructions.push(Instruction::Store(name.clone()));
            // Load the value again so it's available as expression result
            state.instructions.push(Instruction::Load(name.clone()));
        }
    }

    Ok(())
}


/// Net `[` depth outside of `"` string literals (handles `\"` while inside a string).
fn square_bracket_depth(line: &str) -> i32 {
    let mut depth = 0i32;
    let mut in_string = false;
    let mut chars = line.chars().peekable();
    while let Some(ch) = chars.next() {
        if in_string {
            if ch == '\\' {
                chars.next();
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }
        match ch {
            '"' => in_string = true,
            '[' => depth += 1,
            ']' => depth -= 1,
            _ => {}
        }
    }
    depth
}

/// Net `(` depth outside `"` string literals (handles `\"` while inside a string).
fn paren_depth_net(line: &str) -> i32 {
    let mut depth = 0i32;
    let mut in_string = false;
    let mut chars = line.chars().peekable();
    while let Some(ch) = chars.next() {
        if in_string {
            if ch == '\\' {
                chars.next();
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }
        match ch {
            '"' => in_string = true,
            '(' => depth += 1,
            ')' => depth -= 1,
            _ => {}
        }
    }
    depth
}

/// Merge continuation lines until `(` `)` and `[` `]` are both balanced (function signatures
/// split across lines, including generic `[T]` before `(`).
fn merge_multiline_signature_header(
    lines: &[(usize, String)],
    start: usize,
) -> Result<(String, usize), VmError> {
    let mut merged = lines[start].1.trim().to_string();
    let mut i = start;
    loop {
        let pd = paren_depth_net(&merged);
        let bd = square_bracket_depth(&merged);
        if pd == 0 && bd == 0 {
            return Ok((merged, i));
        }
        if pd < 0 {
            return Err(VmError::parse_error_simple(format!(
                "Line {}: unmatched `)` in signature",
                lines[start].0
            )));
        }
        if bd < 0 {
            return Err(VmError::parse_error_simple(format!(
                "Line {}: unmatched `]` in signature",
                lines[start].0
            )));
        }
        i += 1;
        if i >= lines.len() {
            return Err(VmError::parse_error_simple(format!(
                "Line {}: unclosed `(` or `[` in function signature",
                lines[start].0
            )));
        }
        merged.push(' ');
        merged.push_str(lines[i].1.trim());
    }
}

/// True when a line plausibly continues an `if` / `while` / `for` header before `{`
/// (e.g. `&& b`, `|| c`, or a split expression).
fn is_likely_expr_continuation_line(raw: &str) -> bool {
    let t = raw.trim_start();
    if t.is_empty() {
        return false;
    }
    if t.starts_with("&&") || t.starts_with("||") {
        return true;
    }
    // Integer division split across lines (`a` then `// b`).
    if t.starts_with("//") {
        return true;
    }
    if t.starts_with("==")
        || t.starts_with("!=")
        || t.starts_with("<=")
        || t.starts_with(">=")
    {
        return true;
    }
    if t.starts_with('(') || t.starts_with('[') || t.starts_with(')') || t.starts_with(']') {
        return true;
    }
    if t.starts_with(',') || t.starts_with('.') {
        return true;
    }
    matches!(
        t.as_bytes()[0],
        b'+' | b'-' | b'*' | b'/' | b'%' | b'<' | b'>' | b'&' | b'|' | b'^'
    )
}

/// Merge physical lines for `if` / `while` / `for … in/of …` until `()` and `[]` balance
/// and the next normalized line is `{`. Allows multi-line conditions with or without parens.
fn merge_multiline_control_header_until_brace(
    lines: &[(usize, String)],
    start: usize,
) -> Result<(String, usize), VmError> {
    let line_no = lines[start].0;
    let mut merged = lines[start].1.trim().to_string();
    let mut i = start;
    loop {
        let pd = paren_depth_net(&merged);
        let bd = square_bracket_depth(&merged);
        if pd < 0 {
            return Err(VmError::parse_error_simple(format!(
                "Line {line_no}: unmatched `)` in condition",
            )));
        }
        if bd < 0 {
            return Err(VmError::parse_error_simple(format!(
                "Line {line_no}: unmatched `]` in condition",
            )));
        }
        if pd == 0 && bd == 0 {
            if i + 1 >= lines.len() {
                return Err(VmError::parse_error_simple(format!(
                    "Line {line_no}: condition must be followed by `{{`",
                )));
            }
            let next_trim = lines[i + 1].1.trim();
            if next_trim == "{" {
                return Ok((merged, i));
            }
            if is_likely_expr_continuation_line(&lines[i + 1].1) {
                i += 1;
                merged.push(' ');
                merged.push_str(lines[i].1.trim());
                continue;
            }
            let found = lines[i + 1].1.trim();
            let found: String = found.chars().take(48).collect();
            return Err(VmError::parse_error_simple(format!(
                "Line {line_no}: expected `{{` after condition (found `{found}`)",
            )));
        }
        i += 1;
        if i >= lines.len() {
            return Err(VmError::parse_error_simple(format!(
                "Line {line_no}: unclosed `(` or `[` in condition",
            )));
        }
        merged.push(' ');
        merged.push_str(lines[i].1.trim());
    }
}

/// When true, the offside preprocessor must not emit a synthetic `{` after this line: the
/// `if` / `while` / `for` header continues on following source lines.
fn control_flow_skip_synthetic_open_brace(
    content: &str,
    raw_lines: &[(usize, &str)],
    line_index: usize,
) -> bool {
    let t = content.trim();
    let is_if_like = t.starts_with("if ") || t.starts_with("if(")
        || t.starts_with("else if ") || t.starts_with("else if(")
        || t.starts_with("elif ");
    let is_while = t.starts_with("while ") || t.starts_with("while(");
    let is_for =
        t.starts_with("for ") && (t.contains(" in ") || t.contains(" of "));
    if !(is_if_like || is_while || is_for) {
        return false;
    }
    if paren_depth_net(t) != 0 || square_bracket_depth(t) != 0 {
        return true;
    }
    let n = raw_lines.len();
    let mut j = line_index + 1;
    while j < n {
        let (_, lc) = raw_lines[j];
        let lt = lc.trim();
        if lt.is_empty() || lt.starts_with("--") || lt.starts_with("//") {
            j += 1;
            continue;
        }
        let lc_trimmed = strip_trailing_line_comment(lc).trim();
        if lc_trimmed.is_empty() {
            j += 1;
            continue;
        }
        return is_likely_expr_continuation_line(lc);
    }
    false
}

/// Split `a = 1; b = 2` into separate simple statements for one logical line.
/// Semicolons inside `"..."` or `` `...` `` are not separators (so `print("a;b")` stays one stmt).
/// Empty segments are dropped.
fn split_statements_on_semicolon(input: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    let mut in_string = false;
    let mut in_backtick = false;
    let mut chars = input.chars().peekable();
    while let Some(ch) = chars.next() {
        if in_backtick {
            current.push(ch);
            if ch == '`' {
                in_backtick = false;
            }
            continue;
        }
        if in_string {
            current.push(ch);
            if ch == '"' {
                in_string = false;
            } else if ch == '\\' {
                if let Some(&n) = chars.peek() {
                    current.push(n);
                    chars.next();
                }
            }
            continue;
        }
        match ch {
            '"' => {
                in_string = true;
                current.push(ch);
            }
            '`' => {
                in_backtick = true;
                current.push(ch);
            }
            ';' => {
                let t = current.trim().to_string();
                if !t.is_empty() {
                    out.push(t);
                }
                current.clear();
            }
            _ => current.push(ch),
        }
    }
    let t = current.trim().to_string();
    if !t.is_empty() {
        out.push(t);
    }
    out
}

fn compile_block(
    lines: &[(usize, String)],
    cursor: &mut usize,
    state: &mut CompilerState,
    context: &mut CompileContext,
    stop_on_closing_brace: bool,
) -> Result<(), VmError> {
    while *cursor < lines.len() {
        let (line_no, raw_line) = &lines[*cursor];
        let line_no = *line_no;
        let line = raw_line.trim();

        if line == "}" {
            if stop_on_closing_brace {
                *cursor += 1;
                return Ok(());
            }
            return Err(VmError::parse_error_simple(format!(
                "Line {}: unexpected `}}`",
                line_no
            )));
        }

        if line == "{" {
            return Err(VmError::parse_error_simple(format!(
                "Line {}: unexpected `{{`",
                line_no
            )));
        }

        if line.starts_with("fn ") || line.starts_with("kfn ")
            || line.starts_with("async fn ") || line.starts_with("async kfn ")
        {
            compile_fn_definition(lines, cursor, state)?;
            continue;
        }

        // class ClassName { kfn methods... }
        // class Child extends Parent { ... }
        if line.starts_with("class ") {
            compile_class_definition(lines, cursor, state)?;
            continue;
        }

        // v2.2: import statement — `import path` or `import "path.killer"`
        if let Some(rest) = line.strip_prefix("import ") {
            let path = rest.trim().trim_matches('"').to_string();
            state.instructions.push(Instruction::ImportPkg(path));
            *cursor += 1;
            continue;
        }

        if line.starts_with("if ") || line.starts_with("if(") {
            compile_if_statement(lines, cursor, state, context)?;
            continue;
        }

        if line.starts_with("while ") || line.starts_with("while(") {
            compile_while_statement(lines, cursor, state, context)?;
            continue;
        }

        // for x in expr { ... }  /  for x of expr { ... }  — line compiler: both iterate values by index
        // (same codegen; prefer `of` to match JS/Python style for values)
        if line.starts_with("for ") && (line.contains(" in ") || line.contains(" of ")) {
            compile_for_each_line_statement(lines, cursor, state, context)?;
            continue;
        }

        // -- Nova Galaxy Engine v1: @lang { ... } polyglot block --------------
        // Supported syntaxes:
        //   Statement  (no return value):  @python { print(10*5) }
        //   Assignment (captures stdout):  result = @python { print(10*5) }
        //   Multi-line:                    result = @python {
        //                                      x = 10 * 5
        //                                      print(x)
        //                                  }
        // NOTE: normalize_lines() splits on { and }, so "@python {" arrives
        // as two tokens: the "@python" line, then a "{" line.
        {
            // Detect BOTH forms (after normalize_lines splits on braces):
            //   Form A: line == "@lang"           →  statement (no assign)
            //   Form B: line == "name = @lang"    →  assigns result to name
            let polyglot_match: Option<(Option<String>, &str)> = if line.starts_with('@') {
                let rest = &line[1..];
                if rest.chars().next().map(|c| c.is_alphabetic()).unwrap_or(false) {
                    Some((None, rest))
                } else { None }
            } else if let Some(eq_pos) = line.find('=') {
                let before = line[..eq_pos].trim();
                let after  = line[eq_pos + 1..].trim();
                if is_valid_name(before) && after.starts_with('@') {
                    let rest = &after[1..];
                    if rest.chars().next().map(|c| c.is_alphabetic()).unwrap_or(false) {
                        Some((Some(before.to_string()), rest))
                    } else { None }
                } else { None }
            } else { None };

            if let Some((assign_target, polyglot_line)) = polyglot_match {
                // Extract pure lang name (e.g., "python" from "python" or "python {")
                let lang_end = polyglot_line
                    .find(|c: char| !c.is_alphanumeric() && c != '_')
                    .unwrap_or(polyglot_line.len());
                let lang            = &polyglot_line[..lang_end];
                let rest_after_lang = polyglot_line[lang_end..].trim();

                // Advance past the @lang line
                *cursor += 1;

                // After normalize_lines splits braces, the next line will be "{"
                // Skip the opening brace line
                if *cursor < lines.len() && lines[*cursor].1.trim() == "{" {
                    *cursor += 1;
                }

                // If there was inline content before "{" (shouldn't happen after
                // normalize_lines, but handle gracefully)
                let mut code_body = String::new();
                if !rest_after_lang.is_empty() && rest_after_lang != "{" {
                    code_body.push_str(rest_after_lang);
                    code_body.push('\n');
                }

                // Collect body lines until closing "}"
                while *cursor < lines.len() {
                    let (_, body_line) = &lines[*cursor];
                    if body_line.trim() == "}" {
                        *cursor += 1;
                        break;
                    }
                    code_body.push_str(body_line.trim());
                    code_body.push('\n');
                    *cursor += 1;
                }

                // Emit: push lang string, push code string, call polyglot_exec
                state.instructions.push(Instruction::ConstStr(lang.to_string()));
                state.instructions.push(Instruction::ConstStr(code_body.trim_end().to_string()));
                state.instructions.push(Instruction::CallBuiltin("polyglot_exec".to_string(), 2));

                // Store result if assigned, otherwise pop (statement form)
                if let Some(target) = assign_target {
                    let slot = if let Some(&s) = context.slot_map.get(&target) {
                        s
                    } else {
                        let s = context.next_slot;
                        context.slot_map.insert(target.clone(), s);
                        context.next_slot = s.saturating_add(1);
                        s
                    };
                    state.instructions.push(Instruction::StoreSlot(slot));
                } else {
                    state.instructions.push(Instruction::Pop);
                }
                continue;
            }
        }

        let (stmt_line, end_phys) = merge_multiline_signature_header(lines, *cursor)?;
        let pieces = split_statements_on_semicolon(&stmt_line);
        if pieces.is_empty() {
            *cursor = end_phys + 1;
            continue;
        }
        for piece in pieces {
            compile_simple_statement(&piece, line_no, state, context)?;
        }
        *cursor = end_phys + 1;
    }

    if stop_on_closing_brace {
        return Err(VmError::parse_error_simple(
            "Missing closing `}` for block".to_string(),
        ));
    }

    Ok(())
}

fn compile_if_statement(
    lines: &[(usize, String)],
    cursor: &mut usize,
    state: &mut CompilerState,
    context: &mut CompileContext,
) -> Result<(), VmError> {
    let start = *cursor;
    let line_no = lines[start].0;
    let (line_merged, end_phys) = merge_multiline_control_header_until_brace(lines, start)?;
    let condition = extract_keyword_condition(&line_merged, "if", line_no)?;

    *cursor = end_phys + 1;
    expect_open_brace(lines, cursor, line_no, "if")?;

    compile_expr_str(condition, line_no, state, context)?;
    let jump_false_index = state.instructions.len();
    state.instructions.push(Instruction::JumpIfFalse(usize::MAX));

    compile_block(lines, cursor, state, context, true)?;

    let mut has_else = false;
    let mut jump_end_index = 0usize;
    if *cursor < lines.len() {
        let (cur_lno, maybe_else) = &lines[*cursor];
        let trimmed = maybe_else.trim();
        if trimmed == "else" {
            has_else = true;
            *cursor += 1;
            expect_open_brace(lines, cursor, line_no, "else")?;

            jump_end_index = state.instructions.len();
            state.instructions.push(Instruction::Jump(usize::MAX));

            let else_target = state.instructions.len();
            state.instructions[jump_false_index] = Instruction::JumpIfFalse(else_target);

            compile_block(lines, cursor, state, context, true)?;
        } else if trimmed.starts_with("else if ") || trimmed.starts_with("else if(") {
            // else if: rewrite as "if ..." and recurse
            has_else = true;
            jump_end_index = state.instructions.len();
            state.instructions.push(Instruction::Jump(usize::MAX));

            let else_target = state.instructions.len();
            state.instructions[jump_false_index] = Instruction::JumpIfFalse(else_target);

            // Strip `else ` prefix → `if ...` / `if(...)`
            let stripped = trimmed[5..].trim().to_string();
            let mut patched: Vec<(usize, String)> = lines.to_vec();
            patched[*cursor] = (*cur_lno, stripped);
            compile_if_statement(&patched, cursor, state, context)?;
        }
    }

    let end_target = state.instructions.len();
    if has_else {
        state.instructions[jump_end_index] = Instruction::Jump(end_target);
    } else {
        state.instructions[jump_false_index] = Instruction::JumpIfFalse(end_target);
    }
    Ok(())
}

fn compile_while_statement(
    lines: &[(usize, String)],
    cursor: &mut usize,
    state: &mut CompilerState,
    context: &mut CompileContext,
) -> Result<(), VmError> {
    let start = *cursor;
    let line_no = lines[start].0;
    let (line_merged, end_phys) = merge_multiline_control_header_until_brace(lines, start)?;
    let condition = extract_keyword_condition(&line_merged, "while", line_no)?;

    *cursor = end_phys + 1;
    expect_open_brace(lines, cursor, line_no, "while")?;

    let loop_start = state.instructions.len();
    compile_expr_str(condition, line_no, state, context)?;
    let jump_false_index = state.instructions.len();
    state.instructions.push(Instruction::JumpIfFalse(usize::MAX));

    context.loop_stack.push(LoopContext {
        loop_start,
        break_jumps: Vec::new(),
    });

    compile_block(lines, cursor, state, context, true)?;
    state.instructions.push(Instruction::Jump(loop_start));

    let loop_end = state.instructions.len();
    state.instructions[jump_false_index] = Instruction::JumpIfFalse(loop_end);

    let loop_context = context.loop_stack.pop().ok_or_else(|| {
        VmError::runtime_error("Internal compiler error: missing loop context".to_string())
    })?;
    for break_index in loop_context.break_jumps {
        state.instructions[break_index] = Instruction::Jump(loop_end);
    }

    Ok(())
}

fn compile_for_each_line_statement(
    lines: &[(usize, String)],
    cursor: &mut usize,
    state: &mut CompilerState,
    context: &mut CompileContext,
) -> Result<(), VmError> {
    let start = *cursor;
    let line_no = lines[start].0;
    let (line_merged, end_phys) = merge_multiline_control_header_until_brace(lines, start)?;
    // Parse: "for VAR in EXPR" or "for VAR of EXPR"
    let rest = line_merged[4..].trim(); // strip "for "
    let pos_in = rest.find(" in ");
    let pos_of = rest.find(" of ");
    let (sep_pos, sep_len) = match (pos_in, pos_of) {
        (Some(i), Some(o)) if i < o => (i, 4usize), // " in "
        (Some(i), Some(o)) if o < i => (o, 4usize), // " of "
        (Some(i), None) => (i, 4),
        (None, Some(o)) => (o, 4),
        _ => {
            return Err(VmError::parse_error_simple(format!(
                "Line {}: expected `for VAR in EXPR` or `for VAR of EXPR`",
                line_no
            )));
        }
    };
    let var_name = rest[..sep_pos].trim();
    let iter_expr = rest[sep_pos + sep_len..].trim();

    if var_name.is_empty() || iter_expr.is_empty() {
        return Err(VmError::parse_error_simple(format!(
            "Line {}: expected `for VAR in EXPR` or `for VAR of EXPR`",
            line_no
        )));
    }

    *cursor = end_phys + 1;
    expect_open_brace(lines, cursor, line_no, "for")?;

    // Unique names to avoid collision with nested loops
    let iter_var = format!("__for_iter_{}", state.instructions.len());
    let idx_var  = format!("__for_idx_{}",  state.instructions.len());

    // Evaluate the iterable once and store it
    compile_expr_str(iter_expr, line_no, state, context)?;
    state.instructions.push(Instruction::Store(iter_var.clone()));

    // idx = 0
    state.instructions.push(Instruction::ConstNum(0.0));
    state.instructions.push(Instruction::Store(idx_var.clone()));

    let loop_start = state.instructions.len();

    // while idx < len(iter)
    state.instructions.push(Instruction::Load(idx_var.clone()));
    state.instructions.push(Instruction::Load(iter_var.clone()));
    state.instructions.push(Instruction::CallBuiltin("len".to_string(), 1));
    state.instructions.push(Instruction::Lt);
    let jump_false_idx = state.instructions.len();
    state.instructions.push(Instruction::JumpIfFalse(usize::MAX));

    context.loop_stack.push(LoopContext {
        loop_start,
        break_jumps: Vec::new(),
    });

    // var = iter[idx]
    state.instructions.push(Instruction::Load(iter_var.clone()));
    state.instructions.push(Instruction::Load(idx_var.clone()));
    state.instructions.push(Instruction::IndexRead);
    state.instructions.push(Instruction::Store(var_name.to_string()));

    // body
    compile_block(lines, cursor, state, context, true)?;

    // idx = idx + 1
    state.instructions.push(Instruction::Load(idx_var.clone()));
    state.instructions.push(Instruction::ConstNum(1.0));
    state.instructions.push(Instruction::Add);
    state.instructions.push(Instruction::Store(idx_var.clone()));

    state.instructions.push(Instruction::Jump(loop_start));
    let loop_end = state.instructions.len();
    state.instructions[jump_false_idx] = Instruction::JumpIfFalse(loop_end);

    if let Some(loop_ctx) = context.loop_stack.pop() {
        for break_idx in loop_ctx.break_jumps {
            state.instructions[break_idx] = Instruction::Jump(loop_end);
        }
    }

    Ok(())
}

fn expect_open_brace(
    lines: &[(usize, String)],
    cursor: &mut usize,
    owner_line: usize,
    keyword: &str,
) -> Result<(), VmError> {
    if *cursor >= lines.len() {
        return Err(VmError::parse_error_simple(format!(
            "Line {}: `{}` requires a block starting with `{{`",
            owner_line, keyword
        )));
    }

    let (brace_line_no, brace_line) = &lines[*cursor];
    if brace_line.trim() != "{" {
        return Err(VmError::parse_error_simple(format!(
            "Line {}: expected `{{` after `{}` (found `{}` on line {})",
            owner_line,
            keyword,
            brace_line.trim(),
            brace_line_no
        )));
    }

    *cursor += 1;
    Ok(())
}

fn compile_simple_statement(
    line: &str,
    line_no: usize,
    state: &mut CompilerState,
    context: &mut CompileContext,
) -> Result<(), VmError> {
    let mut stmt = line.trim();
    if stmt.ends_with(';') {
        stmt = &stmt[..stmt.len() - 1];
        stmt = stmt.trim();
    }

    if let Some(rest) = stmt.strip_prefix("let ") {
        compile_let(rest, line_no, state, context)?;
        return Ok(());
    }

    // v2.2: spawn statement (fire-and-forget) — e.g. `spawn worker()` or `spawn worker(a,b)`
    if let Some(body) = stmt.strip_prefix("spawn ") {
        emit_spawn_call(body.trim(), line_no, state, context)?;
        state.instructions.push(Instruction::Pop);  // discard Future — fire and forget
        return Ok(());
    }

    if let Some(rest) = stmt.strip_prefix("return") {
        if !context.in_function {
            return Err(VmError::parse_error_simple(format!(
                "Line {}: `return` is only valid inside a function",
                line_no
            )));
        }

        let expr = rest.trim();
        if !expr.is_empty() {
            compile_expr_str(expr, line_no, state, context)?;
        }
        state.instructions.push(Instruction::Ret);
        return Ok(());
    }

    if stmt == "break" {
        let loop_context = context.loop_stack.last_mut().ok_or_else(|| {
            VmError::parse_error_simple(format!("Line {}: `break` is only valid inside a loop", line_no))
        })?;

        let jump_index = state.instructions.len();
        state.instructions.push(Instruction::Jump(usize::MAX));
        loop_context.break_jumps.push(jump_index);
        return Ok(());
    }

    if stmt == "continue" {
        let loop_context = context.loop_stack.last().ok_or_else(|| {
            VmError::parse_error_simple(format!(
                "Line {}: `continue` is only valid inside a loop",
                line_no
            ))
        })?;
        state.instructions.push(Instruction::Jump(loop_context.loop_start));
        return Ok(());
    }

    if let Some(inner) = extract_call_arg(stmt, "print") {
        compile_expr_str(inner, line_no, state, context)?;
        state.instructions.push(Instruction::Print);
        return Ok(());
    }

    if parse_call_expr(stmt).is_some() {
        compile_expr_str(stmt, line_no, state, context)?;
        return Ok(());
    }

    // Method call statement: obj.method(args)  e.g.  f.to("out.csv"), arr.push(x)
    if parse_method_call_str(stmt).is_some() {
        compile_method_call_str(stmt, line_no, state, context)?;
        return Ok(());
    }

    // dot-to-dot conversion syntax:  data.csv.to.data.kore  or  (data.csv).to.(data.kore)
    if let Some((src, dst)) = parse_dot_op_dot(stmt, "to") {
        state.instructions.push(Instruction::ConstStr(src));
        state.instructions.push(Instruction::ConstStr(dst));
        state.instructions.push(Instruction::CallBuiltin("nova_auto_convert".to_string(), 2));
        return Ok(());
    }
    // dot-compress syntax:  (file.txt).compress.(file.nvz)  or  file.txt.compress.file.nvz
    if let Some((src, dst)) = parse_dot_op_dot(stmt, "compress") {
        state.instructions.push(Instruction::ConstStr(src));
        state.instructions.push(Instruction::ConstStr(dst));
        state.instructions.push(Instruction::CallBuiltin("nova_compress".to_string(), 2));
        return Ok(());
    }
    // dot-decompress syntax:  (file.nvz).decompress.(file.txt)
    if let Some((src, dst)) = parse_dot_op_dot(stmt, "decompress") {
        state.instructions.push(Instruction::ConstStr(src));
        state.instructions.push(Instruction::ConstStr(dst));
        state.instructions.push(Instruction::CallBuiltin("nova_decompress".to_string(), 2));
        return Ok(());
    }

    if let Some((base, indices, rhs)) = parse_index_assignment_chain(stmt) {
        compile_chained_index_assign(&base, &indices, rhs, line_no, state, context)?;
        return Ok(());
    }

    // this.field = expr  or  obj.field = expr  — property assignment
    if let Some((obj_field, rhs)) = split_assignment(stmt) {
        if let Some(dot_pos) = obj_field.find('.') {
            let obj_part = obj_field[..dot_pos].trim();
            let field_part = obj_field[dot_pos + 1..].trim();
            if is_valid_name(field_part) && (obj_part == "this" || is_valid_name(obj_part)) {
                // Stack: index (field name), value — VM::IndexWrite loads object from named var.
                state.instructions.push(Instruction::ConstStr(field_part.to_string()));
                compile_expr_str(rhs, line_no, state, context)?;
                emit_index_write_named(obj_part, context, state);
                return Ok(());
            }
        }
    }

    if let Some((name, expr)) = split_assignment(stmt) {
        if !is_valid_name(name) {
            return Err(VmError::parse_error_simple(format!(
                "Line {}: invalid assignment target `{}`",
                line_no, name
            )));
        }

        if let Some(param_index) = context.params.get(name) {
            compile_expr_str(expr, line_no, state, context)?;
            state
                .instructions
                .push(Instruction::Store(format!("arg{}", param_index)));
            return Ok(());
        }

        compile_expr_str(expr, line_no, state, context)?;
        // `__*` temps always use named storage (see other compile paths).
        if name.starts_with("__") {
            state.instructions.push(Instruction::Store(name.to_string()));
            return Ok(());
        }
        // Slot-backed locals (including top-level `i = 0` script vars): always StoreSlot so
        // they stay consistent with LoadSlot / LtSlotConst in conditions and expressions.
        if let Some(&slot) = context.slot_map.get(name) {
            state.instructions.push(Instruction::StoreSlot(slot));
            return Ok(());
        }
        if context.in_function && context.outer_vars.contains(name) {
            // Outer `let` global — named Store so nested scopes update the same binding.
            state.instructions.push(Instruction::Store(name.to_string()));
            return Ok(());
        }
        if !context.in_function && state.known_top_level_vars.contains(name) {
            // Top-level `let` — named Store (no slot entry for these names).
            state.instructions.push(Instruction::Store(name.to_string()));
            return Ok(());
        }
        let s = context.next_slot;
        context.next_slot = context.next_slot.saturating_add(1);
        context.slot_map.insert(name.to_string(), s);
        state.instructions.push(Instruction::StoreSlot(s));
        return Ok(());
    }

    Err(VmError::parse_error_simple(format!(
        "Line {}: unsupported Killer subset statement `{}`",
        line_no, stmt
    )))
}

fn compile_let(
    rest: &str,
    line_no: usize,
    state: &mut CompilerState,
    context: &mut CompileContext,
) -> Result<(), VmError> {
    let mut parts = rest.splitn(2, '=');
    let name = parts.next().unwrap_or("").trim();
    let expr = parts.next().unwrap_or("").trim();

    if name.is_empty() || expr.is_empty() {
        return Err(VmError::parse_error_simple(format!(
            "Line {}: let statement must be `let name = expression`",
            line_no
        )));
    }

    if !is_valid_name(name) {
        return Err(VmError::parse_error_simple(format!(
            "Line {}: invalid variable name `{}`",
            line_no, name
        )));
    }

    if context.params.contains_key(name) {
        return Err(VmError::parse_error_simple(format!(
            "Line {}: parameter `{}` cannot be redeclared with let",
            line_no, name
        )));
    }

    compile_expr_str(expr, line_no, state, context)?;
    // Inside a function: register in slot_map for consistent Load/StoreSlot usage.
    // Without this, `let` emits named Store but later assignments emit StoreSlot,
    // causing split-brain where while-loop conditions read stale named values.
    if context.in_function {
        let slot = if let Some(&s) = context.slot_map.get(name) {
            s
        } else {
            let s = context.next_slot;
            context.slot_map.insert(name.to_string(), s);
            context.next_slot = s.saturating_add(1);
            s
        };
        state.instructions.push(Instruction::StoreSlot(slot));
    } else {
        state.instructions.push(Instruction::Store(name.to_string()));
        // Track top-level variable names so functions can reference them
        state.known_top_level_vars.insert(name.to_string());
    }
    Ok(())
}

/// Compile a K-string interpolation: the raw content between `K"..."` or `k"..."`.
/// Segments like "Hello {name}" compile to  ConstStr("Hello ") + Load(name) + Add + ...
fn compile_kstring(
    inner: &str,
    line_no: usize,
    state: &mut CompilerState,
    context: &CompileContext,
) -> Result<(), VmError> {
    // Parse into alternating literal / expression segments
    let mut segments: Vec<(bool, String)> = Vec::new(); // (is_expr, text)
    let mut buf = String::new();
    let mut in_expr = false;
    let mut brace_depth = 0usize;

    for ch in inner.chars() {
        if !in_expr {
            if ch == '{' {
                if !buf.is_empty() {
                    segments.push((false, buf.clone()));
                    buf.clear();
                }
                in_expr = true;
                brace_depth = 1;
            } else {
                buf.push(ch);
            }
        } else {
            if ch == '{' {
                brace_depth += 1;
                buf.push(ch);
            } else if ch == '}' {
                brace_depth -= 1;
                if brace_depth == 0 {
                    segments.push((true, buf.clone()));
                    buf.clear();
                    in_expr = false;
                } else {
                    buf.push(ch);
                }
            } else {
                buf.push(ch);
            }
        }
    }
    if !buf.is_empty() {
        segments.push((false, buf));
    }

    if segments.is_empty() {
        state.instructions.push(Instruction::ConstStr(String::new()));
        return Ok(());
    }

    // Emit each segment then join with Add (string concat)
    let mut first = true;
    for (is_expr, text) in &segments {
        if *is_expr {
            // Wrap in str() so numbers/bools convert cleanly
            let call_expr = format!("str({})", text.trim());
            compile_expr_str(&call_expr, line_no, state, context)?;
        } else {
            state.instructions.push(Instruction::ConstStr(text.clone()));
        }
        if !first {
            state.instructions.push(Instruction::Add);
        }
        first = false;
    }
    Ok(())
}

fn compile_expr_str(
    expr: &str,
    line_no: usize,
    state: &mut CompilerState,
    context: &CompileContext,
) -> Result<(), VmError> {
    let expr = expr.trim();

    // Parenthesized grouping: (inner) — strip outer parens and recurse
    if expr.starts_with('(') && expr.ends_with(')') {
        // Verify the opening ( at position 0 matches the closing ) at the last position.
        // The depth must never reach 0 before the final character; if it does, the
        // opening `(` closes early (e.g. `(a)*(b)`) and this is NOT a simple grouping.
        let mut depth = 0i32;
        let mut valid = true;
        for (i, ch) in expr.char_indices() {
            match ch {
                '(' => depth += 1,
                ')' => {
                    depth -= 1;
                    if depth == 0 && i < expr.len() - 1 {
                        valid = false;
                        break;
                    }
                }
                _ => {}
            }
        }
        if valid && depth == 0 {
            return compile_expr_str(&expr[1..expr.len()-1], line_no, state, context);
        }
    }

    // Unary minus: -expr  (e.g. -42, -n, -(a+b))
    if let Some(rest) = expr.strip_prefix('-') {
        let rest = rest.trim();
        // Make sure it's not subtraction (left side would be non-empty before strip)
        if !rest.is_empty() {
            // Push 0, compile rest, emit Sub (0 - rest = negation)
            state.instructions.push(Instruction::ConstNum(0.0));
            compile_expr_str(rest, line_no, state, context)?;
            state.instructions.push(Instruction::Sub);
            return Ok(());
        }
    }

    // v2.2: await expr → resolves a Future (AwaitTask opcode)
    if let Some(body) = expr.strip_prefix("await ") {
        compile_expr_str(body.trim(), line_no, state, context)?;
        state.instructions.push(Instruction::AwaitTask);
        return Ok(());
    }

    // Unary logical NOT: !expr  (e.g. !ready, !contains(arr, x))
    if let Some(rest) = expr.strip_prefix('!') {
        let rest = rest.trim();
        if !rest.is_empty() {
            compile_expr_str(rest, line_no, state, context)?;
            state.instructions.push(Instruction::Not);
            return Ok(());
        }
    }
    // v2.2: spawn expr — true OS-thread parallel execution (SpawnCall opcode)
    if let Some(body) = expr.strip_prefix("spawn ") {
        emit_spawn_call(body.trim(), line_no, state, context)?;
        return Ok(());
    }

    // Dict literal: {} or { k: v, ... } (keys/values compiled like AST Expr::Dict)
    if expr.starts_with('{') && expr.ends_with('}') && expr.len() >= 2 {
        let inner = expr[1..expr.len() - 1].trim();
        if let Some(entries) = split_dict_literal_entries(inner) {
            for (key_src, val_src) in &entries {
                compile_expr_str(key_src, line_no, state, context)?;
                compile_expr_str(val_src, line_no, state, context)?;
            }
            state.instructions.push(Instruction::BuildDict(entries.len()));
            return Ok(());
        }
    }

    // K-string: K"..." or k"..." — text with {var} interpolation
    let k_inner = expr
        .strip_prefix("K\"")
        .or_else(|| expr.strip_prefix("k\""))
        .and_then(|s| s.strip_suffix('"'));
    if let Some(inner) = k_inner {
        return compile_kstring(inner, line_no, state, context);
    }

    if expr.starts_with('"') {
        // Find the properly-closing quote (skip escaped \")
        let inner_chars: Vec<char> = expr.chars().collect();
        let mut end_pos = None;
        let mut i = 1; // skip opening "
        while i < inner_chars.len() {
            if inner_chars[i] == '\\' {
                i += 2; // skip escaped char
            } else if inner_chars[i] == '"' {
                end_pos = Some(i);
                break;
            } else {
                i += 1;
            }
        }
        // Only treat as string literal if closing quote is at the very end
        if let Some(pos) = end_pos {
            if pos == inner_chars.len() - 1 {
                let stripped: String = inner_chars[1..pos].iter().collect();
                // Process escape sequences: \n \t \\ \"
                let mut processed = String::with_capacity(stripped.len());
                let mut chars = stripped.chars();
                while let Some(ch) = chars.next() {
                    if ch == '\\' {
                        match chars.next() {
                            Some('n')  => processed.push('\n'),
                            Some('t')  => processed.push('\t'),
                            Some('r')  => processed.push('\r'),
                            Some('\\') => processed.push('\\'),
                            Some('"')  => processed.push('"'),
                            Some(other) => { processed.push('\\'); processed.push(other); }
                            None => processed.push('\\'),
                        }
                    } else {
                        processed.push(ch);
                    }
                }
                state.instructions.push(Instruction::ConstStr(processed));
                return Ok(());
            }
        }
    }

    let lower = expr.to_ascii_lowercase();
    if lower == "true" {
        state.instructions.push(Instruction::ConstBool(true));
        return Ok(());
    }
    if lower == "false" {
        state.instructions.push(Instruction::ConstBool(false));
        return Ok(());
    }
    if lower == "null" || lower == "nil" || lower == "none" {
        state.instructions.push(Instruction::ConstNull);
        return Ok(());
    }

    if let Ok(n) = expr.parse::<f64>() {
        state.instructions.push(Instruction::ConstNum(n));
        return Ok(());
    }

    // `new ClassName(args)` — object instantiation
    if let Some(rest) = expr.strip_prefix("new ") {
        let rest = rest.trim();
        if let Some((class_name, args)) = parse_call_expr(rest) {
            for arg in &args {
                compile_expr_str(arg, line_no, state, context)?;
            }
            state.instructions.push(Instruction::NewObject(class_name));
            return Ok(());
        }
    }

    // `this` keyword — load current object instance
    if expr == "this" {
        state.instructions.push(Instruction::Load("this".to_string()));
        return Ok(());
    }

    // `this.field` — property access on this
    if let Some(field) = expr.strip_prefix("this.") {
        let field = field.trim();
        if is_valid_name(field) {
            state.instructions.push(Instruction::Load("this".to_string()));
            state.instructions.push(Instruction::CallMethodDynamic {
                method_name: field.to_string(),
                arg_count: 0,
            });
            return Ok(());
        }
    }

    // `obj.method(args)` — method call or `obj.field` — property access
    if let Some((receiver, method_name, args)) = parse_dot_call_expr(expr) {
        // Compile the receiver (the object)
        compile_expr_str(receiver, line_no, state, context)?;
        // Compile arguments
        for arg in &args {
            compile_expr_str(arg, line_no, state, context)?;
        }
        state.instructions.push(Instruction::CallMethodDynamic {
            method_name: method_name.to_string(),
            arg_count: args.len(),
        });
        return Ok(());
    }

    if let Some((name, args)) = parse_call_expr(expr) {
        for arg in &args {
            compile_expr_str(arg, line_no, state, context)?;
        }

        if let Some(meta) = state.functions.get(&name) {
            state.instructions.push(Instruction::Call {
                target: meta.start,
                arg_count: args.len(),
            });
        } else {
            let call_index = state.instructions.len();
            state.instructions.push(Instruction::Call {
                target: usize::MAX,
                arg_count: args.len(),
            });
            state.pending_calls.push(PendingCall {
                instruction_index: call_index,
                function_name: name,
                arg_count: args.len(),
                line_no,
            });
        }
        return Ok(());
    }

    if let Some((left, op, right)) = split_logical(expr) {
        compile_expr_str(left, line_no, state, context)?;
        compile_expr_str(right, line_no, state, context)?;
        match op {
            "&&" => state.instructions.push(Instruction::And),
            "||" => state.instructions.push(Instruction::Or),
            _ => {
                return Err(VmError::parse_error_simple(format!(
                    "Line {}: unsupported logical operator `{}`",
                    line_no, op
                )))
            }
        }
        return Ok(());
    }

    if let Some((left, op, right)) = split_comparison(expr) {
        compile_expr_str(left, line_no, state, context)?;
        compile_expr_str(right, line_no, state, context)?;
        match op {
            "==" => state.instructions.push(Instruction::Eq),
            "!=" => state.instructions.push(Instruction::Ne),
            ">=" => state.instructions.push(Instruction::Ge),
            "<=" => state.instructions.push(Instruction::Le),
            ">" => state.instructions.push(Instruction::Gt),
            "<" => state.instructions.push(Instruction::Lt),
            _ => {
                return Err(VmError::parse_error_simple(format!(
                    "Line {}: unsupported comparison operator `{}`",
                    line_no, op
                )))
            }
        }
        return Ok(());
    }

    if let Some((left, op, right)) = split_binary(expr) {
        compile_expr_str(left, line_no, state, context)?;
        compile_expr_str(right, line_no, state, context)?;
        match op {
            "+" => state.instructions.push(Instruction::Add),
            "-" => state.instructions.push(Instruction::Sub),
            "*" => state.instructions.push(Instruction::Mul),
            "//" => state.instructions.push(Instruction::IntDiv),
            "/" => state.instructions.push(Instruction::Div),
            "%" => state.instructions.push(Instruction::Mod),
            "**" => state.instructions.push(Instruction::CallBuiltin("pow".to_string(), 2)),
            _ => {
                return Err(VmError::parse_error_simple(format!(
                    "Line {}: unsupported operator `{}`",
                    line_no, op
                )))
            }
        }
        return Ok(());
    }

    if let Some(param_index) = context.params.get(expr) {
        state
            .instructions
            .push(Instruction::Load(format!("arg{}", param_index)));
        return Ok(());
    }

    if is_valid_name(expr) {
        // PERF: emit slot load for known locals, fallback to named load otherwise
        if let Some(&slot) = context.slot_map.get(expr) {
            state.instructions.push(Instruction::LoadSlot(slot));
        } else {
            state.instructions.push(Instruction::Load(expr.to_string()));
        }
        return Ok(());
    }

    // Array literal: [] or [elem1, elem2, ...]
    if expr.starts_with('[') && expr.ends_with(']') {
        let inner = expr[1..expr.len() - 1].trim();
        if inner.is_empty() {
            state.instructions.push(Instruction::BuildArray(0));
            return Ok(());
        }
        if let Some(elems) = split_arguments(inner) {
            for elem in &elems {
                compile_expr_str(elem.trim(), line_no, state, context)?;
            }
            state.instructions.push(Instruction::BuildArray(elems.len()));
            return Ok(());
        }
    }

    // Indexing: receiver[index_expr] — receiver is any expression (arr[i], (a+b)[0], foo()[1], d["k"])
    if let Some((recv, index_part)) = split_trailing_index_expr(expr) {
        compile_expr_str(recv, line_no, state, context)?;
        compile_expr_str(index_part, line_no, state, context)?;
        state.instructions.push(Instruction::IndexRead);
        return Ok(());
    }

    // dot-to-dot conversion: data.csv.to.data.kore  or  (data.csv).to.(data.kore)
    if let Some((src, dst)) = parse_dot_op_dot(expr, "to") {
        state.instructions.push(Instruction::ConstStr(src));
        state.instructions.push(Instruction::ConstStr(dst));
        state.instructions.push(Instruction::CallBuiltin("nova_auto_convert".to_string(), 2));
        return Ok(());
    }
    if let Some((src, dst)) = parse_dot_op_dot(expr, "compress") {
        state.instructions.push(Instruction::ConstStr(src));
        state.instructions.push(Instruction::ConstStr(dst));
        state.instructions.push(Instruction::CallBuiltin("nova_compress".to_string(), 2));
        return Ok(());
    }
    if let Some((src, dst)) = parse_dot_op_dot(expr, "decompress") {
        state.instructions.push(Instruction::ConstStr(src));
        state.instructions.push(Instruction::ConstStr(dst));
        state.instructions.push(Instruction::CallBuiltin("nova_decompress".to_string(), 2));
        return Ok(());
    }

    // Method call: obj.method(args) — e.g. n.to_string(), lst.push(x)
    if parse_method_call_str(expr).is_some() {
        return compile_method_call_str(expr, line_no, state, context);
    }

    Err(VmError::parse_error_simple(format!(
        "Line {}: unsupported expression `{}`",
        line_no, expr
    )))
}

/// Parse the  data.csv.to.data.kore  and  (data.csv).to.(data.kore)  conversion syntax.
/// Also handles .compress. and .decompress. with the same pattern.
/// Returns (src_path, dst_path) if the expression matches; None otherwise.
fn parse_dot_op_dot(expr: &str, op: &str) -> Option<(String, String)> {
    let expr  = expr.trim();
    let sep   = format!(".{}.", op);  // e.g.  ".to."  ".compress."  ".decompress."

    // Parenthesized form: (src).OP.(dst)  or  (src).OP.dst
    if expr.starts_with('(') {
        let close_src = expr.find(')')?;
        let src = expr[1..close_src].trim().to_string();
        if src.is_empty() { return None; }
        let rest = expr[close_src+1..].trim();
        let rest = rest.strip_prefix(sep.as_str())?;
        let dst = rest.trim();
        let dst = if dst.starts_with('(') && dst.ends_with(')') {
            dst[1..dst.len()-1].trim()
        } else {
            dst
        };
        if dst.is_empty() { return None; }
        return Some((src, dst.to_string()));
    }

    // Bare form: src.OP.dst  — both sides must look like file paths (contain a dot)
    // Guard: do NOT match  f.to("...")  — that has '(' right after the op
    if let Some(pos) = expr.find(sep.as_str()) {
        let src = expr[..pos].trim();
        let dst = expr[pos+sep.len()..].trim();
        if !src.is_empty() && src.contains('.') && !dst.is_empty() && dst.contains('.') {
            return Some((src.to_string(), dst.to_string()));
        }
    }

    None
}

// Kept for backwards compatibility — delegates to the generic form
#[allow(dead_code)]
fn parse_dot_to_dot(expr: &str) -> Option<(String, String)> {
    parse_dot_op_dot(expr, "to")
}

fn split_binary(expr: &str) -> Option<(&str, &str, &str)> {
    // Left-associative + − * / %: split on the rightmost operator at each precedence level
    // (weakest binding first). `**` is handled last: we split at the first top-level `**`,
    // so chains like `a**b**c` compile as `pow(a, pow(b, c))` (Python-style right-assoc).
    if let Some((index, op)) = find_top_level_add_sub_rightmost(expr) {
        let left = expr[..index].trim();
        let right = expr[index + 1..].trim();
        if !left.is_empty() && !right.is_empty() {
            let op_s = if op == b'+' { "+" } else { "-" };
            return Some((left, op_s, right));
        }
    }
    // Multiplicative level: *, /, %, and // (integer division).
    // find_top_level_mul_div_mod_rightmost skips '//' so single-char ops don't
    // collide with the two-char integer-division operator.
    {
        let single = find_top_level_mul_div_mod_rightmost(expr);
        let intdiv = find_top_level_intdiv_rightmost(expr);
        // Pick whichever is rightmost (both are at the same precedence)
        let pick: Option<(usize, &str, usize)> = match (single, intdiv) {
            (Some(a), Some(b)) => {
                if b >= a {
                    Some((b, "//", 2))
                } else {
                    let op = match expr.as_bytes()[a] {
                        b'*' => "*", b'/' => "/", b'%' => "%", _ => return None,
                    };
                    Some((a, op, 1))
                }
            }
            (Some(a), None) => {
                let op = match expr.as_bytes()[a] {
                    b'*' => "*", b'/' => "/", b'%' => "%", _ => return None,
                };
                Some((a, op, 1))
            }
            (None, Some(b)) => Some((b, "//", 2)),
            (None, None) => None,
        };
        if let Some((index, op, op_len)) = pick {
            let left = expr[..index].trim();
            let right = expr[index + op_len..].trim();
            if !left.is_empty() && !right.is_empty() {
                return Some((left, op, right));
            }
        }
    }
    if let Some(index) = find_top_level_op(expr, "**") {
        let left = expr[..index].trim();
        let right = expr[index + 2..].trim();
        if !left.is_empty() && !right.is_empty() {
            return Some((left, "**", right));
        }
    }
    None
}

fn split_comparison(expr: &str) -> Option<(&str, &str, &str)> {
    for op in ["==", "!=", ">=", "<=", ">", "<"] {
        if let Some(index) = find_top_level_op(expr, op) {
            let left = expr[..index].trim();
            let right = expr[index + op.len()..].trim();
            if !left.is_empty() && !right.is_empty() {
                return Some((left, op, right));
            }
        }
    }
    None
}

fn split_logical(expr: &str) -> Option<(&str, &str, &str)> {
    for op in ["&&", "||"] {
        if let Some(index) = find_top_level_op(expr, op) {
            let left = expr[..index].trim();
            let right = expr[index + op.len()..].trim();
            if !left.is_empty() && !right.is_empty() {
                return Some((left, op, right));
            }
        }
    }
    None
}

/// Find the first occurrence of `op` at bracket/paren/brace depth 0, ignoring
/// characters inside string literals. Handles `[]`, `()`, `{}`, and `""`.
fn find_top_level_op(expr: &str, op: &str) -> Option<usize> {
    let mut depth: i32 = 0;
    let mut in_string = false;
    let bytes = expr.as_bytes();
    let op_bytes = op.as_bytes();
    let op_len = op_bytes.len();
    let n = bytes.len();
    let mut i = 0;
    while i < n {
        if in_string {
            if bytes[i] == b'\\' {
                i += 2; // skip escaped character
                continue;
            }
            if bytes[i] == b'"' {
                in_string = false;
            }
        } else {
            match bytes[i] {
                b'"' => in_string = true,
                b'(' | b'[' | b'{' => depth += 1,
                b')' | b']' | b'}' => depth -= 1,
                _ if depth == 0 => {
                    if i + op_len <= n && &bytes[i..i + op_len] == op_bytes {
                        return Some(i);
                    }
                }
                _ => {}
            }
        }
        i += 1;
    }
    None
}

fn char_before_skip_ws(bytes: &[u8], start: usize) -> Option<u8> {
    let mut j = start;
    while j > 0 {
        j -= 1;
        let c = bytes[j];
        if c == b' ' || c == b'\t' {
            continue;
        }
        return Some(c);
    }
    None
}

/// `-` at `minus_idx` is binary subtraction, not unary negation.
fn is_binary_minus_at(bytes: &[u8], minus_idx: usize) -> bool {
    if minus_idx == 0 {
        return false;
    }
    match char_before_skip_ws(bytes, minus_idx) {
        None => false,
        Some(b'(' | b'[' | b'{' | b',') => false,
        Some(b'+' | b'-' | b'*' | b'/' | b'%') => false,
        _ => true,
    }
}

/// Rightmost `+` or binary `-` at depth 0 (left-associative additive chain).
fn find_top_level_add_sub_rightmost(expr: &str) -> Option<(usize, u8)> {
    let bytes = expr.as_bytes();
    let n = bytes.len();
    let mut depth = 0i32;
    let mut in_string = false;
    let mut best: Option<(usize, u8)> = None;
    let mut i = 0usize;
    while i < n {
        if in_string {
            if bytes[i] == b'\\' {
                i += 2;
                continue;
            }
            if bytes[i] == b'"' {
                in_string = false;
            }
            i += 1;
            continue;
        }
        match bytes[i] {
            b'"' => {
                in_string = true;
                i += 1;
            }
            b'(' | b'[' | b'{' => {
                depth += 1;
                i += 1;
            }
            b')' | b']' | b'}' => {
                depth -= 1;
                i += 1;
            }
            b'+' if depth == 0 => {
                best = Some((i, b'+'));
                i += 1;
            }
            b'-' if depth == 0 && is_binary_minus_at(bytes, i) => {
                best = Some((i, b'-'));
                i += 1;
            }
            _ => i += 1,
        }
    }
    best
}

/// Rightmost `*`, `/`, or `%` at depth 0; `*` must not be part of `**`.
fn find_top_level_mul_div_mod_rightmost(expr: &str) -> Option<usize> {
    let bytes = expr.as_bytes();
    let n = bytes.len();
    let mut depth = 0i32;
    let mut in_string = false;
    let mut best: Option<usize> = None;
    let mut i = 0usize;
    while i < n {
        if in_string {
            if bytes[i] == b'\\' {
                i += 2;
                continue;
            }
            if bytes[i] == b'"' {
                in_string = false;
            }
            i += 1;
            continue;
        }
        match bytes[i] {
            b'"' => {
                in_string = true;
                i += 1;
            }
            b'(' | b'[' | b'{' => {
                depth += 1;
                i += 1;
            }
            b')' | b']' | b'}' => {
                depth -= 1;
                i += 1;
            }
            b'*' if depth == 0 => {
                if i + 1 < n && bytes[i + 1] == b'*' {
                    i += 2;
                    continue;
                }
                best = Some(i);
                i += 1;
            }
            b'/' | b'%' if depth == 0 => {
                // Skip '//' (integer division) — it's a 2-char operator handled separately
                if bytes[i] == b'/' && i + 1 < n && bytes[i + 1] == b'/' {
                    i += 2;
                    continue;
                }
                best = Some(i);
                i += 1;
            }
            _ => i += 1,
        }
    }
    best
}

/// Rightmost `//` (integer division) at depth 0 outside strings.
fn find_top_level_intdiv_rightmost(expr: &str) -> Option<usize> {
    let bytes = expr.as_bytes();
    let n = bytes.len();
    let mut depth = 0i32;
    let mut in_string = false;
    let mut best: Option<usize> = None;
    let mut i = 0usize;
    while i < n {
        if in_string {
            if bytes[i] == b'\\' {
                i += 2;
                continue;
            }
            if bytes[i] == b'"' {
                in_string = false;
            }
            i += 1;
            continue;
        }
        match bytes[i] {
            b'"' => { in_string = true; i += 1; }
            b'(' | b'[' | b'{' => { depth += 1; i += 1; }
            b')' | b']' | b'}' => { depth -= 1; i += 1; }
            b'/' if depth == 0 && i + 1 < n && bytes[i + 1] == b'/' => {
                best = Some(i);
                i += 2;
            }
            _ => i += 1,
        }
    }
    best
}

/// First `:` at paren/bracket/brace depth 0 outside strings — for dict `key: value` pairs.
fn find_top_level_colon(expr: &str) -> Option<usize> {
    let mut depth: i32 = 0;
    let mut in_string = false;
    let bytes = expr.as_bytes();
    let n = bytes.len();
    let mut i = 0;
    while i < n {
        if in_string {
            if bytes[i] == b'\\' && i + 1 < n {
                i += 2;
                continue;
            }
            if bytes[i] == b'"' {
                in_string = false;
            }
            i += 1;
            continue;
        }
        match bytes[i] {
            b'"' => {
                in_string = true;
                i += 1;
            }
            b'(' | b'[' | b'{' => {
                depth += 1;
                i += 1;
            }
            b')' | b']' | b'}' => {
                depth -= 1;
                i += 1;
            }
            b':' if depth == 0 => return Some(i),
            _ => i += 1,
        }
    }
    None
}

/// Split `{ inner }` body into `key: value` entries (commas at depth 0 via [`split_arguments`]).
fn split_dict_literal_entries(inner: &str) -> Option<Vec<(String, String)>> {
    let parts = split_arguments(inner)?;
    let mut out = Vec::with_capacity(parts.len());
    for part in parts {
        let idx = find_top_level_colon(&part)?;
        let key = part[..idx].trim();
        let val = part[idx + 1..].trim();
        if key.is_empty() || val.is_empty() {
            return None;
        }
        out.push((key.to_string(), val.to_string()));
    }
    Some(out)
}

/// Split `receiver[index_expr]` where the final `]` closes the suffix index: `a[i]`, `(x+y)[0]`,
/// `lst[i][j]` → (`lst[i]`, `j`). Respects `"` strings. Returns `None` if receiver would be empty
/// (e.g. pure array literal `[1,2]`) or brackets are unbalanced.
fn split_trailing_index_expr(expr: &str) -> Option<(&str, &str)> {
    let e = expr.trim();
    if e.len() < 3 || !e.ends_with(']') {
        return None;
    }
    let bytes = e.as_bytes();
    let mut opens: Vec<usize> = Vec::new();
    let mut in_string = false;
    let mut i = 0usize;
    while i < bytes.len() {
        if in_string {
            if bytes[i] == b'\\' && i + 1 < bytes.len() {
                i += 2;
                continue;
            }
            if bytes[i] == b'"' {
                in_string = false;
            }
            i += 1;
            continue;
        }
        match bytes[i] {
            b'"' => {
                in_string = true;
                i += 1;
            }
            b'[' => {
                opens.push(i);
                i += 1;
            }
            b']' => {
                let open = opens.pop()?;
                i += 1;
                if i == bytes.len() {
                    let recv = e[..open].trim();
                    if recv.is_empty() {
                        return None;
                    }
                    let inner = e[open + 1..bytes.len() - 1].trim();
                    if inner.is_empty() {
                        return None;
                    }
                    return Some((recv, inner));
                }
            }
            _ => i += 1,
        }
    }
    None
}

/// Index of the `=` that separates assignment target from RHS — not `==`, `!=`, `>=`, `<=`,
/// and not inside `()`, `[]`, `{}`, or strings. Enables `x = a == b`, `x = a >= b`, etc.
fn find_assignment_equals(stmt: &str) -> Option<usize> {
    let bytes = stmt.as_bytes();
    let n = bytes.len();
    let mut paren_depth: i32 = 0;
    let mut bracket_depth: i32 = 0;
    let mut brace_depth: i32 = 0;
    let mut in_string = false;
    let mut i = 0;
    while i < n {
        if in_string {
            if bytes[i] == b'\\' && i + 1 < n {
                i += 2;
                continue;
            }
            if bytes[i] == b'"' {
                in_string = false;
            }
            i += 1;
            continue;
        }
        match bytes[i] {
            b'"' => {
                in_string = true;
                i += 1;
            }
            b'(' => {
                paren_depth += 1;
                i += 1;
            }
            b')' => {
                paren_depth -= 1;
                i += 1;
            }
            b'[' => {
                bracket_depth += 1;
                i += 1;
            }
            b']' => {
                bracket_depth -= 1;
                i += 1;
            }
            b'{' => {
                brace_depth += 1;
                i += 1;
            }
            b'}' => {
                brace_depth -= 1;
                i += 1;
            }
            b'=' if paren_depth == 0 && bracket_depth == 0 && brace_depth == 0 => {
                if i + 1 < n && bytes[i + 1] == b'=' {
                    i += 2;
                    continue;
                }
                if i > 0 {
                    match bytes[i - 1] {
                        b'!' | b'>' | b'<' => {
                            i += 1;
                            continue;
                        }
                        _ => {}
                    }
                }
                return Some(i);
            }
            _ => i += 1,
        }
    }
    None
}

fn split_assignment(stmt: &str) -> Option<(&str, &str)> {
    let idx = find_assignment_equals(stmt)?;
    let left = stmt[..idx].trim();
    let right = stmt[idx + 1..].trim();
    if left.is_empty() || right.is_empty() {
        return None;
    }

    if left.contains(' ') || left.contains('(') || left.contains(')') {
        return None;
    }

    Some((left, right))
}

/// `name[idx] = rhs` or `name[i][j] = rhs` (base must be a simple identifier).
fn parse_index_assignment_chain(stmt: &str) -> Option<(String, Vec<String>, &str)> {
    let idx = find_assignment_equals(stmt)?;
    let left = stmt[..idx].trim();
    let right = stmt[idx + 1..].trim();
    if left.is_empty() || right.is_empty() {
        return None;
    }
    let (base, indices) = peel_index_chain(left)?;
    Some((base, indices, right))
}

/// Walk `a[i][j]` from the outside in until the receiver is a single identifier; indices are
/// outer-to-inner (e.g. `m[1][0]` → base `m`, indices `["1","0"]`). Fails for non-identifier bases.
fn peel_index_chain(mut expr: &str) -> Option<(String, Vec<String>)> {
    let mut rev: Vec<String> = Vec::new();
    loop {
        let (recv, idx) = split_trailing_index_expr(expr)?;
        rev.push(idx.to_string());
        let recv = recv.trim();
        if is_valid_name(recv) {
            rev.reverse();
            return Some((recv.to_string(), rev));
        }
        expr = recv;
    }
}

fn compile_chained_index_assign(
    base: &str,
    indices: &[String],
    rhs: &str,
    line_no: usize,
    state: &mut CompilerState,
    context: &mut CompileContext,
) -> Result<(), VmError> {
    if indices.is_empty() {
        return Err(VmError::parse_error_simple(format!(
            "Line {}: index assignment requires at least one index",
            line_no
        )));
    }
    if indices.len() == 1 {
        return compile_index_assign_str(base, &indices[0], rhs, line_no, state, context);
    }
    let k = indices.len();
    let mut temps: Vec<u16> = Vec::with_capacity(k - 1);

    compile_expr_str(base, line_no, state, context)?;
    for j in 0..k - 1 {
        compile_expr_str(&indices[j], line_no, state, context)?;
        state.instructions.push(Instruction::IndexRead);
        let slot = context.next_slot;
        context.next_slot = context.next_slot.saturating_add(1);
        state.instructions.push(Instruction::StoreSlot(slot));
        temps.push(slot);
        if j < k - 2 {
            state.instructions.push(Instruction::LoadSlot(slot));
        }
    }

    compile_expr_str(&indices[k - 1], line_no, state, context)?;
    compile_expr_str(rhs, line_no, state, context)?;
    state.instructions.push(Instruction::IndexWriteSlot(temps[k - 2]));

    // Copy semantics: nested values are cloned on read; write the updated inner container
    // back through each outer level (see `IndexRead` / `IndexWrite` VM behavior).
    for j in (0..k - 1).rev() {
        compile_expr_str(&indices[j], line_no, state, context)?;
        state.instructions.push(Instruction::LoadSlot(temps[j]));
        if j == 0 {
            emit_index_write_named(base, context, state);
        } else {
            state.instructions.push(Instruction::IndexWriteSlot(temps[j - 1]));
        }
    }

    Ok(())
}

fn emit_index_write_named(recv: &str, context: &CompileContext, state: &mut CompilerState) {
    if let Some(param_index) = context.params.get(recv) {
        state.instructions.push(Instruction::IndexWrite(format!(
            "arg{}",
            param_index
        )));
    } else if let Some(&slot) = context.slot_map.get(recv) {
        state.instructions.push(Instruction::IndexWriteSlot(slot));
    } else {
        state.instructions.push(Instruction::IndexWrite(recv.to_string()));
    }
}

fn compile_index_assign_str(
    recv: &str,
    index_src: &str,
    rhs: &str,
    line_no: usize,
    state: &mut CompilerState,
    context: &CompileContext,
) -> Result<(), VmError> {
    compile_expr_str(index_src, line_no, state, context)?;
    compile_expr_str(rhs, line_no, state, context)?;
    emit_index_write_named(recv, context, state);
    Ok(())
}

/// Parse  obj.method(arg1, arg2, ...)  from a text expression.
/// Returns (object_name, method_name, arg_strings) or None.
fn parse_method_call_str(expr: &str) -> Option<(String, String, Vec<String>)> {
    let expr = expr.trim();
    // Find the first top-level '.' that separates obj from method
    let dot_pos = expr.find('.')?;
    let obj = expr[..dot_pos].trim();
    if !is_valid_name(obj) { return None; }
    let rest = &expr[dot_pos+1..];
    // rest must be  method(args)
    let open = rest.find('(')?;
    if !rest.ends_with(')') { return None; }
    let method = rest[..open].trim();
    if !is_valid_name(method) { return None; }
    let inside = &rest[open+1..rest.len()-1];
    let args = split_arguments(inside)?;
    Some((obj.to_string(), method.to_string(), args))
}

/// Compile  obj.method(args)  as an expression that leaves the result on the stack.
fn compile_method_call_str(
    expr: &str,
    line_no: usize,
    state: &mut CompilerState,
    context: &CompileContext,
) -> Result<(), VmError> {
    let Some((obj, method, args)) = parse_method_call_str(expr) else {
        return Err(VmError::parse_error_simple(format!(
            "Line {}: not a valid method call: `{}`", line_no, expr
        )));
    };
    // Load receiver
    if let Some(param_idx) = context.params.get(&obj) {
        state.instructions.push(Instruction::Load(format!("arg{}", param_idx)));
    } else if let Some(&slot) = context.slot_map.get(&obj) {
        state.instructions.push(Instruction::LoadSlot(slot));
    } else {
        state.instructions.push(Instruction::Load(obj));
    }
    // Compile arguments
    for arg in &args {
        compile_expr_str(arg.trim(), line_no, state, context)?;
    }
    // Emit dynamic method call
    state.instructions.push(Instruction::CallMethodDynamic {
        method_name: method,
        arg_count: args.len(),
    });
    Ok(())
}

/// Parse  `obj.method(args)`  or  `obj.field`  expressions.
/// Returns (receiver, name, args) — empty args for property access.
fn parse_dot_call_expr(expr: &str) -> Option<(&str, &str, Vec<String>)> {
    let expr = expr.trim();
    // Find the first top-level '.' that separates receiver from method/field
    // Must handle nested dots: skip dots inside strings, parens, brackets
    let bytes = expr.as_bytes();
    let mut dot_pos = None;
    let mut depth = 0i32;
    let mut in_string = false;
    for (i, &b) in bytes.iter().enumerate() {
        if in_string {
            if b == b'\\' { continue; }
            if b == b'"' { in_string = false; }
            continue;
        }
        match b {
            b'"' => in_string = true,
            b'(' | b'[' => depth += 1,
            b')' | b']' => depth -= 1,
            b'.' if depth == 0 && i > 0 => { dot_pos = Some(i); break; }
            _ => {}
        }
    }
    let dot_pos = dot_pos?;
    let receiver = expr[..dot_pos].trim();
    if receiver.is_empty() { return None; }
    // Don't match strings or numbers as receivers
    if receiver.starts_with('"') || receiver.parse::<f64>().is_ok() { return None; }
    // Don't match file-like patterns: something.csv, something.txt, something.kore
    let rest = &expr[dot_pos + 1..];
    if rest.contains('(') {
        // method call: rest = "method(args)"
        let open = rest.find('(')?;
        if !rest.ends_with(')') { return None; }
        let method = rest[..open].trim();
        if !is_valid_name(method) { return None; }
        let inside = &rest[open + 1..rest.len() - 1];
        let args = if inside.trim().is_empty() { Vec::new() } else { split_arguments(inside)? };
        Some((receiver, method, args))
    } else {
        // property access: rest = "field"
        let field = rest.trim();
        if !is_valid_name(field) { return None; }
        // Exclude known file extensions to avoid matching "data.csv" etc.
        let ext_blacklist = ["csv", "txt", "json", "kore", "nvz", "killer", "toml", "html", "rs", "js", "py"];
        if ext_blacklist.contains(&field) { return None; }
        Some((receiver, field, Vec::new()))
    }
}

fn parse_call_expr(expr: &str) -> Option<(String, Vec<String>)> {
    let open = expr.find('(')?;
    if !expr.ends_with(')') {
        return None;
    }

    let name = expr[..open].trim();
    if !is_valid_name(name) {
        return None;
    }

    let inside = &expr[open + 1..expr.len() - 1];
    let args = split_arguments(inside)?;
    Some((name.to_string(), args))
}

fn split_arguments(input: &str) -> Option<Vec<String>> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Some(Vec::new());
    }

    let mut args = Vec::new();
    let mut current = String::new();
    let mut in_string = false;
    let mut paren_depth = 0usize;
    let mut bracket_depth = 0usize;
    let mut brace_depth = 0usize;

    for ch in trimmed.chars() {
        match ch {
            '"' => {
                in_string = !in_string;
                current.push(ch);
            }
            '(' if !in_string => {
                paren_depth += 1;
                current.push(ch);
            }
            ')' if !in_string => {
                if paren_depth == 0 {
                    return None;
                }
                paren_depth -= 1;
                current.push(ch);
            }
            '[' if !in_string => {
                bracket_depth += 1;
                current.push(ch);
            }
            ']' if !in_string => {
                if bracket_depth == 0 {
                    return None;
                }
                bracket_depth -= 1;
                current.push(ch);
            }
            '{' if !in_string => {
                brace_depth += 1;
                current.push(ch);
            }
            '}' if !in_string => {
                if brace_depth == 0 {
                    return None;
                }
                brace_depth -= 1;
                current.push(ch);
            }
            ',' if !in_string
                && paren_depth == 0
                && bracket_depth == 0
                && brace_depth == 0 =>
            {
                let value = current.trim();
                if value.is_empty() {
                    return None;
                }
                args.push(value.to_string());
                current.clear();
            }
            _ => current.push(ch),
        }
    }

    if in_string || paren_depth != 0 || bracket_depth != 0 || brace_depth != 0 {
        return None;
    }

    let last = current.trim();
    if last.is_empty() {
        return None;
    }
    args.push(last.to_string());

    Some(args)
}

fn compile_fn_definition(
    lines: &[(usize, String)],
    cursor: &mut usize,
    state: &mut CompilerState,
) -> Result<(), VmError> {
    let start = *cursor;
    let line_no = lines[start].0;
    let (sig_line, end_phys) = merge_multiline_signature_header(lines, start)?;
    let (name, params) = parse_function_signature(&sig_line, line_no)?;

    if state.functions.contains_key(&name) {
        return Err(VmError::parse_error_simple(format!(
            "Line {}: duplicate function `{}`",
            line_no, name
        )));
    }

    *cursor = end_phys + 1;
    expect_open_brace(lines, cursor, line_no, "fn")?;

    let skip_jump_index = state.instructions.len();
    state.instructions.push(Instruction::Jump(usize::MAX));

    let fn_start = state.instructions.len();
    state.function_arities.insert(fn_start, params.len());
    state.functions.insert(
        name.clone(),
        FunctionMeta {
            start: fn_start,
            arity: params.len(),
        },
    );

    let mut fn_context = CompileContext {
        loop_stack: Vec::new(),
        in_function: true,
        current_function_name: Some(name),
        params: HashMap::new(),
        slot_map: HashMap::new(),
        next_slot: 0,
        // Collect parent scope variable names so function body can reference them
        // via named Store/Load (instead of creating new local slots).
        outer_vars: state.known_top_level_vars.iter().cloned().collect(),
    };
    for (index, param) in params.into_iter().enumerate() {
        fn_context.params.insert(param, index);
    }

    compile_block(lines, cursor, state, &mut fn_context, true)?;

    if !matches!(state.instructions.last(), Some(Instruction::Ret)) {
        state.instructions.push(Instruction::Ret);
    }

    let after_fn = state.instructions.len();
    state.instructions[skip_jump_index] = Instruction::Jump(after_fn);
    Ok(())
}

/// Parse and compile a class definition in the subset compiler:
///   class ClassName {
///       kfn init(params) { ... }
///       kfn method(params) { ... }
///   }
/// or with inheritance:
///   class Child extends Parent { ... }
fn compile_class_definition(
    lines: &[(usize, String)],
    cursor: &mut usize,
    state: &mut CompilerState,
) -> Result<(), VmError> {
    let start = *cursor;
    let line_no = lines[start].0;
    let line = lines[start].1.trim().to_string();

    // Parse: "class Name" or "class Name extends Parent"
    let rest = line.strip_prefix("class ").ok_or_else(|| {
        VmError::parse_error_simple(format!("Line {}: expected class definition", line_no))
    })?.trim();

    let (class_name, parent) = if let Some(pos) = rest.find(" extends ") {
        let name = rest[..pos].trim().to_string();
        let parent_name = rest[pos + 9..].trim().to_string();
        (name, Some(parent_name))
    } else {
        (rest.to_string(), None)
    };

    *cursor += 1;
    expect_open_brace(lines, cursor, line_no, "class")?;

    // Emit a Jump to skip over all method bytecode during linear execution
    let skip_jump_index = state.instructions.len();
    state.instructions.push(Instruction::Jump(usize::MAX));

    // Collect methods: parse kfn definitions inside the class body
    let mut method_info: Vec<(String, Vec<String>)> = Vec::new();

    while *cursor < lines.len() {
        let (mline_no, raw) = &lines[*cursor];
        let mline = raw.trim();

        if mline == "}" {
            *cursor += 1;
            break;
        }

        if mline.is_empty() || mline.starts_with("//") || mline.starts_with("--") {
            *cursor += 1;
            continue;
        }

        if mline.starts_with("kfn ") || mline.starts_with("fn ") {
            let (sig_line, end_phys) = merge_multiline_signature_header(lines, *cursor)?;
            let (method_name, params) = parse_function_signature(&sig_line, *mline_no)?;
            *cursor = end_phys + 1;
            expect_open_brace(lines, cursor, *mline_no, "method")?;

            let method_start = state.instructions.len();
            state.method_bytecode.insert(
                (class_name.clone(), method_name.clone()),
                method_start,
            );
            state.function_arities.insert(method_start, params.len());

            // Compile method body in its own context
            let mut method_context = CompileContext {
                loop_stack: Vec::new(),
                in_function: true,
                current_function_name: Some(format!("{}.{}", class_name, method_name)),
                params: HashMap::new(),
                slot_map: HashMap::new(),
                next_slot: 0,
                outer_vars: state.known_top_level_vars.iter().cloned().collect(),
            };
            for (index, param) in params.iter().enumerate() {
                method_context.params.insert(param.clone(), index);
            }

            compile_block(lines, cursor, state, &mut method_context, true)?;

            // Implicit return: init returns "this", other methods return null
            if !matches!(state.instructions.last(), Some(Instruction::Ret)) {
                if method_name == "init" {
                    state.instructions.push(Instruction::Load("this".to_string()));
                } else {
                    state.instructions.push(Instruction::ConstNull);
                }
                state.instructions.push(Instruction::Ret);
            }

            method_info.push((method_name, params));
        } else {
            return Err(VmError::parse_error_simple(format!(
                "Line {}: expected method definition (kfn) inside class, got `{}`",
                mline_no, mline
            )));
        }
    }

    // Patch the skip jump
    let after_methods = state.instructions.len();
    state.instructions[skip_jump_index] = Instruction::Jump(after_methods);

    // Emit DefineClass instruction — the VM will register the class
    state.instructions.push(Instruction::DefineClass {
        name: class_name.clone(),
        parent: parent.clone(),
    });

    // Store class metadata for Program
    state.class_defs.insert(class_name, (parent, method_info));

    Ok(())
}

fn parse_function_signature(line: &str, line_no: usize) -> Result<(String, Vec<String>), VmError> {
    let rest = line
        .strip_prefix("fn ")
        .or_else(|| line.strip_prefix("kfn "))
        // v2.2: async function declarations
        .or_else(|| line.strip_prefix("async fn "))
        .or_else(|| line.strip_prefix("async kfn "))
        .ok_or_else(|| VmError::parse_error_simple(format!("Line {}: invalid function declaration", line_no)))?
        .trim();

    let open = rest.find('(').ok_or_else(|| {
        VmError::parse_error_simple(format!(
            "Line {}: function declaration must be `fn name(args)`",
            line_no
        ))
    })?;

    let close = rest.rfind(')').ok_or_else(|| {
        VmError::parse_error_simple(format!(
            "Line {}: function declaration must close `)`",
            line_no
        ))
    })?;

    if close <= open {
        return Err(VmError::parse_error_simple(format!(
            "Line {}: invalid function parameter list",
            line_no
        )));
    }

    let raw_name = rest[..open].trim();
    // v2.2: strip generic type parameters — "add<T, U>" → "add"
    let name = if let Some(lt) = raw_name.find('<') {
        raw_name[..lt].trim()
    } else {
        raw_name
    };
    if !is_valid_name(name) {
        return Err(VmError::parse_error_simple(format!(
            "Line {}: invalid function name `{}`",
            line_no, name
        )));
    }

    let after_close = rest[close + 1..].trim();
    // Allow return-type annotation "-> Type" after the closing paren (strip and ignore it)
    let after_close = if let Some(tail) = after_close.strip_prefix("->") {
        // drop everything after "->"
        let _ = tail;
        ""
    } else {
        after_close
    };
    if !after_close.is_empty() {
        return Err(VmError::parse_error_simple(format!(
            "Line {}: unexpected tokens after function signature",
            line_no
        )));
    }

    let params_text = rest[open + 1..close].trim();
    let mut params = Vec::new();
    if !params_text.is_empty() {
        for raw in params_text.split(',') {
            let raw_trim = raw.trim();
            // `x: Type` → `x` ; Killer-style `x int` / `x Type` → first identifier token
            let name_part = if raw_trim.contains(':') {
                raw_trim
                    .splitn(2, ':')
                    .next()
                    .unwrap_or(raw_trim)
                    .trim()
            } else {
                raw_trim
                    .split_whitespace()
                    .next()
                    .unwrap_or(raw_trim)
                    .trim()
            };
            let name_part = name_part.split('<').next().unwrap_or(name_part).trim();
            if !is_valid_name(name_part) {
                return Err(VmError::parse_error_simple(format!(
                    "Line {}: invalid parameter name `{}`",
                    line_no, name_part
                )));
            }
            if params.iter().any(|existing| existing == name_part) {
                return Err(VmError::parse_error_simple(format!(
                    "Line {}: duplicate parameter `{}`",
                    line_no, name_part
                )));
            }
            params.push(name_part.to_string());
        }
    }

    Ok((name.to_string(), params))
}

/// Emit bytecode for `spawn func(arg0, arg1, …)` or `spawn func`.
/// Emits: [compile args] + SpawnCallDirect{target, arg_count}
/// This uses TRUE OS-thread parallel execution (v2.2+).
fn emit_spawn_call(
    body: &str,
    line_no: usize,
    state: &mut CompilerState,
    context: &CompileContext,
) -> Result<(), VmError> {
    if let Some((name, args)) = parse_call_expr(body) {
        // spawn func(a, b)  —  compile args, then SpawnCallDirect
        for arg in &args {
            compile_expr_str(arg, line_no, state, context)?;
        }
        let idx = state.instructions.len();
        if let Some(meta) = state.functions.get(&name) {
            state.instructions.push(Instruction::SpawnCallDirect {
                target: meta.start,
                arg_count: args.len(),
            });
        } else {
            // Forward reference — patch up after all functions compiled
            state.instructions.push(Instruction::SpawnCallDirect {
                target: usize::MAX,
                arg_count: args.len(),
            });
            state.pending_spawn_calls.push(PendingCall {
                instruction_index: idx,
                function_name: name,
                arg_count: args.len(),
                line_no,
            });
        }
    } else {
        // spawn func   (no args)
        let idx = state.instructions.len();
        if let Some(meta) = state.functions.get(body) {
            state.instructions.push(Instruction::SpawnCallDirect {
                target: meta.start,
                arg_count: 0,
            });
        } else {
            state.instructions.push(Instruction::SpawnCallDirect {
                target: usize::MAX,
                arg_count: 0,
            });
            state.pending_spawn_calls.push(PendingCall {
                instruction_index: idx,
                function_name: body.to_string(),
                arg_count: 0,
                line_no,
            });
        }
    }
    Ok(())
}

fn patch_pending_calls(state: &mut CompilerState) -> Result<(), VmError> {
    // All built-in function names recognised by BuiltinFunctions::call()
    // AI functions, stdlib, and core built-ins are all listed here so that
    // patch_pending_calls emits CallBuiltin instead of failing.
    const BUILTINS: &[&str] = &[
        "print", "println", "len", "length", "push", "pop", "contains", "index_of",
        "get", "setdefault", "copy", "reversed",
        "range", "type_of", "to_string", "to_int", "to_float", "keys", "values",
        "sort", "reverse", "join", "split", "trim", "replace", "upper", "lower",
        "starts_with", "ends_with", "substr", "format", "parse_int", "parse_float",
        "abs", "floor", "ceil", "round", "sqrt", "pow",
        "sin", "cos", "tan", "random",
        "bit_and", "bit_or", "bit_xor", "bit_shl", "bit_shr",
        "min", "max",
        "math_eval", "math_eval_subst",
        "time_now", "sleep",
        "time_ms", "sleep_ms",
        "concat", "iterKeys",
        // insert (dict insert, alias for hash_map_insert)
        "insert",
        "hash_map_new",
        "hash_map_insert",
        "hash_map_get",
        "hash_map_contains",
        "hash_map_remove",
        "hash_map_size",
        "hash_map_keys",
        "hash_map_values",
        // assert builtins
        "assert_eq", "assert_ne", "assert_true", "assert_false", "assert_contains", "assert_nil",
        // Phase 1: Trit — balanced ternary
        "T_NEG", "T_ZERO", "T_POS",
        "trit_and", "trit_or", "trit_not", "trit_add", "trit_mul",
        "trit_to_int", "trit_from_int", "int_to_trit", "trit_to_str",
        "trit_word", "trit_word_to_int",
        // Phase 2: Fuzzy logic
        "fuzzy_and", "fuzzy_or", "fuzzy_not", "fuzzy_threshold", "fuzzy_combine",
        // Phase 3: Cognitive Signal
        "signal_create", "signal_value", "signal_confidence", "signal_reason",
        "signal_and", "signal_or", "signal_confident", "signal_uncertain", "signal_to_str",
        // Phase 4: Qubit — quantum simulation
        "qubit_create", "qubit_hadamard", "qubit_pauli_x", "qubit_pauli_z",
        "qubit_measure", "qubit_prob0", "qubit_prob1", "qubit_phase",
        "qubit_to_str", "qubit_entangle",
        // Phase 5: Tryte — 6-trit balanced ternary word
        "tryte_create", "tryte_from_int", "tryte_to_int", "tryte_to_str",
        "tryte_get", "tryte_set", "tryte_and", "tryte_or", "tryte_not",
        "tryte_add", "tryte_eq", "tryte_zero", "tryte_type",
        // type / conversion builtins
        "str", "int", "type", "String", "Number", "Boolean", "parseInt", "parseFloat",
        "isNaN", "isFinite",
        // string builtins
        "substring", "indexOf", "repeat", "charAt", "charCodeAt",
        // array builtins
        "slice", "includes",
        // Python-style: sorted, sum, enumerate, all, any, zip
        "sorted", "sum", "enumerate", "all", "any", "zip",
        // I/O
        "readFile", "writeFile",
        // time / network
        "system_time_ms", "thread_sleep_ms",
        "http_get", "http_post", "parse_json", "json_stringify", "json_pretty",
        "parse_csv", "to_csv", "to_yaml",
        "now", "parse_datetime", "format_datetime",
        // AI functions
        "ai_generate", "ai_embed", "ai_classify", "ai_extract",
        "ai_local_infer", "ai_provider_set", "ai_provider_get",
        "ai_cache_enable", "ai_cache_clear",
        // Native GGUF inference
        "llm_chat", "llm_ask", "llm_info",
        // Ghost Agent (web search + local LLM)
        "ghost_ask",
        "ghost_smart_solve",
        // RLM — Reasoning Language Models (DeepSeek-R1, QwQ)
        "rlm_think", "rlm_answer", "rlm_thinking",
        // Native Think Engine — 100% Killer-native reasoning
        "native_think",
        // Ghost-108 — parallel multi-agent search, fastest wins
        "ghost_108",
        // KhLM — Killer Hybrid Language Model unified router
        "khlm_ask", "khlm_ask_model",
        // KhLM prefetch — background pre-warm cache for instant khlm_ask returns
        "khlm_prefetch",
        // LLM-as-RLM — any LLM becomes a reasoning model via chain-of-thought prompt
        "llm_reason", "llm_reason_answer",
        // User-composable building blocks — create your own LLM/RLM/KhLM in Killer code
        "khlm_classify",     // classify question: "math" | "factual" | "reasoning"
        "khlm_run",          // custom pipeline: "web" | "rlm" | "web+rlm" | "auto"
        "llm_parallel",      // run N questions in parallel → List<String>
        "rlm_synthesize",    // give RLM your own context → synthesized answer
        // KORE — Killer Optimized Record Exchange file format
        "kore_write",        // kore_write(path, schema, data) → String
        "kore_read",         // kore_read(path) → String (JSON)
        "kore_read_col",     // kore_read_col(path, col_name) → String (JSON array)
        "kore_info",         // kore_info(path) → String (metadata)
        // Nova — KORE-B columnar + LZ77 (smallest + fastest, pure Rust stdlib)
        "nova_write",        // nova_write(csv_path, out_path) → Bool
        "nova_info",         // nova_info(path) → String (metadata)
        "nova_read_col",     // nova_read_col(path, col_name) → Array
        "nova_read_all",     // nova_read_all(path) → Map {col→Array}
        "nova_stats",        // nova_stats(path, col_name) → Map {count,min,max,sum,mean,nulls,unique}
        "nova_filter",       // nova_filter(path, col_name, value) → Array of matching row indices
        "nova_to_csv",       // nova_to_csv(kore, csv)   → Bool
        "nova_to_json",      // nova_to_json(kore, json) → Bool
        "nova_to_tsv",       // nova_to_tsv(kore, tsv)   → Bool
        "nova_from_json",    // nova_from_json(json, kore) → Bool
        "nova_from_tsv",     // nova_from_tsv(tsv, kore)   → Bool
        "nova_from_xml",     // nova_from_xml(xml, kore)         → Bool
        "nova_to_xml",       // nova_to_xml(kore, xml)           → Bool
        "nova_to_ndjson",    // nova_to_ndjson(kore, ndjson)     → Bool
        "nova_from_ndjson",  // nova_from_ndjson(ndjson, kore)   → Bool
        "nova_to_avro",      // nova_to_avro(kore, avro)         → Bool
        "nova_from_avro",    // nova_from_avro(avro, kore)       → Bool
        "nova_to_parquet",   // nova_to_parquet(kore, parquet)   → Bool
        "nova_from_parquet", // nova_from_parquet(parquet, kore) → Bool
        "nova_auto_convert", // nova_auto_convert(src, dst)      → Bool  (dot-to-dot syntax)
        "nova_compress",     // nova_compress(src, dst)          → Bool  (.compress. syntax)
        "nova_decompress",   // nova_decompress(src, dst)        → Bool  (.decompress. syntax)
        // Nova Galaxy Engine v1 — Polyglot @lang{} runtime
        "polyglot_exec",     // polyglot_exec(lang, code)        → String
        "polyglot_list",     // polyglot_list()                  → String (detected runtimes)
        "polyglot_check",    // polyglot_check(lang)             → Bool
        // AI Assassin Assist Layer
        "nova_assist_log",        "nova_assist_status",     "nova_assist_debug",
        "nova_assist_optimize",   "nova_assist_enable",     "nova_assist_disable",
        "nova_assist_set_budget", "nova_assist_set_log",    "nova_assist_clear",
        // E: HTTP Client builtins
        "http_get",          // http_get(url)                    → String
        "http_post",         // http_post(url, body)             → String
        "http_post_json",    // http_post_json(url, json)        → String
        "http_head",         // http_head(url)                   → String (headers)
        "http_status",       // http_status(url)                 → Number (200/404/etc)
        "http_download",     // http_download(url, path)         → String
        // A: Streaming polyglot output
        "polyglot_stream",   // polyglot_stream(lang, code)      → String (streams+returns)
        // B: Vector Memory
        "vmem_store",        // vmem_store(key, text)            → String
        "vmem_recall",       // vmem_recall(key)                 → String
        "vmem_search",       // vmem_search(query, topk?)        → String
        "vmem_forget",       // vmem_forget(key)                 → String
        "vmem_list",         // vmem_list()                      → String
        "vmem_stats",        // vmem_stats()                     → String
        "vmem_clear",        // vmem_clear()                     → String
        "vmem_set_threshold",// vmem_set_threshold(f)            → String
        // C: KhLM Tool Calling
        "tool_register",     // tool_register(name, desc)        → String
        "tool_call",         // tool_call(name, args...)         → String
        "tool_list",         // tool_list()                      → String
        "khlm_with_tools",   // khlm_with_tools(prompt)         → String
        "khlm_tool_status",  // khlm_tool_status()              → String
        "khlm_tool_clear",   // khlm_tool_clear()               → String
        // KhLM-Polyglot: 5-tier AI router (CAG → LLM → RLM → Ghost-108)
        "khlm_debug",        // khlm_debug(code, lang, error?)   → String
        "khlm_suggest",      // khlm_suggest(code, lang)         → String
        "khlm_explain",      // khlm_explain(code, lang)         → String
        "khlm_fix",          // khlm_fix(code, error, lang)      → String
        "khlm_translate",    // khlm_translate(code, from, to)   → String
        "khlm_status",       // khlm_status()                    → String
        "khlm_set_llm",      // khlm_set_llm(provider, key, mdl) → String
        "khlm_set_rlm",      // khlm_set_rlm(model_path)         → String
        "khlm_cache_clear",  // khlm_cache_clear()               → String
        // File system
        "nova_file_read",    "nova_file_write",   "nova_file_append",
        "nova_file_exists",  "nova_file_delete",  "nova_file_size",
        "nova_dir_list",     "nova_dir_exists",   "nova_dir_create",
        // KORE data operations
        "nova_select",       "nova_drop_col",     "nova_rename_col",
        "nova_add_col",      "nova_head",         "nova_tail",
        "nova_sort",         "nova_merge",        "nova_join",
        "nova_group_by",     "nova_distinct",     "nova_sample",
        "nova_filter_op",    "nova_fill",         "nova_read_lines",
        "nova_multi_filter", "nova_cast",         "nova_concat",
        "nova_show",
        // Excel and ORC formats
        "nova_to_xlsx",      "nova_from_xlsx",
        "nova_to_orc",       "nova_from_orc",
        // ── IMAGINATION ENGINE — think beyond, counterfactual, conceptual bridges ──
        "imagine",           "imagine_what_if",   "imagine_connect",
        "imagine_beyond",    "imagine_self",
        // ── AFFECT ENGINE — emotional state, feelings, colored responses ──────────
        "affect_sense",      "affect_state",      "affect_color",
        "affect_reset",      "affect_set",
        // ── GUARDIAN ENGINE — Human Protection Principle (Sai Arun Kumar Katherashala)
        "guardian_check",    "guardian_principles",  "guardian_status",
        // Prose Engine
        "khlm_write",
        // Vision Engine
        "image_load",        "image_describe",       "khlm_vision",
        // ── KALA (काल) — Brand aliases (thin wrappers over KhLM engine) ──────────
        "kala_ask",          "kala_think",
        "kala_write",        "kala_vision",          "kala_describe",
        "kala_debug",        "kala_suggest",         "kala_explain",
        "kala_fix",          "kala_translate",
        "kala_imagine",      "kala_what_if",
        "kala_feel",         "kala_guard",
        "kala_status",       "kala_set_llm",         "kala_prefetch",
        // Kala Chat UI server
        "kala_serve",
        // Kala Generator — image & video generation
        "kala_generate_image",
        "kala_generate_video",
        // ── Compression builtins ──
        "compress",          // compress(text, algo) → String     algo: "rle"|"lz77"|"nova"
        "decompress",        // decompress(data, algo) → String
        "b64_encode",        // b64_encode(text) → String
        "b64_decode",        // b64_decode(b64) → String
        "hex_encode",        // hex_encode(text) → String
        "hex_decode",        // hex_decode(hex) → String
        "compress_ratio",    // compress_ratio(orig, comp) → Number
        "compress_info",     // compress_info(text) → Dict
        // ── Debug Intelligence builtins ──
        "debug_check",       // debug_check(code) → Array of issues
        "auto_fix",          // auto_fix(code) → Array of fix candidates
        "explain_error",     // explain_error(msg, ctx?) → String
        "suggest_refactor",  // suggest_refactor(code) → Array
        "auto_test",         // auto_test(code) → String
        "perf_profile",      // perf_profile(code) → Array of hints
        "ai_pair",           // ai_pair(task) → String
        "killer_debug_agent", // killer_debug_agent(code) → Dict
        "watch_value",       // watch_value(expr, val) → Null (debug intelligence)
        "watch_report",      // watch_report() → String
        // ── Linter builtins ──
        "lint",              // lint(code) → String report
        // ── Production Module (regex, help, db, fmt, lint) ──
        "regex_match", "regex_find", "regex_find_all", "regex_replace", "regex_split", "regex_test",
        "help", "help_search", "help_list",
        "db_open", "db_get", "db_set", "db_delete", "db_keys", "db_keys_prefix",
        "db_count", "db_close", "db_drop",
        "fmt", "fmt_file", "lint_code", "lint_file",
        // ── 10x Module (package manager, LSP, DAP, docs) ──
        "pkg_init", "pkg_add", "pkg_remove", "pkg_list", "pkg_resolve",
        "pkg_install", "pkg_info", "pkg_search", "pkg_publish", "pkg_version",
        "lsp_start", "lsp_stop", "lsp_analyze", "lsp_complete", "lsp_hover", "lsp_format",
        "dap_start", "dap_break", "dap_remove_break", "dap_step", "dap_next",
        "dap_continue", "dap_vars", "dap_stack", "dap_eval", "dap_stop", "dap_list_breaks",
        "docs_generate", "docs_serve", "docs_search", "docs_api", "docs_export",
        // ── Improve Module (errors, imports, watch, stack, REPL, perf, doc comments) ──
        "error_enhance", "suggest",
        "import", "import_list", "import_clear",
        "watch", "watch_dir",
        "stack_push", "stack_pop", "stack_trace", "stack_clear",
        "repl_complete", "repl_complete_sig",
        "bench_run", "bench_all", "bench_save", "bench_compare",
        "doc_parse", "doc_check",
        "ui_core_version", "ui_headless_tick", "ui_headless_snapshot_json", "ui_health", "ui_help", "ui_native_window",
        // ── Android Modules ──
        "mic_init", "mic_start", "mic_stop", "mic_status", "mic_save",
        "mic_encode_m4a", "mic_set_quality", "mic_get_level",
        "call_detect_start", "call_detect_stop", "call_detect_status",
        "phone_contacts", "phone_call_log", "phone_info",
        "phone_sms_send", "phone_sms_read", "phone_battery",
        "phone_network", "phone_volume_set", "phone_vibrate",
        "service_start_foreground", "service_stop_foreground", "service_is_running",
        "service_notification", "service_notification_update",
        "service_auto_start_enable", "service_auto_start_disable",
        "service_wake_lock", "service_wake_unlock",
        "service_request_permissions", "service_check_permissions",
        "service_schedule_task", "service_cancel_task",
        "secure_encrypt", "secure_decrypt", "secure_hash", "secure_hash_file",
        "secure_random_bytes", "secure_pin_set", "secure_pin_verify", "secure_pin_is_set",
        "secure_lock", "secure_is_locked", "secure_unlock",
        "secure_check_integrity", "evidence_hash", "secure_wipe_recordings",
        // ── File I/O aliases ──
        "file_read", "file_write", "file_exists", "file_delete", "file_append",
        "readFile", "writeFile",
        // Interactive I/O
        "readline", "readline_prompt",
        // ── v2.3: OS-Level Primitives ──
        "bit_not", "bit_rotl", "bit_rotr",
        "to_integer", "to_bytes", "to_pointer",
        "bytes_new", "bytes_len", "bytes_get", "bytes_set",
        "bytes_slice", "bytes_from_str", "bytes_to_str", "bytes_concat", "bytes_fill",
        "ptr_new", "ptr_to_int", "ptr_offset",
        "mem_read_u8", "mem_read_u16", "mem_read_u32", "mem_read_u64",
        "mem_write_u8", "mem_write_u16", "mem_write_u32", "mem_write_u64",
        "volatile_read_u8", "volatile_read_u16", "volatile_read_u32", "volatile_read_u64",
        "volatile_write_u8", "volatile_write_u16", "volatile_write_u32", "volatile_write_u64",
        "io_port_in_u8", "io_port_in_u16", "io_port_out_u8", "io_port_out_u16",
        "sha256", "sha256_bytes",
        "mmap_alloc", "mmap_free", "mmap_write", "mmap_read", "mmap_exec",
        "int_to_bytes_le", "int_to_bytes_be", "bytes_to_int_le", "bytes_to_int_be",
        "cli_args", "env_get", "env_set", "process_exit", "errno",
        "sizeof", "alignof",
        "atomic_load", "atomic_store", "atomic_cas", "atomic_add",
        "interrupts_disable", "interrupts_enable", "wfi", "fence",
        "disk_read_block", "disk_write_block",
        "page_alloc", "page_free", "page_map",
        // v2.3: OS-level hardware primitives
        "cpuid", "rdtsc", "gdt_encode", "idt_encode", "call_native",
    ];

    for pending in &state.pending_calls {
        if BUILTINS.contains(&pending.function_name.as_str()) {
            // -- Nova Galaxy: emit native opcodes instead of CallBuiltin ------
            let native_instr: Option<Instruction> = match pending.function_name.as_str() {
                // Phase A: Trit native ALU opcodes
                "T_POS"             => Some(Instruction::ConstTrit(1)),
                "T_NEG"             => Some(Instruction::ConstTrit(-1)),
                "T_ZERO"            => Some(Instruction::ConstTrit(0)),
                "trit_and"          => Some(Instruction::TritAnd),
                "trit_or"           => Some(Instruction::TritOr),
                "trit_not"          => Some(Instruction::TritNot),
                "trit_add"          => Some(Instruction::TritAdd),
                "trit_mul"          => Some(Instruction::TritMul),
                "int_to_trit"       => Some(Instruction::IntToTrit),
                "trit_from_int"     => Some(Instruction::IntToTrit),
                "trit_to_int"       => Some(Instruction::TritToInt),
                // Phase B: Signal fast-path opcodes
                "signal_value"      => Some(Instruction::SignalGetValue),
                "signal_confidence" => Some(Instruction::SignalGetConfidence),
                "signal_reason"     => Some(Instruction::SignalGetReason),
                // Phase C: Qubit native gate opcodes
                "qubit_hadamard"    => Some(Instruction::QubitHadamard),
                "qubit_pauli_x"     => Some(Instruction::QubitPauliX),
                "qubit_measure"     => Some(Instruction::QubitMeasure),
                // Phase D: Fuzzy float native opcodes
                "fuzzy_and"         => Some(Instruction::FuzzyAnd),
                "fuzzy_or"          => Some(Instruction::FuzzyOr),
                "fuzzy_not"         => Some(Instruction::FuzzyNot),
                // Phase E: Tryte native ALU opcodes
                "tryte_and"         => Some(Instruction::TryteAnd),
                "tryte_or"          => Some(Instruction::TryteOr),
                "tryte_not"         => Some(Instruction::TryteNot),
                "tryte_add"         => Some(Instruction::TryteAdd),
                _                   => None,
            };
            if let Some(instr) = native_instr {
                state.instructions[pending.instruction_index] = instr;
                continue;
            }
            state.instructions[pending.instruction_index] =
                Instruction::CallBuiltin(pending.function_name.clone(), pending.arg_count);
            continue;
        }

        let Some(meta) = state.functions.get(&pending.function_name) else {
            return Err(VmError::parse_error_simple(format!(
                "Line {}: unknown function `{}`",
                pending.line_no, pending.function_name
            )));
        };

        state.instructions[pending.instruction_index] = Instruction::Call {
            target: meta.start,
            arg_count: pending.arg_count,
        };
    }

    // v2.2: patch SpawnCallDirect forward references
    for pending in &state.pending_spawn_calls {
        let Some(meta) = state.functions.get(&pending.function_name) else {
            return Err(VmError::parse_error_simple(format!(
                "Line {}: unknown function `{}` in spawn",
                pending.line_no, pending.function_name
            )));
        };
        state.instructions[pending.instruction_index] = Instruction::SpawnCallDirect {
            target: meta.start,
            arg_count: pending.arg_count,
        };
    }
    Ok(())
}

fn extract_keyword_condition<'a>(line: &'a str, keyword: &str, line_no: usize) -> Result<&'a str, VmError> {
    let prefix = format!("{}", keyword);
    if !line.starts_with(&prefix) {
        return Err(VmError::parse_error_simple(format!(
            "Line {}: expected `{}` statement",
            line_no, keyword
        )));
    }

    let rest = line[prefix.len()..].trim();

    // Accept both `if (cond)` and `if cond` — strip optional outer parens
    let condition = if rest.starts_with('(') && rest.ends_with(')') {
        // Check balanced: only strip if the opening ( matches the closing )
        let inner = &rest[1..rest.len()-1];
        let mut depth = 0i32;
        let mut balanced = true;
        for ch in inner.chars() {
            match ch {
                '(' => depth += 1,
                ')' => { depth -= 1; if depth < 0 { balanced = false; break; } }
                _ => {}
            }
        }
        if balanced { inner.trim() } else { rest }
    } else {
        rest
    };

    if condition.is_empty() {
        return Err(VmError::parse_error_simple(format!(
            "Line {}: `{}` requires a condition",
            line_no, keyword
        )));
    }

    Ok(condition)
}

/// Strip trailing `-- …` from a line, only outside `"…"` strings.
/// Note: `//` is the integer-division operator (like Python), NOT a comment.
/// Use `--` for inline comments instead.
fn strip_trailing_line_comment(content: &str) -> &str {
    let mut in_str = false;
    let end;
    let bytes = content.as_bytes();
    let mut j = 0;
    loop {
        if j >= bytes.len() {
            end = content.len();
            break;
        }
        match bytes[j] {
            b'"' => in_str = !in_str,
            b'-' if !in_str && j + 1 < bytes.len() && bytes[j + 1] == b'-' => {
                end = j;
                break;
            }
            b'/' if !in_str && j + 1 < bytes.len() && bytes[j + 1] == b'/' => {
                end = j;
                break;
            }
            _ => {}
        }
        j += 1;
    }
    content[..end].trim()
}

// --- Nova Galaxy Engine v1: @lang{} Pre-processor ----------------------------
//
// Problem: normalize_lines() splits ALL { and } onto their own lines, which
// destroys embedded foreign-language code (e.g. Go requires `{` on the same
// line as `func`).
//
// Solution: walk the raw source BEFORE normalize_lines() and replace every
//   @lang { … }      or     name = @lang { … }
// block with a single synthetic line:
//   polyglot_exec("lang", "escaped_code")   or
//   name = polyglot_exec("lang", "escaped_code")
//
// The synthetic line is safe because the code is inside a quoted string,
// which normalize_lines() leaves intact.

/// Pre-process indentation-based Killer syntax into brace-delimited form.
///
/// Handles:
/// - `--` and `//` line comments (full-line ones converted to `#` so normalize_lines can skip them)
/// - Indentation-based block delimiters (Python-style offside rule → `{ }`)
///
/// Block openers: `kfn`, `fn`, `async kfn`, `async fn`, `if`, `else if`,
///                `else`, `while`, `for ... in ...`, `for ... of ...`
/// The source is considered indentation-based when it contains `kfn` or `fn`
/// declarations WITHOUT an opening brace on/after the signature line.
/// Top-level scripts (no `fn`/`kfn`) always run the offside pass so Python-style
/// `if` / `while` / `for` with indented bodies get `{` / `}` inserted.
fn preprocess_indentation(source: &str) -> String {
    // Fast-path: if the source already uses braces (brace-style), return unchanged.
    // We detect this by checking if any `kfn`/`fn` line is immediately followed
    // by a `{` (either same-line inline or on the very next non-empty line).
    {
        let mut lines_iter = source.lines().peekable();
        let mut has_brace_style = false;
        while let Some(line) = lines_iter.next() {
            let t = line.trim();
            if t.starts_with("kfn ") || t.starts_with("fn ") ||
               t.starts_with("async kfn ") || t.starts_with("async fn ") {
                // Check same-line brace
                if t.ends_with('{') || t.contains(") {") {
                    has_brace_style = true;
                    break;
                }
                // Check next non-empty line
                let mut found_brace = false;
                for next_line in lines_iter.by_ref() {
                    let nt = next_line.trim();
                    if nt.is_empty() { continue; }
                    if nt == "{" { found_brace = true; }
                    break;
                }
                if found_brace { has_brace_style = true; break; }
            }
        }
        // Only skip the offside-rule pass when every `fn`/`kfn` already uses `{` — so brace-style
        // projects stay byte-identical (aside from comment stripping).
        //
        // Scripts with **no** `fn`/`kfn` used to hit `!has_fn` and skip indentation conversion too,
        // which broke Python-style `if` / `while` / `for` with multi-line bodies at top level.
        if has_brace_style {
            let cleaned: String = source.lines()
                .map(|l| {
                    let t = l.trim();
                    if t.starts_with("--") || t.starts_with("//") {
                        "#".to_string()
                    } else {
                        strip_trailing_line_comment(l).to_string()
                    }
                })
                .collect::<Vec<_>>()
                .join("\n");
            return cleaned;
        }
    }

    // ---- Indentation-based mode ----
    // First pass: strip `--` / `//` comments and collect (indent_level, content) pairs.
    let raw_lines: Vec<(usize, &str)> = source.lines()
        .map(|line| {
            let indent = line.len() - line.trim_start().len();
            let content = line.trim();
            (indent, content)
        })
        .collect();

    // Helpers to check if a line is a block opener
    fn is_block_opener(s: &str) -> bool {
        let t = s.trim();
        t.starts_with("kfn ") || t.starts_with("fn ") ||
        t.starts_with("async kfn ") || t.starts_with("async fn ") ||
        t.starts_with("if ") || t.starts_with("if(") || t == "else" ||
        t.starts_with("else if ") || t.starts_with("else if(") || t.starts_with("elif ") ||
        t.starts_with("while ") || t.starts_with("while(") ||
        (t.starts_with("for ") && (t.contains(" in ") || t.contains(" of ")))
    }

    /// True if this line already opens a `{` block (`if x {`, `fn f() {`, …). Offside `{` insertion
    /// and indent stack must be skipped so we do not double-close with synthetic `}`.
    fn line_already_has_block_brace(s: &str) -> bool {
        let t = s.trim_end();
        t.ends_with('{') || t.contains(") {") || t.contains("){")
    }

    fn is_fn_signature_opener(s: &str) -> bool {
        let t = s.trim();
        t.starts_with("kfn ") || t.starts_with("fn ")
            || t.starts_with("async kfn ")
            || t.starts_with("async fn ")
    }

    // Second pass: insert `{` and `}` based on indentation changes.
    // Strategy (like Python's offside rule):
    //   - Maintain a stack of indent levels that opened blocks.
    //   - When a block-opener line is seen, the NEXT non-empty line's indent
    //     defines the block indent. Push that indent, emit `{`.
    //   - When current indent < top of stack, pop and emit `}` until stacks match.
    let mut out_lines: Vec<String> = Vec::new();
    let mut indent_stack: Vec<usize> = vec![0]; // base indent = 0

    // We need to look ahead one line to determine block indent after an opener.
    let n = raw_lines.len();
    let mut i = 0;

    while i < n {
        let (indent, content) = raw_lines[i];

        // Skip blank lines
        if content.is_empty() {
            i += 1;
            continue;
        }

        // Strip full-line `--` / `//` comments
        if content.starts_with("--") || content.starts_with("//") {
            i += 1;
            continue;
        }

        // Inline `--` or `//` on a code line: strip from first marker not inside string
        let content = strip_trailing_line_comment(content);

        if content.is_empty() {
            i += 1;
            continue;
        }

        // Close blocks if we've de-dented
        while indent < *indent_stack.last().unwrap_or(&0) {
            indent_stack.pop();
            out_lines.push("}".to_string());
        }

        // Emit the content line
        out_lines.push(content.to_string());

        // If this is a block-opener, find the next non-empty non-comment line's indent
        // and push that as the new block indent, emitting `{` for Python-style bodies only.
        if is_block_opener(content) {
            if line_already_has_block_brace(content) {
                // Brace-delimited block — not offside.
            } else if is_fn_signature_opener(content)
                && (paren_depth_net(content) != 0 || square_bracket_depth(content) != 0)
            {
                // `kfn name(` continues on the next lines — do not inject `{` before `)` closes.
            } else if control_flow_skip_synthetic_open_brace(content, &raw_lines, i) {
                // `if (` / `if expr` + `&& …` / `for v of a[` … — header spans lines; do not inject `{` yet.
            } else {
                let mut next_indent = indent + 2; // default if no lookahead found
                for look in (i + 1)..n {
                    let (li, lc) = raw_lines[look];
                    if lc.is_empty() || lc.starts_with("--") || lc.starts_with("//") {
                        continue;
                    }
                    let lc_trimmed = strip_trailing_line_comment(lc);
                    if lc_trimmed.is_empty() {
                        continue;
                    }
                    next_indent = li;
                    break;
                }
                if next_indent > indent {
                    indent_stack.push(next_indent);
                    out_lines.push("{".to_string());
                }
            }
            // else: no body (definition with no body follows — edge case, skip brace)
        }

        i += 1;
    }

    // Close any remaining open blocks
    while indent_stack.len() > 1 {
        indent_stack.pop();
        out_lines.push("}".to_string());
    }

    out_lines.join("\n")
}

/// Pre-process @lang{} blocks before normalize_lines().
fn preprocess_polyglot(source: &str) -> String {
    let src_lines: Vec<&str> = source.lines().collect();
    let mut out = String::new();
    let mut i = 0;

    while i < src_lines.len() {
        let raw = src_lines[i];
        let trimmed = raw.trim();

        // Pass blanks and comments through unchanged
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with("//") {
            out.push_str(raw);
            out.push('\n');
            i += 1;
            continue;
        }

        // Try to detect `@lang` annotation (with optional `name =` prefix)
        if let Some((assign_target, lang, rest_after_lang)) = parse_polyglot_header(trimmed) {
            let rest = rest_after_lang.trim();

            // Locate the opening brace
            // Case A: `@lang { ...` — `{` is on the same line right after the ident
            // Case B: `@lang` alone — `{` may be on the very next line (normalize style)
            // Anything else is not a polyglot block.
            let (code, extra_lines) = if rest.starts_with('{') {
                // Opening brace on same line — collect from right after `{`
                let after_brace = &rest[1..];
                collect_polyglot_body(after_brace, &src_lines, i + 1)
            } else if rest.is_empty() {
                // Check immediately next line for a lone `{`
                let next_i = i + 1;
                if next_i < src_lines.len() && src_lines[next_i].trim() == "{" {
                    let (code, consumed) = collect_polyglot_body("", &src_lines, next_i + 1);
                    (code, 1 + consumed) // +1 for the lone `{` line
                } else {
                    // @lang without a block — not polyglot, copy verbatim
                    out.push_str(raw);
                    out.push('\n');
                    i += 1;
                    continue;
                }
            } else {
                // `@lang` followed by something that's not `{` — not polyglot
                out.push_str(raw);
                out.push('\n');
                i += 1;
                continue;
            };

            // Emit the synthetic single-line replacement
            let escaped = polyglot_escape(&code);
            if assign_target.is_empty() {
                // Statement form — auto-print the captured output via a temp var
                // (polyglot_exec captures stdout; discarding would silence all output)
                out.push_str(&format!(
                    "__nova_out__ = polyglot_exec(\"{}\", \"{}\")\nprint(__nova_out__)\n",
                    lang, escaped
                ));
            } else {
                out.push_str(&format!(
                    "{} = polyglot_exec(\"{}\", \"{}\")\n",
                    assign_target, lang, escaped
                ));
            }
            i += 1 + extra_lines; // current @lang line + consumed body lines
        } else {
            out.push_str(raw);
            out.push('\n');
            i += 1;
        }
    }
    out
}

/// Parse a line that may begin a polyglot block.
/// Returns `(assign_target, lang_name, rest_of_line_after_lang_ident)` or None.
/// `assign_target` is "" for statement form (`@lang …`) and "varname" for
/// assignment form (`varname = @lang …`).
fn parse_polyglot_header(line: &str) -> Option<(String, String, String)> {
    let at_pos = line.find('@')?;
    let prefix  = &line[..at_pos];
    let after_at = &line[at_pos + 1..];

    // Read the language identifier
    let lang_end = after_at
        .find(|c: char| !c.is_alphanumeric() && c != '_')
        .unwrap_or(after_at.len());
    if lang_end == 0 {
        return None; // bare `@` with no ident
    }
    let lang = after_at[..lang_end].to_string();
    let rest = after_at[lang_end..].trim().to_string();

    // Validate prefix: must be empty OR end with `=` after a valid identifier
    let prefix_trimmed = prefix.trim();
    let assign_target = if prefix_trimmed.is_empty() {
        String::new()
    } else {
        if !prefix_trimmed.ends_with('=') { return None; }
        let name = prefix_trimmed[..prefix_trimmed.len() - 1].trim().to_string();
        if name.is_empty() || !name.chars().all(|c| c.is_alphanumeric() || c == '_') {
            return None;
        }
        name
    };

    Some((assign_target, lang, rest))
}

/// Collect the body of a `@lang { … }` block, preserving the original text.
///
/// `first_segment` — text right after the opening `{` on the same line (may be "").
/// `lines`         — all source lines.
/// `start`         — index into `lines` to continue from if needed.
///
/// Tracks brace depth (outside strings) so nested `{` / `}` in the foreign
/// language are counted correctly.  Stops when depth returns to 0 (the
/// matching outer `}`).
///
/// Returns `(code_text, extra_lines_consumed)`.
fn collect_polyglot_body(
    first_segment: &str,
    lines: &[&str],
    start: usize,
) -> (String, usize) {
    let mut parts: Vec<String> = Vec::new();
    let mut depth = 1usize; // already inside the opening `{`
    let mut in_str = false;
    let mut prev_bs = false;

    // Process one chunk of text; appends non-outer-close content to `parts`;
    // returns true when the outer `}` (depth → 0) is found.
    let mut process = |chunk: &str, parts: &mut Vec<String>| -> bool {
        let mut piece = String::new();
        for ch in chunk.chars() {
            if in_str {
                piece.push(ch);
                if ch == '"' && !prev_bs { in_str = false; }
                prev_bs = ch == '\\' && !prev_bs;
                continue;
            }
            prev_bs = false;
            match ch {
                '"' => { in_str = true;  piece.push(ch); }
                '{' => { depth += 1;     piece.push(ch); }
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        // Outer close found — flush and stop
                        let t = piece.trim().to_string();
                        if !t.is_empty() { parts.push(t); }
                        return true;
                    }
                    piece.push(ch);
                }
                c   => piece.push(c),
            }
        }
        let t = piece.trim().to_string();
        if !t.is_empty() { parts.push(t); }
        false
    };

    // Process the first segment (same line as `{`)
    if process(first_segment, &mut parts) {
        return (parts.join("\n"), 0);
    }

    // Process subsequent lines
    for (offset, &line) in lines[start..].iter().enumerate() {
        if process(line.trim(), &mut parts) {
            return (parts.join("\n"), offset + 1);
        }
    }

    (parts.join("\n"), lines.len().saturating_sub(start))
}

/// Escape a raw code string for embedding inside a Killer string literal.
fn polyglot_escape(code: &str) -> String {
    code.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "")
}

/// Byte index one past a matching `}` for `{` at `open_idx`, same line only (`None` if unclosed).
/// Respects `"` strings and nested `{}`.
fn consume_balanced_curly_line(s: &str, open_idx: usize) -> Option<usize> {
    let bytes = s.as_bytes();
    if bytes.get(open_idx) != Some(&b'{') {
        return None;
    }
    let mut depth = 0i32;
    let mut in_string = false;
    let mut i = open_idx;
    while i < bytes.len() {
        if in_string {
            if bytes[i] == b'\\' && i + 1 < bytes.len() {
                i += 2;
                continue;
            }
            if bytes[i] == b'"' {
                in_string = false;
            }
            i += 1;
            continue;
        }
        match bytes[i] {
            b'"' => {
                in_string = true;
                i += 1;
            }
            b'{' => {
                depth += 1;
                i += 1;
            }
            b'}' => {
                depth -= 1;
                i += 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => i += 1,
        }
    }
    None
}

/// Tier 4 — line sugar: `ui native_window` → `ui_native_window()`, `x = ui version` → `x = ui_core_version()`, etc.
fn preprocess_ui_sugar(source: &str) -> String {
    let mut lines: Vec<String> = Vec::new();
    for line in source.lines() {
        if let Some(changed) = try_ui_assign_sugar(line) {
            lines.push(changed);
            continue;
        }
        let trimmed = line.trim();
        let replacement = match trimmed {
            "ui native_window" => Some("ui_native_window()"),
            "ui headless_tick" => Some("ui_headless_tick()"),
        "ui snapshot_json" | "ui snapshot" => Some("ui_headless_snapshot_json()"),
        "ui tick" => Some("ui_headless_tick()"),
        "ui version" => Some("ui_core_version()"),
        "ui help" => Some("ui_help()"),
        "ui health" => Some("ui_health()"),
        _ => None,
        };
        if let Some(rep) = replacement {
            let prefix_len = line.len().saturating_sub(line.trim_start().len());
            let indent = &line[..prefix_len];
            lines.push(format!("{indent}{rep}"));
        } else {
            lines.push(line.to_string());
        }
    }
    let mut out = lines.join("\n");
    if source.ends_with('\n') {
        out.push('\n');
    }
    out
}

/// `name = ui snapshot` → `name = ui_headless_snapshot_json()` (and `ui version`, `ui help`, …).
fn try_ui_assign_sugar(raw_line: &str) -> Option<String> {
    let t = raw_line.trim();
    let parts: Vec<&str> = t.split_whitespace().collect();
    if parts.len() < 4 || parts[1] != "=" {
        return None;
    }
    let var = parts[0];
    let rhs = parts[2..].join(" ");
    let rhs_call = match rhs.as_str() {
        "ui version" => "ui_core_version()",
        "ui snapshot" | "ui snapshot_json" => "ui_headless_snapshot_json()",
        "ui headless_tick" | "ui tick" => "ui_headless_tick()",
        "ui native_window" => "ui_native_window()",
        "ui help" => "ui_help()",
        "ui health" => "ui_health()",
        _ => return None,
    };
    let prefix_len = raw_line.len().saturating_sub(raw_line.trim_start().len());
    let indent = &raw_line[..prefix_len];
    Some(format!("{indent}{var} = {rhs_call}"))
}

fn normalize_lines(source: &str) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    for (line_index, raw) in source.lines().enumerate() {
        let line_no = line_index + 1;
        let trimmed = raw.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with("//") {
            continue;
        }

        let mut current = String::new();
        let mut in_string = false; // inside "..." or K"..."
        let mut in_backtick = false; // inside `...`
        let mut i = 0usize;
        while i < trimmed.len() {
            let rest = &trimmed[i..];
            let ch = rest.chars().next().unwrap();
            let clen = ch.len_utf8();

            if in_backtick {
                current.push(ch);
                if ch == '`' {
                    in_backtick = false;
                }
                i += clen;
                continue;
            }
            if in_string {
                current.push(ch);
                if ch == '"' {
                    in_string = false;
                } else if ch == '\\' {
                    if let Some(nc) = rest[clen..].chars().next() {
                        current.push(nc);
                        i += clen + nc.len_utf8();
                        continue;
                    }
                }
                i += clen;
                continue;
            }

            match ch {
                '"' => {
                    in_string = true;
                    current.push(ch);
                    i += clen;
                }
                '`' => {
                    in_backtick = true;
                    current.push(ch);
                    i += clen;
                }
                '{' => {
                    if rest[clen..].starts_with('}') {
                        current.push_str("{}");
                        i += clen + 1;
                        continue;
                    }
                    if let Some(end) = consume_balanced_curly_line(trimmed, i) {
                        current.push_str(&trimmed[i..end]);
                        i = end;
                        continue;
                    }
                    if !current.trim().is_empty() {
                        out.push((line_no, current.trim().to_string()));
                    }
                    out.push((line_no, "{".to_string()));
                    current.clear();
                    i += clen;
                }
                '}' => {
                    if !current.trim().is_empty() {
                        out.push((line_no, current.trim().to_string()));
                    }
                    out.push((line_no, "}".to_string()));
                    current.clear();
                    i += clen;
                }
                _ => {
                    current.push(ch);
                    i += clen;
                }
            }
        }

        if !current.trim().is_empty() {
            out.push((line_no, current.trim().to_string()));
        }
    }
    out
}

fn extract_call_arg<'a>(line: &'a str, name: &str) -> Option<&'a str> {
    let prefix = format!("{}(", name);
    if line.starts_with(&prefix) && line.ends_with(')') {
        return Some(&line[prefix.len()..line.len() - 1]);
    }
    None
}

fn is_valid_name(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };

    if !(first.is_ascii_alphabetic() || first == '_') {
        return false;
    }

    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}
