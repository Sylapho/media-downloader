use super::*;
use crate::process::{ProcessOutput, TokioProcessRunner};
use std::sync::Mutex;

struct MockRunner<F> {
    result: F,
    requests: Mutex<Vec<ProcessRequest>>,
}

impl<F> ProcessRunner for MockRunner<F>
where
    F: Fn(&ProcessRequest) -> Result<ProcessOutput, ProcessError> + Sync,
{
    async fn run(&self, request: ProcessRequest) -> Result<ProcessOutput, ProcessError> {
        let result = (self.result)(&request);
        self.requests.lock().unwrap().push(request);
        result
    }
}

fn output(text: &str) -> Result<ProcessOutput, ProcessError> {
    Ok(ProcessOutput {
        success: true,
        stdout: text.as_bytes().to_vec(),
        stderr: vec![],
    })
}

struct NameResolver;
impl ExecutableResolver for NameResolver {
    fn resolve(&self, tool: Tool) -> Result<PathBuf, DependencyError> {
        Ok(PathBuf::from(if cfg!(windows) {
            format!("{}.exe", tool.name())
        } else {
            tool.name().to_string()
        }))
    }
}
fn default_locations() -> NameResolver {
    NameResolver
}

#[test]
fn parses_supported_release_and_vendor_versions() {
    for (tool, text, version) in [
        (Tool::YtDlp, "2024.01.01\r\n", "2024.01.01"),
        (Tool::YtDlp, "2026.10.01.123456\n", "2026.10.01.123456"),
        (
            Tool::Ffmpeg,
            "ffmpeg version 6.0 Copyright\nlibavutil...",
            "6.0",
        ),
        (
            Tool::Ffmpeg,
            "ffmpeg version 8.0.1-essentials_build-www.gyan.dev Copyright",
            "8.0.1-essentials_build-www.gyan.dev",
        ),
        (Tool::Ffprobe, "ffprobe version n7.1.1 Copyright", "n7.1.1"),
        (Tool::Ffprobe, "ffprobe version 9.0 Copyright", "9.0"),
    ] {
        assert_eq!(parse_version(tool, text), Some((version.into(), true)));
    }
}

#[test]
fn rejects_old_versions_without_losing_the_version() {
    assert_eq!(
        parse_version(Tool::YtDlp, "2023.12.30"),
        Some(("2023.12.30".into(), false))
    );
    assert_eq!(
        parse_version(Tool::Ffmpeg, "ffmpeg version 5.1.6 Copyright"),
        Some(("5.1.6".into(), false))
    );
}

#[test]
fn rejects_malformed_dates_wrong_tools_and_unnumbered_builds() {
    for text in [
        "",
        "garbage",
        "2026.00.01",
        "2026.02.30",
        "2025.02.29",
        "2026.10.00",
        "2026.10.01.x",
        "2026.10.01 extra",
    ] {
        assert_eq!(parse_version(Tool::YtDlp, text), None, "{text}");
    }
    assert!(parse_version(Tool::YtDlp, "2024.02.29").is_some());
    for text in [
        "ffprobe version 8.0",
        "ffmpeg version N-1234-gabcdef",
        "ffmpeg version 8.x",
        "ffmpeg version 8",
        "unrelated 8.0",
    ] {
        assert_eq!(parse_version(Tool::Ffmpeg, text), None, "{text}");
    }
}

#[tokio::test]
async fn returns_all_tools_in_order_and_only_passes_fixed_version_arguments() {
    let runner = MockRunner {
        result: |request: &ProcessRequest| {
            if request.program.to_string_lossy().starts_with("yt-dlp") {
                output("2026.10.01")
            } else if request.program.to_string_lossy().starts_with("ffprobe") {
                output("ffprobe version 8.0.1")
            } else {
                output("ffmpeg version 8.0.1")
            }
        },
        requests: Mutex::new(vec![]),
    };
    let results = check_dependencies(&runner, &default_locations()).await;
    assert_eq!(
        results.iter().map(|s| s.tool).collect::<Vec<_>>(),
        [Tool::YtDlp, Tool::Ffmpeg, Tool::Ffprobe]
    );
    assert!(results
        .iter()
        .all(|s| s.available && s.version.is_some() && s.error.is_none()));
    let requests = runner.requests.lock().unwrap();
    assert_eq!(requests.len(), 3);
    assert_eq!(requests[0].args, ["--version"]);
    assert_eq!(requests[1].args, ["-version"]);
    assert_eq!(requests[2].args, ["-version"]);
    assert!(requests.iter().all(|r| r.timeout == Duration::from_secs(5)));
    #[cfg(windows)]
    assert!(requests
        .iter()
        .all(|r| r.program.extension().unwrap() == "exe"));
}

#[tokio::test]
async fn each_process_failure_is_structured_and_actionable() {
    for (error, code) in [
        (ProcessError::NotFound, ErrorCode::NotFound),
        (ProcessError::PermissionDenied, ErrorCode::PermissionDenied),
        (
            ProcessError::Failed("invalid executable".into()),
            ErrorCode::SpawnFailed,
        ),
        (ProcessError::Timeout, ErrorCode::Timeout),
        (ProcessError::OutputLimit, ErrorCode::OutputLimit),
    ] {
        let runner = MockRunner {
            result: |_: &ProcessRequest| {
                Err(match &error {
                    ProcessError::NotFound => ProcessError::NotFound,
                    ProcessError::PermissionDenied => ProcessError::PermissionDenied,
                    ProcessError::Failed(message) => ProcessError::Failed(message.clone()),
                    ProcessError::Timeout => ProcessError::Timeout,
                    ProcessError::OutputLimit => ProcessError::OutputLimit,
                })
            },
            requests: Mutex::new(vec![]),
        };
        for status in check_dependencies(&runner, &default_locations()).await {
            assert!(!status.available);
            assert!(status.version.is_none());
            let error = status.error.unwrap();
            assert_eq!(error.code, code);
            assert!(error.message.contains(status.tool.environment_variable()));
            assert!(error.message.contains("Install"));
        }
    }
}

#[tokio::test]
async fn broken_outputs_nonzero_exits_and_unsupported_versions_are_unavailable() {
    for (stdout, success, code, version) in [
        (b"".as_slice(), true, ErrorCode::InvalidOutput, None),
        (b"\xff".as_slice(), true, ErrorCode::InvalidOutput, None),
        (
            b"not yt-dlp".as_slice(),
            true,
            ErrorCode::InvalidOutput,
            None,
        ),
        (
            b"2026.10.01".as_slice(),
            false,
            ErrorCode::NonZeroExit,
            None,
        ),
        (
            b"2023.12.30".as_slice(),
            true,
            ErrorCode::UnsupportedVersion,
            Some("2023.12.30"),
        ),
    ] {
        let runner = MockRunner {
            result: |_: &ProcessRequest| {
                Ok(ProcessOutput {
                    success,
                    stdout: stdout.to_vec(),
                    stderr: b"broken executable".to_vec(),
                })
            },
            requests: Mutex::new(vec![]),
        };
        let result = check_tool(Tool::YtDlp, &runner, &default_locations()).await;
        assert!(!result.available);
        assert_eq!(result.version.as_deref(), version);
        assert_eq!(result.error.unwrap().code, code);
    }
}

#[tokio::test]
async fn one_missing_tool_does_not_hide_the_other_tools() {
    let runner = MockRunner {
        result: |request: &ProcessRequest| {
            if request.args == ["--version"] {
                Err(ProcessError::NotFound)
            } else if request.program.to_string_lossy().starts_with("ffprobe") {
                output("ffprobe version 8.0.1")
            } else {
                output("ffmpeg version 8.0.1")
            }
        },
        requests: Mutex::new(vec![]),
    };
    let statuses = check_dependencies(&runner, &default_locations()).await;
    assert!(!statuses[0].available);
    assert!(statuses[1].available && statuses[2].available);
}

#[tokio::test]
async fn unnumbered_ffmpeg_build_displays_its_version_without_claiming_support() {
    let runner = MockRunner {
        result: |_: &ProcessRequest| output("ffmpeg version N-12345-gabcdef Copyright"),
        requests: Mutex::new(vec![]),
    };
    let status = check_tool(Tool::Ffmpeg, &runner, &default_locations()).await;
    assert!(!status.available);
    assert_eq!(status.version.as_deref(), Some("N-12345-gabcdef"));
    assert_eq!(status.error.unwrap().code, ErrorCode::InvalidOutput);
}

#[tokio::test]
async fn configured_paths_are_used_literally_without_falling_back() {
    let path = std::env::temp_dir()
        .join("tools with spaces & literal")
        .join("yt-dlp.exe");
    let locations = LocalExecutables {
        paths: [Some(path.clone()), None, None],
        search_path: vec![],
    };
    let runner = MockRunner {
        result: |_: &ProcessRequest| Err(ProcessError::NotFound),
        requests: Mutex::new(vec![]),
    };
    let status = check_tool(Tool::YtDlp, &runner, &locations).await;
    assert_eq!(status.error.unwrap().code, ErrorCode::NotFound);
    let requests = runner.requests.lock().unwrap();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].program, path);
}

#[tokio::test]
async fn rejects_relative_empty_and_shell_script_configuration_before_running() {
    let runner = MockRunner {
        result: |_: &ProcessRequest| panic!("invalid paths must not execute"),
        requests: Mutex::new(vec![]),
    };
    for path in [
        PathBuf::new(),
        PathBuf::from("relative.exe"),
        std::env::temp_dir().join("tool.CMD"),
        std::env::temp_dir().join("tool.bat"),
    ] {
        let status = check_tool(
            Tool::YtDlp,
            &runner,
            &LocalExecutables {
                paths: [Some(path), None, None],
                search_path: vec![],
            },
        )
        .await;
        assert_eq!(status.error.unwrap().code, ErrorCode::InvalidConfiguration);
    }
}

#[tokio::test]
async fn ipc_serialization_matches_the_frontend_contract() {
    let status = failure(
        Tool::YtDlp,
        Some("2023.12.30".into()),
        ErrorCode::UnsupportedVersion,
        "Too old.",
    );
    let value = serde_json::to_value(status).unwrap();
    assert_eq!(value["tool"], "yt-dlp");
    assert_eq!(value["available"], false);
    assert_eq!(value["version"], "2023.12.30");
    assert_eq!(value["error"]["code"], "unsupported_version");
    assert!(value["error"]["message"]
        .as_str()
        .unwrap()
        .contains("Install"));
}

// Compile a tiny native fixture offline instead of invoking a shell or requiring
// users/CI to install media tools. Exercise the actual Windows process APIs too.
#[tokio::test]
async fn native_processes_handle_spaces_broken_files_limits_timeouts_and_cancellation() {
    let root = std::env::temp_dir().join(format!(
        "media-downloader-process-test-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&root).unwrap();
    struct Cleanup(PathBuf);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    let _cleanup = Cleanup(root.clone());
    let dir = root.join("tools with spaces & literal");
    std::fs::create_dir(&dir).unwrap();
    let executable = dir.join(format!("fixture{}", std::env::consts::EXE_SUFFIX));
    let compile = std::process::Command::new("rustc")
        .arg("--edition=2021")
        .arg(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/version_tool.rs"))
        .arg("-o")
        .arg(&executable)
        .output()
        .unwrap();
    assert!(
        compile.status.success(),
        "{}",
        String::from_utf8_lossy(&compile.stderr)
    );
    let copy = |name: &str| {
        let path = dir.join(format!("{name}{}", std::env::consts::EXE_SUFFIX));
        std::fs::copy(&executable, &path).unwrap();
        path
    };
    let runner = TokioProcessRunner;
    let locations = LocalExecutables {
        paths: [
            Some(copy("yt-dlp")),
            Some(copy("ffmpeg")),
            Some(copy("ffprobe")),
        ],
        search_path: vec![],
    };
    let statuses = check_dependencies(&runner, &locations).await;
    assert!(statuses.iter().all(|s| s.available), "{statuses:?}");
    let from_path = LocalExecutables {
        paths: [None, None, None],
        search_path: vec![PathBuf::from("relative"), dir.clone()],
    };
    let statuses = check_dependencies(&runner, &from_path).await;
    assert!(
        statuses.iter().all(|s| s.available),
        "PATH lookup: {statuses:?}"
    );
    assert!(LocalExecutables {
        paths: [None, None, None],
        search_path: vec![]
    }
    .resolve(Tool::YtDlp)
    .is_err());

    let run = |program, timeout| {
        runner.run(ProcessRequest {
            program,
            args: vec!["--version".into()],
            timeout,
            stdout_limit: 16 * 1024,
            stderr_limit: 16 * 1024,
        })
    };
    let failed = run(copy("failed"), Duration::from_secs(2)).await.unwrap();
    assert!(!failed.success);
    assert!(String::from_utf8_lossy(&failed.stderr).contains("deliberate failure"));
    assert_eq!(
        run(copy("flood"), Duration::from_secs(2)).await.err(),
        Some(ProcessError::OutputLimit)
    );
    assert_eq!(
        run(copy("stderr-flood"), Duration::from_secs(2))
            .await
            .err(),
        Some(ProcessError::OutputLimit)
    );
    assert_eq!(
        run(copy("slow"), Duration::from_millis(500)).await.err(),
        Some(ProcessError::Timeout)
    );
    let slow = copy("cancelled");
    let task = tokio::spawn(async move {
        TokioProcessRunner
            .run(ProcessRequest {
                program: slow,
                args: vec!["--version".into()],
                timeout: Duration::from_secs(5),
                stdout_limit: 16 * 1024,
                stderr_limit: 16 * 1024,
            })
            .await
    });
    tokio::time::timeout(Duration::from_secs(2), async {
        while !dir.join("cancelled-started").exists() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("child must start before cancellation");
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    assert!(
        dir.join("slow-started").exists(),
        "child must start before timing out"
    );
    tokio::time::sleep(Duration::from_millis(2100)).await;
    assert!(
        !dir.join("slow-completed").exists(),
        "timed-out child survived"
    );
    assert!(
        !dir.join("cancelled-completed").exists(),
        "cancelled child survived"
    );
    assert_eq!(
        run(dir.join("missing.exe"), Duration::from_secs(2))
            .await
            .err(),
        Some(ProcessError::NotFound)
    );
    let broken = dir.join("broken.exe");
    std::fs::write(&broken, "not an executable").unwrap();
    assert!(run(broken, Duration::from_secs(2)).await.is_err());
}

#[tokio::test]
#[ignore = "manual smoke test requiring FFmpeg and ffprobe installed locally"]
async fn locally_installed_ffmpeg_and_ffprobe_are_detected() {
    let statuses =
        check_dependencies(&TokioProcessRunner, &LocalExecutables::from_environment()).await;
    println!("{}", serde_json::to_string_pretty(&statuses).unwrap());
    for status in statuses.iter().filter(|status| status.tool != Tool::YtDlp) {
        assert!(status.available, "{status:?}");
    }
}
