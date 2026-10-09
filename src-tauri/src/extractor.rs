use crate::{
    dependencies::{ErrorCode as DependencyErrorCode, ExecutableResolver, Tool},
    process::{ProcessError, ProcessRequest, ProcessRunner},
};
use serde::Serialize;
use serde_json::Value;
use std::{collections::HashSet, net::IpAddr, time::Duration};
use url::{Host, Url};

#[cfg(test)]
mod tests;

const EXTRACTION_TIMEOUT: Duration = Duration::from_secs(30);
const JSON_LIMIT: u64 = 4 * 1024 * 1024;
const DIAGNOSTIC_LIMIT: u64 = 64 * 1024;

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ExtractionErrorCode {
    InvalidUrl,
    MissingYtDlp,
    InvalidConfiguration,
    PermissionDenied,
    Timeout,
    OutputLimit,
    UnsupportedContent,
    AuthenticationRequired,
    RuntimeRequired,
    NetworkError,
    ExtractionFailed,
    InvalidMetadata,
}

#[derive(Debug, Serialize)]
pub(crate) struct ExtractionError {
    pub code: ExtractionErrorCode,
    pub message: String,
}

impl ExtractionError {
    fn new(code: ExtractionErrorCode, message: &str) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MediaInfo {
    pub id: String,
    pub title: Option<String>,
    /// Seconds; unknown/live durations remain null.
    pub duration: Option<f64>,
    pub thumbnail: Option<String>,
    pub uploader: Option<String>,
    pub formats: Vec<MediaFormat>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MediaFormat {
    pub format_id: String,
    pub extension: Option<String>,
    pub format_note: Option<String>,
    pub audio_codec: Option<String>,
    pub video_codec: Option<String>,
    pub has_audio: Option<bool>,
    pub has_video: Option<bool>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub fps: Option<f64>,
    /// Bitrates in kilobits per second, sample rate in Hz, sizes in bytes.
    pub audio_bitrate: Option<f64>,
    pub video_bitrate: Option<f64>,
    pub total_bitrate: Option<f64>,
    pub sample_rate: Option<u32>,
    pub file_size: Option<f64>,
    pub file_size_approx: Option<f64>,
}

/// The yt-dlp adapter owns command policy and normalization; execution and
/// executable resolution remain replaceable for offline tests/future sidecars.
pub(crate) struct YtDlpExtractor<'a, R, E> {
    runner: &'a R,
    resolver: &'a E,
}

impl<'a, R: ProcessRunner, E: ExecutableResolver> YtDlpExtractor<'a, R, E> {
    pub fn new(runner: &'a R, resolver: &'a E) -> Self {
        Self { runner, resolver }
    }

    pub async fn extract(&self, input: &str) -> Result<MediaInfo, ExtractionError> {
        let url = validate_url(input)?;
        let program = self.resolver.resolve(Tool::YtDlp).map_err(|error| match error.code {
            DependencyErrorCode::NotFound => missing_tool(),
            _ => ExtractionError::new(ExtractionErrorCode::InvalidConfiguration, "Fix MEDIA_DOWNLOADER_YT_DLP_PATH: use an absolute native executable path, without surrounding quotes or shell scripts."),
        })?;
        let mut args = [
            "--ignore-config",
            "--no-playlist",
            "--dump-single-json",
            "--simulate",
            "--skip-download",
            "--no-cache-dir",
            "--no-plugin-dirs",
            "--js-runtimes",
            "node",
            "--no-progress",
            "--no-color",
            "--socket-timeout",
            "15",
            "--retries",
            "0",
            "--extractor-retries",
            "0",
            "--playlist-items",
            "1",
            "--",
        ]
        .map(str::to_owned)
        .to_vec();
        args.push(url);
        let output = self
            .runner
            .run(ProcessRequest {
                program,
                args,
                timeout: EXTRACTION_TIMEOUT,
                stdout_limit: JSON_LIMIT,
                stderr_limit: DIAGNOSTIC_LIMIT,
            })
            .await
            .map_err(process_error)?;
        if !output.success {
            return Err(command_error(&output.stderr));
        }
        normalize_metadata(&output.stdout)
    }
}

fn missing_tool() -> ExtractionError {
    ExtractionError::new(ExtractionErrorCode::MissingYtDlp, "yt-dlp was not found. Install a current yt-dlp release and add it to PATH, or set MEDIA_DOWNLOADER_YT_DLP_PATH to its absolute executable path.")
}

fn process_error(error: ProcessError) -> ExtractionError {
    use ExtractionErrorCode as Code;
    match error {
        ProcessError::NotFound => missing_tool(),
        ProcessError::PermissionDenied => ExtractionError::new(Code::PermissionDenied, "Cannot execute yt-dlp. Check its file permissions and configured path."),
        ProcessError::Timeout => ExtractionError::new(Code::Timeout, "Metadata extraction exceeded 30 seconds. Check your connection and try again."),
        ProcessError::OutputLimit => ExtractionError::new(Code::OutputLimit, "yt-dlp returned too much output (4 MiB metadata or 64 KiB diagnostics). Use a single media URL and update yt-dlp."),
        ProcessError::Failed(_) => ExtractionError::new(Code::ExtractionFailed, "Could not run yt-dlp. Check or replace the executable and try again."),
    }
}

fn command_error(stderr: &[u8]) -> ExtractionError {
    use ExtractionErrorCode as Code;
    let text = String::from_utf8_lossy(stderr).to_lowercase();
    let contains = |patterns: &[&str]| patterns.iter().any(|pattern| text.contains(pattern));
    // Do not expose raw tool diagnostics (which may contain signed URLs/tokens).
    if contains(&[
        "javascript runtime",
        "js runtime",
        "yt-dlp-ejs",
        "ejs challenge",
        "ejs script",
        "challenge solving",
        "js challenge",
    ]) {
        ExtractionError::new(Code::RuntimeRequired, "This platform requires yt-dlp EJS and a supported JavaScript runtime. Update yt-dlp and follow its EJS installation guide.")
    } else if contains(&[
        "sign in",
        "sign-in",
        "login",
        "log in",
        "private video",
        "cookies",
        "age-restricted",
        "age restricted",
        "members-only",
        "premium",
    ]) {
        ExtractionError::new(Code::AuthenticationRequired, "This media requires authentication or access permissions. Choose a public media URL; cookies and login are not supported.")
    } else if contains(&[
        "unsupported url",
        "no suitable extractor",
        "not supported",
        "unavailable",
        "not available",
        "has been removed",
        "copyright",
        "geo-restricted",
        "geo restricted",
    ]) {
        ExtractionError::new(Code::UnsupportedContent, "This URL or media is unsupported or unavailable. Check the URL, choose a public single media item and update yt-dlp.")
    } else if contains(&["timed out", "timeout"]) {
        ExtractionError::new(
            Code::Timeout,
            "The media site did not respond in time. Check your connection and try again.",
        )
    } else if contains(&[
        "unable to download",
        "unable to connect",
        "connection",
        "http error",
        "dns",
        "name resolution",
        "network",
    ]) {
        ExtractionError::new(
            Code::NetworkError,
            "Could not reach the media site. Check your connection or try again later.",
        )
    } else {
        ExtractionError::new(Code::ExtractionFailed, "yt-dlp could not extract this media. Update yt-dlp, verify it is a supported public URL and try again.")
    }
}

fn validate_url(input: &str) -> Result<String, ExtractionError> {
    let invalid = || {
        ExtractionError::new(ExtractionErrorCode::InvalidUrl, "Enter one public HTTP or HTTPS media URL, without credentials or local/private addresses.")
    };
    if input.len() > 4096 || input.chars().any(char::is_control) {
        return Err(invalid());
    }
    let input = input.trim();
    let (scheme, authority) = input.split_once("://").ok_or_else(invalid)?;
    if input.chars().any(char::is_whitespace)
        || input.contains('\\')
        || !(scheme.eq_ignore_ascii_case("http") || scheme.eq_ignore_ascii_case("https"))
        || authority.starts_with('/')
        || authority.is_empty()
    {
        return Err(invalid());
    }
    let url = Url::parse(input).map_err(|_| invalid())?;
    if !matches!(url.scheme(), "http" | "https")
        || !url.username().is_empty()
        || url.password().is_some()
        || url.port() == Some(0)
    {
        return Err(invalid());
    }
    match url.host().ok_or_else(invalid)? {
        Host::Domain(host)
            if host.trim_end_matches('.') == "localhost"
                || host.trim_end_matches('.') == "localhost.localdomain"
                || host.trim_end_matches('.').ends_with(".localhost")
                || host.trim_end_matches('.').ends_with(".local")
                || !host.trim_end_matches('.').contains('.') =>
        {
            return Err(invalid())
        }
        Host::Ipv4(ip) if !public_ip(IpAddr::V4(ip)) => return Err(invalid()),
        Host::Ipv6(ip) if !public_ip(IpAddr::V6(ip)) => return Err(invalid()),
        _ => (),
    }
    Ok(url.to_string())
}

fn public_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => {
            !ip.is_private()
                && !ip.is_loopback()
                && !ip.is_link_local()
                && !ip.is_unspecified()
                && !ip.is_broadcast()
                && !ip.is_multicast()
                && ip.octets()[0] != 0
        }
        IpAddr::V6(ip) => ip
            .to_ipv4_mapped()
            .map(|ip| public_ip(IpAddr::V4(ip)))
            .unwrap_or(
                !ip.is_loopback()
                    && !ip.is_unspecified()
                    && !ip.is_unicast_link_local()
                    && !ip.is_unique_local()
                    && !ip.is_multicast(),
            ),
    }
}

fn text(value: &Value, field: &str) -> Option<String> {
    value
        .get(field)?
        .as_str()
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(str::to_owned)
}

fn identifier(value: &Value, field: &str) -> Option<String> {
    text(value, field).or_else(|| value.get(field)?.as_u64().map(|id| id.to_string()))
}

fn number(value: &Value, field: &str) -> Option<f64> {
    value
        .get(field)?
        .as_f64()
        .filter(|number| number.is_finite() && *number >= 0.0 && *number <= 9_007_199_254_740_991.0)
}

fn integer(value: &Value, field: &str) -> Option<u32> {
    number(value, field)
        .filter(|n| n.fract() == 0.0 && *n <= u32::MAX as f64)
        .map(|n| n as u32)
}

fn image_url(value: &Value, field: &str) -> Option<String> {
    let text = text(value, field)?;
    let url = Url::parse(&text).ok()?;
    (matches!(url.scheme(), "http" | "https")
        && url.host().is_some()
        && url.username().is_empty()
        && url.password().is_none())
    .then(|| url.to_string())
}

fn codec(value: &Value, field: &str) -> (Option<String>, Option<bool>) {
    match text(value, field) {
        Some(codec) if codec.eq_ignore_ascii_case("none") => (None, Some(false)),
        Some(codec) => (Some(codec), Some(true)),
        None => (None, None),
    }
}

fn normalize_format(format: &Value) -> Option<MediaFormat> {
    let format_id = identifier(format, "format_id")?;
    if format.get("has_drm").and_then(Value::as_bool) == Some(true)
        || text(format, "protocol").as_deref() == Some("mhtml")
        || text(format, "vcodec").as_deref() == Some("images")
    {
        return None;
    }
    let (audio_codec, has_audio) = codec(format, "acodec");
    let (video_codec, has_video) = codec(format, "vcodec");
    Some(MediaFormat {
        format_id,
        extension: text(format, "ext"),
        format_note: text(format, "format_note"),
        audio_codec,
        video_codec,
        has_audio,
        has_video,
        width: integer(format, "width"),
        height: integer(format, "height"),
        fps: number(format, "fps"),
        audio_bitrate: number(format, "abr"),
        video_bitrate: number(format, "vbr"),
        total_bitrate: number(format, "tbr"),
        sample_rate: integer(format, "asr"),
        file_size: number(format, "filesize"),
        file_size_approx: number(format, "filesize_approx"),
    })
}

fn normalize_metadata(bytes: &[u8]) -> Result<MediaInfo, ExtractionError> {
    let invalid = || {
        ExtractionError::new(
            ExtractionErrorCode::InvalidMetadata,
            "yt-dlp returned invalid media metadata. Update yt-dlp and try again.",
        )
    };
    let value: Value = serde_json::from_slice(bytes).map_err(|_| invalid())?;
    if !value.is_object() {
        return Err(invalid());
    }
    if value.get("entries").is_some_and(|v| !v.is_null())
        || text(&value, "_type").is_some_and(|kind| kind != "video")
    {
        return Err(ExtractionError::new(ExtractionErrorCode::UnsupportedContent, "Playlists, multi-item media and unresolved URLs are not supported. Enter a single public media URL."));
    }
    if text(&value, "availability").is_some_and(|availability| {
        matches!(
            availability.as_str(),
            "private" | "premium_only" | "subscriber_only" | "needs_auth"
        )
    }) {
        return Err(ExtractionError::new(ExtractionErrorCode::AuthenticationRequired, "This media requires authentication. Choose a public media URL; login and cookies are not supported."));
    }
    let id = identifier(&value, "id").ok_or_else(invalid)?;
    let mut formats = Vec::new();
    let mut seen = HashSet::new();
    if let Some(raw) = value.get("formats").filter(|value| !value.is_null()) {
        for format in raw.as_array().ok_or_else(invalid)? {
            if let Some(format) = normalize_format(format) {
                if seen.insert(format.format_id.clone()) {
                    formats.push(format);
                }
            }
        }
    }
    // Generic/direct media extractors may supply one format at the top level.
    if value.get("formats").is_none_or(Value::is_null) {
        if let Some(format) = normalize_format(&value) {
            formats.push(format);
        }
    }
    let thumbnail = image_url(&value, "thumbnail").or_else(|| {
        value
            .get("thumbnails")?
            .as_array()?
            .iter()
            .filter_map(|thumbnail| {
                image_url(thumbnail, "url")
                    .map(|url| (integer(thumbnail, "width").unwrap_or(0), url))
            })
            .max_by_key(|(width, _)| *width)
            .map(|(_, url)| url)
    });
    Ok(MediaInfo {
        id,
        title: text(&value, "title"),
        duration: number(&value, "duration"),
        thumbnail,
        uploader: text(&value, "uploader").or_else(|| text(&value, "channel")),
        formats,
    })
}
