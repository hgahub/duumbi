//! Bounded local process verification for generated HTTP/SQLite/JSON services.
//!
//! The authoring contract supplies a port before mutation and SQL over stdin at
//! launch. No graph IDs, module layout, or generated constants are rewritten.

mod bindings;
#[cfg(windows)]
mod windows;
#[cfg(windows)]
use windows::WindowsJob;

use crate::intent::external_verifier::ExternalVerifier;
use std::future::Future;
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::process::Stdio;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt};
use tokio::process::{Child, Command};
use tokio::time::{sleep, timeout};

use crate::intent::spec::IntentSpec;
use crate::intent::verifier::{TestReport, TestResult};
use crate::knowledge::learning::redact_secret_text;

const LOG_LIMIT: usize = 8 * 1024;
const RESPONSE_LIMIT: usize = 16 * 1024;

/// Stage at which process verification failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProcessStage {
    /// Local verifier setup failed.
    Setup,
    /// Compilation or linking failed or exceeded its deadline.
    Build,
    /// The generated program could not start listening.
    Start,
    /// Sending the loopback HTTP request failed.
    Request,
    /// Reading or decoding the HTTP response failed.
    Response,
    /// Status or JSON semantics did not match the contract.
    Assertion,
    /// The program did not exit successfully after serving one request.
    Exit,
}

impl ProcessStage {
    /// Stable lowercase stage name used in signatures and failure codes.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Setup => "setup",
            Self::Build => "build",
            Self::Start => "start",
            Self::Request => "request",
            Self::Response => "response",
            Self::Assertion => "assertion",
            Self::Exit => "exit",
        }
    }
}

/// Separates generated-program defects from verifier infrastructure failures.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProcessFailureKind {
    /// The generated program failed an applicable check.
    Program,
    /// Local setup, process spawning, or cleanup failed.
    Infrastructure,
}

/// Structured failure, safe to include in retained evidence and repair prompts.
#[derive(Debug, Clone, Serialize, Deserialize, thiserror::Error)]
#[error("{stage:?}: {detail}")]
pub struct ProcessFailure {
    /// Failing lifecycle stage.
    pub stage: ProcessStage,
    /// Ownership of the failure.
    pub kind: ProcessFailureKind,
    /// Bounded, redacted diagnostic.
    pub detail: String,
}

fn failure(
    stage: ProcessStage,
    kind: ProcessFailureKind,
    detail: impl AsRef<str>,
) -> ProcessFailure {
    ProcessFailure {
        stage,
        kind,
        detail: sanitize(detail.as_ref().as_bytes(), LOG_LIMIT),
    }
}

fn build_failure_kind(stderr: &str) -> ProcessFailureKind {
    // These messages describe the fixed native toolchain/runtime, which graph
    // repair cannot fix. Generated graph/Cranelift/link failures remain program
    // failures unless the compiler or linker could not be started at all.
    if [
        "Failed to run C compiler",
        "Failed to run linker",
        "C compiler failed to compile runtime",
    ]
    .iter()
    .any(|message| stderr.contains(message))
    {
        ProcessFailureKind::Infrastructure
    } else {
        ProcessFailureKind::Program
    }
}

fn program_failure(stage: ProcessStage, detail: impl AsRef<str>) -> ProcessFailure {
    failure(stage, ProcessFailureKind::Program, detail)
}

fn infrastructure(stage: ProcessStage, error: impl std::fmt::Display) -> ProcessFailure {
    failure(stage, ProcessFailureKind::Infrastructure, error.to_string())
}

/// Bounded output and termination facts for a child process.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProcessOutput {
    /// Exit code; absent on a signal or when no child was launched.
    pub exit_code: Option<i32>,
    /// Whether a deadline expired.
    pub timed_out: bool,
    /// Whether the harness killed a still-running child.
    pub killed: bool,
    /// Whether the direct child was reaped.
    pub reaped: bool,
    /// Sanitized stdout, limited to 8 KiB.
    pub stdout: String,
    /// Sanitized stderr, limited to 8 KiB.
    pub stderr: String,
}

/// One fresh service launch and its observable SQL-backed HTTP result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessScenarioEvidence {
    /// Stable dataset identifier.
    pub scenario: String,
    /// Ephemeral loopback port supplied during authoring.
    pub port: u16,
    /// Exact SQL input used for this fresh in-memory database.
    pub input_sql: String,
    /// Requested loopback route.
    pub route: String,
    /// HTTP response status, when received.
    pub status: Option<u16>,
    /// Bounded, sanitized response body; never stores headers or credentials.
    pub response: Option<String>,
    /// Exact expected public JSON values.
    pub expected: Value,
    /// Whether all response assertions passed.
    pub assertions_passed: bool,
    /// Child termination and output facts.
    pub process: ProcessOutput,
    /// First failing stage, if any.
    pub failure: Option<ProcessFailure>,
}

/// One build and verification pass, before or after the normal intent repair.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessEvidence {
    /// Stable version of the stdin/HTTP verification contract.
    pub contract: String,
    /// Build command with workspace-independent paths.
    pub build_command: String,
    /// Build subprocess output and termination.
    pub build: ProcessOutput,
    /// Fresh database scenarios actually attempted.
    pub scenarios: Vec<ProcessScenarioEvidence>,
    /// First failing stage, if any.
    pub failure: Option<ProcessFailure>,
}

/// Deadlines for each process-verification phase.
#[derive(Debug, Clone, Copy)]
pub struct ProcessLimits {
    /// Maximum compiler subprocess lifetime.
    pub build: Duration,
    /// Maximum wait for the service's first connection.
    pub startup: Duration,
    /// Total deadline for request and response I/O.
    pub request: Duration,
    /// Maximum wait for a one-request service to exit.
    pub exit: Duration,
}

impl Default for ProcessLimits {
    fn default() -> Self {
        Self {
            build: Duration::from_secs(30),
            startup: Duration::from_secs(5),
            request: Duration::from_secs(2),
            exit: Duration::from_secs(2),
        }
    }
}

/// Stateful process verifier reused by the initial verification and repair pass.
pub struct ProcessVerifier {
    executable: PathBuf,
    reservation: Option<TcpListener>,
    port: u16,
    limits: ProcessLimits,
    /// Ordered evidence, including the initial failure when repair succeeds.
    pub evidence: Vec<ProcessEvidence>,
}

impl ProcessVerifier {
    /// Reserves an ephemeral loopback port until the first generated launch.
    #[must_use = "port reservation failures must be handled"]
    pub fn new(executable: PathBuf, limits: ProcessLimits) -> Result<Self, ProcessFailure> {
        let reservation = TcpListener::bind(("127.0.0.1", 0))
            .map_err(|e| infrastructure(ProcessStage::Setup, e))?;
        let port = reservation
            .local_addr()
            .map_err(|e| infrastructure(ProcessStage::Setup, e))?
            .port();
        Ok(Self {
            executable,
            reservation: Some(reservation),
            port,
            limits,
            evidence: Vec::new(),
        })
    }

    /// Uses the running CLI, or its adjacent CLI binary in a Cargo test run.
    #[must_use = "verifier setup failures must be handled"]
    pub fn for_current_executable() -> Result<Self, ProcessFailure> {
        let mut path =
            std::env::current_exe().map_err(|e| infrastructure(ProcessStage::Setup, e))?;
        if path
            .parent()
            .and_then(Path::file_name)
            .is_some_and(|name| name == "deps")
        {
            path.pop();
            path.pop();
            path.push(format!("duumbi{}", std::env::consts::EXE_SUFFIX));
        }
        Self::new(path, ProcessLimits::default())
    }

    /// Adds the selected port to the intent before any provider mutation.
    pub fn prepare_spec(&self, spec: &mut IntentSpec) {
        spec.acceptance_criteria.push(format!("Bind only to 127.0.0.1 on TCP port {}. This port is fixed for this run; use it as the server port.", self.port));
    }

    /// Whether repairing the generated graph could address the latest failure.
    #[must_use]
    pub fn repairable(&self) -> bool {
        self.evidence
            .last()
            .and_then(|e| e.failure.as_ref())
            .is_none_or(|e| e.kind == ProcessFailureKind::Program)
    }

    /// Builds the generated workspace and verifies fresh SQL-driven service runs.
    #[must_use]
    pub async fn verify(&mut self, workspace: &Path) -> TestReport {
        let evidence = self.check(workspace).await;
        let passed = evidence.failure.is_none();
        let error = evidence.failure.as_ref().map(ToString::to_string);
        self.evidence.push(evidence);
        TestReport {
            passed: usize::from(passed),
            failed: usize::from(!passed),
            results: vec![TestResult {
                name: "http_sqlite_json_process".into(),
                function: "GET /facts (two fresh SQLite datasets)".into(),
                args: Vec::new(),
                expected: 1,
                actual: passed.then_some(1),
                passed,
                error,
            }],
        }
    }

    async fn check(&mut self, workspace: &Path) -> ProcessEvidence {
        let output = crate::workspace::workspace_output_path(workspace);
        let mut evidence = ProcessEvidence {
            contract: "duumbi.http-sqlite-json.v1".into(),
            build_command: "duumbi build --output .duumbi/build/output".into(),
            build: ProcessOutput::default(),
            scenarios: Vec::new(),
            failure: None,
        };
        // `build` resolves only local workspace/vendor/cache sources; it never
        // installs dependencies or calls a registry. `init` provides stdlib.
        let mut command = Command::new(&self.executable);
        command.env_clear();
        for name in [
            "PATH",
            "CC",
            "CFLAGS",
            "LDFLAGS",
            "DUUMBI_CC",
            "DUUMBI_CFLAGS",
            "DUUMBI_LDFLAGS",
            "SDKROOT",
            "MACOSX_DEPLOYMENT_TARGET",
            "SystemRoot",
        ] {
            if let Some(value) = std::env::var_os(name) {
                command.env(name, value);
            }
        }
        let build_tmp = workspace.join(".duumbi/process-tmp");
        if let Err(error) = tokio::fs::create_dir_all(&build_tmp).await {
            evidence.failure = Some(infrastructure(ProcessStage::Setup, error));
            return evidence;
        }
        for name in ["TMPDIR", "TMP", "TEMP"] {
            command.env(name, &build_tmp);
        }
        command
            .args(["build", "--output"])
            .arg(&output)
            .current_dir(workspace);
        let mut child = match ManagedChild::spawn(command) {
            Ok(child) => child,
            Err(e) => {
                evidence.failure = Some(infrastructure(ProcessStage::Build, e));
                return evidence;
            }
        };
        let result = child.finish(self.limits.build).await;
        match result {
            Ok(output) => {
                if output.timed_out || output.exit_code != Some(0) {
                    evidence.failure = Some(failure(
                        ProcessStage::Build,
                        build_failure_kind(&output.stderr),
                        format!(
                            "build failed (exit={:?}, deadline_exceeded={}); {}",
                            output.exit_code, output.timed_out, output.stderr
                        ),
                    ));
                }
                evidence.build = output;
            }
            Err(e) => evidence.failure = Some(infrastructure(ProcessStage::Build, e)),
        }
        if evidence.failure.is_some() {
            return evidence;
        }
        if let Err(error) = bindings::validate(workspace, self.port).await {
            evidence.failure = Some(error);
            return evidence;
        }
        // The runtime cannot inherit a listening socket. Release immediately
        // before launch; a conflicting binder is a visible startup failure.
        self.reservation.take();
        for (name, sql, count, first) in [
            (
                "one_row",
                "INSERT INTO facts(name) VALUES ('Ada Lovelace')",
                1,
                "Ada Lovelace",
            ),
            (
                "two_rows",
                "INSERT INTO facts(name) VALUES ('Grace Hopper'), ('Katherine Johnson')",
                2,
                "Grace Hopper",
            ),
        ] {
            let expected = json!({"service":"scaled-http-sqlite-json", "route":"/facts", "count":count, "first_fact":first, "storage":"sqlite-memory"});
            let scenario = self.scenario(workspace, &output, name, sql, expected).await;
            if evidence.failure.is_none() {
                evidence.failure = scenario.failure.clone();
            }
            evidence.scenarios.push(scenario);
            if evidence.failure.is_some() {
                break;
            }
        }
        evidence
    }

    async fn scenario(
        &self,
        workspace: &Path,
        executable: &Path,
        name: &str,
        sql: &str,
        expected: Value,
    ) -> ProcessScenarioEvidence {
        let mut evidence = ProcessScenarioEvidence {
            scenario: name.into(),
            port: self.port,
            input_sql: sql.into(),
            route: "/facts".into(),
            status: None,
            response: None,
            expected,
            assertions_passed: false,
            process: ProcessOutput::default(),
            failure: None,
        };
        // Bind and release before every launch, including repaired passes and
        // subsequent attempts. A port taken by another process is infrastructure,
        // and must never be mistaken for a generated-program assertion failure.
        if let Err(error) = wait_for_port_handoff(self.port).await {
            evidence.failure = Some(infrastructure(ProcessStage::Start, error));
            return evidence;
        }
        let mut command = Command::new(executable);
        // Generated children receive no provider credentials or proxy settings.
        command.current_dir(workspace).env_clear();
        // Windows runtime DLLs (including libcurl) are discovered via PATH.
        for name in ["PATH", "SystemRoot"] {
            if let Some(value) = std::env::var_os(name) {
                command.env(name, value);
            }
        }
        let mut child = match ManagedChild::spawn(command) {
            Ok(child) => child,
            Err(e) => {
                evidence.failure = Some(infrastructure(ProcessStage::Start, e));
                return evidence;
            }
        };
        let result = async {
            if let Some(mut stdin) = child.child.stdin.take() {
                deadline(self.limits.request, ProcessStage::Start, async {
                    stdin
                        .write_all(format!("{sql}\n").as_bytes())
                        .await
                        .map_err(|e| program_failure(ProcessStage::Start, e.to_string()))
                })
                .await?;
            }
            // A successful readiness connection is the actual request socket:
            // probing and closing would consume a one-request server's budget.
            let mut stream = connect(&mut child.child, self.port, self.limits.startup).await?;
            deadline(self.limits.request, ProcessStage::Request, async {
                stream
                    .write_all(
                        b"GET /facts HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n",
                    )
                    .await
                    .map_err(|e| program_failure(ProcessStage::Request, e.to_string()))
            })
            .await?;
            let response = deadline(
                self.limits.request,
                ProcessStage::Response,
                read_response(&mut stream),
            )
            .await?;
            let (status, body) = parse_response(&response)?;
            evidence.status = Some(status);
            evidence.response = Some(sanitize(body, RESPONSE_LIMIT));
            assert_response(status, body, &evidence.expected)?;
            evidence.assertions_passed = true;
            Ok::<(), ProcessFailure>(())
        }
        .await;
        evidence.failure = result.err();
        let wait = if evidence.failure.is_some() {
            Duration::ZERO
        } else {
            self.limits.exit
        };
        match child.finish(wait).await {
            Ok(output) => {
                if evidence.failure.is_none() && (output.timed_out || output.exit_code != Some(0)) {
                    evidence.failure = Some(program_failure(
                        ProcessStage::Exit,
                        "service must exit successfully after one request",
                    ));
                }
                evidence.process = output;
            }
            Err(e) => evidence.failure = Some(infrastructure(ProcessStage::Exit, e)),
        }
        evidence
    }
}

impl ExternalVerifier for ProcessVerifier {
    fn prepare_spec(&self, spec: &mut IntentSpec) {
        ProcessVerifier::prepare_spec(self, spec);
    }

    fn verify<'a>(
        &'a mut self,
        workspace: &'a Path,
    ) -> Pin<Box<dyn Future<Output = TestReport> + Send + 'a>> {
        Box::pin(ProcessVerifier::verify(self, workspace))
    }

    fn repairable(&self) -> bool {
        ProcessVerifier::repairable(self)
    }

    fn kind(&self) -> &str {
        "bounded HTTP/SQLite/JSON process"
    }
}

/// Give a just-released reservation a short, bounded handoff window. Concurrent
/// native launches can briefly delay socket release; never connect to the port
/// as a probe or launch while another listener still owns it.
async fn wait_for_port_handoff(port: u16) -> std::io::Result<()> {
    let end = tokio::time::Instant::now() + Duration::from_millis(250);
    loop {
        match check_port_available(port) {
            Err(error)
                if error.kind() == std::io::ErrorKind::AddrInUse
                    && tokio::time::Instant::now() < end =>
            {
                sleep(Duration::from_millis(10)).await;
            }
            result => return result,
        }
    }
}

/// Check the runtime bind without confusing a preceding connection's TIME_WAIT
/// with a live listener. No connection is opened to another application's port.
fn check_port_available(port: u16) -> std::io::Result<()> {
    #[cfg(windows)]
    {
        windows::check_port_available(port)
    }
    #[cfg(not(windows))]
    {
        let socket = tokio::net::TcpSocket::new_v4()?;
        socket.set_reuseaddr(true)?;
        socket.bind(std::net::SocketAddr::from(([127, 0, 0, 1], port)))?;
        Ok(())
    }
}

async fn deadline<T>(
    duration: Duration,
    stage: ProcessStage,
    work: impl Future<Output = Result<T, ProcessFailure>>,
) -> Result<T, ProcessFailure> {
    timeout(duration, work)
        .await
        .map_err(|_| program_failure(stage, "deadline exceeded"))?
}

async fn connect(
    child: &mut Child,
    port: u16,
    duration: Duration,
) -> Result<tokio::net::TcpStream, ProcessFailure> {
    deadline(duration, ProcessStage::Start, async {
        loop {
            if let Ok(stream) = tokio::net::TcpStream::connect(("127.0.0.1", port)).await {
                return Ok(stream);
            }
            if child
                .try_wait()
                .map_err(|e| infrastructure(ProcessStage::Start, e))?
                .is_some()
            {
                return Err(program_failure(
                    ProcessStage::Start,
                    "service exited before listening",
                ));
            }
            sleep(Duration::from_millis(20)).await;
        }
    })
    .await
}

async fn read_response(stream: &mut (impl AsyncRead + Unpin)) -> Result<Vec<u8>, ProcessFailure> {
    let mut bytes = Vec::new();
    let mut chunk = [0; 1024];
    loop {
        match stream.read(&mut chunk).await {
            Ok(0) => break,
            Ok(n) => {
                if bytes.len() + n > RESPONSE_LIMIT {
                    return Err(program_failure(
                        ProcessStage::Response,
                        "HTTP response exceeds 16 KiB",
                    ));
                }
                bytes.extend_from_slice(&chunk[..n]);
            }
            Err(e) if e.kind() == std::io::ErrorKind::ConnectionReset && !bytes.is_empty() => break,
            Err(e) => return Err(program_failure(ProcessStage::Response, e.to_string())),
        }
    }
    Ok(bytes)
}

fn parse_response(response: &[u8]) -> Result<(u16, &[u8]), ProcessFailure> {
    let split = response
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .ok_or_else(|| program_failure(ProcessStage::Response, "missing HTTP headers"))?;
    let headers = std::str::from_utf8(&response[..split])
        .map_err(|_| program_failure(ProcessStage::Response, "invalid HTTP headers"))?;
    let mut status = headers
        .lines()
        .next()
        .unwrap_or_default()
        .split_whitespace();
    if !matches!(status.next(), Some("HTTP/1.1" | "HTTP/1.0")) {
        return Err(program_failure(
            ProcessStage::Response,
            "invalid HTTP version",
        ));
    }
    let status = status
        .next()
        .and_then(|v| v.parse::<u16>().ok())
        .filter(|s| (100..=599).contains(s))
        .ok_or_else(|| program_failure(ProcessStage::Response, "invalid HTTP status"))?;
    let body = &response[split + 4..];
    for line in headers.lines().skip(1) {
        if let Some((name, value)) = line.split_once(':') {
            if name.eq_ignore_ascii_case("content-length")
                && value.trim().parse::<usize>().ok() != Some(body.len())
            {
                return Err(program_failure(
                    ProcessStage::Response,
                    "Content-Length mismatch",
                ));
            }
            if name.eq_ignore_ascii_case("transfer-encoding") {
                return Err(program_failure(
                    ProcessStage::Response,
                    "expected bounded unencoded response",
                ));
            }
        }
    }
    Ok((status, body))
}

fn assert_response(status: u16, body: &[u8], expected: &Value) -> Result<(), ProcessFailure> {
    if status != 200 {
        return Err(program_failure(
            ProcessStage::Assertion,
            format!("expected HTTP 200, got {status}"),
        ));
    }
    let actual: Value = serde_json::from_slice(body)
        .map_err(|_| program_failure(ProcessStage::Assertion, "response is not valid JSON"))?;
    for field in ["service", "route", "count", "first_fact", "storage"] {
        if actual.get(field) != expected.get(field) {
            return Err(program_failure(
                ProcessStage::Assertion,
                format!(
                    "JSON field {field} did not match the SQL-backed expected value {}",
                    expected[field]
                ),
            ));
        }
    }
    Ok(())
}

fn sanitize(bytes: &[u8], limit: usize) -> String {
    let redacted = redact_secret_text(&String::from_utf8_lossy(bytes));
    let mut result: String = redacted
        .chars()
        .filter(|c| !c.is_control() || matches!(c, '\n' | '\t'))
        .collect();
    let mut end = result.len().min(limit);
    while !result.is_char_boundary(end) {
        end -= 1;
    }
    result.truncate(end);
    result
}

// Drain continuously even after the retained cap, so a noisy child cannot fill
// its pipes and deadlock. Only redacted, bounded data crosses the evidence seam.
async fn drain(mut pipe: impl AsyncRead + Unpin) -> Vec<u8> {
    let mut output = Vec::new();
    let mut truncated = false;
    let mut chunk = [0; 4096];
    while let Ok(n) = pipe.read(&mut chunk).await {
        if n == 0 {
            break;
        }
        truncated |= n > LOG_LIMIT.saturating_sub(output.len());
        output.extend_from_slice(&chunk[..n.min(LOG_LIMIT.saturating_sub(output.len()))]);
    }
    if truncated {
        let complete = output
            .iter()
            .rposition(|byte| *byte == b'\n')
            .map_or(0, |index| index + 1);
        output.truncate(complete);
    }
    output
}

struct ManagedChild {
    child: Child,
    #[cfg(unix)]
    pid: Option<u32>,
    #[cfg(windows)]
    job: WindowsJob,
    stdout: tokio::task::JoinHandle<Vec<u8>>,
    stderr: tokio::task::JoinHandle<Vec<u8>>,
}

impl ManagedChild {
    fn spawn(mut command: Command) -> std::io::Result<Self> {
        command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        #[cfg(unix)]
        command.process_group(0);
        #[cfg(not(windows))]
        let mut child = command.spawn()?;
        #[cfg(windows)]
        let mut child = WindowsJob::spawn_suspended(command)?;
        #[cfg(unix)]
        let pid = Some(
            child
                .id()
                .expect("invariant: freshly spawned child has a pid"),
        );
        #[cfg(windows)]
        let job = WindowsJob::attach(&child)?;
        #[cfg(windows)]
        job.resume(&child)?;
        let stdout = tokio::spawn(drain(
            child.stdout.take().expect("invariant: stdout is piped"),
        ));
        let stderr = tokio::spawn(drain(
            child.stderr.take().expect("invariant: stderr is piped"),
        ));
        Ok(Self {
            child,
            #[cfg(unix)]
            pid,
            #[cfg(windows)]
            job,
            stdout,
            stderr,
        })
    }

    async fn finish(&mut self, duration: Duration) -> std::io::Result<ProcessOutput> {
        let result = timeout(duration, self.child.wait()).await;
        let timed_out = result.is_err();
        let status = match result {
            Ok(status) => status?,
            Err(_) => {
                self.kill_tree();
                let _ = self.child.start_kill();
                timeout(Duration::from_secs(2), self.child.wait())
                    .await
                    .map_err(|_| std::io::Error::other("child cleanup deadline exceeded"))??
            }
        };
        // Also remove descendants after the direct child exits (e.g. a linker
        // wrapper), otherwise they can retain pipes or outlive the workspace.
        self.kill_tree();
        let stdout = timeout(Duration::from_secs(1), &mut self.stdout)
            .await
            .ok()
            .and_then(Result::ok)
            .unwrap_or_default();
        let stderr = timeout(Duration::from_secs(1), &mut self.stderr)
            .await
            .ok()
            .and_then(Result::ok)
            .unwrap_or_default();
        Ok(ProcessOutput {
            exit_code: status.code(),
            timed_out,
            killed: timed_out,
            reaped: true,
            stdout: sanitize(&stdout, LOG_LIMIT),
            stderr: sanitize(&stderr, LOG_LIMIT),
        })
    }

    fn kill_tree(&mut self) {
        #[cfg(unix)]
        if let Some(pid) = self.pid.take() {
            // SAFETY: each child owns a new process group whose id is its
            // positive pid. Consume the id once to avoid signaling it again
            // after the OS might have recycled it.
            unsafe {
                libc::kill(-(pid as i32), libc::SIGKILL);
            }
        }
        #[cfg(windows)]
        self.job.terminate();
    }
}

impl Drop for ManagedChild {
    fn drop(&mut self) {
        self.kill_tree();
        let _ = self.child.start_kill();
        self.stdout.abort();
        self.stderr.abort();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn expected() -> Value {
        json!({"service":"scaled-http-sqlite-json", "route":"/facts", "count":1, "first_fact":"Ada Lovelace", "storage":"sqlite-memory"})
    }

    #[test]
    fn semantic_checks_reject_wrong_values_types_status_and_json() {
        let expected = expected();
        assert!(assert_response(200, expected.to_string().as_bytes(), &expected).is_ok());
        for (key, value) in [
            ("count", json!("1")),
            ("count", json!(2)),
            ("first_fact", json!("wrong")),
            ("service", json!("wrong")),
            ("storage", json!("fake")),
            ("route", json!("/wrong")),
        ] {
            let mut actual = expected.clone();
            actual[key] = value;
            assert_eq!(
                assert_response(200, actual.to_string().as_bytes(), &expected)
                    .expect_err("mismatch")
                    .stage,
                ProcessStage::Assertion
            );
        }
        assert!(assert_response(500, expected.to_string().as_bytes(), &expected).is_err());
        for body in [b"not JSON".as_slice(), b"{}", b"[]", b"null"] {
            assert!(assert_response(200, body, &expected).is_err());
        }
    }

    #[test]
    fn native_toolchain_failures_are_not_graph_repair_candidates() {
        for error in [
            "Failed to run C compiler 'missing': not found",
            "Failed to run linker 'missing'",
            "C compiler failed to compile runtime: missing curl/curl.h",
        ] {
            assert_eq!(
                build_failure_kind(error),
                ProcessFailureKind::Infrastructure
            );
        }
        assert_eq!(
            build_failure_kind("Cranelift compilation failed: invalid SSA"),
            ProcessFailureKind::Program
        );
    }

    #[test]
    fn framing_rejects_malformed_and_truncated_responses() {
        for response in [
            b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\n\r\n{}".as_slice(),
            b"hello\r\n\r\n{}",
            b"HTTP/1.1 nope\r\n\r\n{}",
            b"HTTP/1.1 200 OK",
        ] {
            assert_eq!(
                parse_response(response).expect_err("invalid HTTP").stage,
                ProcessStage::Response
            );
        }
        assert_eq!(
            parse_response(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\n{}")
                .expect("HTTP")
                .0,
            200
        );
    }

    #[tokio::test]
    async fn body_limit_and_deadline_bound_even_a_trickling_peer() {
        assert!(
            read_response(&mut &vec![b'x'; RESPONSE_LIMIT + 1][..])
                .await
                .is_err()
        );
        let failure = deadline(
            Duration::from_millis(10),
            ProcessStage::Response,
            std::future::pending::<Result<(), ProcessFailure>>(),
        )
        .await
        .expect_err("deadline");
        assert_eq!(failure.stage, ProcessStage::Response);
        assert_eq!(failure.kind, ProcessFailureKind::Program);
    }

    #[tokio::test]
    async fn port_handoff_waits_for_delayed_release() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).expect("port");
        let port = listener.local_addr().expect("address").port();
        let release = tokio::spawn(async move {
            sleep(Duration::from_millis(40)).await;
            drop(listener);
        });
        wait_for_port_handoff(port)
            .await
            .expect("released within handoff bound");
        release.await.expect("release task");
    }

    #[tokio::test]
    async fn occupied_run_port_is_start_infrastructure_without_launching() {
        let mut verifier = ProcessVerifier::new(PathBuf::from("missing"), ProcessLimits::default())
            .expect("verifier");
        let _conflict = verifier.reservation.take().expect("occupied port");
        let workspace = tempfile::tempdir().expect("workspace");
        let scenario = verifier
            .scenario(
                workspace.path(),
                Path::new("must-not-launch"),
                "one_row",
                "INSERT",
                expected(),
            )
            .await;
        let failure = scenario.failure.as_ref().expect("port conflict");
        assert_eq!(failure.stage, ProcessStage::Start);
        assert_eq!(failure.kind, ProcessFailureKind::Infrastructure);
        assert!(!scenario.process.reaped);
        verifier.evidence.push(ProcessEvidence {
            contract: "test".into(),
            build_command: String::new(),
            build: ProcessOutput::default(),
            failure: scenario.failure.clone(),
            scenarios: vec![scenario],
        });
        assert!(!verifier.repairable());
    }

    #[test]
    fn retained_logs_are_redacted_bounded_and_valid_utf8() {
        let logs = format!(
            "Authorization: Bearer example-secret\nAPI_KEY=private\n{}",
            "á".repeat(LOG_LIMIT)
        );
        let clean = sanitize(logs.as_bytes(), LOG_LIMIT);
        assert!(!clean.contains("example-secret"));
        assert!(!clean.contains("private"));
        assert!(clean.len() <= LOG_LIMIT);
        assert!(!sanitize(b"\x1b[31mtest\0", LOG_LIMIT).contains('\x1b'));
    }

    // Portable fake child: the Rust test executable itself, so these lifecycle
    // tests require neither a shell nor Python nor live provider access.
    #[test]
    fn process_child_fixture() {
        let Ok(mode) = std::env::var("DUUMBI_780_CHILD_MODE") else {
            return;
        };
        match mode.as_str() {
            "flood" => {
                let mut out = std::io::stdout().lock();
                for _ in 0..128 {
                    out.write_all(&[b'x'; 4096]).expect("stdout");
                }
                std::io::stderr()
                    .write_all(b"Authorization: Bearer child-secret\n")
                    .expect("stderr");
            }
            "tree" => {
                let descendant =
                    std::process::Command::new(std::env::current_exe().expect("fixture binary"))
                        .args([
                            "--exact",
                            "bench::process::tests::process_child_fixture",
                            "--nocapture",
                        ])
                        .env("DUUMBI_780_CHILD_MODE", "hang")
                        .spawn()
                        .expect("descendant");
                std::fs::write(
                    std::env::var_os("DUUMBI_780_PID_FILE").expect("pid path"),
                    descendant.id().to_string(),
                )
                .expect("pid file");
                std::thread::sleep(Duration::from_secs(60));
                // Reached only if the parent was not terminated by the test.
                let mut descendant = descendant;
                let _ = descendant.kill();
                let _ = descendant.wait();
            }
            "hang" => std::thread::sleep(Duration::from_secs(60)),
            "fail" => std::process::exit(17),
            _ => panic!("unknown child mode"),
        }
    }

    fn child(mode: &str) -> ManagedChild {
        let mut command = Command::new(std::env::current_exe().expect("test binary"));
        command
            .args([
                "--exact",
                "bench::process::tests::process_child_fixture",
                "--nocapture",
            ])
            .env("DUUMBI_780_CHILD_MODE", mode);
        ManagedChild::spawn(command).expect("fake process")
    }

    #[tokio::test]
    async fn noisy_child_cannot_deadlock_and_logs_stay_bounded() {
        let output = child("flood")
            .finish(Duration::from_secs(5))
            .await
            .expect("finish");
        assert_eq!(output.exit_code, Some(0));
        assert!(output.reaped && !output.timed_out);
        assert!(output.stdout.len() <= LOG_LIMIT);
        assert!(!output.stderr.contains("child-secret"));
    }

    #[tokio::test]
    async fn deadline_kills_and_reaps_hanging_child() {
        let output = child("hang")
            .finish(Duration::from_millis(40))
            .await
            .expect("finish");
        assert!(output.timed_out && output.killed && output.reaped);
        assert_ne!(output.exit_code, Some(0));
    }

    #[tokio::test]
    async fn startup_detects_early_exit_without_consuming_a_request() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).expect("port");
        let port = listener.local_addr().expect("address").port();
        drop(listener);
        let mut child = child("fail");
        let failure = connect(&mut child.child, port, Duration::from_secs(2))
            .await
            .expect_err("early exit");
        assert_eq!(failure.stage, ProcessStage::Start);
        assert_eq!(
            child
                .finish(Duration::from_secs(1))
                .await
                .expect("cleanup")
                .exit_code,
            Some(17)
        );
    }

    #[tokio::test]
    async fn missing_compiler_is_infrastructure_and_does_not_request_repair() {
        let tmp = tempfile::tempdir().expect("workspace");
        let mut verifier =
            ProcessVerifier::new(tmp.path().join("missing-duumbi"), ProcessLimits::default())
                .expect("verifier");
        assert!(!verifier.verify(tmp.path()).await.all_passed());
        assert!(!verifier.repairable());
        assert_eq!(
            verifier.evidence[0].failure.as_ref().expect("failure").kind,
            ProcessFailureKind::Infrastructure
        );
    }

    #[cfg(unix)]
    async fn unix_process_alive(pid: u32) -> bool {
        // SAFETY: signal 0 only probes existence; it does not signal.
        if unsafe { libc::kill(pid as i32, 0) } != 0 {
            return std::io::Error::last_os_error().raw_os_error() != Some(libc::ESRCH);
        }
        #[cfg(target_os = "linux")]
        {
            // Container PID 1 may leave dead, orphaned descendants as zombies.
            // A zombie still has a PID but cannot execute or retain open pipes.
            // Field 3 follows the parenthesized comm, which can contain spaces
            // and parentheses: https://man7.org/linux/man-pages/man5/proc_pid_stat.5.html
            match tokio::fs::read_to_string(format!("/proc/{pid}/stat")).await {
                Ok(stat) => !matches!(
                    stat.rsplit_once(')')
                        .and_then(|(_, fields)| fields.split_whitespace().next()),
                    Some("Z" | "X")
                ),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
                Err(error) => panic!("cannot inspect child process state: {error}"),
            }
        }
        #[cfg(not(target_os = "linux"))]
        true
    }

    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn liveness_probe_excludes_an_unreaped_zombie() {
        // std::process::Child does not reap on drop or through Tokio's reaper,
        // so this exercises a real zombie independently of PID 1's behavior.
        let mut child =
            std::process::Command::new(std::env::current_exe().expect("fixture binary"))
                .args([
                    "--exact",
                    "bench::process::tests::process_child_fixture",
                    "--nocapture",
                ])
                .env("DUUMBI_780_CHILD_MODE", "hang")
                .spawn()
                .expect("child");
        let pid = child.id();
        let was_alive = unix_process_alive(pid).await;
        child.kill().expect("terminate fixture");
        let terminated = timeout(Duration::from_secs(3), async {
            while unix_process_alive(pid).await {
                sleep(Duration::from_millis(10)).await;
            }
        })
        .await;
        // Reap even if the probe was wrong, before asserting its result.
        child.wait().expect("reap fixture");
        assert!(was_alive, "running child must be detected");
        terminated.expect("unreaped zombie must count as terminated");
        assert!(!unix_process_alive(pid).await, "reaped child is absent");
    }

    #[tokio::test]
    async fn cancellation_kills_the_child_and_its_descendants() {
        let tmp = tempfile::tempdir().expect("pid directory");
        let pid_file = tmp.path().join("descendant.pid");
        let mut command = Command::new(std::env::current_exe().expect("fixture binary"));
        command
            .args([
                "--exact",
                "bench::process::tests::process_child_fixture",
                "--nocapture",
            ])
            .env("DUUMBI_780_CHILD_MODE", "tree")
            .env("DUUMBI_780_PID_FILE", &pid_file);
        let managed = ManagedChild::spawn(command).expect("tree");
        let parent_pid = managed.child.id().expect("parent pid");
        let wait = tokio::spawn(async move {
            let mut managed = managed;
            managed.finish(Duration::from_secs(60)).await
        });
        let descendant_pid: u32 = timeout(Duration::from_secs(3), async {
            loop {
                if let Ok(text) = tokio::fs::read_to_string(&pid_file).await
                    && let Ok(pid) = text.parse()
                {
                    break pid;
                }
                sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("descendant started");
        wait.abort();
        assert!(wait.await.expect_err("cancelled").is_cancelled());
        timeout(Duration::from_secs(3), async {
            loop {
                #[cfg(unix)]
                let alive = unix_process_alive(parent_pid).await
                    || unix_process_alive(descendant_pid).await;
                #[cfg(windows)]
                let alive =
                    windows::process_alive(parent_pid) || windows::process_alive(descendant_pid);
                if !alive {
                    break;
                }
                sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("process tree terminated after cancellation");
    }
}
