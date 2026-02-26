//! Cranelift IR lowering — graph nodes to Cranelift instructions.
//!
//! Compiles a validated `SemanticGraph` into a native object file
//! using the Cranelift code generator.

use std::collections::HashMap;

use cranelift_codegen::ir::types;
use cranelift_codegen::ir::{AbiParam, InstBuilder};
use cranelift_codegen::settings::{self, Configurable};
use cranelift_codegen::{self, Context, isa};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_module::{Linkage, Module};
use cranelift_object::{ObjectBuilder, ObjectModule};
use petgraph::visit::EdgeRef;
use target_lexicon::Triple;

use crate::graph::{GraphEdge, SemanticGraph};
use crate::types::{NodeId, Op};

use super::CompileError;

/// Compiles a validated semantic graph to a native object file.
///
/// Returns the raw bytes of the object file (Mach-O on macOS, ELF on Linux).
#[must_use = "compilation errors should be handled"]
pub fn compile_to_object(graph: &SemanticGraph) -> Result<Vec<u8>, CompileError> {
    let triple = Triple::host();

    let mut settings_builder = settings::builder();
    settings_builder
        .set("opt_level", "speed")
        .map_err(|e| CompileError::Cranelift {
            message: format!("Failed to set opt_level: {e}"),
        })?;
    // Enable PIC for macOS linker compatibility
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

    let mut obj_module = ObjectModule::new(obj_builder);

    // Declare external function: duumbi_print_i64(i64) -> void
    let mut print_sig = obj_module.make_signature();
    print_sig.params.push(AbiParam::new(types::I64));
    let print_func_id = obj_module
        .declare_function("duumbi_print_i64", Linkage::Import, &print_sig)
        .map_err(|e| CompileError::Cranelift {
            message: format!("Failed to declare duumbi_print_i64: {e}"),
        })?;

    // Declare main function: () -> i64
    let mut main_sig = obj_module.make_signature();
    main_sig.returns.push(AbiParam::new(types::I64));
    let main_func_id = obj_module
        .declare_function("main", Linkage::Export, &main_sig)
        .map_err(|e| CompileError::Cranelift {
            message: format!("Failed to declare main: {e}"),
        })?;

    // Build main function IR
    let mut ctx = Context::new();
    ctx.func.signature = main_sig;

    let mut fn_builder_ctx = FunctionBuilderContext::new();
    {
        let mut builder = FunctionBuilder::new(&mut ctx.func, &mut fn_builder_ctx);
        let entry_block = builder.create_block();
        builder.switch_to_block(entry_block);
        builder.seal_block(entry_block);

        // Import the print function reference
        let print_func_ref = obj_module.declare_func_in_func(print_func_id, builder.func);

        // Walk the graph in topological order, building SSA values
        let mut value_map: HashMap<NodeId, cranelift_codegen::ir::Value> = HashMap::new();

        // Process blocks in order for each function
        for func_info in &graph.functions {
            if func_info.name.0 != "main" {
                continue;
            }
            for block_info in &func_info.blocks {
                for &node_idx in &block_info.nodes {
                    let node = &graph.graph[node_idx];

                    match &node.op {
                        Op::Const(val) => {
                            let cl_val = builder.ins().iconst(types::I64, *val);
                            value_map.insert(node.id.clone(), cl_val);
                        }
                        Op::Add | Op::Sub | Op::Mul | Op::Div => {
                            let (left_val, right_val) =
                                get_binary_operands(graph, node_idx, &value_map)?;

                            let result = match node.op {
                                Op::Add => builder.ins().iadd(left_val, right_val),
                                Op::Sub => builder.ins().isub(left_val, right_val),
                                Op::Mul => builder.ins().imul(left_val, right_val),
                                Op::Div => builder.ins().sdiv(left_val, right_val),
                                _ => unreachable!(),
                            };
                            value_map.insert(node.id.clone(), result);
                        }
                        Op::Print => {
                            let operand_val = get_unary_operand(graph, node_idx, &value_map)?;
                            builder.ins().call(print_func_ref, &[operand_val]);
                            // Print does not produce a value
                        }
                        Op::Return => {
                            let operand_val = get_unary_operand(graph, node_idx, &value_map)?;
                            builder.ins().return_(&[operand_val]);
                            // Return does not produce a value
                        }
                    }
                }
            }
        }

        builder.finalize();
    }

    // Define and emit the function
    obj_module
        .define_function(main_func_id, &mut ctx)
        .map_err(|e| CompileError::Cranelift {
            message: format!("Failed to define main: {e}"),
        })?;

    // Finish and produce the object bytes
    let product = obj_module.finish();
    let bytes = product.emit().map_err(|e| CompileError::ObjectEmission {
        message: format!("Failed to emit object: {e}"),
    })?;

    Ok(bytes)
}

/// Resolves the left and right operand SSA values for a binary operation node.
fn get_binary_operands(
    graph: &SemanticGraph,
    node_idx: petgraph::stable_graph::NodeIndex,
    value_map: &HashMap<NodeId, cranelift_codegen::ir::Value>,
) -> Result<(cranelift_codegen::ir::Value, cranelift_codegen::ir::Value), CompileError> {
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
            GraphEdge::Operand => {}
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

/// Resolves the single operand SSA value for a unary operation node (Print, Return).
fn get_unary_operand(
    graph: &SemanticGraph,
    node_idx: petgraph::stable_graph::NodeIndex,
    value_map: &HashMap<NodeId, cranelift_codegen::ir::Value>,
) -> Result<cranelift_codegen::ir::Value, CompileError> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::builder::build_graph;
    use crate::parser::parse_jsonld;

    fn fixture_add() -> String {
        std::fs::read_to_string("tests/fixtures/add.jsonld")
            .expect("invariant: add.jsonld fixture must exist")
    }

    #[test]
    fn compile_add_graph_produces_object() {
        let module = parse_jsonld(&fixture_add()).expect("invariant: fixture must parse");
        let sg = build_graph(&module).expect("invariant: fixture must build");
        let obj_bytes = compile_to_object(&sg).expect("compilation should succeed");

        // Object file should not be empty and should have a valid header
        assert!(!obj_bytes.is_empty());
        // Mach-O magic: 0xFEEDFACF (64-bit) or ELF magic: 0x7F454C46
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
    fn compile_const_return() {
        // Minimal graph: Const(42) -> Return
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
                blocks: vec![BlockInfo {
                    label: BlockLabel("entry".to_string()),
                    nodes: vec![c, r],
                }],
            }],
        };

        let obj_bytes = compile_to_object(&sg).expect("compilation should succeed");
        assert!(!obj_bytes.is_empty());
    }
}
