# Op Reference

All operations (`@type` values) in the DUUMBI JSON-LD schema.

## Integer & float constants

| Op | Fields | Cranelift | Phase |
|----|--------|-----------|-------|
| `duumbi:Const` | `duumbi:value` (i64), `duumbi:resultType` | `iconst` | 0 |
| `duumbi:ConstF64` | `duumbi:value` (f64), `duumbi:resultType` | `f64const` | 1 |
| `duumbi:ConstBool` | `duumbi:value` (bool), `duumbi:resultType` | `iconst` (i8) | 1 |

## Arithmetic

| Op | Fields | Cranelift | Phase |
|----|--------|-----------|-------|
| `duumbi:Add` | `duumbi:left`, `duumbi:right`, `duumbi:resultType` | `iadd` / `fadd` | 0 |
| `duumbi:Sub` | `duumbi:left`, `duumbi:right`, `duumbi:resultType` | `isub` / `fsub` | 0 |
| `duumbi:Mul` | `duumbi:left`, `duumbi:right`, `duumbi:resultType` | `imul` / `fmul` | 0 |
| `duumbi:Div` | `duumbi:left`, `duumbi:right`, `duumbi:resultType` | `sdiv` / `fdiv` | 0 |

## Control flow

| Op | Fields | Cranelift | Phase |
|----|--------|-----------|-------|
| `duumbi:Compare` | `duumbi:left`, `duumbi:right`, `duumbi:op` (eq/lt/le/gt/ge/ne), `duumbi:resultType` | `icmp` / `fcmp` | 1 |
| `duumbi:Branch` | `duumbi:condition`, `duumbi:trueBlock`, `duumbi:falseBlock` | `brif` | 1 |
| `duumbi:Return` | `duumbi:value` (optional) | `return` | 0 |

## Functions

| Op | Fields | Cranelift | Phase |
|----|--------|-----------|-------|
| `duumbi:Call` | `duumbi:function` (`@id` ref), `duumbi:args` (array of `@id` refs), `duumbi:resultType` | `call` | 1 |

## Variables

| Op | Fields | Cranelift | Phase |
|----|--------|-----------|-------|
| `duumbi:Load` | `duumbi:variable` (name), `duumbi:resultType` | `use_var` | 1 |
| `duumbi:Store` | `duumbi:variable` (name), `duumbi:value` | `def_var` | 1 |

## I/O

| Op | Fields | Cranelift | Phase |
|----|--------|-----------|-------|
| `duumbi:Print` | `duumbi:operand` (`@id` ref) | `call duumbi_print_*` | 0 |

## Types

| Type | Description |
|------|-------------|
| `i64` | 64-bit signed integer |
| `f64` | 64-bit IEEE 754 float |
| `bool` | Boolean (compiled as i8) |
| `void` | No value (function return only) |
