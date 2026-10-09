use super::*;
use crate::{
    dependencies::DependencyError,
    process::{ProcessOutput, TokioProcessRunner},
};
use std::{path::PathBuf, sync::Mutex};

const MEDIA: &[u8] = include_bytes!("../../tests/fixtures/media.json");

struct Resolver;
impl ExecutableResolver for Resolver {
    fn resolve(&self, tool: Tool) -> Result<PathBuf, DependencyError> {
        assert_eq!(tool, Tool::YtDlp);
        Ok(std::env::temp_dir()
            .join("tools with spaces")
            .join("yt-dlp.exe"))
    }
}

struct Runner<F> {
    response: F,
    requests: Mutex<Vec<ProcessRequest>>,
}
impl<F: Fn() -> Result<ProcessOutput, ProcessError> + Sync> ProcessRunner for Runner<F> {
    async fn run(&self, request: ProcessRequest) -> Result<ProcessOutput, ProcessError> {
        self.requests.lock().unwrap().push(request);
        (self.response)()
    }
}

fn output(bytes: &[u8]) -> Result<ProcessOutput, ProcessError> {
    Ok(ProcessOutput {
        success: true,
        stdout: bytes.to_vec(),
        stderr: vec![],
    })
}

#[test]
fn normalizes_audio_video_and_combined_formats_to_the_shared_contract() {
    let media = normalize_metadata(MEDIA).unwrap();
    let actual = serde_json::to_value(media).unwrap();
    let expected: Value =
        serde_json::from_slice(include_bytes!("../../tests/fixtures/media.normalized.json"))
            .unwrap();
    assert_eq!(actual, expected);
    assert!(!actual.to_string().contains("token=secret"));
    assert!(!actual.to_string().contains("webpage_url"));
}

#[test]
fn preserves_unknowns_filters_invalid_numbers_and_falls_back_to_a_safe_thumbnail() {
    let media =
        normalize_metadata(include_bytes!("../../tests/fixtures/media-unknown.json")).unwrap();
    assert_eq!(media.id, "42");
    assert!(media.title.is_none() && media.duration.is_none());
    assert_eq!(media.uploader.as_deref(), Some("Fallback channel"));
    assert_eq!(
        media.thumbnail.as_deref(),
        Some("https://media.example.org/large.jpg")
    );
    assert_eq!(media.formats.len(), 1);
    let format = &media.formats[0];
    assert_eq!(format.format_id, "7");
    assert!(format.has_audio.is_none() && format.has_video.is_none());
    assert!(
        format.width.is_none()
            && format.height.is_none()
            && format.fps.is_none()
            && format.file_size.is_none()
    );
}

#[test]
fn handles_missing_null_and_top_level_formats() {
    for json in [
        r#"{"id":"live"}"#,
        r#"{"id":"live","formats":null,"duration":-1}"#,
        r#"{"id":"live","formats":[]}"#,
    ] {
        let media = normalize_metadata(json.as_bytes()).unwrap();
        assert!(media.duration.is_none());
        assert!(media.formats.is_empty());
        assert!(media.thumbnail.is_none());
    }
    let media =
        normalize_metadata(include_bytes!("../../tests/fixtures/media-direct.json")).unwrap();
    assert_eq!(media.formats.len(), 1);
    assert_eq!(media.formats[0].format_id, "mp4");
    assert_eq!(media.formats[0].has_video, None);
}

#[test]
fn rejects_invalid_json_wrong_root_missing_ids_and_invalid_format_containers() {
    for bytes in [
        b"not JSON".as_slice(),
        b"\xff",
        b"[]",
        b"null",
        b"{}",
        br#"{"id":""}"#,
        br#"{"id":"x","formats":{}}"#,
        br#"{"id":"x","duration":NaN}"#,
    ] {
        assert_eq!(
            normalize_metadata(bytes).unwrap_err().code,
            ExtractionErrorCode::InvalidMetadata,
            "{bytes:?}"
        );
    }
}

#[test]
fn rejects_playlists_multi_item_and_nonpublic_metadata() {
    for json in [
        r#"{"_type":"playlist","id":"x","entries":[]}"#,
        r#"{"_type":"multi_video","id":"x"}"#,
        r#"{"_type":"url","id":"x"}"#,
        r#"{"id":"x","entries":[]}"#,
    ] {
        assert_eq!(
            normalize_metadata(json.as_bytes()).unwrap_err().code,
            ExtractionErrorCode::UnsupportedContent
        );
    }
    for availability in ["private", "premium_only", "subscriber_only", "needs_auth"] {
        let json = serde_json::json!({"id":"x", "availability":availability});
        assert_eq!(
            normalize_metadata(&serde_json::to_vec(&json).unwrap())
                .unwrap_err()
                .code,
            ExtractionErrorCode::AuthenticationRequired
        );
    }
}

#[test]
fn accepts_public_urls_and_keeps_queries_as_one_argument() {
    for input in [
        "HTTPS://media.example.org/watch",
        "https://media.example.org/watch?v=x&list=y",
        " http://media.example.org/a%20b ",
        "https://8.8.8.8/video",
        "https://[2606:4700:4700::1111]/video",
        "https://media.example.org/watch?x=%26%3B%24%28whoami%29",
    ] {
        assert!(validate_url(input).is_ok(), "{input}");
    }
}

#[tokio::test]
async fn invalid_input_never_resolves_or_runs_a_process() {
    struct NoResolver;
    impl ExecutableResolver for NoResolver {
        fn resolve(&self, _: Tool) -> Result<PathBuf, DependencyError> {
            panic!("invalid input reached resolver")
        }
    }
    let runner = Runner {
        response: || panic!("invalid input ran process"),
        requests: Mutex::new(vec![]),
    };
    let extractor = YtDlpExtractor::new(&runner, &NoResolver);
    for url in [
        "https:///media.example.org/watch",
        "https://media.example.org\\private",
        "http://localhost./watch",
        "http://localhost.localdomain/watch",
        "",
        "not a url",
        "--exec=calc.exe",
        "file:///C:/private",
        "ftp://media.example.org/file",
        "https://user:pass@media.example.org/x",
        "https://media.example.org/a b",
        "https://media.example.org/\n--exec=bad",
        "http://localhost/x",
        "http://127.0.0.1/x",
        "http://2130706433/x",
        "http://10.0.0.1/x",
        "http://192.168.1.1/x",
        "http://169.254.169.254/x",
        "http://[::1]/x",
        "http://[::ffff:127.0.0.1]/x",
        "http://[fc00::1]/x",
        "http://[fe80::1]/x",
        "https://media.example.org:0/x",
    ] {
        assert_eq!(
            extractor.extract(url).await.unwrap_err().code,
            ExtractionErrorCode::InvalidUrl,
            "{url}"
        );
    }
    assert_eq!(
        extractor
            .extract(&format!("https://media.example.org/{}", "x".repeat(4096)))
            .await
            .unwrap_err()
            .code,
        ExtractionErrorCode::InvalidUrl
    );
}

#[tokio::test]
async fn adapter_uses_separate_safe_arguments_and_bounded_process_policy() {
    let runner = Runner {
        response: || output(MEDIA),
        requests: Mutex::new(vec![]),
    };
    let url = "https://media.example.org/watch?v=x&text=hello;%24(calc)";
    let media = YtDlpExtractor::new(&runner, &Resolver)
        .extract(url)
        .await
        .unwrap();
    assert_eq!(media.id, "sample-video");
    let requests = runner.requests.lock().unwrap();
    assert_eq!(requests.len(), 1);
    let request = &requests[0];
    assert!(request
        .program
        .to_string_lossy()
        .contains("tools with spaces"));
    let last = request.args.len() - 1;
    assert_eq!(request.args[last - 1], "--");
    assert_eq!(request.args[last], validate_url(url).unwrap());
    for flag in [
        "--no-playlist",
        "--dump-single-json",
        "--simulate",
        "--skip-download",
        "--ignore-config",
        "--no-cache-dir",
    ] {
        assert!(request.args.contains(&flag.to_string()));
    }
    assert!(!request
        .args
        .iter()
        .any(|arg| arg == "--exec" || arg == "--cookies" || arg == "--no-simulate"));
    assert_eq!(request.timeout, Duration::from_secs(30));
    assert_eq!(request.stdout_limit, 4 * 1024 * 1024);
    assert_eq!(request.stderr_limit, 64 * 1024);
}

#[tokio::test]
async fn process_failures_have_distinct_actionable_codes() {
    for (error, expected) in [
        (ProcessError::NotFound, ExtractionErrorCode::MissingYtDlp),
        (ProcessError::Timeout, ExtractionErrorCode::Timeout),
        (ProcessError::OutputLimit, ExtractionErrorCode::OutputLimit),
        (
            ProcessError::PermissionDenied,
            ExtractionErrorCode::PermissionDenied,
        ),
        (
            ProcessError::Failed("failure".into()),
            ExtractionErrorCode::ExtractionFailed,
        ),
    ] {
        let runner = Runner {
            response: || {
                Err(match &error {
                    ProcessError::NotFound => ProcessError::NotFound,
                    ProcessError::Timeout => ProcessError::Timeout,
                    ProcessError::OutputLimit => ProcessError::OutputLimit,
                    ProcessError::PermissionDenied => ProcessError::PermissionDenied,
                    ProcessError::Failed(message) => ProcessError::Failed(message.clone()),
                })
            },
            requests: Mutex::new(vec![]),
        };
        let error = YtDlpExtractor::new(&runner, &Resolver)
            .extract("https://media.example.org/watch")
            .await
            .unwrap_err();
        assert_eq!(error.code, expected);
        assert!(!error.message.is_empty());
    }
}

#[tokio::test]
async fn missing_and_invalid_configuration_do_not_run_yt_dlp() {
    struct BadResolver(DependencyErrorCode);
    impl ExecutableResolver for BadResolver {
        fn resolve(&self, _: Tool) -> Result<PathBuf, DependencyError> {
            Err(DependencyError {
                code: self.0.clone(),
                message: "bad config".into(),
            })
        }
    }
    let runner = Runner {
        response: || panic!("bad resolver should not run process"),
        requests: Mutex::new(vec![]),
    };
    for (code, expected) in [
        (
            DependencyErrorCode::NotFound,
            ExtractionErrorCode::MissingYtDlp,
        ),
        (
            DependencyErrorCode::InvalidConfiguration,
            ExtractionErrorCode::InvalidConfiguration,
        ),
    ] {
        let error = YtDlpExtractor::new(&runner, &BadResolver(code))
            .extract("https://media.example.org/watch")
            .await
            .unwrap_err();
        assert_eq!(error.code, expected);
    }
}

#[tokio::test]
async fn nonzero_exits_classify_content_network_and_runtime_failures_without_leaking_urls() {
    for (message, expected) in [
        (
            "ERROR: Unsupported URL: https://media.example.org/?token=secret",
            ExtractionErrorCode::UnsupportedContent,
        ),
        (
            "ERROR: Video unavailable",
            ExtractionErrorCode::UnsupportedContent,
        ),
        (
            "ERROR: Sign in to confirm your age",
            ExtractionErrorCode::AuthenticationRequired,
        ),
        (
            "ERROR: No supported JavaScript runtime found",
            ExtractionErrorCode::RuntimeRequired,
        ),
        (
            "ERROR: yt-dlp-ejs challenge solving failed",
            ExtractionErrorCode::RuntimeRequired,
        ),
        ("ERROR: connection timed out", ExtractionErrorCode::Timeout),
        (
            "ERROR: Unable to download webpage: HTTP Error 503",
            ExtractionErrorCode::NetworkError,
        ),
        (
            "ERROR: unexpected failure",
            ExtractionErrorCode::ExtractionFailed,
        ),
    ] {
        let runner = Runner {
            response: || {
                Ok(ProcessOutput {
                    success: false,
                    stdout: vec![],
                    stderr: message.as_bytes().to_vec(),
                })
            },
            requests: Mutex::new(vec![]),
        };
        let error = YtDlpExtractor::new(&runner, &Resolver)
            .extract("https://media.example.org/watch")
            .await
            .unwrap_err();
        assert_eq!(error.code, expected, "{message}");
        assert!(!error.message.contains("secret"));
        assert!(!error.message.contains("https://media.example.org"));
        let json = serde_json::to_value(error).unwrap();
        assert!(json["code"].is_string() && json["message"].is_string());
    }
}

#[tokio::test]
#[ignore = "manual integration check: installed yt-dlp and an authorized public URL required"]
async fn authorized_public_url_integration() {
    let url = std::env::var("MEDIA_DOWNLOADER_TEST_URL")
        .expect("set MEDIA_DOWNLOADER_TEST_URL to an authorized public single-media URL");
    let runner = TokioProcessRunner;
    let resolver = crate::dependencies::LocalExecutables::from_environment();
    let info = YtDlpExtractor::new(&runner, &resolver)
        .extract(&url)
        .await
        .expect("metadata-only extraction failed");
    assert!(!info.id.is_empty());
    assert!(
        !info.formats.is_empty(),
        "expected selectable formats for the integration URL"
    );
    println!(
        "id={}, title={:?}, duration={:?} seconds, uploader={:?}, formats={}",
        info.id,
        info.title,
        info.duration,
        info.uploader,
        info.formats.len()
    );
}

#[tokio::test]
async fn native_adapter_handles_a_large_json_response_and_literal_url_arguments() {
    let root = std::env::temp_dir().join(format!(
        "media-downloader-native-extractor-{}-{}",
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
    let directory = root.join("tools with spaces & literal");
    std::fs::create_dir(&directory).unwrap();
    let executable = directory.join(format!("metadata{}", std::env::consts::EXE_SUFFIX));
    let compiled = std::process::Command::new("rustc")
        .arg("--edition=2021")
        .arg(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/version_tool.rs"))
        .arg("-o")
        .arg(&executable)
        .output()
        .unwrap();
    assert!(
        compiled.status.success(),
        "{}",
        String::from_utf8_lossy(&compiled.stderr)
    );
    struct NativeResolver(PathBuf);
    impl ExecutableResolver for NativeResolver {
        fn resolve(&self, _: Tool) -> Result<PathBuf, DependencyError> {
            Ok(self.0.clone())
        }
    }
    let info = YtDlpExtractor::new(&TokioProcessRunner, &NativeResolver(executable))
        .extract("https://media.example.org/watch?v=x&text=hello;%24(calc)")
        .await
        .unwrap();
    assert_eq!(info.id, "sample-video");
    assert_eq!(info.formats.len(), 3);
}
