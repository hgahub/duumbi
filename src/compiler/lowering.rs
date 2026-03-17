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
use cranelift_module::{DataDescription, DataId, FuncId, Linkage, Module};
use cranelift_object::{ObjectBuilder, ObjectModule};
use petgraph::visit::EdgeRef;
use target_lexicon::Triple;

use crate::graph::program::Program;
use crate::graph::{FunctionInfo, GraphEdge, SemanticGraph};
use crate::types::{CompareOp, DuumbiType, NodeId, Op};

use super::CompileError;

/// Holds FuncIds for all C runtime functions declared in the object module.
///
/// Replaces individual print function parameters — all runtime function
/// references are grouped here for clean passing through the compilation pipeline.
struct RuntimeFuncs {
    print_i64: FuncId,
    print_f64: FuncId,
    print_bool: FuncId,
    print_string: FuncId,
    string_new: FuncId,
    string_free: FuncId,
    string_len: FuncId,
    string_concat: FuncId,
    string_equals: FuncId,
    string_compare: FuncId,
    string_slice: FuncId,
    string_contains: FuncId,
    string_find: FuncId,
    string_from_i64: FuncId,
    array_new: FuncId,
    array_push: FuncId,
    array_get: FuncId,
    array_set: FuncId,
    array_len: FuncId,
    array_free: FuncId,
    struct_new: FuncId,
    struct_field_get: FuncId,
    struct_field_set: FuncId,
    struct_free: FuncId,

    // Result functions (Phase 9a-3)
    result_new_ok: FuncId,
    result_new_err: FuncId,
    result_is_ok: FuncId,
    result_unwrap: FuncId,
    result_unwrap_err: FuncId,
    result_free: FuncId,

    // Option functions (Phase 9a-3)
    option_new_some: FuncId,
    option_new_none: FuncId,
    option_is_some: FuncId,
    option_unwrap: FuncId,
    option_free: FuncId,

    // Math functions (Phase 9A)
    sqrt: FuncId,
    pow: FuncId,
    powi64: FuncId,
    fmod: FuncId,
}

/// Helper to declare an imported C function with given param/return types.
fn declare_runtime_fn(
    module: &mut ObjectModule,
    name: &str,
    params: &[cranelift_codegen::ir::Type],
    returns: &[cranelift_codegen::ir::Type],
) -> Result<FuncId, CompileError> {
    let mut sig = module.make_signature();
    for &p in params {
        sig.params.push(AbiParam::new(p));
    }
    for &r in returns {
        sig.returns.push(AbiParam::new(r));
    }
    module
        .declare_function(name, Linkage::Import, &sig)
        .map_err(|e| CompileError::Cranelift {
            message: format!("Failed to declare {name}: {e}"),
        })
}

/// Declares all C runtime functions in the object module.
fn declare_all_runtime_fns(module: &mut ObjectModule) -> Result<RuntimeFuncs, CompileError> {
    let i64t = types::I64;
    let f64t = types::F64;
    let i8t = types::I8;

    Ok(RuntimeFuncs {
        // Print functions
        print_i64: declare_runtime_fn(module, "duumbi_print_i64", &[i64t], &[])?,
        print_f64: declare_runtime_fn(module, "duumbi_print_f64", &[f64t], &[])?,
        print_bool: declare_runtime_fn(module, "duumbi_print_bool", &[i8t], &[])?,
        print_string: declare_runtime_fn(module, "duumbi_print_string", &[i64t], &[])?,

        // String functions (ptr = i64)
        string_new: declare_runtime_fn(module, "duumbi_string_new", &[i64t, i64t], &[i64t])?,
        string_free: declare_runtime_fn(module, "duumbi_string_free", &[i64t], &[])?,
        string_len: declare_runtime_fn(module, "duumbi_string_len", &[i64t], &[i64t])?,
        string_concat: declare_runtime_fn(module, "duumbi_string_concat", &[i64t, i64t], &[i64t])?,
        string_equals: declare_runtime_fn(module, "duumbi_string_equals", &[i64t, i64t], &[i8t])?,
        string_compare: declare_runtime_fn(
            module,
            "duumbi_string_compare",
            &[i64t, i64t],
            &[i64t],
        )?,
        string_slice: declare_runtime_fn(
            module,
            "duumbi_string_slice",
            &[i64t, i64t, i64t],
            &[i64t],
        )?,
        string_contains: declare_runtime_fn(
            module,
            "duumbi_string_contains",
            &[i64t, i64t],
            &[i8t],
        )?,
        string_find: declare_runtime_fn(module, "duumbi_string_find", &[i64t, i64t], &[i64t])?,
        string_from_i64: declare_runtime_fn(module, "duumbi_string_from_i64", &[i64t], &[i64t])?,

        // Array functions (push returns new ptr, get returns i64 value)
        array_new: declare_runtime_fn(module, "duumbi_array_new", &[i64t], &[i64t])?,
        array_push: declare_runtime_fn(module, "duumbi_array_push", &[i64t, i64t], &[i64t])?,
        array_get: declare_runtime_fn(module, "duumbi_array_get", &[i64t, i64t], &[i64t])?,
        array_set: declare_runtime_fn(module, "duumbi_array_set", &[i64t, i64t, i64t], &[])?,
        array_len: declare_runtime_fn(module, "duumbi_array_len", &[i64t], &[i64t])?,
        array_free: declare_runtime_fn(module, "duumbi_array_free", &[i64t], &[])?,

        // Struct functions
        struct_new: declare_runtime_fn(module, "duumbi_struct_new", &[i64t], &[i64t])?,
        struct_field_get: declare_runtime_fn(
            module,
            "duumbi_struct_field_get",
            &[i64t, i64t],
            &[i64t],
        )?,
        struct_field_set: declare_runtime_fn(
            module,
            "duumbi_struct_field_set",
            &[i64t, i64t, i64t],
            &[],
        )?,
        struct_free: declare_runtime_fn(module, "duumbi_struct_free", &[i64t], &[])?,

        // Result functions (Phase 9a-3) — all pointers represented as i64
        result_new_ok: declare_runtime_fn(module, "duumbi_result_new_ok", &[i64t], &[i64t])?,
        result_new_err: declare_runtime_fn(module, "duumbi_result_new_err", &[i64t], &[i64t])?,
        result_is_ok: declare_runtime_fn(module, "duumbi_result_is_ok", &[i64t], &[i8t])?,
        result_unwrap: declare_runtime_fn(module, "duumbi_result_unwrap", &[i64t], &[i64t])?,
        result_unwrap_err: declare_runtime_fn(
            module,
            "duumbi_result_unwrap_err",
            &[i64t],
            &[i64t],
        )?,
        result_free: declare_runtime_fn(module, "duumbi_result_free", &[i64t], &[])?,

        // Option functions (Phase 9a-3)
        option_new_some: declare_runtime_fn(module, "duumbi_option_new_some", &[i64t], &[i64t])?,
        option_new_none: declare_runtime_fn(module, "duumbi_option_new_none", &[], &[i64t])?,
        option_is_some: declare_runtime_fn(module, "duumbi_option_is_some", &[i64t], &[i8t])?,
        option_unwrap: declare_runtime_fn(module, "duumbi_option_unwrap", &[i64t], &[i64t])?,
        option_free: declare_runtime_fn(module, "duumbi_option_free", &[i64t], &[])?,

        // Math functions (Phase 9A) — link with -lm
        sqrt: declare_runtime_fn(module, "duumbi_sqrt", &[f64t], &[f64t])?,
        pow: declare_runtime_fn(module, "duumbi_pow", &[f64t, f64t], &[f64t])?,
        powi64: declare_runtime_fn(module, "duumbi_powi64", &[i64t, i64t], &[i64t])?,
        fmod: declare_runtime_fn(module, "duumbi_fmod", &[f64t, f64t], &[f64t])?,
    })
}

/// Converts a `DuumbiType` to a Cranelift IR type.
///
/// Heap types (String, Array, Struct) are represented as pointers (I64)
/// in Cranelift IR — all heap values are opaque pointers to C runtime
/// allocated memory.
fn duumbi_type_to_cl(ty: &DuumbiType) -> cranelift_codegen::ir::Type {
    match ty {
        DuumbiType::I64 => types::I64,
        DuumbiType::F64 => types::F64,
        DuumbiType::Bool => types::I8,
        DuumbiType::Void => types::I64, // should not be used for values
        // Heap types are pointer-sized (opaque pointers to C runtime memory)
        DuumbiType::String | DuumbiType::Array(_) | DuumbiType::Struct(_) => types::I64,
        // References are pointer-sized (Phase 9a-2)
        DuumbiType::Ref(_) | DuumbiType::RefMut(_) => types::I64,
        // Result/Option are pointer-sized tagged unions (Phase 9a-3)
        DuumbiType::Result(_, _) | DuumbiType::Option(_) => types::I64,
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
            .push(AbiParam::new(duumbi_type_to_cl(&param.param_type)));
    }
    if func_info.return_type != DuumbiType::Void {
        sig.returns
            .push(AbiParam::new(duumbi_type_to_cl(&func_info.return_type)));
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

    // Declare all C runtime functions
    let runtime = declare_all_runtime_fns(&mut obj_module)?;

    // Collect and embed string constants as data sections
    let string_data = embed_string_constants(graph, &mut obj_module)?;

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
            &runtime,
            &string_data,
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

/// Collects all string constants from the graph and embeds them as data sections.
///
/// Returns a map from the string literal content to its [`DataId`], so the
/// compiler can reference the data at use sites via `symbol_value`.
fn embed_string_constants(
    graph: &SemanticGraph,
    obj_module: &mut ObjectModule,
) -> Result<HashMap<String, DataId>, CompileError> {
    let mut string_data: HashMap<String, DataId> = HashMap::new();
    let mut counter = 0u32;

    for node in graph.graph.node_weights() {
        if let Op::ConstString(ref s) = node.op {
            if string_data.contains_key(s) {
                continue; // deduplicate
            }
            let data_name = format!(".str.{counter}");
            counter += 1;

            let data_id = obj_module
                .declare_data(&data_name, Linkage::Local, false, false)
                .map_err(|e| CompileError::Cranelift {
                    message: format!("Failed to declare string data '{data_name}': {e}"),
                })?;

            let mut desc = DataDescription::new();
            desc.define(s.as_bytes().to_vec().into_boxed_slice());

            obj_module
                .define_data(data_id, &desc)
                .map_err(|e| CompileError::Cranelift {
                    message: format!("Failed to define string data '{data_name}': {e}"),
                })?;

            string_data.insert(s.clone(), data_id);
        }
    }

    Ok(string_data)
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
    runtime: &RuntimeFuncs,
    string_data: &HashMap<String, DataId>,
) -> Result<(), CompileError> {
    let mut builder = FunctionBuilder::new(&mut ctx.func, fn_builder_ctx);

    // Import runtime function references into this function
    let print_i64_ref = obj_module.declare_func_in_func(runtime.print_i64, builder.func);
    let print_f64_ref = obj_module.declare_func_in_func(runtime.print_f64, builder.func);
    let print_bool_ref = obj_module.declare_func_in_func(runtime.print_bool, builder.func);
    let print_string_ref = obj_module.declare_func_in_func(runtime.print_string, builder.func);
    let string_new_ref = obj_module.declare_func_in_func(runtime.string_new, builder.func);
    let string_free_ref = obj_module.declare_func_in_func(runtime.string_free, builder.func);
    let string_len_ref = obj_module.declare_func_in_func(runtime.string_len, builder.func);
    let string_concat_ref = obj_module.declare_func_in_func(runtime.string_concat, builder.func);
    let string_equals_ref = obj_module.declare_func_in_func(runtime.string_equals, builder.func);
    let string_compare_ref = obj_module.declare_func_in_func(runtime.string_compare, builder.func);
    let string_slice_ref = obj_module.declare_func_in_func(runtime.string_slice, builder.func);
    let string_contains_ref =
        obj_module.declare_func_in_func(runtime.string_contains, builder.func);
    let string_find_ref = obj_module.declare_func_in_func(runtime.string_find, builder.func);
    let string_from_i64_ref =
        obj_module.declare_func_in_func(runtime.string_from_i64, builder.func);
    let array_new_ref = obj_module.declare_func_in_func(runtime.array_new, builder.func);
    let array_push_ref = obj_module.declare_func_in_func(runtime.array_push, builder.func);
    let array_get_ref = obj_module.declare_func_in_func(runtime.array_get, builder.func);
    let array_set_ref = obj_module.declare_func_in_func(runtime.array_set, builder.func);
    let array_len_ref = obj_module.declare_func_in_func(runtime.array_len, builder.func);
    let array_free_ref = obj_module.declare_func_in_func(runtime.array_free, builder.func);
    let struct_new_ref = obj_module.declare_func_in_func(runtime.struct_new, builder.func);
    let struct_field_get_ref =
        obj_module.declare_func_in_func(runtime.struct_field_get, builder.func);
    let struct_field_set_ref =
        obj_module.declare_func_in_func(runtime.struct_field_set, builder.func);
    let struct_free_ref = obj_module.declare_func_in_func(runtime.struct_free, builder.func);

    // Result/Option function refs (Phase 9a-3)
    let result_new_ok_ref = obj_module.declare_func_in_func(runtime.result_new_ok, builder.func);
    let result_new_err_ref = obj_module.declare_func_in_func(runtime.result_new_err, builder.func);
    let result_is_ok_ref = obj_module.declare_func_in_func(runtime.result_is_ok, builder.func);
    let result_unwrap_ref = obj_module.declare_func_in_func(runtime.result_unwrap, builder.func);
    let result_unwrap_err_ref =
        obj_module.declare_func_in_func(runtime.result_unwrap_err, builder.func);
    let result_free_ref = obj_module.declare_func_in_func(runtime.result_free, builder.func);
    let option_new_some_ref =
        obj_module.declare_func_in_func(runtime.option_new_some, builder.func);
    let option_new_none_ref =
        obj_module.declare_func_in_func(runtime.option_new_none, builder.func);
    let option_is_some_ref = obj_module.declare_func_in_func(runtime.option_is_some, builder.func);
    let option_unwrap_ref = obj_module.declare_func_in_func(runtime.option_unwrap, builder.func);
    let option_free_ref = obj_module.declare_func_in_func(runtime.option_free, builder.func);

    // Math function references (Phase 9A)
    let sqrt_ref = obj_module.declare_func_in_func(runtime.sqrt, builder.func);
    let pow_ref = obj_module.declare_func_in_func(runtime.pow, builder.func);
    let powi64_ref = obj_module.declare_func_in_func(runtime.powi64, builder.func);
    let fmod_ref = obj_module.declare_func_in_func(runtime.fmod, builder.func);

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
        builder.append_block_param(entry_block, duumbi_type_to_cl(&param.param_type));
    }

    // SSA value map: NodeId -> Cranelift Value
    let mut value_map: HashMap<NodeId, Value> = HashMap::new();

    // Variable map for Load/Store and function params
    let mut var_map: HashMap<String, Variable> = HashMap::new();

    // Struct field offset map: field_name → byte offset (sequential, 8 bytes each)
    let mut field_offsets: HashMap<String, i64> = HashMap::new();

    // Track heap-allocated values for automatic Drop insertion at scope exits.
    // Ordered Vec for deterministic LIFO (last-allocated freed first) ordering.
    // Entries: (NodeId, SSA Value, DuumbiType). Removed on explicit Drop or Move.
    let mut heap_allocs: Vec<(NodeId, Value, DuumbiType)> = Vec::new();

    // Process each block
    for (block_idx, block_info) in func_info.blocks.iter().enumerate() {
        let cl_block = block_map[&block_info.label.0];
        builder.switch_to_block(cl_block);

        // Make function parameters available as named variables in the entry block
        if block_idx == 0 {
            for (i, param) in func_info.params.iter().enumerate() {
                let param_val = builder.block_params(cl_block)[i];
                let cl_type = duumbi_type_to_cl(&param.param_type);
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
                            .as_ref()
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

                    // Auto-drop: free remaining heap values before return (LIFO order).
                    // Skip the value being returned (it escapes to the caller).
                    let return_source_id = find_return_operand_node_id(graph, node_idx);
                    // LIFO order: iterate in reverse (last-allocated freed first),
                    // skipping the returned value.
                    let to_free: Vec<(Value, DuumbiType)> = heap_allocs
                        .iter()
                        .rev()
                        .filter(|(id, _, _)| return_source_id.as_ref() != Some(id))
                        .map(|(_, val, ty)| (*val, ty.clone()))
                        .collect();
                    for (val, ty) in &to_free {
                        match ty {
                            DuumbiType::String => {
                                builder.ins().call(string_free_ref, &[*val]);
                            }
                            DuumbiType::Array(_) => {
                                builder.ins().call(array_free_ref, &[*val]);
                            }
                            DuumbiType::Struct(_) => {
                                builder.ins().call(struct_free_ref, &[*val]);
                            }
                            DuumbiType::Result(_, _) => {
                                builder.ins().call(result_free_ref, &[*val]);
                            }
                            DuumbiType::Option(_) => {
                                builder.ins().call(option_free_ref, &[*val]);
                            }
                            _ => {}
                        }
                    }
                    heap_allocs.clear();

                    builder.ins().return_(&[operand_val]);
                }
                // -- Phase 9a-1: String ops --
                Op::ConstString(s) => {
                    // Get the embedded data address, then call duumbi_string_new(ptr, len)
                    let data_id = string_data.get(s).ok_or_else(|| CompileError::Cranelift {
                        message: format!("String constant data not found for '{s}'"),
                    })?;
                    let gv = obj_module.declare_data_in_func(*data_id, builder.func);
                    let ptr = builder.ins().global_value(types::I64, gv);
                    let len = builder.ins().iconst(types::I64, s.len() as i64);
                    let call = builder.ins().call(string_new_ref, &[ptr, len]);
                    let result = builder.inst_results(call)[0];
                    value_map.insert(node.id.clone(), result);
                }
                Op::PrintString => {
                    let operand_val = get_unary_operand(graph, node_idx, &value_map)?;
                    builder.ins().call(print_string_ref, &[operand_val]);
                }
                Op::StringConcat => {
                    let (left_val, right_val) = get_binary_operands(graph, node_idx, &value_map)?;
                    let call = builder
                        .ins()
                        .call(string_concat_ref, &[left_val, right_val]);
                    let result = builder.inst_results(call)[0];
                    value_map.insert(node.id.clone(), result);
                }
                Op::StringEquals => {
                    let (left_val, right_val) = get_binary_operands(graph, node_idx, &value_map)?;
                    let call = builder
                        .ins()
                        .call(string_equals_ref, &[left_val, right_val]);
                    let result = builder.inst_results(call)[0];
                    value_map.insert(node.id.clone(), result);
                }
                Op::StringCompare(_) => {
                    let (left_val, right_val) = get_binary_operands(graph, node_idx, &value_map)?;
                    let call = builder
                        .ins()
                        .call(string_compare_ref, &[left_val, right_val]);
                    let cmp_result = builder.inst_results(call)[0];
                    // Convert i64 compare result to bool based on CompareOp
                    let zero = builder.ins().iconst(types::I64, 0);
                    let Op::StringCompare(ref cmp_op) = node.op else {
                        unreachable!()
                    };
                    let cc = compare_op_to_intcc(cmp_op);
                    let bool_result = builder.ins().icmp(cc, cmp_result, zero);
                    value_map.insert(node.id.clone(), bool_result);
                }
                Op::StringLength => {
                    let operand_val = get_unary_operand(graph, node_idx, &value_map)?;
                    let call = builder.ins().call(string_len_ref, &[operand_val]);
                    let result = builder.inst_results(call)[0];
                    value_map.insert(node.id.clone(), result);
                }
                Op::StringSlice => {
                    // operand = string, left = start index, right = end index
                    let operand_val = get_unary_operand(graph, node_idx, &value_map)?;
                    let (start_val, end_val) = get_binary_operands(graph, node_idx, &value_map)?;
                    let call = builder
                        .ins()
                        .call(string_slice_ref, &[operand_val, start_val, end_val]);
                    let result = builder.inst_results(call)[0];
                    value_map.insert(node.id.clone(), result);
                }
                Op::StringContains => {
                    let (left_val, right_val) = get_binary_operands(graph, node_idx, &value_map)?;
                    let call = builder
                        .ins()
                        .call(string_contains_ref, &[left_val, right_val]);
                    let result = builder.inst_results(call)[0];
                    value_map.insert(node.id.clone(), result);
                }
                Op::StringFind => {
                    let (left_val, right_val) = get_binary_operands(graph, node_idx, &value_map)?;
                    let call = builder.ins().call(string_find_ref, &[left_val, right_val]);
                    let result = builder.inst_results(call)[0];
                    value_map.insert(node.id.clone(), result);
                }
                Op::StringFromI64 => {
                    let operand_val = get_unary_operand(graph, node_idx, &value_map)?;
                    let call = builder.ins().call(string_from_i64_ref, &[operand_val]);
                    let result = builder.inst_results(call)[0];
                    value_map.insert(node.id.clone(), result);
                }

                // -- Phase 9a-1: Array ops --
                Op::ArrayNew => {
                    // elem_size from result_type: Array<i64> → 8, Array<String> → 8 (ptr)
                    let elem_size = match &node.result_type {
                        Some(DuumbiType::Array(inner)) => type_size(inner),
                        _ => 8, // default pointer size
                    };
                    let size_val = builder.ins().iconst(types::I64, elem_size);
                    let call = builder.ins().call(array_new_ref, &[size_val]);
                    let result = builder.inst_results(call)[0];
                    value_map.insert(node.id.clone(), result);
                }
                Op::ArrayPush => {
                    let (arr_val, elem_val) = get_binary_operands(graph, node_idx, &value_map)?;
                    let call = builder.ins().call(array_push_ref, &[arr_val, elem_val]);
                    // Push returns the (possibly reallocated) array pointer.
                    // Update the array value in the value map so subsequent ops
                    // use the new pointer. We update the source array node's entry.
                    let new_arr = builder.inst_results(call)[0];
                    // Find the array source node and update its value
                    for edge_ref in graph
                        .graph
                        .edges_directed(node_idx, petgraph::Direction::Incoming)
                    {
                        if matches!(edge_ref.weight(), GraphEdge::Left) {
                            let source_node = &graph.graph[edge_ref.source()];
                            value_map.insert(source_node.id.clone(), new_arr);
                        }
                    }
                }
                Op::ArrayGet => {
                    let (arr_val, idx_val) = get_binary_operands(graph, node_idx, &value_map)?;
                    let call = builder.ins().call(array_get_ref, &[arr_val, idx_val]);
                    let result = builder.inst_results(call)[0];
                    value_map.insert(node.id.clone(), result);
                }
                Op::ArrayTryGet => {
                    // ArrayTryGet requires Option<T> return type (Phase 9a-3).
                    // Until then, report as unimplemented rather than silently
                    // panicking like ArrayGet on out-of-bounds access.
                    return Err(CompileError::Cranelift {
                        message: format!(
                            "ArrayTryGet requires Option<T> type (Phase 9a-3), \
                             use ArrayGet for now — node '{}'",
                            node.id
                        ),
                    });
                }
                Op::ArraySet => {
                    // operand = array, left = index, right = value
                    let operand_val = get_unary_operand(graph, node_idx, &value_map)?;
                    let (idx_val, elem_val) = get_binary_operands(graph, node_idx, &value_map)?;
                    builder
                        .ins()
                        .call(array_set_ref, &[operand_val, idx_val, elem_val]);
                }
                Op::ArrayLength => {
                    let operand_val = get_unary_operand(graph, node_idx, &value_map)?;
                    let call = builder.ins().call(array_len_ref, &[operand_val]);
                    let result = builder.inst_results(call)[0];
                    value_map.insert(node.id.clone(), result);
                }

                // -- Phase 9a-1: Struct ops --
                Op::StructNew { .. } => {
                    // Allocate enough space for fields. We use a generous default
                    // since proper struct layout requires a struct registry (Phase 9a-2).
                    // 8 fields × 8 bytes = 64 bytes covers most simple structs.
                    let total_size = builder.ins().iconst(types::I64, 64);
                    let call = builder.ins().call(struct_new_ref, &[total_size]);
                    let result = builder.inst_results(call)[0];
                    value_map.insert(node.id.clone(), result);
                }
                Op::FieldGet { field_name } => {
                    let operand_val = get_unary_operand(graph, node_idx, &value_map)?;
                    let offset_val = field_name_to_offset(field_name, &mut field_offsets);
                    let offset = builder.ins().iconst(types::I64, offset_val);
                    let call = builder
                        .ins()
                        .call(struct_field_get_ref, &[operand_val, offset]);
                    let result = builder.inst_results(call)[0];
                    value_map.insert(node.id.clone(), result);
                }
                Op::FieldSet { field_name } => {
                    let operand_val = get_unary_operand(graph, node_idx, &value_map)?;
                    let value_val = get_right_operand(graph, node_idx, &value_map)?;
                    let offset_val = field_name_to_offset(field_name, &mut field_offsets);
                    let offset = builder.ins().iconst(types::I64, offset_val);
                    builder
                        .ins()
                        .call(struct_field_set_ref, &[operand_val, offset, value_val]);
                }
                // -- Ownership ops (Phase 9a-2) --
                Op::Alloc { alloc_type } => {
                    // Allocate a new heap value based on type.
                    // Each type uses its specific _new() constructor.
                    let cl_val = match alloc_type {
                        DuumbiType::String => {
                            let zero = builder.ins().iconst(types::I64, 0);
                            let inst = builder.ins().call(string_new_ref, &[zero, zero]);
                            builder.inst_results(inst)[0]
                        }
                        DuumbiType::Array(_) => {
                            let elem_size = match alloc_type {
                                DuumbiType::Array(inner) => type_size(inner),
                                _ => 8,
                            };
                            let size_val = builder.ins().iconst(types::I64, elem_size);
                            let inst = builder.ins().call(array_new_ref, &[size_val]);
                            builder.inst_results(inst)[0]
                        }
                        DuumbiType::Struct(_) => {
                            let cap = builder.ins().iconst(types::I64, 8);
                            let inst = builder.ins().call(struct_new_ref, &[cap]);
                            builder.inst_results(inst)[0]
                        }
                        _ => builder.ins().iconst(types::I64, 0),
                    };
                    value_map.insert(node.id.clone(), cl_val);
                    // Track for automatic Drop insertion at scope exit
                    if alloc_type.is_heap_type() {
                        heap_allocs.push((node.id.clone(), cl_val, alloc_type.clone()));
                    }
                }
                Op::Move { .. } => {
                    // Move is a pointer copy — no runtime cost.
                    // The source SSA value is simply forwarded.
                    // Remove source from heap_allocs (ownership transferred).
                    let source_node_id = find_operand_node_id(graph, node_idx);
                    let operand_val = get_operand(graph, node_idx, &value_map)?;
                    value_map.insert(node.id.clone(), operand_val);
                    if let Some(ref src_id) = source_node_id {
                        // Remove source from tracking, transfer to move result
                        if let Some(pos) = heap_allocs.iter().position(|(id, _, _)| id == src_id) {
                            let (_, val, ty) = heap_allocs.remove(pos);
                            heap_allocs.push((node.id.clone(), val, ty));
                        }
                    }
                }
                Op::Borrow { .. } => {
                    // Borrow (shared or mutable) is a pointer copy — no runtime cost.
                    // Safety is enforced by the validator, not at runtime.
                    // Does NOT transfer ownership — source stays in heap_allocs.
                    let operand_val = get_operand(graph, node_idx, &value_map)?;
                    value_map.insert(node.id.clone(), operand_val);
                }
                Op::Drop { .. } => {
                    // Explicit Drop — dispatch to type-specific free function.
                    let source_node_id = find_operand_node_id(graph, node_idx);
                    let operand_val = get_operand(graph, node_idx, &value_map)?;
                    let source_type = find_operand_type(graph, node_idx);
                    match source_type {
                        Some(DuumbiType::String) => {
                            builder.ins().call(string_free_ref, &[operand_val]);
                        }
                        Some(DuumbiType::Array(_)) => {
                            builder.ins().call(array_free_ref, &[operand_val]);
                        }
                        Some(DuumbiType::Struct(_)) => {
                            builder.ins().call(struct_free_ref, &[operand_val]);
                        }
                        Some(DuumbiType::Result(_, _)) => {
                            builder.ins().call(result_free_ref, &[operand_val]);
                        }
                        Some(DuumbiType::Option(_)) => {
                            builder.ins().call(option_free_ref, &[operand_val]);
                        }
                        _ => {}
                    }
                    // Remove from heap_allocs — explicitly freed
                    if let Some(ref src_id) = source_node_id
                        && let Some(pos) = heap_allocs.iter().position(|(id, _, _)| id == src_id)
                    {
                        heap_allocs.remove(pos);
                    }
                }
                // -- Phase 9a-3: Result ops --
                Op::ResultOk => {
                    let operand_val = get_unary_operand(graph, node_idx, &value_map)?;
                    let call = builder.ins().call(result_new_ok_ref, &[operand_val]);
                    let result = builder.inst_results(call)[0];
                    value_map.insert(node.id.clone(), result);
                }
                Op::ResultErr => {
                    let operand_val = get_unary_operand(graph, node_idx, &value_map)?;
                    let call = builder.ins().call(result_new_err_ref, &[operand_val]);
                    let result = builder.inst_results(call)[0];
                    value_map.insert(node.id.clone(), result);
                }
                Op::ResultIsOk => {
                    let operand_val = get_unary_operand(graph, node_idx, &value_map)?;
                    let call = builder.ins().call(result_is_ok_ref, &[operand_val]);
                    let result = builder.inst_results(call)[0];
                    value_map.insert(node.id.clone(), result);
                }
                Op::ResultUnwrap => {
                    let operand_val = get_unary_operand(graph, node_idx, &value_map)?;
                    let call = builder.ins().call(result_unwrap_ref, &[operand_val]);
                    let result = builder.inst_results(call)[0];
                    value_map.insert(node.id.clone(), result);
                }
                Op::ResultUnwrapErr => {
                    let operand_val = get_unary_operand(graph, node_idx, &value_map)?;
                    let call = builder.ins().call(result_unwrap_err_ref, &[operand_val]);
                    let result = builder.inst_results(call)[0];
                    value_map.insert(node.id.clone(), result);
                }

                // -- Phase 9a-3: Option ops --
                Op::OptionSome => {
                    let operand_val = get_unary_operand(graph, node_idx, &value_map)?;
                    let call = builder.ins().call(option_new_some_ref, &[operand_val]);
                    let result = builder.inst_results(call)[0];
                    value_map.insert(node.id.clone(), result);
                }
                Op::OptionNone => {
                    // option_new_none() takes no arguments
                    let call = builder.ins().call(option_new_none_ref, &[]);
                    let result = builder.inst_results(call)[0];
                    value_map.insert(node.id.clone(), result);
                }
                Op::OptionIsSome => {
                    let operand_val = get_unary_operand(graph, node_idx, &value_map)?;
                    let call = builder.ins().call(option_is_some_ref, &[operand_val]);
                    let result = builder.inst_results(call)[0];
                    value_map.insert(node.id.clone(), result);
                }
                Op::OptionUnwrap => {
                    let operand_val = get_unary_operand(graph, node_idx, &value_map)?;
                    let call = builder.ins().call(option_unwrap_ref, &[operand_val]);
                    let result = builder.inst_results(call)[0];
                    value_map.insert(node.id.clone(), result);
                }

                // -- Phase 9a-3: Match op --
                Op::Match {
                    ok_block,
                    err_block,
                } => {
                    // Get the Result/Option value being matched
                    let operand_val = get_unary_operand(graph, node_idx, &value_map)?;

                    // Determine discriminant: is_ok for Result, is_some for Option
                    let operand_type = get_operand_output_type(graph, node_idx);
                    let discriminant = match &operand_type {
                        Some(DuumbiType::Option(_)) => {
                            let call = builder.ins().call(option_is_some_ref, &[operand_val]);
                            builder.inst_results(call)[0]
                        }
                        // Default (Result or unknown) — use is_ok
                        _ => {
                            let call = builder.ins().call(result_is_ok_ref, &[operand_val]);
                            builder.inst_results(call)[0]
                        }
                    };

                    // Look up target Cranelift blocks
                    let ok_cl_block = block_map.get(ok_block).copied().ok_or_else(|| {
                        CompileError::Cranelift {
                            message: format!(
                                "Match ok_block '{}' not found in function '{}'",
                                ok_block, func_info.name
                            ),
                        }
                    })?;
                    let err_cl_block = block_map.get(err_block).copied().ok_or_else(|| {
                        CompileError::Cranelift {
                            message: format!(
                                "Match err_block '{}' not found in function '{}'",
                                err_block, func_info.name
                            ),
                        }
                    })?;

                    // Branch: non-zero discriminant → ok_block, zero → err_block
                    builder
                        .ins()
                        .brif(discriminant, ok_cl_block, &[], err_cl_block, &[]);
                }

                // -- Phase 9A: Math ops --
                Op::Modulo => {
                    let (left_val, right_val) = get_binary_operands(graph, node_idx, &value_map)?;
                    let is_float = node.result_type == Some(DuumbiType::F64);
                    let result = if is_float {
                        // f64 modulo via C shim (fmod)
                        let call = builder.ins().call(fmod_ref, &[left_val, right_val]);
                        builder.inst_results(call)[0]
                    } else {
                        // i64 modulo: signed remainder
                        builder.ins().srem(left_val, right_val)
                    };
                    value_map.insert(node.id.clone(), result);
                }
                Op::Negate => {
                    let operand_val = get_unary_operand(graph, node_idx, &value_map)?;
                    let is_float = node.result_type == Some(DuumbiType::F64);
                    let result = if is_float {
                        builder.ins().fneg(operand_val)
                    } else {
                        builder.ins().ineg(operand_val)
                    };
                    value_map.insert(node.id.clone(), result);
                }
                Op::Sqrt => {
                    let operand_val = get_unary_operand(graph, node_idx, &value_map)?;
                    let call = builder.ins().call(sqrt_ref, &[operand_val]);
                    let result = builder.inst_results(call)[0];
                    value_map.insert(node.id.clone(), result);
                }
                Op::Pow => {
                    let (left_val, right_val) = get_binary_operands(graph, node_idx, &value_map)?;
                    let call = builder.ins().call(pow_ref, &[left_val, right_val]);
                    let result = builder.inst_results(call)[0];
                    value_map.insert(node.id.clone(), result);
                }
                Op::PowI64 => {
                    let (left_val, right_val) = get_binary_operands(graph, node_idx, &value_map)?;
                    let call = builder.ins().call(powi64_ref, &[left_val, right_val]);
                    let result = builder.inst_results(call)[0];
                    value_map.insert(node.id.clone(), result);
                }

                // -- Phase 9A: Bitwise ops --
                Op::BitwiseAnd => {
                    let (left_val, right_val) = get_binary_operands(graph, node_idx, &value_map)?;
                    let result = builder.ins().band(left_val, right_val);
                    value_map.insert(node.id.clone(), result);
                }
                Op::BitwiseOr => {
                    let (left_val, right_val) = get_binary_operands(graph, node_idx, &value_map)?;
                    let result = builder.ins().bor(left_val, right_val);
                    value_map.insert(node.id.clone(), result);
                }
                Op::BitwiseXor => {
                    let (left_val, right_val) = get_binary_operands(graph, node_idx, &value_map)?;
                    let result = builder.ins().bxor(left_val, right_val);
                    value_map.insert(node.id.clone(), result);
                }
                Op::BitwiseNot => {
                    let operand_val = get_unary_operand(graph, node_idx, &value_map)?;
                    let result = builder.ins().bnot(operand_val);
                    value_map.insert(node.id.clone(), result);
                }
                Op::ShiftLeft => {
                    let (left_val, right_val) = get_binary_operands(graph, node_idx, &value_map)?;
                    let result = builder.ins().ishl(left_val, right_val);
                    value_map.insert(node.id.clone(), result);
                }
                Op::ShiftRight => {
                    let (left_val, right_val) = get_binary_operands(graph, node_idx, &value_map)?;
                    let result = builder.ins().sshr(left_val, right_val);
                    value_map.insert(node.id.clone(), result);
                }
            }

            // Track heap-producing non-ownership ops for auto-drop.
            // ConstString, StringConcat, StringSlice, StringFromI64, ArrayNew, StructNew,
            // ResultOk, ResultErr, OptionSome, OptionNone all allocate heap memory.
            // Match and Branch are control-flow terminators with no result value.
            if !matches!(
                &node.op,
                Op::Alloc { .. }
                    | Op::Move { .. }
                    | Op::Drop { .. }
                    | Op::Return
                    | Op::Branch
                    | Op::Match { .. }
            ) && let Some(ref rt) = node.result_type
                && rt.is_heap_type()
                && let Some(&val) = value_map.get(&node.id)
            {
                heap_allocs.push((node.id.clone(), val, rt.clone()));
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
    node.op.output_type(&node.result_type)
}

/// Assigns sequential byte offsets to struct field names (8 bytes per field).
///
/// Each unique field name gets the next available offset. This is a simplified
/// layout scheme — Phase 9a-2 will introduce a proper struct registry with
/// type-aware field sizes and alignment.
fn field_name_to_offset(field_name: &str, offsets: &mut HashMap<String, i64>) -> i64 {
    let next_offset = offsets.len() as i64 * 8;
    *offsets.entry(field_name.to_string()).or_insert(next_offset)
}

/// Returns the size in bytes for a DuumbiType (for array element sizing).
fn type_size(ty: &DuumbiType) -> i64 {
    match ty {
        DuumbiType::I64 => 8,
        DuumbiType::F64 => 8,
        DuumbiType::Bool => 1,
        DuumbiType::Void => 0,
        // Heap types are pointer-sized
        DuumbiType::String | DuumbiType::Array(_) | DuumbiType::Struct(_) => 8,
        // References are pointer-sized (Phase 9a-2)
        DuumbiType::Ref(_) | DuumbiType::RefMut(_) => 8,
        // Result/Option are pointer-sized tagged unions (Phase 9a-3)
        DuumbiType::Result(_, _) | DuumbiType::Option(_) => 8,
    }
}

/// Resolves the right operand SSA value for a node.
fn get_right_operand(
    graph: &SemanticGraph,
    node_idx: petgraph::stable_graph::NodeIndex,
    value_map: &HashMap<NodeId, Value>,
) -> Result<Value, CompileError> {
    for edge_ref in graph
        .graph
        .edges_directed(node_idx, petgraph::Direction::Incoming)
    {
        if matches!(edge_ref.weight(), GraphEdge::Right) {
            let source_node = &graph.graph[edge_ref.source()];
            return value_map.get(&source_node.id).copied().ok_or_else(|| {
                CompileError::Cranelift {
                    message: format!(
                        "SSA value not found for right operand '{}' of node '{}'",
                        source_node.id, graph.graph[node_idx].id
                    ),
                }
            });
        }
    }
    Err(CompileError::Cranelift {
        message: format!(
            "Missing right operand for node '{}'",
            graph.graph[node_idx].id
        ),
    })
}

/// Resolves the single operand SSA value for ownership ops.
///
/// Follows incoming Operand, MovesFrom, BorrowsFrom, or Drops edges
/// to find the source value.
fn get_operand(
    graph: &SemanticGraph,
    node_idx: petgraph::stable_graph::NodeIndex,
    value_map: &HashMap<NodeId, Value>,
) -> Result<Value, CompileError> {
    for edge_ref in graph
        .graph
        .edges_directed(node_idx, petgraph::Direction::Incoming)
    {
        match edge_ref.weight() {
            GraphEdge::Operand
            | GraphEdge::MovesFrom
            | GraphEdge::BorrowsFrom
            | GraphEdge::Drops => {
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
            _ => {}
        }
    }
    Err(CompileError::Cranelift {
        message: format!(
            "Missing operand for ownership op '{}'",
            graph.graph[node_idx].id
        ),
    })
}

/// Finds the output type of the operand connected via ownership edges.
///
/// Used by Drop to determine which type-specific free function to call.
fn find_operand_type(
    graph: &SemanticGraph,
    node_idx: petgraph::stable_graph::NodeIndex,
) -> Option<DuumbiType> {
    for edge_ref in graph
        .graph
        .edges_directed(node_idx, petgraph::Direction::Incoming)
    {
        match edge_ref.weight() {
            GraphEdge::Operand
            | GraphEdge::MovesFrom
            | GraphEdge::BorrowsFrom
            | GraphEdge::Drops => {
                let source_node = &graph.graph[edge_ref.source()];
                return resolve_node_output_type(source_node);
            }
            _ => {}
        }
    }
    None
}

/// Finds the NodeId of the Return operand (the value being returned).
///
/// Used to exclude the returned value from automatic Drop insertion.
fn find_return_operand_node_id(
    graph: &SemanticGraph,
    node_idx: petgraph::stable_graph::NodeIndex,
) -> Option<NodeId> {
    for edge_ref in graph
        .graph
        .edges_directed(node_idx, petgraph::Direction::Incoming)
    {
        if matches!(edge_ref.weight(), GraphEdge::Operand) {
            return Some(graph.graph[edge_ref.source()].id.clone());
        }
    }
    None
}

/// Finds the NodeId of the operand connected via ownership edges.
///
/// Used for heap_allocs tracking — to know which allocation to remove
/// when a value is Moved or Dropped.
fn find_operand_node_id(
    graph: &SemanticGraph,
    node_idx: petgraph::stable_graph::NodeIndex,
) -> Option<NodeId> {
    for edge_ref in graph
        .graph
        .edges_directed(node_idx, petgraph::Direction::Incoming)
    {
        match edge_ref.weight() {
            GraphEdge::Operand
            | GraphEdge::MovesFrom
            | GraphEdge::BorrowsFrom
            | GraphEdge::Drops => {
                return Some(graph.graph[edge_ref.source()].id.clone());
            }
            _ => {}
        }
    }
    None
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
            owner: None,
            lifetime: None,
            lifetime_param: None,
        });
        let r = g.add_node(GraphNode {
            id: NodeId("r".to_string()),
            op: Op::Return,
            result_type: None,
            function: FunctionName("main".to_string()),
            block: BlockLabel("entry".to_string()),
            owner: None,
            lifetime: None,
            lifetime_param: None,
        });
        g.add_edge(c, r, GraphEdge::Operand);

        let sg = SemanticGraph {
            graph: g,
            node_map: HashMap::new(),
            functions: vec![FunctionInfo {
                name: FunctionName("main".to_string()),
                return_type: DuumbiType::I64,
                params: vec![],
                lifetime_params: Vec::new(),
                blocks: vec![BlockInfo {
                    label: BlockLabel("entry".to_string()),
                    nodes: vec![c, r],
                }],
            }],
            branch_targets: HashMap::new(),
            module_name: ModuleName("test".to_string()),
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
            owner: None,
            lifetime: None,
            lifetime_param: None,
        });
        let print = g.add_node(GraphNode {
            id: NodeId("p".to_string()),
            op: Op::Print,
            result_type: None,
            function: FunctionName("main".to_string()),
            block: BlockLabel("entry".to_string()),
            owner: None,
            lifetime: None,
            lifetime_param: None,
        });
        let zero = g.add_node(GraphNode {
            id: NodeId("z".to_string()),
            op: Op::Const(0),
            result_type: Some(DuumbiType::I64),
            function: FunctionName("main".to_string()),
            block: BlockLabel("entry".to_string()),
            owner: None,
            lifetime: None,
            lifetime_param: None,
        });
        let r = g.add_node(GraphNode {
            id: NodeId("r".to_string()),
            op: Op::Return,
            result_type: None,
            function: FunctionName("main".to_string()),
            block: BlockLabel("entry".to_string()),
            owner: None,
            lifetime: None,
            lifetime_param: None,
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
                lifetime_params: Vec::new(),
                blocks: vec![BlockInfo {
                    label: BlockLabel("entry".to_string()),
                    nodes: vec![c, print, zero, r],
                }],
            }],
            branch_targets: HashMap::new(),
            module_name: ModuleName("test".to_string()),
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
