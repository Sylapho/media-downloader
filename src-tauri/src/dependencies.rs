use crate::process::{ProcessError, ProcessRequest, ProcessRunner};
use serde::Serialize;
use std::{path::PathBuf, time::Duration};

#[cfg(test)]
mod tests;

const CHECK_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum Tool {
    YtDlp,
    Ffmpeg,
    Ffprobe,
}

impl Tool {
    pub fn name(self) -> &'static str {
        match self {
            Self::YtDlp => "yt-dlp",
            Self::Ffmpeg => "ffmpeg",
            Self::Ffprobe => "ffprobe",
        }
    }

    pub fn environment_variable(self) -> &'static str {
        match self {
            Self::YtDlp => "MEDIA_DOWNLOADER_YT_DLP_PATH",
            Self::Ffmpeg => "MEDIA_DOWNLOADER_FFMPEG_PATH",
            Self::Ffprobe => "MEDIA_DOWNLOADER_FFPROBE_PATH",
        }
    }

    fn args(self) -> &'static [&'static str] {
        match self {
            Self::YtDlp => &["--version"],
            _ => &["-version"],
        }
    }

    fn requirement(self) -> &'static str {
        match self {
            Self::YtDlp => "yt-dlp >= 2024.01.01",
            _ => "FFmpeg/ffprobe >= 6.0 (numbered release)",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ErrorCode {
    NotFound,
    PermissionDenied,
    SpawnFailed,
    Timeout,
    OutputLimit,
    NonZeroExit,
    InvalidOutput,
    UnsupportedVersion,
    InvalidConfiguration,
}

#[derive(Debug, Serialize)]
pub(crate) struct DependencyError {
    pub code: ErrorCode,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct DependencyStatus {
    pub tool: Tool,
    pub available: bool,
    pub version: Option<String>,
    pub error: Option<DependencyError>,
}

// Executable resolution is separate from probing. A future sidecar resolver can
// implement this interface without changing version checks or the IPC contract.
pub(crate) trait ExecutableResolver: Sync {
    fn resolve(&self, tool: Tool) -> Result<PathBuf, DependencyError>;
}

pub(crate) struct LocalExecutables {
    paths: [Option<PathBuf>; 3],
    search_path: Vec<PathBuf>,
}

impl LocalExecutables {
    pub fn from_environment() -> Self {
        Self {
            paths: [Tool::YtDlp, Tool::Ffmpeg, Tool::Ffprobe]
                .map(|tool| std::env::var_os(tool.environment_variable()).map(PathBuf::from)),
            search_path: std::env::var_os("PATH")
                .map(|path| std::env::split_paths(&path).collect())
                .unwrap_or_default(),
        }
    }
}

impl ExecutableResolver for LocalExecutables {
    fn resolve(&self, tool: Tool) -> Result<PathBuf, DependencyError> {
        let index = match tool {
            Tool::YtDlp => 0,
            Tool::Ffmpeg => 1,
            Tool::Ffprobe => 2,
        };
        if let Some(path) = &self.paths[index] {
            let shell_script = path.extension().is_some_and(|extension| {
                extension.eq_ignore_ascii_case("bat") || extension.eq_ignore_ascii_case("cmd")
            });
            if !path.is_absolute() || shell_script {
                return Err(DependencyError { code: ErrorCode::InvalidConfiguration, message: format!("{} must be an absolute executable file path without surrounding quotes; .bat/.cmd shell scripts are not supported.", tool.environment_variable()) });
            }
            return Ok(path.clone());
        }
        // Only native .exe programs on Windows; never implicitly resolve .cmd/.bat.
        let filename = if cfg!(windows) {
            format!("{}.exe", tool.name())
        } else {
            tool.name().to_string()
        };
        // Resolve PATH explicitly: Windows process creation otherwise also searches
        // the app/current directory. Ignore relative entries to avoid that ambiguity.
        self.search_path
            .iter()
            .filter(|dir| dir.is_absolute())
            .map(|dir| dir.join(&filename))
            .find(|path| path.is_file())
            .ok_or_else(|| DependencyError {
                code: ErrorCode::NotFound,
                message: "Executable not found in an absolute PATH directory.".into(),
            })
    }
}

pub(crate) async fn check_dependencies(
    runner: &impl ProcessRunner,
    resolver: &impl ExecutableResolver,
) -> Vec<DependencyStatus> {
    let (yt_dlp, ffmpeg, ffprobe) = tokio::join!(
        check_tool(Tool::YtDlp, runner, resolver),
        check_tool(Tool::Ffmpeg, runner, resolver),
        check_tool(Tool::Ffprobe, runner, resolver),
    );
    vec![yt_dlp, ffmpeg, ffprobe]
}

fn failure(tool: Tool, version: Option<String>, code: ErrorCode, detail: &str) -> DependencyStatus {
    DependencyStatus { tool, available: false, version, error: Some(DependencyError {
        code, message: format!("{detail} Install {} and add its directory to PATH, or set {} to its absolute executable path, then restart the app and recheck.", tool.requirement(), tool.environment_variable()),
    }) }
}

async fn check_tool(
    tool: Tool,
    runner: &impl ProcessRunner,
    resolver: &impl ExecutableResolver,
) -> DependencyStatus {
    let program = match resolver.resolve(tool) {
        Ok(program) => program,
        Err(error) => return failure(tool, None, error.code, &error.message),
    };
    let output = match runner
        .run(ProcessRequest {
            program,
            args: tool.args().iter().map(|arg| (*arg).to_owned()).collect(),
            timeout: CHECK_TIMEOUT,
            stdout_limit: 16 * 1024,
            stderr_limit: 16 * 1024,
        })
        .await
    {
        Ok(output) => output,
        Err(error) => {
            let (code, detail) = match error {
                ProcessError::NotFound => (ErrorCode::NotFound, "Executable not found.".to_string()),
                ProcessError::PermissionDenied => (ErrorCode::PermissionDenied, "Permission denied. Check executable access permissions.".to_string()),
                ProcessError::Failed(detail) => (ErrorCode::SpawnFailed, format!("Could not run executable: {detail}")),
                ProcessError::Timeout => (ErrorCode::Timeout, "Version check timed out after 5 seconds. Check or replace the executable.".to_string()),
                ProcessError::OutputLimit => (ErrorCode::OutputLimit, "Version output exceeded the 16 KiB safety limit. Check or replace the executable.".to_string()),
            };
            return failure(tool, None, code, &detail);
        }
    };
    if !output.success {
        // Limit diagnostics further so a broken executable cannot flood the UI.
        let diagnostic = String::from_utf8_lossy(&output.stderr)
            .chars()
            .filter(|c| !c.is_control() || *c == ' ')
            .take(300)
            .collect::<String>();
        return failure(
            tool,
            None,
            ErrorCode::NonZeroExit,
            &format!("Version command failed. {diagnostic}"),
        );
    }
    let text = match std::str::from_utf8(&output.stdout) {
        Ok(text) if !text.trim().is_empty() => text,
        _ => {
            return failure(
                tool,
                None,
                ErrorCode::InvalidOutput,
                "Executable returned empty or invalid UTF-8 version output.",
            )
        }
    };
    let Some((version, supported)) = parse_version(tool, text) else {
        // Preserve a genuine FFmpeg banner's version even if an unnumbered
        // snapshot cannot establish the supported minimum.
        let detected = if tool == Tool::YtDlp {
            None
        } else {
            banner_version(tool, text).map(str::to_owned)
        };
        return failure(tool, detected, ErrorCode::InvalidOutput, "Executable returned an unrecognized version. Use a numbered release of the expected tool.");
    };
    if !supported {
        return failure(
            tool,
            Some(version),
            ErrorCode::UnsupportedVersion,
            "Installed version is below the supported minimum.",
        );
    }
    DependencyStatus {
        tool,
        available: true,
        version: Some(version),
        error: None,
    }
}

fn banner_version(tool: Tool, text: &str) -> Option<&str> {
    let line = text.lines().find(|line| !line.trim().is_empty())?.trim();
    let mut words = line.split_whitespace();
    if words.next()? != tool.name() || words.next()? != "version" {
        return None;
    }
    words.next()
}

fn parse_version(tool: Tool, text: &str) -> Option<(String, bool)> {
    let line = text.lines().find(|line| !line.trim().is_empty())?.trim();
    if tool == Tool::YtDlp {
        if line.split_whitespace().count() != 1 {
            return None;
        }
        let mut parts = line.split('.');
        let year: u32 = parts.next()?.parse().ok()?;
        let month: u32 = parts.next()?.parse().ok()?;
        let day: u32 = parts.next()?.parse().ok()?;
        let leap =
            year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400));
        let days = match month {
            2 if leap => 29,
            2 => 28,
            4 | 6 | 9 | 11 => 30,
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            _ => return None,
        };
        if year < 2000
            || day == 0
            || day > days
            || parts.any(|p| p.is_empty() || !p.chars().all(|c| c.is_ascii_digit()))
        {
            return None;
        }
        return Some((line.to_string(), (year, month, day) >= (2024, 1, 1)));
    }
    let version = banner_version(tool, text)?;
    let numeric = version
        .strip_prefix('n')
        .or_else(|| version.strip_prefix('v'))
        .unwrap_or(version);
    let core = numeric.split('-').next()?;
    let mut parts = core.split('.');
    let major: u32 = parts.next()?.parse().ok()?;
    let _: u32 = parts.next()?.parse().ok()?;
    for part in parts {
        let _: u32 = part.parse().ok()?;
    }
    Some((version.to_string(), major >= 6))
}
