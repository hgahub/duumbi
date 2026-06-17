//! Core types shared across all duumbi modules.
//!
//! Defines newtypes for identifiers, the Op enum for operations,
//! `CompareOp` for comparison operators, and the `DuumbiType` representation.

use std::fmt;

/// Unique identifier for a node in the semantic graph.
///
/// Wraps the `@id` string from JSON-LD (e.g. `"duumbi:main/main/entry/0"`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NodeId(pub String);

impl fmt::Display for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Unique identifier for an edge in the semantic graph.
#[allow(dead_code)] // Will be used in future phases for edge tracking
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EdgeId(pub String);

/// Label for a basic block within a function.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BlockLabel(pub String);

impl fmt::Display for BlockLabel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Name of a function in the semantic graph.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FunctionName(pub String);

impl fmt::Display for FunctionName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Name of a module in the semantic graph.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ModuleName(pub String);

impl fmt::Display for ModuleName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Comparison operator for `Compare` operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)] // Variants used in Phase 1 parser and compiler
pub enum CompareOp {
    /// Equal.
    Eq,
    /// Not equal.
    Ne,
    /// Less than.
    Lt,
    /// Less than or equal.
    Le,
    /// Greater than.
    Gt,
    /// Greater than or equal.
    Ge,
}

impl fmt::Display for CompareOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CompareOp::Eq => f.write_str("eq"),
            CompareOp::Ne => f.write_str("ne"),
            CompareOp::Lt => f.write_str("lt"),
            CompareOp::Le => f.write_str("le"),
            CompareOp::Gt => f.write_str("gt"),
            CompareOp::Ge => f.write_str("ge"),
        }
    }
}

/// Operation types in the duumbi instruction set.
///
/// Each variant corresponds to a `duumbi:` prefixed `@type` in JSON-LD.
/// Phase 0 ops: Const, Add, Sub, Mul, Div, Print, Return.
/// Phase 1 ops: ConstF64, ConstBool, Compare, Branch, Call, Load, Store.
/// Phase 9a-1 ops: ConstString, String*, Array*, Struct*, PrintString.
/// DUUMBI-378 ops: ReadLine, PrintLn, ReadFile, WriteFile, FileExists,
/// ListDir, PathJoin.
/// DUUMBI-380 ops: Http*, Db* integration stdlib operations.
#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)] // Variants used as parser/compiler are extended
pub enum Op {
    /// Integer constant: `duumbi:Const` with `resultType: "i64"`.
    Const(i64),
    /// Float constant: `duumbi:Const` with `resultType: "f64"`.
    ConstF64(f64),
    /// Boolean constant: `duumbi:Const` with `resultType: "bool"`.
    ConstBool(bool),
    /// String constant: `duumbi:Const` with `resultType: "string"`.
    ConstString(String),
    /// Addition: `duumbi:Add`
    Add,
    /// Subtraction: `duumbi:Sub`
    Sub,
    /// Multiplication: `duumbi:Mul`
    Mul,
    /// Division: `duumbi:Div`
    Div,
    /// Checked integer addition returning Result<i64,string>: `duumbi:AddChecked`
    AddChecked,
    /// Checked integer subtraction returning Result<i64,string>: `duumbi:SubChecked`
    SubChecked,
    /// Checked integer multiplication returning Result<i64,string>: `duumbi:MulChecked`
    MulChecked,
    /// Checked integer division returning Result<i64,string>: `duumbi:DivChecked`
    DivChecked,
    /// Comparison: `duumbi:Compare`
    Compare(CompareOp),
    /// Conditional branch: `duumbi:Branch`
    Branch,
    /// Function call: `duumbi:Call`
    Call {
        /// Optional module qualifier for cross-module calls.
        module: Option<String>,
        /// Name of the function to call.
        function: String,
    },
    /// Load variable: `duumbi:Load`
    Load {
        /// Name of the variable to load.
        variable: String,
    },
    /// Store variable: `duumbi:Store`
    Store {
        /// Name of the variable to store into.
        variable: String,
    },
    /// Print value to stdout: `duumbi:Print`
    Print,
    /// Print string to stdout: `duumbi:PrintString`
    PrintString,
    /// Read one UTF-8 line from stdin: `duumbi:ReadLine`
    ReadLine,
    /// Print a string plus exactly one newline: `duumbi:PrintLn`
    PrintLn,
    /// Read a UTF-8 file with an explicit byte bound: `duumbi:ReadFile`
    ReadFile,
    /// Overwrite a UTF-8 file: `duumbi:WriteFile`
    WriteFile,
    /// Check whether a workspace-confined path exists: `duumbi:FileExists`
    FileExists,
    /// List directory entry names deterministically: `duumbi:ListDir`
    ListDir,
    /// Join two DUUMBI-relative path components: `duumbi:PathJoin`
    PathJoin,
    /// Return value from function: `duumbi:Return`
    Return,

    // -- String operations (Phase 9a-1) --
    /// Concatenate two strings: `duumbi:StringConcat`
    StringConcat,
    /// String equality check → bool: `duumbi:StringEquals`
    StringEquals,
    /// Lexicographic string comparison: `duumbi:StringCompare`
    StringCompare(CompareOp),
    /// String length → i64: `duumbi:StringLength`
    StringLength,
    /// Substring extraction: `duumbi:StringSlice`
    StringSlice,
    /// Check if string contains substring → bool: `duumbi:StringContains`
    StringContains,
    /// Find index of substring → i64 (-1 if not found): `duumbi:StringFind`
    StringFind,
    /// Convert i64 to string: `duumbi:StringFromI64`
    StringFromI64,
    /// Trim leading/trailing whitespace from a string: `duumbi:StringTrim`
    StringTrim,
    /// Convert string to ASCII uppercase: `duumbi:StringToUpper`
    StringToUpper,
    /// Convert string to ASCII lowercase: `duumbi:StringToLower`
    StringToLower,
    /// Replace first occurrence of needle with replacement: `duumbi:StringReplace`
    StringReplace,

    // -- Type cast operations (Phase 9A) --
    /// Cast i64 to f64: `duumbi:CastI64ToF64`
    CastI64ToF64,
    /// Cast f64 to i64 (saturating): `duumbi:CastF64ToI64`
    CastF64ToI64,

    // -- Array operations (Phase 9a-1) --
    /// Create empty array: `duumbi:ArrayNew`
    ArrayNew,
    /// Append element to array: `duumbi:ArrayPush`
    ArrayPush,
    /// Get element at index (panic on OOB): `duumbi:ArrayGet`
    ArrayGet,
    /// Set element at index (panic on OOB): `duumbi:ArraySet`
    ArraySet,
    /// Array length → i64: `duumbi:ArrayLength`
    ArrayLength,
    /// Safe get → Option<T> (no panic): `duumbi:ArrayTryGet`
    ArrayTryGet,

    // -- Struct operations (Phase 9a-1) --
    /// Create new struct instance: `duumbi:StructNew`
    StructNew {
        /// Name of the struct type to instantiate.
        struct_name: String,
    },
    /// Get field value from struct: `duumbi:FieldGet`
    FieldGet {
        /// Name of the field to read.
        field_name: String,
    },
    /// Set field value on struct: `duumbi:FieldSet`
    FieldSet {
        /// Name of the field to write.
        field_name: String,
    },

    // -- Ownership operations (Phase 9a-2) --
    /// Allocate heap value: `duumbi:Alloc`
    Alloc {
        /// Type to allocate.
        alloc_type: DuumbiType,
    },
    /// Move ownership: `duumbi:Move`
    Move {
        /// Source node ID to move from.
        source: String,
    },
    /// Borrow (shared or mutable): `duumbi:Borrow` / `duumbi:BorrowMut`
    Borrow {
        /// Source node ID to borrow from.
        source: String,
        /// Whether the borrow is mutable.
        mutable: bool,
    },
    /// Drop heap value: `duumbi:Drop`
    Drop {
        /// Target node ID to drop.
        target: String,
    },

    // -- Result operations (Phase 9a-3) --
    /// Wrap value in Ok: `duumbi:ResultOk`
    ResultOk,
    /// Wrap value in Err: `duumbi:ResultErr`
    ResultErr,
    /// Check if Result is Ok → bool: `duumbi:ResultIsOk`
    ResultIsOk,
    /// Extract Ok payload (panics on Err): `duumbi:ResultUnwrap`
    ResultUnwrap,
    /// Extract Err payload (panics on Ok): `duumbi:ResultUnwrapErr`
    ResultUnwrapErr,

    // -- Option operations (Phase 9a-3) --
    /// Wrap value in Some: `duumbi:OptionSome`
    OptionSome,
    /// Create None: `duumbi:OptionNone`
    OptionNone,
    /// Check if Option is Some → bool: `duumbi:OptionIsSome`
    OptionIsSome,
    /// Extract Some payload (panics on None): `duumbi:OptionUnwrap`
    OptionUnwrap,

    // -- JSON operations (DUUMBI-379) --
    /// Parse a string as JSON: `duumbi:JsonParse`
    JsonParse,
    /// Stringify a JSON value: `duumbi:JsonStringify`
    JsonStringify,
    /// Get an object field from a JSON value: `duumbi:JsonGetField`
    JsonGetField,
    /// Get the length of a JSON array: `duumbi:JsonArrayLen`
    JsonArrayLen,
    /// Get an item from a JSON array: `duumbi:JsonArrayGet`
    JsonArrayGet,

    // -- TCP operations (DUUMBI-379) --
    /// Connect to a TCP endpoint: `duumbi:TcpConnect`
    TcpConnect,
    /// Create a TCP listener: `duumbi:TcpListen`
    TcpListen,
    /// Accept one TCP connection: `duumbi:TcpAccept`
    TcpAccept,
    /// Read text from a TCP socket: `duumbi:TcpRead`
    TcpRead,
    /// Write text to a TCP socket: `duumbi:TcpWrite`
    TcpWrite,
    /// Close a TCP socket: `duumbi:TcpClose`
    TcpClose,
    /// Close a TCP listener: `duumbi:TcpListenerClose`
    TcpListenerClose,

    // -- HTTP server operations (DUUMBI-381) --
    /// Create a bounded local HTTP server: `duumbi:ServerNew`
    ServerNew,
    /// Register a static HTTP route: `duumbi:RouteAddStatic`
    RouteAddStatic,
    /// Serve bounded requests: `duumbi:ServerStart`
    ServerStart,
    /// Close an HTTP server resource: `duumbi:ServerClose`
    ServerClose,

    // -- HTTP operations (DUUMBI-380) --
    /// HTTP GET request: `duumbi:HttpGet`
    HttpGet,
    /// HTTP POST request: `duumbi:HttpPost`
    HttpPost,
    /// HTTP PUT request: `duumbi:HttpPut`
    HttpPut,
    /// HTTP DELETE request: `duumbi:HttpDelete`
    HttpDelete,
    /// Read response status: `duumbi:HttpStatus`
    HttpStatus,
    /// Read response body: `duumbi:HttpBody`
    HttpBody,
    /// Read response headers as JSON: `duumbi:HttpHeaders`
    HttpHeaders,
    /// Close/free an HTTP response resource: `duumbi:HttpResponseFree`
    HttpResponseFree,

    // -- Database operations (DUUMBI-380) --
    /// Open a SQLite database: `duumbi:DbOpen`
    DbOpen,
    /// Execute a SQLite statement: `duumbi:DbExecute`
    DbExecute,
    /// Query SQLite rows: `duumbi:DbQuery`
    DbQuery,
    /// Return row count: `duumbi:DbRowsLen`
    DbRowsLen,
    /// Read a row column as string: `duumbi:DbRowGet`
    DbRowGet,
    /// Close/free a DB connection resource: `duumbi:DbClose`
    DbClose,
    /// Close/free DB rows resource: `duumbi:DbRowsFree`
    DbRowsFree,

    // -- Match operation (Phase 9a-3) --
    /// Pattern match on Result/Option — branches to Ok/Some or Err/None block: `duumbi:Match`
    Match {
        /// Block label for the Ok/Some branch.
        ok_block: String,
        /// Block label for the Err/None branch.
        err_block: String,
    },

    // -- Math operations (Phase 9A) --
    /// Modulo (remainder): `duumbi:Modulo` — i64: `srem`, f64: `duumbi_fmod` C shim
    Modulo,
    /// Negate: `duumbi:Negate` — i64: `ineg`, f64: `fneg`
    Negate,
    /// Square root (f64 only): `duumbi:Sqrt` — via C shim `duumbi_sqrt`
    Sqrt,
    /// Float power (f64 only): `duumbi:Pow` — via C shim `duumbi_pow`
    Pow,
    /// Integer power (i64 only): `duumbi:PowI64` — via C shim `duumbi_powi64`
    PowI64,

    // -- Bitwise operations (Phase 9A) --
    /// Bitwise AND (i64): `duumbi:BitwiseAnd` — `band`
    BitwiseAnd,
    /// Bitwise OR (i64): `duumbi:BitwiseOr` — `bor`
    BitwiseOr,
    /// Bitwise XOR (i64): `duumbi:BitwiseXor` — `bxor`
    BitwiseXor,
    /// Bitwise NOT (i64): `duumbi:BitwiseNot` — `bnot`
    BitwiseNot,
    /// Shift left (i64): `duumbi:ShiftLeft` — `ishl`
    ShiftLeft,
    /// Shift right arithmetic (i64): `duumbi:ShiftRight` — `sshr`
    ShiftRight,
}

impl fmt::Display for Op {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Op::Const(v) => write!(f, "Const({v})"),
            Op::ConstF64(v) => write!(f, "ConstF64({v})"),
            Op::ConstBool(v) => write!(f, "ConstBool({v})"),
            Op::ConstString(v) => write!(f, "ConstString(\"{v}\")"),
            Op::Add => f.write_str("Add"),
            Op::Sub => f.write_str("Sub"),
            Op::Mul => f.write_str("Mul"),
            Op::Div => f.write_str("Div"),
            Op::AddChecked => f.write_str("AddChecked"),
            Op::SubChecked => f.write_str("SubChecked"),
            Op::MulChecked => f.write_str("MulChecked"),
            Op::DivChecked => f.write_str("DivChecked"),
            Op::Compare(op) => write!(f, "Compare({op})"),
            Op::Branch => f.write_str("Branch"),
            Op::Call { module, function } => {
                if let Some(module) = module {
                    write!(f, "Call({module}::{function})")
                } else {
                    write!(f, "Call({function})")
                }
            }
            Op::Load { variable } => write!(f, "Load({variable})"),
            Op::Store { variable } => write!(f, "Store({variable})"),
            Op::Print => f.write_str("Print"),
            Op::PrintString => f.write_str("PrintString"),
            Op::ReadLine => f.write_str("ReadLine"),
            Op::PrintLn => f.write_str("PrintLn"),
            Op::ReadFile => f.write_str("ReadFile"),
            Op::WriteFile => f.write_str("WriteFile"),
            Op::FileExists => f.write_str("FileExists"),
            Op::ListDir => f.write_str("ListDir"),
            Op::PathJoin => f.write_str("PathJoin"),
            Op::Return => f.write_str("Return"),
            Op::StringConcat => f.write_str("StringConcat"),
            Op::StringEquals => f.write_str("StringEquals"),
            Op::StringCompare(op) => write!(f, "StringCompare({op})"),
            Op::StringLength => f.write_str("StringLength"),
            Op::StringSlice => f.write_str("StringSlice"),
            Op::StringContains => f.write_str("StringContains"),
            Op::StringFind => f.write_str("StringFind"),
            Op::StringFromI64 => f.write_str("StringFromI64"),
            Op::StringTrim => f.write_str("StringTrim"),
            Op::StringToUpper => f.write_str("StringToUpper"),
            Op::StringToLower => f.write_str("StringToLower"),
            Op::StringReplace => f.write_str("StringReplace"),
            Op::CastI64ToF64 => f.write_str("CastI64ToF64"),
            Op::CastF64ToI64 => f.write_str("CastF64ToI64"),
            Op::ArrayNew => f.write_str("ArrayNew"),
            Op::ArrayPush => f.write_str("ArrayPush"),
            Op::ArrayGet => f.write_str("ArrayGet"),
            Op::ArraySet => f.write_str("ArraySet"),
            Op::ArrayLength => f.write_str("ArrayLength"),
            Op::ArrayTryGet => f.write_str("ArrayTryGet"),
            Op::StructNew { struct_name } => write!(f, "StructNew({struct_name})"),
            Op::FieldGet { field_name } => write!(f, "FieldGet({field_name})"),
            Op::FieldSet { field_name } => write!(f, "FieldSet({field_name})"),
            Op::Alloc { alloc_type } => write!(f, "Alloc({alloc_type})"),
            Op::Move { source } => write!(f, "Move({source})"),
            Op::Borrow {
                source,
                mutable: true,
            } => write!(f, "BorrowMut({source})"),
            Op::Borrow {
                source,
                mutable: false,
            } => write!(f, "Borrow({source})"),
            Op::Drop { target } => write!(f, "Drop({target})"),
            Op::ResultOk => f.write_str("ResultOk"),
            Op::ResultErr => f.write_str("ResultErr"),
            Op::ResultIsOk => f.write_str("ResultIsOk"),
            Op::ResultUnwrap => f.write_str("ResultUnwrap"),
            Op::ResultUnwrapErr => f.write_str("ResultUnwrapErr"),
            Op::OptionSome => f.write_str("OptionSome"),
            Op::OptionNone => f.write_str("OptionNone"),
            Op::OptionIsSome => f.write_str("OptionIsSome"),
            Op::OptionUnwrap => f.write_str("OptionUnwrap"),
            Op::JsonParse => f.write_str("JsonParse"),
            Op::JsonStringify => f.write_str("JsonStringify"),
            Op::JsonGetField => f.write_str("JsonGetField"),
            Op::JsonArrayLen => f.write_str("JsonArrayLen"),
            Op::JsonArrayGet => f.write_str("JsonArrayGet"),
            Op::TcpConnect => f.write_str("TcpConnect"),
            Op::TcpListen => f.write_str("TcpListen"),
            Op::TcpAccept => f.write_str("TcpAccept"),
            Op::TcpRead => f.write_str("TcpRead"),
            Op::TcpWrite => f.write_str("TcpWrite"),
            Op::TcpClose => f.write_str("TcpClose"),
            Op::TcpListenerClose => f.write_str("TcpListenerClose"),
            Op::ServerNew => f.write_str("ServerNew"),
            Op::RouteAddStatic => f.write_str("RouteAddStatic"),
            Op::ServerStart => f.write_str("ServerStart"),
            Op::ServerClose => f.write_str("ServerClose"),
            Op::HttpGet => f.write_str("HttpGet"),
            Op::HttpPost => f.write_str("HttpPost"),
            Op::HttpPut => f.write_str("HttpPut"),
            Op::HttpDelete => f.write_str("HttpDelete"),
            Op::HttpStatus => f.write_str("HttpStatus"),
            Op::HttpBody => f.write_str("HttpBody"),
            Op::HttpHeaders => f.write_str("HttpHeaders"),
            Op::HttpResponseFree => f.write_str("HttpResponseFree"),
            Op::DbOpen => f.write_str("DbOpen"),
            Op::DbExecute => f.write_str("DbExecute"),
            Op::DbQuery => f.write_str("DbQuery"),
            Op::DbRowsLen => f.write_str("DbRowsLen"),
            Op::DbRowGet => f.write_str("DbRowGet"),
            Op::DbClose => f.write_str("DbClose"),
            Op::DbRowsFree => f.write_str("DbRowsFree"),
            Op::Match {
                ok_block,
                err_block,
            } => write!(f, "Match({ok_block},{err_block})"),
            Op::Modulo => f.write_str("Modulo"),
            Op::Negate => f.write_str("Negate"),
            Op::Sqrt => f.write_str("Sqrt"),
            Op::Pow => f.write_str("Pow"),
            Op::PowI64 => f.write_str("PowI64"),
            Op::BitwiseAnd => f.write_str("BitwiseAnd"),
            Op::BitwiseOr => f.write_str("BitwiseOr"),
            Op::BitwiseXor => f.write_str("BitwiseXor"),
            Op::BitwiseNot => f.write_str("BitwiseNot"),
            Op::ShiftLeft => f.write_str("ShiftLeft"),
            Op::ShiftRight => f.write_str("ShiftRight"),
        }
    }
}

impl Op {
    /// Resolves the output type of this operation.
    ///
    /// For ops whose output type depends on context (e.g. `Const`, `Add`, `Load`),
    /// the `result_type` from the graph node is returned. For ops with a fixed
    /// output type (e.g. `Compare` → Bool, `Print` → Void), the fixed type is
    /// returned regardless of `result_type`.
    ///
    /// Returns `None` for `Return` and `Branch` (no output value).
    #[must_use]
    pub fn output_type(&self, result_type: &Option<DuumbiType>) -> Option<DuumbiType> {
        match self {
            Op::Const(_)
            | Op::ConstF64(_)
            | Op::ConstBool(_)
            | Op::ConstString(_)
            | Op::Add
            | Op::Sub
            | Op::Mul
            | Op::Div
            | Op::AddChecked
            | Op::SubChecked
            | Op::MulChecked
            | Op::DivChecked
            | Op::Load { .. }
            | Op::Call { .. }
            | Op::ArrayNew
            | Op::ArrayGet
            | Op::ArrayTryGet
            | Op::StructNew { .. }
            | Op::FieldGet { .. }
            | Op::Alloc { .. }
            | Op::Move { .. }
            | Op::Borrow { .. }
            | Op::ReadLine
            | Op::PrintLn
            | Op::ReadFile
            | Op::WriteFile
            | Op::FileExists
            | Op::ListDir
            | Op::PathJoin
            | Op::JsonParse
            | Op::JsonStringify
            | Op::JsonGetField
            | Op::JsonArrayLen
            | Op::JsonArrayGet
            | Op::TcpConnect
            | Op::TcpListen
            | Op::TcpAccept
            | Op::TcpRead
            | Op::TcpWrite
            | Op::TcpClose
            | Op::TcpListenerClose
            | Op::ServerNew
            | Op::RouteAddStatic
            | Op::ServerStart
            | Op::ServerClose
            | Op::HttpGet
            | Op::HttpPost
            | Op::HttpPut
            | Op::HttpDelete
            | Op::HttpStatus
            | Op::HttpBody
            | Op::HttpHeaders
            | Op::HttpResponseFree
            | Op::DbOpen
            | Op::DbExecute
            | Op::DbQuery
            | Op::DbRowsLen
            | Op::DbRowGet
            | Op::DbClose
            | Op::DbRowsFree => result_type.clone(),
            Op::Compare(_) | Op::StringEquals | Op::StringContains => Some(DuumbiType::Bool),
            Op::StringCompare(_) => Some(DuumbiType::Bool),
            Op::StringConcat
            | Op::StringSlice
            | Op::StringFromI64
            | Op::StringTrim
            | Op::StringToUpper
            | Op::StringToLower
            | Op::StringReplace => Some(DuumbiType::String),
            Op::StringLength | Op::StringFind | Op::ArrayLength => Some(DuumbiType::I64),
            Op::CastI64ToF64 => Some(DuumbiType::F64),
            Op::CastF64ToI64 => Some(DuumbiType::I64),
            Op::Print
            | Op::PrintString
            | Op::Store { .. }
            | Op::ArrayPush
            | Op::ArraySet
            | Op::FieldSet { .. }
            | Op::Drop { .. } => Some(DuumbiType::Void),
            // Math ops with context-dependent output type (i64 or f64)
            Op::Modulo | Op::Negate => result_type.clone(),
            // Sqrt/Pow always produce f64; PowI64 always produces i64
            Op::Sqrt | Op::Pow => Some(DuumbiType::F64),
            Op::PowI64 => Some(DuumbiType::I64),
            // Bitwise ops always produce i64
            Op::BitwiseAnd
            | Op::BitwiseOr
            | Op::BitwiseXor
            | Op::BitwiseNot
            | Op::ShiftLeft
            | Op::ShiftRight => Some(DuumbiType::I64),
            // Result/Option ops with context-dependent output
            Op::ResultOk | Op::ResultErr | Op::OptionSome | Op::OptionNone => result_type.clone(),
            // Result/Option ops with fixed output types
            Op::ResultIsOk | Op::OptionIsSome => Some(DuumbiType::Bool),
            // Unwrap extracts the payload — output type from context
            Op::ResultUnwrap | Op::ResultUnwrapErr | Op::OptionUnwrap => result_type.clone(),
            Op::Return | Op::Branch | Op::Match { .. } => None,
        }
    }
}

/// Type in the duumbi type system.
///
/// Primitive types (`I64`, `F64`, `Bool`, `Void`) are stack-allocated.
/// Heap types (`String`, `Array`, `Struct`) are pointer-sized at the
/// Cranelift level and require runtime allocation/deallocation.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DuumbiType {
    /// 64-bit signed integer.
    I64,
    /// 64-bit floating point.
    F64,
    /// Boolean (true/false).
    Bool,
    /// No return value (used by Print).
    Void,
    /// Heap-allocated UTF-8 string.
    #[allow(dead_code)] // Used starting from Phase 9a-1 string ops
    String,
    /// Opaque runtime-owned JSON value.
    Json,
    /// Opaque runtime-owned TCP socket resource.
    TcpSocket,
    /// Opaque runtime-owned TCP listener resource.
    TcpListener,
    /// Opaque runtime-owned HTTP server resource.
    HttpServer,
    /// Opaque runtime-owned HTTP response resource.
    HttpResponse,
    /// Opaque runtime-owned SQLite database connection resource.
    DbConnection,
    /// Opaque runtime-owned SQLite row-set resource.
    DbRows,
    /// Homogeneous dynamic array, generic over element type.
    #[allow(dead_code)] // Used starting from Phase 9a-1 array ops
    Array(Box<DuumbiType>),
    /// Named struct type. Field definitions are stored in the struct registry,
    /// not in this enum — only the struct name is carried here.
    #[allow(dead_code)] // Used starting from Phase 9a-1 struct ops
    Struct(std::string::String),
    /// Shared reference to a value.
    #[allow(dead_code)] // Used starting from Phase 9a-2 ownership ops
    Ref(Box<DuumbiType>),
    /// Mutable reference to a value.
    #[allow(dead_code)] // Used starting from Phase 9a-2 ownership ops
    RefMut(Box<DuumbiType>),
    /// Result type with Ok and Err variants: `result<T, E>`.
    #[allow(dead_code)] // Used starting from Phase 9a-3 error handling ops
    Result(Box<DuumbiType>, Box<DuumbiType>),
    /// Option type with Some and None variants: `option<T>`.
    #[allow(dead_code)] // Used starting from Phase 9a-3 error handling ops
    Option(Box<DuumbiType>),
}

impl DuumbiType {
    /// Returns `true` if this type requires heap allocation.
    #[must_use]
    #[allow(dead_code)] // Used starting from Phase 9a-1 codegen
    pub fn is_heap_type(&self) -> bool {
        matches!(
            self,
            DuumbiType::String
                | DuumbiType::Json
                | DuumbiType::TcpSocket
                | DuumbiType::TcpListener
                | DuumbiType::HttpServer
                | DuumbiType::HttpResponse
                | DuumbiType::DbConnection
                | DuumbiType::DbRows
                | DuumbiType::Array(_)
                | DuumbiType::Struct(_)
                | DuumbiType::Result(_, _)
                | DuumbiType::Option(_)
        )
    }

    /// Returns `true` if this type is a reference (`&T` or `&mut T`).
    #[must_use]
    #[allow(dead_code)] // Used starting from Phase 9a-2 ownership checks
    pub fn is_reference(&self) -> bool {
        matches!(self, DuumbiType::Ref(_) | DuumbiType::RefMut(_))
    }

    /// Returns the inner type for references, or `self` for non-references.
    #[must_use]
    #[allow(dead_code)] // Used starting from Phase 9a-2 ownership checks
    pub fn inner_type(&self) -> &DuumbiType {
        match self {
            DuumbiType::Ref(inner) | DuumbiType::RefMut(inner) => inner,
            other => other,
        }
    }

    /// Returns `true` if this type is a mutable reference (`&mut T`).
    #[must_use]
    #[allow(dead_code)] // Used starting from Phase 9a-2 ownership checks
    pub fn is_mutable_ref(&self) -> bool {
        matches!(self, DuumbiType::RefMut(_))
    }

    /// Returns `true` if this type is a `Result<T, E>`.
    #[must_use]
    #[allow(dead_code)] // Used starting from Phase 9a-3 error handling
    pub fn is_result(&self) -> bool {
        matches!(self, DuumbiType::Result(_, _))
    }

    /// Returns `true` if this type is an `Option<T>`.
    #[must_use]
    #[allow(dead_code)] // Used starting from Phase 9a-3 error handling
    pub fn is_option(&self) -> bool {
        matches!(self, DuumbiType::Option(_))
    }
}

impl fmt::Display for DuumbiType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DuumbiType::I64 => f.write_str("i64"),
            DuumbiType::F64 => f.write_str("f64"),
            DuumbiType::Bool => f.write_str("bool"),
            DuumbiType::Void => f.write_str("void"),
            DuumbiType::String => f.write_str("string"),
            DuumbiType::Json => f.write_str("json"),
            DuumbiType::TcpSocket => f.write_str("tcp_socket"),
            DuumbiType::TcpListener => f.write_str("tcp_listener"),
            DuumbiType::HttpServer => f.write_str("http_server"),
            DuumbiType::HttpResponse => f.write_str("http_response"),
            DuumbiType::DbConnection => f.write_str("db_connection"),
            DuumbiType::DbRows => f.write_str("db_rows"),
            DuumbiType::Array(elem) => write!(f, "array<{elem}>"),
            DuumbiType::Struct(name) => write!(f, "struct<{name}>"),
            DuumbiType::Ref(inner) => write!(f, "&{inner}"),
            DuumbiType::RefMut(inner) => write!(f, "&mut {inner}"),
            DuumbiType::Result(ok, err) => write!(f, "result<{ok},{err}>"),
            DuumbiType::Option(inner) => write!(f, "option<{inner}>"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compare_op_display() {
        assert_eq!(CompareOp::Eq.to_string(), "eq");
        assert_eq!(CompareOp::Ne.to_string(), "ne");
        assert_eq!(CompareOp::Lt.to_string(), "lt");
        assert_eq!(CompareOp::Le.to_string(), "le");
        assert_eq!(CompareOp::Gt.to_string(), "gt");
        assert_eq!(CompareOp::Ge.to_string(), "ge");
    }

    #[test]
    fn op_display_phase1_variants() {
        assert_eq!(Op::ConstF64(2.5).to_string(), "ConstF64(2.5)");
        assert_eq!(Op::ConstBool(true).to_string(), "ConstBool(true)");
        assert_eq!(Op::Compare(CompareOp::Lt).to_string(), "Compare(lt)");
        assert_eq!(Op::Branch.to_string(), "Branch");
        assert_eq!(
            Op::Call {
                module: None,
                function: "fib".to_string()
            }
            .to_string(),
            "Call(fib)"
        );
        assert_eq!(
            Op::Call {
                module: Some("math/ops".to_string()),
                function: "fib".to_string()
            }
            .to_string(),
            "Call(math/ops::fib)"
        );
        assert_eq!(
            Op::Load {
                variable: "x".to_string()
            }
            .to_string(),
            "Load(x)"
        );
        assert_eq!(
            Op::Store {
                variable: "x".to_string()
            }
            .to_string(),
            "Store(x)"
        );
    }

    #[test]
    fn duumbi_type_display_phase1() {
        assert_eq!(DuumbiType::F64.to_string(), "f64");
        assert_eq!(DuumbiType::Bool.to_string(), "bool");
    }

    #[test]
    fn duumbi_type_display_heap_types() {
        assert_eq!(DuumbiType::String.to_string(), "string");
        assert_eq!(
            DuumbiType::Array(Box::new(DuumbiType::I64)).to_string(),
            "array<i64>"
        );
        assert_eq!(
            DuumbiType::Array(Box::new(DuumbiType::String)).to_string(),
            "array<string>"
        );
        assert_eq!(
            DuumbiType::Struct("Point".to_string()).to_string(),
            "struct<Point>"
        );
        assert_eq!(DuumbiType::Json.to_string(), "json");
        assert_eq!(DuumbiType::TcpSocket.to_string(), "tcp_socket");
        assert_eq!(DuumbiType::TcpListener.to_string(), "tcp_listener");
        assert_eq!(DuumbiType::HttpServer.to_string(), "http_server");
        assert_eq!(DuumbiType::HttpServer.to_string(), "http_server");
        assert_eq!(DuumbiType::HttpResponse.to_string(), "http_response");
        assert_eq!(DuumbiType::DbConnection.to_string(), "db_connection");
        assert_eq!(DuumbiType::DbRows.to_string(), "db_rows");
    }

    #[test]
    fn duumbi_type_is_heap_type() {
        assert!(!DuumbiType::I64.is_heap_type());
        assert!(!DuumbiType::F64.is_heap_type());
        assert!(!DuumbiType::Bool.is_heap_type());
        assert!(!DuumbiType::Void.is_heap_type());
        assert!(DuumbiType::String.is_heap_type());
        assert!(DuumbiType::Json.is_heap_type());
        assert!(DuumbiType::TcpSocket.is_heap_type());
        assert!(DuumbiType::TcpListener.is_heap_type());
        assert!(DuumbiType::HttpServer.is_heap_type());
        assert!(DuumbiType::HttpServer.is_heap_type());
        assert!(DuumbiType::HttpResponse.is_heap_type());
        assert!(DuumbiType::DbConnection.is_heap_type());
        assert!(DuumbiType::DbRows.is_heap_type());
        assert!(DuumbiType::Array(Box::new(DuumbiType::I64)).is_heap_type());
        assert!(DuumbiType::Struct("Point".to_string()).is_heap_type());
    }

    #[test]
    fn duumbi_type_equality() {
        assert_eq!(DuumbiType::String, DuumbiType::String);
        assert_eq!(
            DuumbiType::Array(Box::new(DuumbiType::I64)),
            DuumbiType::Array(Box::new(DuumbiType::I64))
        );
        assert_ne!(
            DuumbiType::Array(Box::new(DuumbiType::I64)),
            DuumbiType::Array(Box::new(DuumbiType::F64))
        );
        assert_eq!(
            DuumbiType::Struct("Point".to_string()),
            DuumbiType::Struct("Point".to_string())
        );
        assert_ne!(
            DuumbiType::Struct("Point".to_string()),
            DuumbiType::Struct("Vec2".to_string())
        );
    }

    #[test]
    fn op_display_ownership_variants() {
        assert_eq!(
            Op::Alloc {
                alloc_type: DuumbiType::String
            }
            .to_string(),
            "Alloc(string)"
        );
        assert_eq!(
            Op::Move {
                source: "x".to_string()
            }
            .to_string(),
            "Move(x)"
        );
        assert_eq!(
            Op::Borrow {
                source: "x".to_string(),
                mutable: false
            }
            .to_string(),
            "Borrow(x)"
        );
        assert_eq!(
            Op::Borrow {
                source: "x".to_string(),
                mutable: true
            }
            .to_string(),
            "BorrowMut(x)"
        );
        assert_eq!(
            Op::Drop {
                target: "x".to_string()
            }
            .to_string(),
            "Drop(x)"
        );
    }

    #[test]
    fn duumbi_type_display_references() {
        assert_eq!(
            DuumbiType::Ref(Box::new(DuumbiType::String)).to_string(),
            "&string"
        );
        assert_eq!(
            DuumbiType::RefMut(Box::new(DuumbiType::String)).to_string(),
            "&mut string"
        );
        assert_eq!(
            DuumbiType::Ref(Box::new(DuumbiType::Array(Box::new(DuumbiType::I64)))).to_string(),
            "&array<i64>"
        );
    }

    #[test]
    fn duumbi_type_reference_helpers() {
        let shared = DuumbiType::Ref(Box::new(DuumbiType::String));
        assert!(shared.is_reference());
        assert!(!shared.is_mutable_ref());
        assert_eq!(*shared.inner_type(), DuumbiType::String);

        let mutable = DuumbiType::RefMut(Box::new(DuumbiType::I64));
        assert!(mutable.is_reference());
        assert!(mutable.is_mutable_ref());
        assert_eq!(*mutable.inner_type(), DuumbiType::I64);

        let plain = DuumbiType::I64;
        assert!(!plain.is_reference());
        assert!(!plain.is_mutable_ref());
        assert_eq!(*plain.inner_type(), DuumbiType::I64);
    }

    #[test]
    fn duumbi_type_ref_not_heap() {
        assert!(!DuumbiType::Ref(Box::new(DuumbiType::String)).is_heap_type());
        assert!(!DuumbiType::RefMut(Box::new(DuumbiType::String)).is_heap_type());
    }

    #[test]
    fn ownership_op_output_types() {
        let alloc = Op::Alloc {
            alloc_type: DuumbiType::String,
        };
        assert_eq!(
            alloc.output_type(&Some(DuumbiType::String)),
            Some(DuumbiType::String)
        );

        let mv = Op::Move {
            source: "x".to_string(),
        };
        assert_eq!(
            mv.output_type(&Some(DuumbiType::String)),
            Some(DuumbiType::String)
        );

        let borrow = Op::Borrow {
            source: "x".to_string(),
            mutable: false,
        };
        assert_eq!(
            borrow.output_type(&Some(DuumbiType::Ref(Box::new(DuumbiType::String)))),
            Some(DuumbiType::Ref(Box::new(DuumbiType::String)))
        );

        let drop = Op::Drop {
            target: "x".to_string(),
        };
        assert_eq!(drop.output_type(&None), Some(DuumbiType::Void));
    }

    #[test]
    fn duumbi_type_result_display() {
        assert_eq!(
            DuumbiType::Result(Box::new(DuumbiType::I64), Box::new(DuumbiType::I64)).to_string(),
            "result<i64,i64>"
        );
        assert_eq!(
            DuumbiType::Result(Box::new(DuumbiType::F64), Box::new(DuumbiType::String)).to_string(),
            "result<f64,string>"
        );
    }

    #[test]
    fn duumbi_type_option_display() {
        assert_eq!(
            DuumbiType::Option(Box::new(DuumbiType::I64)).to_string(),
            "option<i64>"
        );
        assert_eq!(
            DuumbiType::Option(Box::new(DuumbiType::String)).to_string(),
            "option<string>"
        );
    }

    #[test]
    fn duumbi_type_result_option_helpers() {
        let result_ty = DuumbiType::Result(Box::new(DuumbiType::I64), Box::new(DuumbiType::String));
        assert!(result_ty.is_result());
        assert!(!result_ty.is_option());
        assert!(result_ty.is_heap_type());
        assert!(!result_ty.is_reference());

        let option_ty = DuumbiType::Option(Box::new(DuumbiType::I64));
        assert!(option_ty.is_option());
        assert!(!option_ty.is_result());
        assert!(option_ty.is_heap_type());
        assert!(!option_ty.is_reference());
    }

    #[test]
    fn json_tcp_op_output_types() {
        let json_result =
            DuumbiType::Result(Box::new(DuumbiType::Json), Box::new(DuumbiType::String));
        assert_eq!(
            Op::JsonParse.output_type(&Some(json_result.clone())),
            Some(json_result.clone())
        );
        assert_eq!(
            Op::JsonGetField.output_type(&Some(json_result.clone())),
            Some(json_result)
        );

        let socket_result = DuumbiType::Result(
            Box::new(DuumbiType::TcpSocket),
            Box::new(DuumbiType::String),
        );
        assert_eq!(
            Op::TcpConnect.output_type(&Some(socket_result.clone())),
            Some(socket_result.clone())
        );
        assert_eq!(
            Op::TcpAccept.output_type(&Some(socket_result.clone())),
            Some(socket_result)
        );

        let close_result =
            DuumbiType::Result(Box::new(DuumbiType::I64), Box::new(DuumbiType::String));
        assert_eq!(
            Op::TcpClose.output_type(&Some(close_result.clone())),
            Some(close_result.clone())
        );
        assert_eq!(
            Op::TcpListenerClose.output_type(&Some(close_result.clone())),
            Some(close_result.clone())
        );

        let server_result = DuumbiType::Result(
            Box::new(DuumbiType::HttpServer),
            Box::new(DuumbiType::String),
        );
        assert_eq!(
            Op::ServerNew.output_type(&Some(server_result.clone())),
            Some(server_result)
        );
        assert_eq!(
            Op::RouteAddStatic.output_type(&Some(close_result.clone())),
            Some(close_result.clone())
        );
        assert_eq!(
            Op::ServerStart.output_type(&Some(close_result.clone())),
            Some(close_result.clone())
        );
        assert_eq!(
            Op::ServerClose.output_type(&Some(close_result.clone())),
            Some(close_result)
        );
    }

    #[test]
    fn http_db_op_output_types() {
        let http_result = DuumbiType::Result(
            Box::new(DuumbiType::HttpResponse),
            Box::new(DuumbiType::String),
        );
        assert_eq!(
            Op::HttpGet.output_type(&Some(http_result.clone())),
            Some(http_result.clone())
        );
        assert_eq!(
            Op::HttpPost.output_type(&Some(http_result.clone())),
            Some(http_result)
        );

        let db_result = DuumbiType::Result(
            Box::new(DuumbiType::DbConnection),
            Box::new(DuumbiType::String),
        );
        assert_eq!(
            Op::DbOpen.output_type(&Some(db_result.clone())),
            Some(db_result)
        );

        let rows_result =
            DuumbiType::Result(Box::new(DuumbiType::DbRows), Box::new(DuumbiType::String));
        assert_eq!(
            Op::DbQuery.output_type(&Some(rows_result.clone())),
            Some(rows_result)
        );
    }

    #[test]
    fn duumbi_type_result_option_equality() {
        assert_eq!(
            DuumbiType::Result(Box::new(DuumbiType::I64), Box::new(DuumbiType::I64)),
            DuumbiType::Result(Box::new(DuumbiType::I64), Box::new(DuumbiType::I64))
        );
        assert_ne!(
            DuumbiType::Result(Box::new(DuumbiType::I64), Box::new(DuumbiType::I64)),
            DuumbiType::Result(Box::new(DuumbiType::I64), Box::new(DuumbiType::F64))
        );
        assert_eq!(
            DuumbiType::Option(Box::new(DuumbiType::I64)),
            DuumbiType::Option(Box::new(DuumbiType::I64))
        );
        assert_ne!(
            DuumbiType::Option(Box::new(DuumbiType::I64)),
            DuumbiType::Option(Box::new(DuumbiType::String))
        );
    }

    #[test]
    fn duumbi_type_nested_result_option() {
        // option<result<i64,string>>
        let nested = DuumbiType::Option(Box::new(DuumbiType::Result(
            Box::new(DuumbiType::I64),
            Box::new(DuumbiType::String),
        )));
        assert_eq!(nested.to_string(), "option<result<i64,string>>");
        assert!(nested.is_option());
        assert!(!nested.is_result());
    }
}
