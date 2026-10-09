mod dependencies;
mod extractor;
mod process;

#[tauri::command]
async fn get_media_info(url: String) -> Result<extractor::MediaInfo, extractor::ExtractionError> {
    extractor::YtDlpExtractor::new(
        &process::TokioProcessRunner,
        &dependencies::LocalExecutables::from_environment(),
    )
    .extract(&url)
    .await
}

#[tauri::command]
async fn check_dependencies() -> Vec<dependencies::DependencyStatus> {
    dependencies::check_dependencies(
        &process::TokioProcessRunner,
        &dependencies::LocalExecutables::from_environment(),
    )
    .await
}

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    configure_builder(tauri::Builder::default())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn configure_builder<R: tauri::Runtime>(builder: tauri::Builder<R>) -> tauri::Builder<R> {
    builder
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            check_dependencies,
            get_media_info
        ])
}

#[cfg(test)]
mod tests {
    use super::greet;

    #[test]
    fn greets_a_name() {
        assert_eq!(greet("Ada"), "Hello, Ada! You've been greeted from Rust!");
    }

    #[test]
    fn greets_an_empty_name() {
        assert_eq!(greet(""), "Hello, ! You've been greeted from Rust!");
    }

    #[test]
    fn preserves_unicode_names() {
        assert_eq!(
            greet("Élodie 世界"),
            "Hello, Élodie 世界! You've been greeted from Rust!"
        );
    }

    #[test]
    fn dependency_command_is_registered_and_returns_typed_ipc_statuses() {
        let app = super::configure_builder(tauri::test::mock_builder())
            .build(tauri::test::mock_context(tauri::test::noop_assets()))
            .unwrap();
        let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
            .build()
            .unwrap();
        let response = tauri::test::get_ipc_response(
            &webview,
            tauri::webview::InvokeRequest {
                cmd: "check_dependencies".into(),
                callback: tauri::ipc::CallbackFn(0),
                error: tauri::ipc::CallbackFn(1),
                url: if cfg!(windows) {
                    "http://tauri.localhost"
                } else {
                    "tauri://localhost"
                }
                .parse()
                .unwrap(),
                body: tauri::ipc::InvokeBody::default(),
                headers: Default::default(),
                invoke_key: tauri::test::INVOKE_KEY.into(),
            },
        )
        .unwrap()
        .deserialize::<serde_json::Value>()
        .unwrap();
        let statuses = response.as_array().unwrap();
        assert_eq!(statuses.len(), 3);
        for (status, tool) in statuses.iter().zip(["yt-dlp", "ffmpeg", "ffprobe"]) {
            assert_eq!(status["tool"], tool);
            if status["available"].as_bool().unwrap() {
                assert!(status["version"].is_string());
                assert!(status["error"].is_null());
            } else {
                assert!(status["error"]["code"].is_string());
                assert!(status["error"]["message"].is_string());
            }
        }
    }

    #[test]
    #[ignore = "manual metadata-only IPC check: installed yt-dlp and an authorized public URL required"]
    fn authorized_public_url_ipc_integration() {
        let url = std::env::var("MEDIA_DOWNLOADER_TEST_URL")
            .expect("set MEDIA_DOWNLOADER_TEST_URL to an authorized public single-media URL");
        let app = super::configure_builder(tauri::test::mock_builder())
            .build(tauri::test::mock_context(tauri::test::noop_assets()))
            .unwrap();
        let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
            .build()
            .unwrap();
        let response = tauri::test::get_ipc_response(
            &webview,
            tauri::webview::InvokeRequest {
                cmd: "get_media_info".into(),
                callback: tauri::ipc::CallbackFn(0),
                error: tauri::ipc::CallbackFn(1),
                url: if cfg!(windows) {
                    "http://tauri.localhost"
                } else {
                    "tauri://localhost"
                }
                .parse()
                .unwrap(),
                body: tauri::ipc::InvokeBody::Json(serde_json::json!({"url": url})),
                headers: Default::default(),
                invoke_key: tauri::test::INVOKE_KEY.into(),
            },
        )
        .expect("metadata IPC failed")
        .deserialize::<serde_json::Value>()
        .unwrap();
        for key in [
            "id",
            "title",
            "duration",
            "thumbnail",
            "uploader",
            "formats",
        ] {
            assert!(
                response.get(key).is_some(),
                "missing MediaInfo field: {key}"
            );
        }
        assert!(!response["id"].as_str().unwrap().is_empty());
        let formats = response["formats"].as_array().unwrap();
        assert!(!formats.is_empty());
        for format in formats {
            assert!(format["formatId"].is_string());
            assert!(format.get("format_id").is_none());
        }
        println!(
            "IPC metadata: title={}, duration={} seconds, formats={}",
            response["title"],
            response["duration"],
            formats.len()
        );
    }

    #[test]
    fn media_command_is_registered_and_serializes_invalid_url_errors() {
        let app = super::configure_builder(tauri::test::mock_builder())
            .build(tauri::test::mock_context(tauri::test::noop_assets()))
            .unwrap();
        let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
            .build()
            .unwrap();
        let error = tauri::test::get_ipc_response(
            &webview,
            tauri::webview::InvokeRequest {
                cmd: "get_media_info".into(),
                callback: tauri::ipc::CallbackFn(0),
                error: tauri::ipc::CallbackFn(1),
                url: if cfg!(windows) {
                    "http://tauri.localhost"
                } else {
                    "tauri://localhost"
                }
                .parse()
                .unwrap(),
                body: tauri::ipc::InvokeBody::Json(serde_json::json!({"url":"file:///private"})),
                headers: Default::default(),
                invoke_key: tauri::test::INVOKE_KEY.into(),
            },
        )
        .unwrap_err();
        assert_eq!(error["code"], "invalid_url");
        assert!(error["message"].as_str().unwrap().contains("HTTP"));
    }
}
