//! Opt-in **external-runner** backend (IRON first).
//!
//! Wraps a stack that already runs on this machine (mlir-aie / IRON `python` or
//! `make` under `programming_examples/`) without claiming product Pass.
//!
//! # Honesty (hard)
//!
//! - [`EXECUTION_EXTERNAL_RUNNER`] (`execution=external_runner`) is the only
//!   execution label this backend emits.
//! - [`RAN_ON_NPU`] is always `false`. Child stdout containing `PASS!` or `PASS`
//!   is **not** product `ran_on_npu`.
//! - [`INVENT_PASS`] is always `0` (hard-coded; never taken from mutable fields).
//! - Never sets or forwards `XDNA2_ALLOW_EXEC_CMD`.
//! - Never ingests IRON peels as product known-good.
//! - Missing runner binary → [`status`] available=false and [`Backend::run`]
//!   returns an explicit error. **Not** a silent simulation Pass.
//! - **Not** selected by [`crate::DragonNpu::new`]. Construct via
//!   [`ExternalRunnerBackend::new`] / [`crate::DragonNpu::with_backend`].
//!
//! # Intended IRON argv (sibling tree; do not edit mlir-aie from this crate)
//!
//! Typical host recipe (operator or CLI `--external-runner` / `--external-arg` /
//! `--external-cwd`):
//!
//! ```text
//! /usr/bin/python3 vector_scalar_add.py --dev npu2
//! ```
//!
//! Prefer `just npu-iron` so cwd is resolved even when `$MLIR_AIE` is unset.
//! Manual `--external-cwd` is
//! `$MLIR_AIE/programming_examples/basic/vector_scalar_add`
//! only after `MLIR_AIE` is a real directory
//! (see `.agents/reports/plan-46-explore-working-stacks.md`).
//!
//! Larger examples use the same shape under `programming_examples/ml/mobilenet`
//! (`make` / `test_mobilenet.py`). eepy is a second runner after IRON.
//!
//! Spawn is always [`std::process::Command::new`] plus an argv array. Never
//! `sh -c` of untrusted strings.
//!
//! See `docs/SECONDARY_BACKENDS.md`.

use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;

use crate::ModelCompiler;
use crate::backends::Backend;
use crate::{CompiledModel, DragonNpuError, NpuStatus};

/// Backend id in errors and status.
pub const BACKEND_ID: &str = "external-runner";

/// Product execution label. Hard-coded. Never derived from child stdout.
pub const EXECUTION_EXTERNAL_RUNNER: &str = "external_runner";

/// Product `ran_on_npu`. Always false. Hard-coded.
pub const RAN_ON_NPU: bool = false;

/// Soft invent token. Always 0. Hard-coded.
pub const INVENT_PASS: u8 = 0;

/// Product known-good claim. Always false. Hard-coded.
pub const PRODUCT_KNOWN_GOOD: bool = false;

/// Env: absolute path to the runner binary (opt-in; default off).
pub const ENV_EXTERNAL_RUNNER: &str = "DRAGON_NPU_EXTERNAL_RUNNER";

/// Env: optional absolute working directory for the child.
pub const ENV_EXTERNAL_CWD: &str = "DRAGON_NPU_EXTERNAL_CWD";

/// Never forwarded into the child.
pub const ALLOW_EXEC_CMD_ENV: &str = "XDNA2_ALLOW_EXEC_CMD";

/// Status / device label (not product silicon).
pub const DEVICE_NAME: &str = "external-runner (opt-in IRON/eepy wrap; not product ran_on_npu)";

/// Honesty note printed with reports (tokens the code actually emits).
pub const HONESTY_NOTE: &str = "execution=external_runner. ran_on_npu=false always. \
     invent_pass=0. Child PASS!/PASS is the external stack result, not product Pass. \
     IRON peels are not product known-good. This backend never sets XDNA2_ALLOW_EXEC_CMD.";

/// Child stdout / stderr after a spawn (or a test double).
#[derive(Debug, Clone)]
pub struct ChildOutput {
    /// Process exit code when known.
    pub status_code: Option<i32>,
    /// Lossy UTF-8 stdout.
    pub stdout: String,
    /// Lossy UTF-8 stderr.
    pub stderr: String,
}

/// What to spawn. No shell. Argv array only.
#[derive(Debug, Clone)]
pub struct SpawnRequest {
    /// Absolute runner program.
    pub program: PathBuf,
    /// Extra argv (not passed through `sh -c`).
    pub args: Vec<String>,
    /// Optional absolute working directory.
    pub cwd: Option<PathBuf>,
    /// When true, the real runner must strip [`ALLOW_EXEC_CMD_ENV`].
    pub strip_allow_exec_cmd: bool,
}

/// Injected child (tests) or [`ProcessCommandRunner`] (product).
pub trait CommandRunner: Send + Sync + std::fmt::Debug {
    /// Run `request.program` with `request.args`. Never `sh -c`.
    ///
    /// # Errors
    ///
    /// I/O or spawn failure from the real runner.
    fn run(&self, request: &SpawnRequest) -> Result<ChildOutput, DragonNpuError>;
}

/// Product spawn: `Command::new(program).args(args)`.
#[derive(Debug, Clone, Copy, Default)]
pub struct ProcessCommandRunner;

impl CommandRunner for ProcessCommandRunner {
    fn run(&self, request: &SpawnRequest) -> Result<ChildOutput, DragonNpuError> {
        let mut cmd = Command::new(&request.program);
        cmd.args(&request.args);
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
        if request.strip_allow_exec_cmd {
            cmd.env_remove(ALLOW_EXEC_CMD_ENV);
        }
        if let Some(cwd) = &request.cwd {
            cmd.current_dir(cwd);
        }
        let output = cmd.output().map_err(|e| DragonNpuError::BackendError {
            backend: BACKEND_ID.to_string(),
            message: format!(
                "failed to spawn external runner '{}': {e}",
                request.program.display()
            ),
        })?;
        Ok(ChildOutput {
            status_code: output.status.code(),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        })
    }
}

/// Validated program + argv + optional cwd.
#[derive(Debug, Clone)]
pub struct ExternalRunnerConfig {
    program: PathBuf,
    extra_args: Vec<String>,
    cwd: Option<PathBuf>,
}

impl ExternalRunnerConfig {
    /// Validate `program` (and optional `cwd`). `program` must be an absolute
    /// path with no `..` components. Missing file is allowed (status unavailable).
    ///
    /// # Errors
    ///
    /// Relative path, `..`, empty, or invalid cwd.
    pub fn new(
        program: impl Into<String>,
        extra_args: Vec<String>,
        cwd: Option<String>,
    ) -> Result<Self, DragonNpuError> {
        let program = validate_runner_program(&program.into())?;
        let cwd = match cwd {
            Some(c) => Some(validate_runner_cwd(&c)?),
            None => None,
        };
        Ok(Self {
            program,
            extra_args,
            cwd,
        })
    }

    /// Read [`ENV_EXTERNAL_RUNNER`] / [`ENV_EXTERNAL_CWD`]. `None` if unset.
    ///
    /// Extra argv is not taken from the environment (use CLI `--external-arg`).
    pub fn from_env() -> Option<Result<Self, DragonNpuError>> {
        match std::env::var(ENV_EXTERNAL_RUNNER) {
            Ok(p) if !p.trim().is_empty() => {
                let cwd = std::env::var(ENV_EXTERNAL_CWD)
                    .ok()
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty());
                Some(Self::new(p.trim().to_string(), Vec::new(), cwd))
            }
            _ => None,
        }
    }

    /// Absolute program path (as configured; not necessarily present).
    #[must_use]
    pub fn program(&self) -> &Path {
        &self.program
    }

    /// Extra argv.
    #[must_use]
    pub fn extra_args(&self) -> &[String] {
        &self.extra_args
    }

    /// Optional child working directory.
    #[must_use]
    pub fn cwd(&self) -> Option<&Path> {
        self.cwd.as_deref()
    }
}

/// Product view of one child run. Honesty fields are constants.
#[derive(Debug, Clone)]
pub struct ExternalRunnerReport {
    /// Always [`EXECUTION_EXTERNAL_RUNNER`].
    pub execution: &'static str,
    /// Always [`RAN_ON_NPU`] (`false`).
    pub ran_on_npu: bool,
    /// Always [`INVENT_PASS`] (`0`).
    pub invent_pass: u8,
    /// Always [`PRODUCT_KNOWN_GOOD`] (`false`).
    pub product_known_good: bool,
    /// Child exit code when known.
    pub child_exit_code: Option<i32>,
    /// Informational: stdout mentions `PASS!` or a `PASS` token.
    pub child_said_pass: bool,
    /// Child stdout (lossy).
    pub stdout: String,
    /// Child stderr (lossy).
    pub stderr: String,
    /// Configured program path.
    pub runner_path: String,
    /// Fixed honesty note.
    pub note: &'static str,
}

impl ExternalRunnerReport {
    /// Build a report from child bytes. Honesty fields are **not** taken from
    /// stdout or from mutable backend state.
    #[must_use]
    pub fn from_child(runner_path: &Path, output: &ChildOutput) -> Self {
        let said = child_stdout_mentions_pass(&output.stdout);
        Self {
            execution: EXECUTION_EXTERNAL_RUNNER,
            // Hard-coded. Never derived from stdout, PASS!, or backend fields.
            ran_on_npu: RAN_ON_NPU,
            invent_pass: INVENT_PASS,
            product_known_good: PRODUCT_KNOWN_GOOD,
            child_exit_code: output.status_code,
            child_said_pass: said,
            stdout: output.stdout.clone(),
            stderr: output.stderr.clone(),
            runner_path: runner_path.display().to_string(),
            note: HONESTY_NOTE,
        }
    }

    /// True when the child exit code is `Some(0)`.
    #[must_use]
    pub fn child_exit_ok(&self) -> bool {
        self.child_exit_code == Some(0)
    }

    /// JSON object the CLI prints (tokens match this type, not docs-only).
    #[must_use]
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "execution": EXECUTION_EXTERNAL_RUNNER,
            "ran_on_npu": RAN_ON_NPU,
            "invent_pass": INVENT_PASS,
            "product_known_good": PRODUCT_KNOWN_GOOD,
            "child_exit_code": self.child_exit_code,
            "child_exit_ok": self.child_exit_ok(),
            "child_said_pass": self.child_said_pass,
            "runner_path": self.runner_path,
            "stdout": self.stdout,
            "stderr": self.stderr,
            "note": self.note,
        })
    }
}

/// True when stdout contains `PASS!` or a whitespace-delimited `PASS` token.
#[must_use]
pub fn child_stdout_mentions_pass(stdout: &str) -> bool {
    if stdout.contains("PASS!") {
        return true;
    }
    stdout.split_whitespace().any(|w| w == "PASS")
}

/// Opt-in external-runner backend.
#[derive(Debug, Clone)]
pub struct ExternalRunnerBackend {
    config: ExternalRunnerConfig,
    runner: Arc<dyn CommandRunner>,
}

impl ExternalRunnerBackend {
    /// Construct with the real process runner.
    ///
    /// # Errors
    ///
    /// Invalid program / cwd policy (not "file missing").
    pub fn new(config: ExternalRunnerConfig) -> Self {
        Self {
            config,
            runner: Arc::new(ProcessCommandRunner),
        }
    }

    /// Inject a [`CommandRunner`] (unit tests; no mlir-aie on PATH required).
    #[must_use]
    pub fn with_runner(config: ExternalRunnerConfig, runner: Arc<dyn CommandRunner>) -> Self {
        Self { config, runner }
    }

    /// Whether the configured program exists as a regular file (symlink-to-file OK).
    #[must_use]
    pub fn runner_present(&self) -> bool {
        runner_file_present(self.config.program())
    }

    /// Parseable child vector from this report (stdout and/or this-run npy).
    /// Empty when the child emitted no numbers. Never invents OFM. Not Pass.
    #[must_use]
    pub fn parsed_output(&self, report: &ExternalRunnerReport) -> Vec<f32> {
        let npy = self.config.cwd().and_then(|cwd| {
            crate::backends::iron_kernel::load_this_run_npy_bytes(
                cwd,
                &report.stdout,
                &report.stderr,
            )
        });
        crate::backends::iron_kernel::parse_child_f32_vector(&report.stdout, npy.as_deref())
    }

    /// Spawn (or use the injected runner) and build a product report.
    ///
    /// # Errors
    ///
    /// Missing binary, or the process runner failed to spawn.
    pub fn run_external(&self) -> Result<ExternalRunnerReport, DragonNpuError> {
        if !self.runner_present() {
            return Err(Self::unavailable_error());
        }
        let request = SpawnRequest {
            program: self.config.program().to_path_buf(),
            args: self.config.extra_args().to_vec(),
            cwd: self.config.cwd().map(Path::to_path_buf),
            strip_allow_exec_cmd: true,
        };
        let output = self.runner.run(&request)?;
        Ok(ExternalRunnerReport::from_child(
            self.config.program(),
            &output,
        ))
    }

    fn unavailable_error() -> DragonNpuError {
        DragonNpuError::BackendError {
            backend: BACKEND_ID.to_string(),
            message: format!(
                "external runner binary is unavailable (not a silent host Pass); \
                 execution={EXECUTION_EXTERNAL_RUNNER} ran_on_npu=false invent_pass=0"
            ),
        }
    }
}

impl Backend for ExternalRunnerBackend {
    fn status(&self) -> Result<NpuStatus, DragonNpuError> {
        let present = self.runner_present();
        Ok(NpuStatus {
            vendor: BACKEND_ID.to_string(),
            available: present,
            compute_units: 0,
            memory_mb: 0,
            peak_tops: 0.0,
            device_name: DEVICE_NAME.to_string(),
            driver_version: format!("program={}", self.config.program().display()),
            supported_dtypes: vec![],
            simulated: true,
        })
    }

    fn compile_model(
        &self,
        model_path: &str,
        optimization_level: u8,
        quantization: &str,
    ) -> Result<CompiledModel, DragonNpuError> {
        let compiler = ModelCompiler::new();
        compiler.compile(
            model_path,
            optimization_level,
            quantization,
            Arc::new(self.clone()),
        )
    }

    fn run(&self, _input: &[f32]) -> Result<Vec<f32>, DragonNpuError> {
        if !self.runner_present() {
            return Err(Self::unavailable_error());
        }
        let report = self.run_external()?;
        if !report.child_exit_ok() {
            return Err(DragonNpuError::BackendError {
                backend: BACKEND_ID.to_string(),
                message: format!(
                    "external runner exited {:?}; execution={} ran_on_npu=false invent_pass=0",
                    report.child_exit_code, EXECUTION_EXTERNAL_RUNNER
                ),
            });
        }
        // Return parseable child numbers when present. Never invent OFM.
        // Leftover npy without a this-run mention stays empty. Not Pass.
        Ok(self.parsed_output(&report))
    }

    fn submit_and_run(
        &self,
        _model: &CompiledModel,
        input: &[f32],
    ) -> Result<Vec<f32>, DragonNpuError> {
        self.run(input)
    }
}

/// Absolute path, no `..`, non-empty. File may be missing.
///
/// # Errors
///
/// Empty, relative, or `..` components.
pub fn validate_runner_program(path: &str) -> Result<PathBuf, DragonNpuError> {
    validate_absolute_no_dotdot(path, "runner program")
}

/// Absolute existing directory, no `..`.
///
/// # Errors
///
/// Empty, relative, `..`, or not a directory.
pub fn validate_runner_cwd(path: &str) -> Result<PathBuf, DragonNpuError> {
    let p = validate_absolute_no_dotdot(path, "runner cwd")?;
    if !p.is_dir() {
        return Err(DragonNpuError::BackendError {
            backend: BACKEND_ID.to_string(),
            message: format!("runner cwd is not a directory: {}", p.display()),
        });
    }
    Ok(p)
}

fn validate_absolute_no_dotdot(path: &str, what: &str) -> Result<PathBuf, DragonNpuError> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err(DragonNpuError::BackendError {
            backend: BACKEND_ID.to_string(),
            message: format!("{what} path is empty"),
        });
    }
    let p = PathBuf::from(trimmed);
    if !p.is_absolute() {
        return Err(DragonNpuError::BackendError {
            backend: BACKEND_ID.to_string(),
            message: format!(
                "{what} must be an absolute path (no PATH search, no sh -c): {trimmed}"
            ),
        });
    }
    if p.components().any(|c| matches!(c, Component::ParentDir)) {
        return Err(DragonNpuError::BackendError {
            backend: BACKEND_ID.to_string(),
            message: format!("{what} must not contain '..': {trimmed}"),
        });
    }
    Ok(p)
}

fn runner_file_present(path: &Path) -> bool {
    path.is_file()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    #[derive(Debug)]
    struct ScriptedRunner {
        stdout: String,
        stderr: String,
        code: i32,
        last: Mutex<Option<SpawnRequest>>,
    }

    impl CommandRunner for ScriptedRunner {
        fn run(&self, request: &SpawnRequest) -> Result<ChildOutput, DragonNpuError> {
            *self.last.lock().expect("scripted lock") = Some(request.clone());
            Ok(ChildOutput {
                status_code: Some(self.code),
                stdout: self.stdout.clone(),
                stderr: self.stderr.clone(),
            })
        }
    }

    fn present_config() -> ExternalRunnerConfig {
        // /bin/true or /usr/bin/true exists on this Linux host; used only as a
        // present-file token for injected-runner tests (child is not that binary).
        let program = if Path::new("/usr/bin/true").is_file() {
            "/usr/bin/true"
        } else {
            "/bin/true"
        };
        ExternalRunnerConfig::new(program, vec!["--dev".into(), "npu2".into()], None)
            .expect("absolute true")
    }

    #[test]
    fn external_runner_child_pass_does_not_set_ran_on_npu() {
        let scripted = Arc::new(ScriptedRunner {
            stdout: "verify: PASS!\nGFLOPS 3.2\n".into(),
            stderr: String::new(),
            code: 0,
            last: Mutex::new(None),
        });
        let backend = ExternalRunnerBackend::with_runner(present_config(), scripted);
        let report = backend.run_external().expect("scripted child");
        assert_eq!(report.execution, EXECUTION_EXTERNAL_RUNNER);
        assert_eq!(report.ran_on_npu, RAN_ON_NPU);
        assert!(
            !report.ran_on_npu,
            "child PASS! must not set product ran_on_npu, got ran_on_npu={}",
            report.ran_on_npu
        );
        assert_eq!(report.invent_pass, INVENT_PASS);
        assert!(!report.product_known_good);
        assert!(report.child_said_pass);
        assert!(report.child_exit_ok());
        let j = report.to_json();
        assert_eq!(j["execution"], EXECUTION_EXTERNAL_RUNNER);
        assert_eq!(j["ran_on_npu"], false);
        assert_eq!(j["invent_pass"], 0);
        assert_eq!(j["product_known_good"], false);
        let out = backend.run(&[1.0, 2.0]).expect("child exit 0");
        assert!(
            out.is_empty(),
            "PASS! / GFLOPS must not invent a tensor: {out:?}"
        );
    }

    #[test]
    fn external_runner_run_returns_stdout_json_array_not_pass() {
        let scripted = Arc::new(ScriptedRunner {
            stdout: "verify: PASS!\nresult: [1.0, 2.5, -3]\n".into(),
            stderr: String::new(),
            code: 0,
            last: Mutex::new(None),
        });
        let backend = ExternalRunnerBackend::with_runner(present_config(), scripted);
        let report = backend.run_external().expect("scripted child");
        assert_eq!(report.execution, EXECUTION_EXTERNAL_RUNNER);
        assert!(!report.ran_on_npu);
        assert_eq!(report.invent_pass, INVENT_PASS);
        assert!(report.child_said_pass);
        let out = backend.run(&[0.0]).expect("child exit 0");
        assert_eq!(out, vec![1.0, 2.5, -3.0]);
        let j = report.to_json();
        assert_eq!(j["ran_on_npu"], false);
        assert_eq!(j["invent_pass"], 0);
        assert_eq!(j["execution"], EXECUTION_EXTERNAL_RUNNER);
    }

    #[test]
    fn external_runner_json_honesty_uses_constants_not_mutated_fields() {
        let mut report = ExternalRunnerReport::from_child(
            Path::new("/usr/bin/true"),
            &ChildOutput {
                status_code: Some(0),
                stdout: "PASS!".into(),
                stderr: String::new(),
            },
        );
        report.ran_on_npu = true;
        report.invent_pass = 1;
        report.execution = "host_or_simulation_run";
        let j = report.to_json();
        assert_eq!(
            j["ran_on_npu"], false,
            "to_json must emit RAN_ON_NPU, not mutated ran_on_npu"
        );
        assert_eq!(
            j["invent_pass"], 0,
            "to_json must emit INVENT_PASS, not mutated invent_pass"
        );
        assert_eq!(j["execution"], EXECUTION_EXTERNAL_RUNNER);
    }

    #[test]
    fn external_runner_missing_binary_is_unavailable() {
        let cfg =
            ExternalRunnerConfig::new("/no/such/dragon_npu_external_runner_xyz", Vec::new(), None)
                .expect("absolute missing path is valid config");
        let backend = ExternalRunnerBackend::new(cfg);
        let st = backend.status().expect("status");
        assert!(
            !st.available,
            "missing binary must not report available: {st:?}"
        );
        assert!(st.device_name.contains("external-runner"));
        let err = backend
            .run(&[0.5_f32; 4])
            .expect_err("missing binary must be an explicit error, not silent host Pass");
        let msg = err.to_string();
        assert!(
            msg.contains("unavailable"),
            "expected unavailable error, got: {msg}"
        );
        assert!(
            msg.contains("ran_on_npu=false"),
            "missing-binary error must emit ran_on_npu=false, got: {msg}"
        );
        assert!(
            msg.contains("invent_pass=0"),
            "missing-binary error must emit invent_pass=0, got: {msg}"
        );
    }

    #[test]
    fn external_runner_refuses_shell_injection() {
        let err = ExternalRunnerConfig::new("python3 -c 'import os'", Vec::new(), None)
            .expect_err("relative / shell string must refuse");
        assert!(err.to_string().contains("absolute"), "got: {err}");

        let err = ExternalRunnerConfig::new("/usr/bin/python3/../sh", Vec::new(), None)
            .expect_err("dotdot must refuse");
        assert!(err.to_string().contains(".."), "got: {err}");

        let scripted = Arc::new(ScriptedRunner {
            stdout: String::new(),
            stderr: String::new(),
            code: 0,
            last: Mutex::new(None),
        });
        let last = Arc::clone(&scripted);
        let backend = ExternalRunnerBackend::with_runner(present_config(), scripted);
        let _ = backend.run_external().expect("scripted");
        let req = last.last.lock().expect("lock").clone().expect("spawned");
        assert_ne!(req.program, PathBuf::from("sh"));
        assert_ne!(req.program, PathBuf::from("/bin/sh"));
        assert!(req.strip_allow_exec_cmd);
        assert_eq!(req.args, vec!["--dev".to_string(), "npu2".to_string()]);
        assert!(!req.args.iter().any(|a| a == "-c"));
    }

    #[test]
    fn external_runner_empty_or_malformed_stdout() {
        let scripted = Arc::new(ScriptedRunner {
            stdout: String::new(),
            stderr: "not a tensor".into(),
            code: 0,
            last: Mutex::new(None),
        });
        let backend = ExternalRunnerBackend::with_runner(present_config(), scripted);
        let report = backend
            .run_external()
            .expect("empty stdout is still a wrap");
        assert_eq!(report.execution, EXECUTION_EXTERNAL_RUNNER);
        assert!(!report.ran_on_npu);
        assert_eq!(report.invent_pass, 0);
        assert!(!report.child_said_pass);
    }

    #[test]
    fn external_runner_real_echo_argv_is_not_shell() {
        let echo = if Path::new("/usr/bin/echo").is_file() {
            "/usr/bin/echo"
        } else if Path::new("/bin/echo").is_file() {
            "/bin/echo"
        } else {
            panic!(
                "external_runner_real_echo_argv_is_not_shell requires /usr/bin/echo or /bin/echo; \
                 do not silent-skip this argv lock"
            );
        };
        let cfg =
            ExternalRunnerConfig::new(echo, vec!["hello; rm -rf /".into(), "PASS!".into()], None)
                .expect("absolute echo");
        let backend = ExternalRunnerBackend::new(cfg);
        let report = backend
            .run_external()
            .expect("echo must spawn via Command::new argv");
        assert!(
            report.stdout.contains("hello; rm -rf /"),
            "argv must not go through sh -c; stdout={:?}",
            report.stdout
        );
        assert!(report.child_said_pass);
        assert!(!report.ran_on_npu);
        assert_eq!(report.execution, EXECUTION_EXTERNAL_RUNNER);
        assert_eq!(report.invent_pass, 0);
    }

    #[test]
    fn external_runner_missing_or_unusable_cwd_and_empty_program() {
        let err = ExternalRunnerConfig::new(
            "/usr/bin/true",
            Vec::new(),
            Some("/no/such/dragon_npu_cwd_xyz".into()),
        )
        .expect_err("missing cwd must refuse");
        let msg = err.to_string();
        assert!(
            msg.contains("directory") || msg.contains("cwd"),
            "expected cwd/directory error, got: {msg}"
        );

        let file_cwd = if Path::new("/usr/bin/true").is_file() {
            "/usr/bin/true"
        } else {
            "/bin/true"
        };
        let err = ExternalRunnerConfig::new("/usr/bin/true", Vec::new(), Some(file_cwd.into()))
            .expect_err("file-as-cwd must refuse");
        assert!(
            err.to_string().contains("directory"),
            "expected not-a-directory, got: {err}"
        );

        let err = ExternalRunnerConfig::new("   ", Vec::new(), None)
            .expect_err("whitespace-only program must refuse");
        assert!(err.to_string().contains("empty"), "got: {err}");

        let err =
            ExternalRunnerConfig::new("", Vec::new(), None).expect_err("empty program must refuse");
        assert!(err.to_string().contains("empty"), "got: {err}");

        let err = ExternalRunnerConfig::new("./runner", Vec::new(), None)
            .expect_err("relative ./runner must refuse");
        assert!(err.to_string().contains("absolute"), "got: {err}");

        let err = ExternalRunnerConfig::new("python3", Vec::new(), None)
            .expect_err("PATH-relative python3 must refuse");
        assert!(err.to_string().contains("absolute"), "got: {err}");
    }

    fn tiny_npy_f32(vals: &[f32]) -> Vec<u8> {
        let header = format!(
            "{{'descr': '<f4', 'fortran_order': False, 'shape': ({},), }}",
            vals.len()
        );
        let mut header_bytes = header.into_bytes();
        header_bytes.push(b'\n');
        let mut out = b"\x93NUMPY\x01\x00".to_vec();
        let hlen = u16::try_from(header_bytes.len()).expect("tiny header");
        out.extend_from_slice(&hlen.to_le_bytes());
        out.extend_from_slice(&header_bytes);
        for v in vals {
            out.extend_from_slice(&v.to_le_bytes());
        }
        out
    }

    #[test]
    fn external_runner_run_returns_cwd_npy_not_pass() {
        let tmp = std::env::temp_dir().join(format!(
            "dragon_npu_ext_npy_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(3, |d| d.as_nanos())
        ));
        std::fs::create_dir_all(&tmp).expect("temp cwd");
        std::fs::write(tmp.join("output.npy"), tiny_npy_f32(&[0.25, 0.5])).expect("npy");
        let program = if Path::new("/usr/bin/true").is_file() {
            "/usr/bin/true"
        } else {
            "/bin/true"
        };
        let cfg =
            ExternalRunnerConfig::new(program, vec![], Some(tmp.to_string_lossy().into_owned()))
                .expect("cwd + present program");
        let scripted = Arc::new(ScriptedRunner {
            // This-run mention required; leftover files are not OFM.
            stdout: "wrote output.npy\nPASS!\nresult: [9.0]\n".into(),
            stderr: String::new(),
            code: 0,
            last: Mutex::new(None),
        });
        let backend = ExternalRunnerBackend::with_runner(cfg, scripted);
        let report = backend.run_external().expect("scripted");
        let out = backend.run(&[0.0]).expect("child exit 0");
        let _ = std::fs::remove_dir_all(&tmp);
        assert_eq!(
            out,
            vec![0.25, 0.5],
            "this-run npy must win over stdout JSON"
        );
        assert!(!report.ran_on_npu);
        assert_eq!(report.invent_pass, INVENT_PASS);
        assert_eq!(report.execution, EXECUTION_EXTERNAL_RUNNER);
        assert!(report.child_said_pass);
    }

    #[test]
    fn dragon_npu_new_does_not_select_external_runner() {
        let npu = crate::DragonNpu::new().expect("default new");
        let s = npu.status().expect("status");
        assert!(
            !s.device_name.contains("external-runner"),
            "DragonNpu::new must not select the external runner, got {}",
            s.device_name
        );
        assert_ne!(s.vendor, BACKEND_ID);
    }
}
