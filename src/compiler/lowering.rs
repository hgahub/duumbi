//! Cranelift IR lowering — graph nodes to Cranelift instructions.
//!
//! Compiles a validated `SemanticGraph` into a native object file
//! using the Cranelift code generator. Supports multi-function,
//! multi-block compilation with f64/bool types, Compare, Branch,
//! Call, Load, and Store operations.

use std::collections::{HashMap, HashSet};

use cranelift_codegen::ir::condcodes::{FloatCC, IntCC};
use cranelift_codegen::ir::types;
use cranelift_codegen::ir::{AbiParam, InstBuilder, Value};
use cranelift_codegen::settings::{self, Configurable};
use cranelift_codegen::{self, Context, isa};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext, Variable};
use cranelift_module::{FuncId, Linkage, Module};
use cranelift_object::{ObjectBuilder, ObjectModule};
use petgraph::visit::EdgeRef;
use target_lexicon::Triple;

use crate::graph::program::Program;
use crate::graph::{FunctionInfo, GraphEdge, SemanticGraph};
use crate::types::{CompareOp, DuumbiType, NodeId, Op};

use super::CompileError;

/// Converts a `DuumbiType` to a Cranelift IR type.
fn duumbi_type_to_cl(ty: DuumbiType) -> cranelift_codegen::ir::Type {
    match ty {
        DuumbiType::I64 => types::I64,
        DuumbiType::F64 => types::F64,
        DuumbiType::Bool => types::I8,
        DuumbiType::Void => types::I64, // should not be used for values
    }
}

/// Converts a `CompareOp` to Cranelift integer condition code.
fn compare_op_to_intcc(op: &CompareOp) -> IntCC {
    match op {
        CompareOp::Eq => IntCC::Equal,
        CompareOp::Ne => IntCC::NotEqual,
        CompareOp::Lt => IntCC::SignedLessThan,
        CompareOp::Le => IntCC::SignedLessThanOrEqual,
        CompareOp::Gt => IntCC::SignedGreaterThan,
        CompareOp::Ge => IntCC::SignedGreaterThanOrEqual,
    }
}

/// Converts a `CompareOp` to Cranelift float condition code.
fn compare_op_to_floatcc(op: &CompareOp) -> FloatCC {
    match op {
        CompareOp::Eq => FloatCC::Equal,
        CompareOp::Ne => FloatCC::NotEqual,
        CompareOp::Lt => FloatCC::LessThan,
        CompareOp::Le => FloatCC::LessThanOrEqual,
        CompareOp::Gt => FloatCC::GreaterThan,
        CompareOp::Ge => FloatCC::GreaterThanOrEqual,
    }
}

/// Creates the ISA and ObjectModule shared setup.
fn create_object_module() -> Result<ObjectModule, CompileError> {
    let triple = Triple::host();

    let mut settings_builder = settings::builder();
    settings_builder
        .set("opt_level", "speed")
        .map_err(|e| CompileError::Cranelift {
            message: format!("Failed to set opt_level: {e}"),
        })?;
    settings_builder
        .set("is_pic", "true")
        .map_err(|e| CompileError::Cranelift {
            message: format!("Failed to set is_pic: {e}"),
        })?;

    let flags = settings::Flags::new(settings_builder);

    let isa = isa::lookup(triple.clone())
        .map_err(|e| CompileError::Cranelift {
            message: format!("ISA lookup failed: {e}"),
        })?
        .finish(flags)
        .map_err(|e| CompileError::Cranelift {
            message: format!("ISA finish failed: {e}"),
        })?;

    let obj_builder = ObjectBuilder::new(
        isa.clone(),
        "duumbi_output",
        cranelift_module::default_libcall_names(),
    )
    .map_err(|e| CompileError::ObjectEmission {
        message: format!("ObjectBuilder creation failed: {e}"),
    })?;

    Ok(ObjectModule::new(obj_builder))
}

/// Declares a function signature in the Cranelift module.
fn make_func_signature(
    obj_module: &ObjectModule,
    func_info: &FunctionInfo,
) -> cranelift_codegen::ir::Signature {
    let mut sig = obj_module.make_signature();
    for param in &func_info.params {
        sig.params
            .push(AbiParam::new(duumbi_type_to_cl(param.param_type)));
    }
    if func_info.return_type != DuumbiType::Void {
        sig.returns
            .push(AbiParam::new(duumbi_type_to_cl(func_info.return_type)));
    }
    sig
}

/// Compiles a validated semantic graph to a native object file.
///
/// Returns the raw bytes of the object file (Mach-O on macOS, ELF on Linux).
/// For multi-module programs use [`compile_program`] instead.
#[must_use = "compilation errors should be handled"]
pub fn compile_to_object(graph: &SemanticGraph) -> Result<Vec<u8>, CompileError> {
    compile_to_object_impl(graph, &HashSet::new(), &[])
}

/// Compiles a multi-module [`Program`] to per-module native object files.
///
/// Returns a map of `module_name → object_bytes`. Each exported function
/// receives `Linkage::Export`; each cross-module callee is declared with
/// `Linkage::Import` so the linker can resolve the symbol.
///
/// # Errors
///
/// Returns [`CompileError`] if more than one module defines `main`, or if
/// Cranelift IR generation fails for any module.
#[allow(dead_code)] // Called by CLI in upcoming phase (#61)
#[must_use = "compilation errors should be handled"]
pub fn compile_program(program: &Program) -> Result<HashMap<String, Vec<u8>>, CompileError> {
    // Build a cross-module function info lookup: fn_name → &FunctionInfo
    let all_fn_info: HashMap<&str, &FunctionInfo> = program
        .modules
        .values()
        .flat_map(|sg| sg.functions.iter().map(|fi| (fi.name.0.as_str(), fi)))
        .collect();

    // Validate: at most one module may define `main`
    let main_count = program
        .modules
        .values()
        .filter(|sg| sg.functions.iter().any(|f| f.name.0 == "main"))
        .count();
    if main_count > 1 {
        return Err(CompileError::Cranelift {
            message: "Multiple modules define 'main': only one entry module is allowed".to_string(),
        });
    }

    let mut objects: HashMap<String, Vec<u8>> = HashMap::new();

    for (module_name, sg) in &program.modules {
        // Functions exported by this module
        let exported_fns: HashSet<String> = program
            .exports
            .iter()
            .filter(|(_, mn)| mn.0 == module_name.0)
            .map(|(fn_name, _)| fn_name.0.clone())
            .collect();

        // Local function names defined in this module
        let local_fn_names: HashSet<&str> =
            sg.functions.iter().map(|f| f.name.0.as_str()).collect();

        // Cross-module calls: Call ops targeting functions not in this module
        let mut imported: HashMap<String, &FunctionInfo> = HashMap::new();
        for node in sg.graph.node_weights() {
            if let Op::Call { function } = &node.op
                && !local_fn_names.contains(function.as_str())
                && !imported.contains_key(function.as_str())
                && let Some(fi) = all_fn_info.get(function.as_str())
            {
                imported.insert(function.clone(), fi);
            }
        }

        let imported_list: Vec<(&str, &FunctionInfo)> =
            imported.iter().map(|(n, fi)| (n.as_str(), *fi)).collect();

        let obj_bytes = compile_to_object_impl(sg, &exported_fns, &imported_list)?;
        objects.insert(module_name.0.clone(), obj_bytes);
    }

    Ok(objects)
}

/// Internal implementation shared by [`compile_to_object`] and [`compile_program`].
///
/// - `exported_fns`: function names in this module that get `Linkage::Export`
///   (in addition to `main` which is always exported).
/// - `imported_fns`: `(name, FunctionInfo)` pairs for functions defined in other
///   modules; declared with `Linkage::Import` so the linker resolves them.
fn compile_to_object_impl(
    graph: &SemanticGraph,
    exported_fns: &HashSet<String>,
    imported_fns: &[(&str, &FunctionInfo)],
) -> Result<Vec<u8>, CompileError> {
    let mut obj_module = create_object_module()?;

    // Declare external print functions
    let mut print_i64_sig = obj_module.make_signature();
    print_i64_sig.params.push(AbiParam::new(types::I64));
    let print_i64_id = obj_module
        .declare_function("duumbi_print_i64", Linkage::Import, &print_i64_sig)
        .map_err(|e| CompileError::Cranelift {
            message: format!("Failed to declare duumbi_print_i64: {e}"),
        })?;

    let mut print_f64_sig = obj_module.make_signature();
    print_f64_sig.params.push(AbiParam::new(types::F64));
    let print_f64_id = obj_module
        .declare_function("duumbi_print_f64", Linkage::Import, &print_f64_sig)
        .map_err(|e| CompileError::Cranelift {
            message: format!("Failed to declare duumbi_print_f64: {e}"),
        })?;

    let mut print_bool_sig = obj_module.make_signature();
    print_bool_sig.params.push(AbiParam::new(types::I8));
    let print_bool_id = obj_module
        .declare_function("duumbi_print_bool", Linkage::Import, &print_bool_sig)
        .map_err(|e| CompileError::Cranelift {
            message: format!("Failed to declare duumbi_print_bool: {e}"),
        })?;

    let mut func_ids: HashMap<String, FuncId> = HashMap::new();
    let mut func_sigs: HashMap<String, cranelift_codegen::ir::Signature> = HashMap::new();

    // Declare imported cross-module functions (Linkage::Import) first.
    // Their FuncIds are added to func_ids so compile_function can resolve calls.
    for (name, fi) in imported_fns {
        let sig = make_func_signature(&obj_module, fi);
        let func_id = obj_module
            .declare_function(name, Linkage::Import, &sig)
            .map_err(|e| CompileError::Cranelift {
                message: format!("Failed to declare imported function '{name}': {e}"),
            })?;
        func_ids.insert((*name).to_string(), func_id);
        func_sigs.insert((*name).to_string(), sig);
    }

    // Declare all local functions.
    // `main` and explicitly exported functions get Linkage::Export.
    for func_info in &graph.functions {
        let sig = make_func_signature(&obj_module, func_info);
        let linkage = if func_info.name.0 == "main" || exported_fns.contains(&func_info.name.0) {
            Linkage::Export
        } else {
            Linkage::Local
        };
        let func_id = obj_module
            .declare_function(&func_info.name.0, linkage, &sig)
            .map_err(|e| CompileError::Cranelift {
                message: format!("Failed to declare function '{}': {e}", func_info.name),
            })?;
        func_ids.insert(func_info.name.0.clone(), func_id);
        func_sigs.insert(func_info.name.0.clone(), sig);
    }

    // Define each local function (emit Cranelift IR).
    let mut fn_builder_ctx = FunctionBuilderContext::new();
    for func_info in &graph.functions {
        let func_id = func_ids[&func_info.name.0];
        let sig = func_sigs[&func_info.name.0].clone();

        let mut ctx = Context::new();
        ctx.func.signature = sig;

        compile_function(
            graph,
            func_info,
            &mut ctx,
            &mut fn_builder_ctx,
            &mut obj_module,
            &func_ids,
            &func_sigs,
            print_i64_id,
            print_f64_id,
            print_bool_id,
        )?;

        obj_module
            .define_function(func_id, &mut ctx)
            .map_err(|e| CompileError::Cranelift {
                message: format!("Failed to define function '{}': {e}", func_info.name),
            })?;
    }

    // Finish and produce the object bytes
    let product = obj_module.finish();
    let bytes = product.emit().map_err(|e| CompileError::ObjectEmission {
        message: format!("Failed to emit object: {e}"),
    })?;

    Ok(bytes)
}

/// Compiles a single function into Cranelift IR.
#[allow(clippy::too_many_arguments)] // Internal helper with many Cranelift context params
fn compile_function(
    graph: &SemanticGraph,
    func_info: &FunctionInfo,
    ctx: &mut Context,
    fn_builder_ctx: &mut FunctionBuilderContext,
    obj_module: &mut ObjectModule,
    func_ids: &HashMap<String, FuncId>,
    func_sigs: &HashMap<String, cranelift_codegen::ir::Signature>,
    print_i64_id: FuncId,
    print_f64_id: FuncId,
    print_bool_id: FuncId,
) -> Result<(), CompileError> {
    let mut builder = FunctionBuilder::new(&mut ctx.func, fn_builder_ctx);

    // Import print function references
    let print_i64_ref = obj_module.declare_func_in_func(print_i64_id, builder.func);
    let print_f64_ref = obj_module.declare_func_in_func(print_f64_id, builder.func);
    let print_bool_ref = obj_module.declare_func_in_func(print_bool_id, builder.func);

    // Import all callable function references
    let mut func_refs: HashMap<String, cranelift_codegen::ir::FuncRef> = HashMap::new();
    for (name, &fid) in func_ids {
        let fref = obj_module.declare_func_in_func(fid, builder.func);
        func_refs.insert(name.clone(), fref);
    }

    // Create all blocks up front (needed for forward branch references)
    let mut block_map: HashMap<String, cranelift_codegen::ir::Block> = HashMap::new();
    for block_info in &func_info.blocks {
        let cl_block = builder.create_block();
        block_map.insert(block_info.label.0.clone(), cl_block);
    }

    // Add entry block params for function parameters
    let entry_block = block_map
        .get(func_info.blocks.first().map_or("entry", |b| &b.label.0))
        .copied()
        .ok_or_else(|| CompileError::Cranelift {
            message: format!("No blocks in function '{}'", func_info.name),
        })?;

    for param in &func_info.params {
        builder.append_block_param(entry_block, duumbi_type_to_cl(param.param_type));
    }

    // SSA value map: NodeId -> Cranelift Value
    let mut value_map: HashMap<NodeId, Value> = HashMap::new();

    // Variable map for Load/Store and function params
    let mut var_map: HashMap<String, Variable> = HashMap::new();

    // Process each block
    for (block_idx, block_info) in func_info.blocks.iter().enumerate() {
        let cl_block = block_map[&block_info.label.0];
        builder.switch_to_block(cl_block);

        // Make function parameters available as named variables in the entry block
        if block_idx == 0 {
            for (i, param) in func_info.params.iter().enumerate() {
                let param_val = builder.block_params(cl_block)[i];
                let cl_type = duumbi_type_to_cl(param.param_type);
                let var = builder.declare_var(cl_type);
                builder.def_var(var, param_val);
                var_map.insert(param.name.clone(), var);
            }
        }

        // Emit instructions for each node
        for &node_idx in &block_info.nodes {
            let node = &graph.graph[node_idx];

            match &node.op {
                Op::Const(val) => {
                    let cl_val = builder.ins().iconst(types::I64, *val);
                    value_map.insert(node.id.clone(), cl_val);
                }
                Op::ConstF64(val) => {
                    let cl_val = builder.ins().f64const(*val);
                    value_map.insert(node.id.clone(), cl_val);
                }
                Op::ConstBool(val) => {
                    let cl_val = builder.ins().iconst(types::I8, i64::from(*val as u8));
                    value_map.insert(node.id.clone(), cl_val);
                }
                Op::Add | Op::Sub | Op::Mul | Op::Div => {
                    let (left_val, right_val) = get_binary_operands(graph, node_idx, &value_map)?;

                    let is_float = node.result_type == Some(DuumbiType::F64);

                    let result = if is_float {
                        match &node.op {
                            Op::Add => builder.ins().fadd(left_val, right_val),
                            Op::Sub => builder.ins().fsub(left_val, right_val),
                            Op::Mul => builder.ins().fmul(left_val, right_val),
                            Op::Div => builder.ins().fdiv(left_val, right_val),
                            _ => unreachable!(),
                        }
                    } else {
                        match &node.op {
                            Op::Add => builder.ins().iadd(left_val, right_val),
                            Op::Sub => builder.ins().isub(left_val, right_val),
                            Op::Mul => builder.ins().imul(left_val, right_val),
                            Op::Div => builder.ins().sdiv(left_val, right_val),
                            _ => unreachable!(),
                        }
                    };
                    value_map.insert(node.id.clone(), result);
                }
                Op::Compare(cmp_op) => {
                    let (left_val, right_val) = get_binary_operands(graph, node_idx, &value_map)?;

                    // Determine operand type from left edge source
                    let left_is_float =
                        get_left_operand_type(graph, node_idx) == Some(DuumbiType::F64);

                    let result = if left_is_float {
                        let cc = compare_op_to_floatcc(cmp_op);
                        builder.ins().fcmp(cc, left_val, right_val)
                    } else {
                        let cc = compare_op_to_intcc(cmp_op);
                        builder.ins().icmp(cc, left_val, right_val)
                    };
                    value_map.insert(node.id.clone(), result);
                }
                Op::Branch => {
                    let cond_val = get_condition_operand(graph, node_idx, &value_map)?;
                    let (true_label, false_label) = get_branch_targets(graph, node_idx)?;

                    let true_block = block_map.get(&true_label).copied().ok_or_else(|| {
                        CompileError::Cranelift {
                            message: format!("Branch true target block '{true_label}' not found"),
                        }
                    })?;
                    let false_block = block_map.get(&false_label).copied().ok_or_else(|| {
                        CompileError::Cranelift {
                            message: format!("Branch false target block '{false_label}' not found"),
                        }
                    })?;

                    builder
                        .ins()
                        .brif(cond_val, true_block, &[], false_block, &[]);
                }
                Op::Call { function } => {
                    let func_ref = func_refs.get(function).copied().ok_or_else(|| {
                        CompileError::Cranelift {
                            message: format!("Function '{function}' not found for call"),
                        }
                    })?;

                    let args = get_call_args(graph, node_idx, &value_map)?;
                    let call_inst = builder.ins().call(func_ref, &args);

                    // Get return value if the called function returns something
                    if let Some(target_sig) = func_sigs.get(function)
                        && !target_sig.returns.is_empty()
                    {
                        let ret_val = builder.inst_results(call_inst)[0];
                        value_map.insert(node.id.clone(), ret_val);
                    }
                }
                Op::Load { variable } => {
                    let var =
                        var_map
                            .get(variable)
                            .copied()
                            .ok_or_else(|| CompileError::Cranelift {
                                message: format!("Variable '{variable}' not declared for Load"),
                            })?;
                    let val = builder.use_var(var);
                    value_map.insert(node.id.clone(), val);
                }
                Op::Store { variable } => {
                    let operand_val = get_unary_operand(graph, node_idx, &value_map)?;
                    if let Some(&var) = var_map.get(variable) {
                        builder.def_var(var, operand_val);
                    } else {
                        // Infer type from the operand's output type
                        let cl_type = get_operand_output_type(graph, node_idx)
                            .map_or(types::I64, duumbi_type_to_cl);
                        let var = builder.declare_var(cl_type);
                        builder.def_var(var, operand_val);
                        var_map.insert(variable.clone(), var);
                    }
                }
                Op::Print => {
                    let operand_val = get_unary_operand(graph, node_idx, &value_map)?;

                    // Determine which print function to call based on operand type
                    let operand_type = get_operand_output_type(graph, node_idx);
                    let print_ref = match operand_type {
                        Some(DuumbiType::F64) => print_f64_ref,
                        Some(DuumbiType::Bool) => print_bool_ref,
                        _ => print_i64_ref,
                    };
                    builder.ins().call(print_ref, &[operand_val]);
                }
                Op::Return => {
                    let operand_val = get_unary_operand(graph, node_idx, &value_map)?;
                    builder.ins().return_(&[operand_val]);
                }
            }
        }
    }

    // Seal all blocks
    builder.seal_all_blocks();
    builder.finalize();

    Ok(())
}

/// Resolves the left and right operand SSA values for a binary operation node.
fn get_binary_operands(
    graph: &SemanticGraph,
    node_idx: petgraph::stable_graph::NodeIndex,
    value_map: &HashMap<NodeId, Value>,
) -> Result<(Value, Value), CompileError> {
    let mut left_val = None;
    let mut right_val = None;

    for edge_ref in graph
        .graph
        .edges_directed(node_idx, petgraph::Direction::Incoming)
    {
        let source_node = &graph.graph[edge_ref.source()];
        let val = value_map
            .get(&source_node.id)
            .ok_or_else(|| CompileError::Cranelift {
                message: format!(
                    "SSA value not found for operand '{}' of node '{}'",
                    source_node.id, graph.graph[node_idx].id
                ),
            })?;

        match edge_ref.weight() {
            GraphEdge::Left => left_val = Some(*val),
            GraphEdge::Right => right_val = Some(*val),
            _ => {}
        }
    }

    let left = left_val.ok_or_else(|| CompileError::Cranelift {
        message: format!(
            "Missing left operand for node '{}'",
            graph.graph[node_idx].id
        ),
    })?;
    let right = right_val.ok_or_else(|| CompileError::Cranelift {
        message: format!(
            "Missing right operand for node '{}'",
            graph.graph[node_idx].id
        ),
    })?;

    Ok((left, right))
}

/// Resolves the single operand SSA value for a unary operation node.
fn get_unary_operand(
    graph: &SemanticGraph,
    node_idx: petgraph::stable_graph::NodeIndex,
    value_map: &HashMap<NodeId, Value>,
) -> Result<Value, CompileError> {
    for edge_ref in graph
        .graph
        .edges_directed(node_idx, petgraph::Direction::Incoming)
    {
        if matches!(edge_ref.weight(), GraphEdge::Operand) {
            let source_node = &graph.graph[edge_ref.source()];
            return value_map.get(&source_node.id).copied().ok_or_else(|| {
                CompileError::Cranelift {
                    message: format!(
                        "SSA value not found for operand '{}' of node '{}'",
                        source_node.id, graph.graph[node_idx].id
                    ),
                }
            });
        }
    }

    Err(CompileError::Cranelift {
        message: format!("Missing operand for node '{}'", graph.graph[node_idx].id),
    })
}

/// Resolves the condition operand for a Branch node.
fn get_condition_operand(
    graph: &SemanticGraph,
    node_idx: petgraph::stable_graph::NodeIndex,
    value_map: &HashMap<NodeId, Value>,
) -> Result<Value, CompileError> {
    for edge_ref in graph
        .graph
        .edges_directed(node_idx, petgraph::Direction::Incoming)
    {
        if matches!(edge_ref.weight(), GraphEdge::Condition) {
            let source_node = &graph.graph[edge_ref.source()];
            return value_map.get(&source_node.id).copied().ok_or_else(|| {
                CompileError::Cranelift {
                    message: format!(
                        "SSA value not found for condition '{}' of node '{}'",
                        source_node.id, graph.graph[node_idx].id
                    ),
                }
            });
        }
    }

    Err(CompileError::Cranelift {
        message: format!(
            "Missing condition for Branch node '{}'",
            graph.graph[node_idx].id
        ),
    })
}

/// Gets the branch target block labels from a Branch node's AST data.
fn get_branch_targets(
    graph: &SemanticGraph,
    node_idx: petgraph::stable_graph::NodeIndex,
) -> Result<(String, String), CompileError> {
    // The branch targets are stored in the Op::Branch node's associated AST,
    // but since the graph only stores Op::Branch without the labels,
    // we need to find TrueBlock/FalseBlock edges or store labels differently.
    // In our design, Branch targets come from OpAst.true_block/false_block
    // which aren't stored in the graph node directly. We need another approach.
    //
    // Solution: Walk outgoing edges looking for TrueBlock/FalseBlock edge types
    // that were stored during graph building. But we didn't add those edges since
    // they point to blocks, not nodes. Instead, we'll look at the graph node's
    // metadata. Since we don't store labels in GraphNode, we need to extract them
    // from the AST-level data that was preserved during parsing.
    //
    // Actually, the proper approach is: the builder should have stored the
    // true_block/false_block labels somewhere accessible. For now, we'll
    // scan the AST info stored in the OpAst. But we don't have the AST at
    // compile time — only the graph.
    //
    // Revised approach: Store branch target labels directly in the GraphNode
    // or in a side table. For Phase 1, let's use a simple approach:
    // We look at the node's ID and find the corresponding function/block
    // to look up target labels from the original parse data.
    //
    // Simplest approach: Add the target labels to the graph node somehow.
    // Since we already have the block_map built, let's store target info
    // in a new field. But we can't modify GraphNode now without breaking things.
    //
    // PRAGMATIC FIX: We'll store branch metadata in the Op enum itself.
    // But Op::Branch has no fields. Let's check if we can get the info
    // another way.

    // In the current design, branch targets should be found by looking at
    // the AST. Since we don't have direct AST access here, we need to
    // refactor. For now, scan for nodes in target blocks that have edges.
    //
    // ACTUAL SOLUTION: We need to modify the graph builder to store branch
    // target block labels. We'll use a side-table approach in SemanticGraph.

    let _node = &graph.graph[node_idx];

    // Look up branch targets from the branch_targets map
    if let Some(targets) = graph.branch_targets.get(&graph.graph[node_idx].id) {
        return Ok((targets.0.clone(), targets.1.clone()));
    }

    Err(CompileError::Cranelift {
        message: format!(
            "Branch target labels not found for node '{}'",
            graph.graph[node_idx].id
        ),
    })
}

/// Gets the output type of the operand connected to a node via Operand edge.
fn get_operand_output_type(
    graph: &SemanticGraph,
    node_idx: petgraph::stable_graph::NodeIndex,
) -> Option<DuumbiType> {
    for edge_ref in graph
        .graph
        .edges_directed(node_idx, petgraph::Direction::Incoming)
    {
        if matches!(edge_ref.weight(), GraphEdge::Operand) {
            let source_node = &graph.graph[edge_ref.source()];
            return resolve_node_output_type(source_node);
        }
    }
    None
}

/// Gets the output type of the left operand of a binary/compare node.
fn get_left_operand_type(
    graph: &SemanticGraph,
    node_idx: petgraph::stable_graph::NodeIndex,
) -> Option<DuumbiType> {
    for edge_ref in graph
        .graph
        .edges_directed(node_idx, petgraph::Direction::Incoming)
    {
        if matches!(edge_ref.weight(), GraphEdge::Left) {
            let source_node = &graph.graph[edge_ref.source()];
            return resolve_node_output_type(source_node);
        }
    }
    None
}

/// Resolves the output type of a graph node for lowering decisions.
fn resolve_node_output_type(node: &crate::graph::GraphNode) -> Option<DuumbiType> {
    match &node.op {
        Op::Const(_)
        | Op::ConstF64(_)
        | Op::ConstBool(_)
        | Op::Add
        | Op::Sub
        | Op::Mul
        | Op::Div
        | Op::Load { .. }
        | Op::Call { .. } => node.result_type,
        Op::Compare(_) => Some(DuumbiType::Bool),
        Op::Print | Op::Store { .. } => Some(DuumbiType::Void),
        Op::Return | Op::Branch => None,
    }
}

/// Collects Call argument values in order.
fn get_call_args(
    graph: &SemanticGraph,
    node_idx: petgraph::stable_graph::NodeIndex,
    value_map: &HashMap<NodeId, Value>,
) -> Result<Vec<Value>, CompileError> {
    let mut args: Vec<(usize, Value)> = Vec::new();

    for edge_ref in graph
        .graph
        .edges_directed(node_idx, petgraph::Direction::Incoming)
    {
        if let GraphEdge::Arg(idx) = edge_ref.weight() {
            let source_node = &graph.graph[edge_ref.source()];
            let val =
                value_map
                    .get(&source_node.id)
                    .copied()
                    .ok_or_else(|| CompileError::Cranelift {
                        message: format!(
                            "SSA value not found for arg '{}' of Call node '{}'",
                            source_node.id, graph.graph[node_idx].id
                        ),
                    })?;
            args.push((*idx, val));
        }
    }

    args.sort_by_key(|(idx, _)| *idx);
    Ok(args.into_iter().map(|(_, v)| v).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::builder::build_graph;
    use crate::parser::parse_jsonld;

    fn fixture_add() -> String {
        std::fs::read_to_string("tests/fixtures/add.jsonld")
            .expect("invariant: add.jsonld fixture must exist")
    }

    fn assert_valid_object(obj_bytes: &[u8]) {
        assert!(!obj_bytes.is_empty());
        let is_macho = obj_bytes.len() >= 4
            && (obj_bytes[0..4] == [0xCF, 0xFA, 0xED, 0xFE]
                || obj_bytes[0..4] == [0xFE, 0xED, 0xFA, 0xCF]);
        let is_elf = obj_bytes.len() >= 4 && obj_bytes[0..4] == [0x7F, 0x45, 0x4C, 0x46];
        assert!(
            is_macho || is_elf,
            "Output should be a valid Mach-O or ELF object file"
        );
    }

    #[test]
    fn compile_add_graph_produces_object() {
        let module = parse_jsonld(&fixture_add()).expect("invariant: fixture must parse");
        let sg = build_graph(&module).expect("invariant: fixture must build");
        let obj_bytes = compile_to_object(&sg).expect("compilation should succeed");
        assert_valid_object(&obj_bytes);
    }

    #[test]
    fn compile_const_return() {
        use crate::graph::*;
        use crate::types::*;
        use petgraph::stable_graph::StableGraph;

        let mut g = StableGraph::new();
        let c = g.add_node(GraphNode {
            id: NodeId("c".to_string()),
            op: Op::Const(42),
            result_type: Some(DuumbiType::I64),
            function: FunctionName("main".to_string()),
            block: BlockLabel("entry".to_string()),
        });
        let r = g.add_node(GraphNode {
            id: NodeId("r".to_string()),
            op: Op::Return,
            result_type: None,
            function: FunctionName("main".to_string()),
            block: BlockLabel("entry".to_string()),
        });
        g.add_edge(c, r, GraphEdge::Operand);

        let sg = SemanticGraph {
            graph: g,
            node_map: HashMap::new(),
            functions: vec![FunctionInfo {
                name: FunctionName("main".to_string()),
                return_type: DuumbiType::I64,
                params: vec![],
                blocks: vec![BlockInfo {
                    label: BlockLabel("entry".to_string()),
                    nodes: vec![c, r],
                }],
            }],
            branch_targets: HashMap::new(),
        };

        let obj_bytes = compile_to_object(&sg).expect("compilation should succeed");
        assert!(!obj_bytes.is_empty());
    }

    #[test]
    fn compile_f64_const_return() {
        use crate::graph::*;
        use crate::types::*;
        use petgraph::stable_graph::StableGraph;

        let mut g = StableGraph::new();
        let c = g.add_node(GraphNode {
            id: NodeId("c".to_string()),
            op: Op::ConstF64(2.5),
            result_type: Some(DuumbiType::F64),
            function: FunctionName("main".to_string()),
            block: BlockLabel("entry".to_string()),
        });
        let print = g.add_node(GraphNode {
            id: NodeId("p".to_string()),
            op: Op::Print,
            result_type: None,
            function: FunctionName("main".to_string()),
            block: BlockLabel("entry".to_string()),
        });
        let zero = g.add_node(GraphNode {
            id: NodeId("z".to_string()),
            op: Op::Const(0),
            result_type: Some(DuumbiType::I64),
            function: FunctionName("main".to_string()),
            block: BlockLabel("entry".to_string()),
        });
        let r = g.add_node(GraphNode {
            id: NodeId("r".to_string()),
            op: Op::Return,
            result_type: None,
            function: FunctionName("main".to_string()),
            block: BlockLabel("entry".to_string()),
        });
        g.add_edge(c, print, GraphEdge::Operand);
        g.add_edge(zero, r, GraphEdge::Operand);

        let sg = SemanticGraph {
            graph: g,
            node_map: HashMap::new(),
            functions: vec![FunctionInfo {
                name: FunctionName("main".to_string()),
                return_type: DuumbiType::I64,
                params: vec![],
                blocks: vec![BlockInfo {
                    label: BlockLabel("entry".to_string()),
                    nodes: vec![c, print, zero, r],
                }],
            }],
            branch_targets: HashMap::new(),
        };

        let obj_bytes = compile_to_object(&sg).expect("f64 compilation should succeed");
        assert!(!obj_bytes.is_empty());
    }

    // -----------------------------------------------------------------------
    // Multi-module compile_program tests
    // -----------------------------------------------------------------------

    /// Builds a minimal valid module JSON-LD string for testing.
    fn make_module_jsonld(name: &str, exports: &[&str]) -> String {
        let exports_json = if exports.is_empty() {
            String::new()
        } else {
            let items: Vec<String> = exports.iter().map(|e| format!("\"{e}\"")).collect();
            format!(",\n    \"duumbi:exports\": [{}]", items.join(", "))
        };
        format!(
            r#"{{
    "@context": {{"duumbi": "https://duumbi.dev/ns/core#"}},
    "@type": "duumbi:Module",
    "@id": "duumbi:{name}",
    "duumbi:name": "{name}"{exports_json},
    "duumbi:functions": [{{
        "@type": "duumbi:Function",
        "@id": "duumbi:{name}/main",
        "duumbi:name": "main",
        "duumbi:returnType": "i64",
        "duumbi:blocks": [{{
            "@type": "duumbi:Block",
            "@id": "duumbi:{name}/main/entry",
            "duumbi:label": "entry",
            "duumbi:ops": [
                {{"@type": "duumbi:Const", "@id": "duumbi:{name}/main/entry/0",
                  "duumbi:value": 0, "duumbi:resultType": "i64"}},
                {{"@type": "duumbi:Return", "@id": "duumbi:{name}/main/entry/1",
                  "duumbi:operand": {{"@id": "duumbi:{name}/main/entry/0"}}}}
            ]
        }}]
    }}]
}}"#
        )
    }

    fn write_program_workspace(files: &[(&str, &str)]) -> tempfile::TempDir {
        let dir = tempfile::TempDir::new().expect("invariant: tempdir must be creatable");
        let graph_dir = dir.path().join(".duumbi").join("graph");
        std::fs::create_dir_all(&graph_dir).expect("invariant: must create graph dir");
        for (filename, content) in files {
            std::fs::write(graph_dir.join(filename), content)
                .expect("invariant: must write fixture file");
        }
        dir
    }

    #[test]
    fn compile_program_single_module_produces_object() {
        use crate::graph::program::Program;

        let module = make_module_jsonld("main", &[]);
        let ws = write_program_workspace(&[("main.jsonld", &module)]);
        let program = Program::load(ws.path()).expect("invariant: single-module program must load");

        let objects = compile_program(&program).expect("compilation must succeed");
        assert_eq!(objects.len(), 1);
        assert!(
            objects.contains_key("main"),
            "must have 'main' module object"
        );
        assert_valid_object(&objects["main"]);
    }

    /// Builds a module with a single exported function named `helper` (not `main`).
    ///
    /// Useful for multi-module tests where only one module should have `main`.
    fn make_helper_module_jsonld(name: &str) -> String {
        format!(
            r#"{{
    "@context": {{"duumbi": "https://duumbi.dev/ns/core#"}},
    "@type": "duumbi:Module",
    "@id": "duumbi:{name}",
    "duumbi:name": "{name}",
    "duumbi:exports": ["helper"],
    "duumbi:functions": [{{
        "@type": "duumbi:Function",
        "@id": "duumbi:{name}/helper",
        "duumbi:name": "helper",
        "duumbi:returnType": "i64",
        "duumbi:blocks": [{{
            "@type": "duumbi:Block",
            "@id": "duumbi:{name}/helper/entry",
            "duumbi:label": "entry",
            "duumbi:ops": [
                {{"@type": "duumbi:Const", "@id": "duumbi:{name}/helper/entry/0",
                  "duumbi:value": 1, "duumbi:resultType": "i64"}},
                {{"@type": "duumbi:Return", "@id": "duumbi:{name}/helper/entry/1",
                  "duumbi:operand": {{"@id": "duumbi:{name}/helper/entry/0"}}}}
            ]
        }}]
    }}]
}}"#
        )
    }

    #[test]
    fn compile_program_two_modules_both_produce_objects() {
        use crate::graph::program::Program;

        // math exports `helper` (not `main`) — only `entry` has the real `main`
        let math = make_helper_module_jsonld("math");
        let entry = make_module_jsonld("entry", &[]);

        let ws = write_program_workspace(&[("math.jsonld", &math), ("entry.jsonld", &entry)]);
        let program = Program::load(ws.path()).expect("invariant: program must load");

        let objects = compile_program(&program).expect("compilation must succeed");
        assert_eq!(objects.len(), 2);
        assert!(
            objects.contains_key("math"),
            "must have 'math' module object"
        );
        assert!(
            objects.contains_key("entry"),
            "must have 'entry' module object"
        );
        assert_valid_object(&objects["math"]);
        assert_valid_object(&objects["entry"]);
    }

    #[test]
    fn stdlib_math_module_parses_builds_and_compiles() {
        use crate::graph::program::Program;

        // Validate the embedded stdlib math module end-to-end.
        // It exports abs, max, min — no main function.
        const MATH_JSONLD: &str = include_str!("../../stdlib/math.jsonld");
        let main_module = make_module_jsonld("main", &[]);

        // Write math as the stdlib dep + main
        let dir = tempfile::TempDir::new().expect("invariant: tempdir");
        let graph_dir = dir.path().join(".duumbi").join("graph");
        std::fs::create_dir_all(&graph_dir).expect("invariant: create graph dir");
        std::fs::write(graph_dir.join("main.jsonld"), &main_module).expect("write main");
        std::fs::write(graph_dir.join("math.jsonld"), MATH_JSONLD).expect("write math");

        let program = Program::load(dir.path()).expect("program must load with math stdlib");
        assert!(
            program
                .modules
                .contains_key(&crate::types::ModuleName("math".to_string()))
        );
        assert_eq!(
            program.exports.len(),
            3,
            "abs + max + min should be exported"
        );

        let objects = compile_program(&program).expect("stdlib math must compile");
        assert_eq!(objects.len(), 2, "main + math modules");
        assert_valid_object(&objects["math"]);
    }

    #[test]
    fn compile_program_multiple_main_returns_error() {
        use crate::graph::program::Program;

        // Both modules define `main` — compile_program should reject this
        let mod_a = make_module_jsonld("a", &[]);
        let mod_b = make_module_jsonld("b", &[]);

        let ws = write_program_workspace(&[("a.jsonld", &mod_a), ("b.jsonld", &mod_b)]);
        let program = Program::load(ws.path()).expect("invariant: program must load");

        let result = compile_program(&program);
        assert!(result.is_err(), "multiple mains should be rejected");
        let err = result.unwrap_err();
        assert!(
            format!("{err}").contains("Multiple modules define 'main'"),
            "unexpected error: {err}"
        );
    }
}
