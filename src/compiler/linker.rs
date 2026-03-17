//! Linker invocation and C runtime compilation.
//!
//! Compiles `duumbi_runtime.c` to an object file and links it with
//! the Cranelift output to produce a native binary.

use std::path::Path;
use std::process::Command;

use crate::errors::codes;

use super::CompileError;

/// Finds the C compiler to use for linking.
///
/// Checks `$CC` environment variable first, falls back to `cc`.
#[must_use]
pub fn find_cc() -> String {
    std::env::var("CC").unwrap_or_else(|_| "cc".to_string())
}

/// Returns extra linker flags needed for the current platform.
///
/// On macOS, Cranelift object files lack the `LC_BUILD_VERSION` Mach-O load
/// command, causing `ld` to emit "no platform load command found" warnings.
/// This is a known Cranelift limitation — the generated binaries work correctly.
/// We suppress linker warnings with `-w` to avoid confusing users.
fn platform_link_args() -> Vec<&'static str> {
    if cfg!(target_os = "macos") {
        vec!["-Wl,-w", "-lm"]
    } else {
        vec!["-lm"]
    }
}

/// Compiles the C runtime shim to an object file.
///
/// Runs `cc -c runtime_c_path -o output_o_path`.
#[must_use = "compilation errors should be handled"]
pub fn compile_runtime(runtime_c: &Path, output_o: &Path) -> Result<(), CompileError> {
    let cc = find_cc();

    let status = Command::new(&cc)
        .args([
            "-c",
            &runtime_c.to_string_lossy(),
            "-o",
            &output_o.to_string_lossy(),
        ])
        .status()
        .map_err(|e| CompileError::CompilerNotFound {
            code: codes::E008_LINK_FAILED,
            message: format!("Failed to run C compiler '{cc}': {e}"),
        })?;

    if !status.success() {
        return Err(CompileError::LinkFailed {
            code: codes::E008_LINK_FAILED,
            message: format!(
                "C compiler failed to compile runtime (exit code: {})",
                status
                    .code()
                    .map_or("signal".to_string(), |c| c.to_string())
            ),
        });
    }

    Ok(())
}

/// Links multiple object files with the runtime object to produce a binary.
///
/// Runs `cc module1.o module2.o ... runtime_o -o binary_path`.
/// Used for multi-module programs where each module compiles to its own `.o`.
#[allow(dead_code)] // Called by CLI in upcoming phase (#61)
#[must_use = "link errors should be handled"]
pub fn link_multi(
    object_paths: &[&Path],
    runtime_o: &Path,
    binary_path: &Path,
) -> Result<(), CompileError> {
    let cc = find_cc();

    let mut args: Vec<String> = object_paths
        .iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect();
    args.push(runtime_o.to_string_lossy().into_owned());
    args.push("-o".to_string());
    args.push(binary_path.to_string_lossy().into_owned());
    args.extend(platform_link_args().iter().map(|s| (*s).to_string()));

    let status =
        Command::new(&cc)
            .args(&args)
            .status()
            .map_err(|e| CompileError::CompilerNotFound {
                code: codes::E008_LINK_FAILED,
                message: format!("Failed to run linker '{cc}': {e}"),
            })?;

    if !status.success() {
        return Err(CompileError::link_failed(format!(
            "Linker failed (exit code: {})",
            status
                .code()
                .map_or("signal".to_string(), |c| c.to_string())
        )));
    }

    Ok(())
}

/// Links the Cranelift object file with the runtime object to produce a binary.
///
/// Runs `cc output_o runtime_o -o binary_path`.
#[must_use = "link errors should be handled"]
pub fn link(output_o: &Path, runtime_o: &Path, binary_path: &Path) -> Result<(), CompileError> {
    let cc = find_cc();

    let output_o_str = output_o.to_string_lossy().into_owned();
    let runtime_o_str = runtime_o.to_string_lossy().into_owned();
    let binary_str = binary_path.to_string_lossy().into_owned();

    let mut args = vec![
        output_o_str.as_str(),
        runtime_o_str.as_str(),
        "-o",
        binary_str.as_str(),
    ];
    let platform_args = platform_link_args();
    args.extend(&platform_args);

    let status =
        Command::new(&cc)
            .args(&args)
            .status()
            .map_err(|e| CompileError::CompilerNotFound {
                code: codes::E008_LINK_FAILED,
                message: format!("Failed to run linker '{cc}': {e}"),
            })?;

    if !status.success() {
        return Err(CompileError::link_failed(format!(
            "Linker failed (exit code: {})",
            status
                .code()
                .map_or("signal".to_string(), |c| c.to_string())
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn find_cc_returns_something() {
        let cc = find_cc();
        assert!(!cc.is_empty());
    }

    #[test]
    fn compile_runtime_succeeds() {
        let tmp_dir = std::env::temp_dir().join("duumbi_test_runtime");
        fs::create_dir_all(&tmp_dir).expect("invariant: temp dir must be creatable");

        let runtime_c = Path::new("runtime/duumbi_runtime.c");
        let runtime_o = tmp_dir.join("duumbi_runtime.o");

        let result = compile_runtime(runtime_c, &runtime_o);
        assert!(result.is_ok(), "compile_runtime failed: {result:?}");
        assert!(runtime_o.exists(), "runtime .o file should exist");

        // Cleanup
        let _ = fs::remove_dir_all(&tmp_dir);
    }

    #[test]
    fn link_invalid_object_fails() {
        let tmp_dir = std::env::temp_dir().join("duumbi_test_link_fail");
        fs::create_dir_all(&tmp_dir).expect("invariant: temp dir must be creatable");

        // Write garbage as an "object" file
        let fake_o = tmp_dir.join("fake.o");
        fs::write(&fake_o, b"not a real object file")
            .expect("invariant: must be able to write temp file");

        let runtime_o = tmp_dir.join("runtime.o");
        fs::write(&runtime_o, b"also not real")
            .expect("invariant: must be able to write temp file");

        let output = tmp_dir.join("output_binary");
        let result = link(&fake_o, &runtime_o, &output);
        assert!(result.is_err(), "Linking garbage should fail");

        // Cleanup
        let _ = fs::remove_dir_all(&tmp_dir);
    }
}
