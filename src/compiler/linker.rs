//! Linker invocation and C runtime compilation.
//!
//! Compiles `duumbi_runtime.c` to an object file and links it with
//! the Cranelift output to produce a native binary.

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use crate::errors::codes;

use super::CompileError;

#[cfg(test)]
static RUNTIME_COMPILE_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Finds the C compiler to use for linking.
///
/// Checks `$DUUMBI_CC`, then `$CC`, and falls back to `cc`.
#[must_use]
pub fn find_cc() -> String {
    std::env::var("DUUMBI_CC")
        .or_else(|_| std::env::var("CC"))
        .unwrap_or_else(|_| "cc".to_string())
}

fn split_env_flags(value: &str) -> Vec<String> {
    value
        .split_whitespace()
        .filter(|flag| !flag.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn runtime_cflags() -> Vec<String> {
    std::env::var("DUUMBI_CFLAGS")
        .or_else(|_| std::env::var("CFLAGS"))
        .map(|flags| split_env_flags(&flags))
        .unwrap_or_default()
}

fn runtime_ldflags() -> Vec<String> {
    std::env::var("DUUMBI_LDFLAGS")
        .or_else(|_| std::env::var("LDFLAGS"))
        .map(|flags| split_env_flags(&flags))
        .unwrap_or_default()
}

fn command_failure_details(output: &Output) -> String {
    let mut details = String::new();
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !stderr.trim().is_empty() {
        details.push_str("; stderr: ");
        details.push_str(stderr.trim());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    if !stdout.trim().is_empty() {
        details.push_str("; stdout: ");
        details.push_str(stdout.trim());
    }

    const MAX_DETAILS_LEN: usize = 4000;
    if details.len() > MAX_DETAILS_LEN {
        details.truncate(MAX_DETAILS_LEN);
        details.push_str("...");
    }

    details
}

/// Returns extra linker flags needed for the current platform.
///
/// On macOS, Cranelift object files lack the `LC_BUILD_VERSION` Mach-O load
/// command, causing `ld` to emit "no platform load command found" warnings.
/// This is a known Cranelift limitation — the generated binaries work correctly.
/// On macOS we suppress linker warnings with `-Wl,-w` to avoid confusing users.
/// HTTP/HTTPS runtime support requires libcurl linkage. On Windows, TCP
/// runtime support requires Winsock linkage.
fn platform_link_args() -> Vec<&'static str> {
    if cfg!(target_os = "windows") {
        vec!["-lm", "-lws2_32", "-lcurl"]
    } else if cfg!(target_os = "macos") {
        vec!["-Wl,-w", "-lm", "-lcurl"]
    } else {
        vec!["-lm", "-lcurl", "-ldl", "-lpthread"]
    }
}

fn ensure_runtime_deps_present(runtime_c: &Path) -> Result<(), CompileError> {
    let runtime_dir = runtime_c.parent().unwrap_or_else(|| Path::new("."));
    let sqlite_dir = runtime_dir.join("third_party").join("sqlite");
    let sqlite_c = sqlite_dir.join("sqlite3.c");
    let sqlite_h = sqlite_dir.join("sqlite3.h");

    if sqlite_c.exists() && sqlite_h.exists() {
        return Ok(());
    }

    let source_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("runtime")
        .join("third_party")
        .join("sqlite");
    let source_c = source_dir.join("sqlite3.c");
    let source_h = source_dir.join("sqlite3.h");
    if source_dir != sqlite_dir && source_c.exists() && source_h.exists() {
        fs::create_dir_all(&sqlite_dir).map_err(|e| CompileError::LinkFailed {
            code: codes::E008_LINK_FAILED,
            message: format!("Failed to create temp SQLite runtime directory: {e}"),
        })?;
        fs::copy(&source_c, &sqlite_c).map_err(|e| CompileError::LinkFailed {
            code: codes::E008_LINK_FAILED,
            message: format!("Failed to copy vendored SQLite source: {e}"),
        })?;
        fs::copy(&source_h, &sqlite_h).map_err(|e| CompileError::LinkFailed {
            code: codes::E008_LINK_FAILED,
            message: format!("Failed to copy vendored SQLite header: {e}"),
        })?;
        return Ok(());
    }

    Err(CompileError::LinkFailed {
        code: codes::E008_LINK_FAILED,
        message: format!(
            "Vendored SQLite runtime sources are missing beside '{}'; expected '{}' and '{}'",
            runtime_c.display(),
            sqlite_c.display(),
            sqlite_h.display()
        ),
    })
}

/// Compiles the C runtime shim to an object file.
///
/// Runs `cc -c runtime_c_path -o output_o_path`.
#[must_use = "compilation errors should be handled"]
pub fn compile_runtime(runtime_c: &Path, output_o: &Path) -> Result<(), CompileError> {
    #[cfg(test)]
    let _compile_guard = RUNTIME_COMPILE_LOCK
        .lock()
        .expect("invariant: runtime compile lock must not be poisoned");

    let cc = find_cc();
    ensure_runtime_deps_present(runtime_c)?;

    let mut args = runtime_cflags();
    args.extend([
        "-c".to_string(),
        runtime_c.to_string_lossy().into_owned(),
        "-o".to_string(),
        output_o.to_string_lossy().into_owned(),
    ]);

    let output =
        Command::new(&cc)
            .args(&args)
            .output()
            .map_err(|e| CompileError::CompilerNotFound {
                code: codes::E008_LINK_FAILED,
                message: format!("Failed to run C compiler '{cc}': {e}"),
            })?;

    if !output.status.success() {
        return Err(CompileError::LinkFailed {
            code: codes::E008_LINK_FAILED,
            message: format!(
                "C compiler failed to compile runtime (exit code: {}){}",
                output
                    .status
                    .code()
                    .map_or("signal".to_string(), |c| c.to_string()),
                command_failure_details(&output)
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
    args.extend(runtime_ldflags());
    args.extend(platform_link_args().iter().map(|s| (*s).to_string()));

    let output =
        Command::new(&cc)
            .args(&args)
            .output()
            .map_err(|e| CompileError::CompilerNotFound {
                code: codes::E008_LINK_FAILED,
                message: format!("Failed to run linker '{cc}': {e}"),
            })?;

    if !output.status.success() {
        return Err(CompileError::link_failed(format!(
            "Linker failed (exit code: {}){}",
            output
                .status
                .code()
                .map_or("signal".to_string(), |c| c.to_string()),
            command_failure_details(&output)
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

    let mut args = vec![
        output_o.to_string_lossy().into_owned(),
        runtime_o.to_string_lossy().into_owned(),
        "-o".to_string(),
        binary_path.to_string_lossy().into_owned(),
    ];
    args.extend(runtime_ldflags());
    args.extend(platform_link_args().iter().map(|s| (*s).to_string()));

    let output =
        Command::new(&cc)
            .args(&args)
            .output()
            .map_err(|e| CompileError::CompilerNotFound {
                code: codes::E008_LINK_FAILED,
                message: format!("Failed to run linker '{cc}': {e}"),
            })?;

    if !output.status.success() {
        return Err(CompileError::link_failed(format!(
            "Linker failed (exit code: {}){}",
            output
                .status
                .code()
                .map_or("signal".to_string(), |c| c.to_string()),
            command_failure_details(&output)
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn find_cc_returns_something() {
        let cc = find_cc();
        assert!(!cc.is_empty());
    }

    #[test]
    fn compile_runtime_succeeds() {
        let tmp_dir = TempDir::new().expect("invariant: temp dir must be creatable");

        let runtime_c = Path::new("runtime/duumbi_runtime.c");
        let runtime_o = tmp_dir.path().join("duumbi_runtime.o");

        let result = compile_runtime(runtime_c, &runtime_o);
        assert!(result.is_ok(), "compile_runtime failed: {result:?}");
        assert!(runtime_o.exists(), "runtime .o file should exist");
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

    #[test]
    fn platform_link_args_match_current_target() {
        let args = platform_link_args();
        assert!(args.contains(&"-lm"));
        assert!(args.contains(&"-lcurl"));

        #[cfg(target_os = "macos")]
        assert!(args.contains(&"-Wl,-w"));

        #[cfg(not(target_os = "macos"))]
        assert!(!args.contains(&"-Wl,-w"));

        #[cfg(target_os = "windows")]
        assert!(args.contains(&"-lws2_32"));
    }

    #[test]
    fn runtime_dependency_link_probe_succeeds() {
        let tmp_dir = TempDir::new().expect("invariant: temp dir must be creatable");
        let cc = find_cc();

        let main_c = tmp_dir.path().join("main.c");
        fs::write(&main_c, "int main(void) { return 0; }\n")
            .expect("invariant: must be able to write temp source");

        let main_o = tmp_dir.path().join("main.o");
        let compile_main = Command::new(&cc)
            .args([
                "-c",
                &main_c.to_string_lossy(),
                "-o",
                &main_o.to_string_lossy(),
            ])
            .status()
            .expect("invariant: C compiler must run");
        assert!(compile_main.success(), "test main object must compile");

        let runtime_c = Path::new("runtime/duumbi_runtime.c");
        let runtime_o = tmp_dir.path().join("duumbi_runtime.o");
        compile_runtime(runtime_c, &runtime_o).expect("runtime must compile");

        let binary = tmp_dir.path().join("duumbi_dependency_link_probe");
        link(&main_o, &runtime_o, &binary).expect("runtime dependency link must succeed");
        #[cfg(target_os = "windows")]
        let binary_exists = binary.exists() || binary.with_extension("exe").exists();
        #[cfg(not(target_os = "windows"))]
        let binary_exists = binary.exists();

        assert!(binary_exists, "linked probe binary should exist");
    }

    #[test]
    fn compile_runtime_copies_vendored_sqlite_to_temp_runtime_tree() {
        let tmp_dir = TempDir::new().expect("invariant: temp dir must be creatable");
        let runtime_c = tmp_dir.path().join("duumbi_runtime.c");
        fs::write(
            &runtime_c,
            "int duumbi_dependency_probe(void) { return 0; }\n",
        )
        .expect("invariant: must be able to write temp runtime source");

        let runtime_o = tmp_dir.path().join("duumbi_runtime.o");
        let result = compile_runtime(&runtime_c, &runtime_o);
        assert!(
            result.is_ok(),
            "temp runtime should compile after copying vendored SQLite deps, got: {result:?}"
        );
        assert!(
            tmp_dir.path().join("third_party/sqlite/sqlite3.c").exists(),
            "compile_runtime should materialize SQLite source into temp runtime tree"
        );
        assert!(
            tmp_dir.path().join("third_party/sqlite/sqlite3.h").exists(),
            "compile_runtime should materialize SQLite header into temp runtime tree"
        );
    }

    #[test]
    fn link_multi_invalid_objects_fails() {
        let tmp_dir = TempDir::new().expect("invariant: temp dir must be creatable");

        let fake_module_a = tmp_dir.path().join("module_a.o");
        fs::write(&fake_module_a, b"not a real object file")
            .expect("invariant: must be able to write temp file");

        let fake_module_b = tmp_dir.path().join("module_b.o");
        fs::write(&fake_module_b, b"still not a real object file")
            .expect("invariant: must be able to write temp file");

        let runtime_o = tmp_dir.path().join("runtime.o");
        fs::write(&runtime_o, b"also not real")
            .expect("invariant: must be able to write temp file");

        let object_paths = [fake_module_a.as_path(), fake_module_b.as_path()];
        let output = tmp_dir.path().join("output_binary");
        let result = link_multi(&object_paths, &runtime_o, &output);
        assert!(
            result.is_err(),
            "Linking multiple garbage objects should fail"
        );
    }
}
