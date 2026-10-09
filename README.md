# Media Downloader

A lightweight, open-source desktop application for downloading and managing media, built with **Tauri 2, Rust, Svelte 5, SvelteKit and TypeScript**.

> [!IMPORTANT]
> **Early development (v0.1.0).** The desktop shell analyzes public media URLs through Rust and displays metadata and source formats. Settings include local tool checks. Downloads and processing are planned.

## Goals

- Provide a clean, responsive desktop interface for media downloads.
- Keep downloads, process management, and filesystem operations in Rust.
- Use **yt-dlp** for media extraction and downloads and **FFmpeg** for media processing.
- Keep idle RAM and CPU usage low; avoid unnecessary background processes.
- Start with Windows, then work toward Linux and macOS support.
- Design for maintainability, testability, accessibility, and secure process execution.

Only download content you are authorized to access, and respect applicable laws and platform terms.

## Technology stack

| Layer                      | Technology             |
| -------------------------- | ---------------------- |
| Desktop application        | Tauri 2                |
| Backend                    | Rust                   |
| Frontend                   | Svelte 5 + SvelteKit 2 |
| Frontend language          | TypeScript             |
| Frontend tooling           | Vite + pnpm            |
| Media extraction           | yt-dlp                 |
| Media processing (planned) | FFmpeg                 |
| Asynchronous operations    | Tokio                  |
| Styling                    | Tailwind CSS 4         |

## Project status and roadmap

The application shell runs locally; the downloader features are being developed incrementally.

- [ ] [CORE-001: Configure project tooling and quality checks](https://github.com/Sylapho/media-downloader/issues/1)
- [ ] [CORE-002: Detect yt-dlp and FFmpeg dependencies from Rust](https://github.com/Sylapho/media-downloader/issues/2)
- [ ] [CORE-003: Extract media metadata with yt-dlp](https://github.com/Sylapho/media-downloader/issues/3)
- [ ] [UI-001: Build the application's main layout](https://github.com/Sylapho/media-downloader/issues/4)
- [ ] [UI-002: Connect URL input and media preview to Rust extraction](https://github.com/Sylapho/media-downloader/issues/5)

Later milestones include individual downloads, format selection, live progress, cancellation, download queues, history, packaging of external tools, and native installers. See [GitHub Issues](https://github.com/Sylapho/media-downloader/issues) for progress.

## Development

### Prerequisites

- [Node.js](https://nodejs.org/) **24.x** and [pnpm](https://pnpm.io/) **10.33.0** (pinned in `package.json`; install with `npm install --global pnpm@10.33.0`)
- [Rust](https://rustup.rs/) stable with the appropriate target toolchain, plus `rustup component add rustfmt clippy`
- [Tauri 2 system prerequisites](https://v2.tauri.app/start/prerequisites/) for your operating system

On **Windows**, install the Microsoft C++ Build Tools with **Desktop development with C++**, including MSVC and the Windows SDK. The Microsoft WebView2 runtime is also required.

Use the `x86_64-pc-windows-msvc` Rust toolchain on Windows. On Linux or macOS, install the native dependencies listed in the linked Tauri prerequisites before running Cargo or Tauri commands.

**Note:** Missing media tools do not prevent the application from starting. Dependency checks use version-only commands; metadata extraction requires yt-dlp. The app does not install or download external binaries automatically.

### Local media dependencies

Install [yt-dlp](https://github.com/yt-dlp/yt-dlp#installation) and a [FFmpeg package](https://ffmpeg.org/download.html) containing both `ffmpeg` and `ffprobe`. On Windows, use the native `.exe` binaries. Add their executable folders to `PATH`, then restart the app (and the terminal used to launch it) so it inherits the new environment. Open **Settings**: the **Media dependencies** section displays each tool's availability, detected version and any corrective guidance. **Check again** refreshes the results.

The initial supported-version policy is **yt-dlp ≥ 2024.01.01**, **FFmpeg ≥ 6.0** and **ffprobe ≥ 6.0**. Prefer current releases. Numbered FFmpeg releases with vendor suffixes (for example `8.0.1-essentials_build`) and yt-dlp date versions with numeric nightly suffixes are recognized. Unnumbered FFmpeg Git snapshots cannot establish the minimum supported version and are reported unavailable. This check verifies executability and version, not future compatibility with every media site or codec.

To override `PATH` lookup, set any of these environment variables to the **absolute executable file path**, not its containing directory:

| Variable                        | Tool    |
| ------------------------------- | ------- |
| `MEDIA_DOWNLOADER_YT_DLP_PATH`  | yt-dlp  |
| `MEDIA_DOWNLOADER_FFMPEG_PATH`  | ffmpeg  |
| `MEDIA_DOWNLOADER_FFPROBE_PATH` | ffprobe |

Example in PowerShell, before `pnpm tauri dev`:

```powershell
$env:MEDIA_DOWNLOADER_YT_DLP_PATH = 'C:\Media Tools\yt-dlp.exe'
$env:MEDIA_DOWNLOADER_FFMPEG_PATH = 'C:\Media Tools\ffmpeg.exe'
$env:MEDIA_DOWNLOADER_FFPROBE_PATH = 'C:\Media Tools\ffprobe.exe'
pnpm tauri dev
```

Spaces are supported; the quotes above are PowerShell syntax and must not be part of the variable's value. An invalid configured path reports an error without silently falling back to another executable. Empty or relative configured paths and `.bat`/`.cmd` scripts are rejected. With no override, lookup uses absolute `PATH` directories and native executable names; relative/empty PATH entries and implicit current-directory lookup are excluded.

Rust runs checks concurrently through `tokio::process::Command`, with fixed version arguments, no shell and a five-second timeout per process. Output is limited to 16 KiB per stream; timed-out or cancelled processes are terminated. The frontend only invokes the typed `check_dependencies` Tauri command. Executable resolution and the process runner are separate interfaces so bundled sidecars can be added later without changing the status contract.

### Media metadata API

Rust's dedicated `YtDlpExtractor` service exposes the asynchronous Tauri command `get_media_info({ url })`. The typed frontend adapter is [`src/lib/media-info.ts`](src/lib/media-info.ts):

```typescript
import { getMediaInfo } from "$lib/media-info";

const info = await getMediaInfo("https://www.youtube.com/watch?v=jNQXAC9IVRw");
console.log(info.title, info.duration, info.formats);
```

In **New media**, paste a public URL and click **Preview media** or press Enter. The shell invokes this command and displays title, uploader, duration, thumbnail and source formats. It never starts a download from the preview.

`MediaInfo` contains `id`, `title`, `duration`, `thumbnail`, `uploader` and `formats`. Serialized fields use camelCase in both Rust and TypeScript. Optional/unknown metadata is `null`; `formats` is always an array. Each format contains `formatId`, `extension`, `formatNote`, codecs, `hasAudio`/`hasVideo`, dimensions, frame rate, bitrates, sample rate and exact/estimated file sizes. Duration is in seconds, bitrates in kbps, sample rate in Hz and sizes in bytes. A codec explicitly marked absent produces `hasAudio` or `hasVideo: false`; missing codec information produces `null`. Storyboards, known DRM formats and duplicate IDs are omitted. Raw stream URLs, headers and cookies are not returned.

Input must be one HTTP(S) URL of at most 4,096 bytes, without credentials, control characters or literal local/private addresses. This initial URL screening does not control yt-dlp's subsequent DNS resolution or redirects. Executable paths and arguments cannot be supplied through IPC.

The adapter passes separate arguments, including `--no-playlist --dump-single-json --simulate --skip-download`. It ignores user configuration and plugin directories, disables the disk cache, and limits extraction to one item. It never downloads media or writes metadata files. The process has a 30-second deadline, a 4 MiB JSON limit and a 64 KiB diagnostic limit. Timeouts, output-limit errors and cancellation terminate the process. FFmpeg/ffprobe are not prerequisites for this metadata operation.

Failures reject the IPC promise with a typed `{ code, message }` object. `invalid_url`, `unsupported_content`, `timeout` and `missing_yt_dlp` are distinct. Additional codes cover executable configuration/permissions, authentication requirements, JavaScript runtime requirements, network failures, excessive output, invalid metadata and other extraction failures. Messages provide corrective guidance without exposing raw tool diagnostics. Cookies, login and playlist extraction are outside this API.

Use a current yt-dlp release: passing the dependency version check does not guarantee site compatibility. YouTube extraction requires the EJS challenge solver and a supported JavaScript runtime. Official yt-dlp executables include EJS; Python installations should use `yt-dlp[default]`, while third-party packages may need additional setup. Deno 2.3+ is enabled by yt-dlp by default; Node 22+ requires explicit activation. This adapter enables Node with `--js-runtimes node`, so the project's Node 24 prerequisite can serve as the runtime when available on the application's `PATH`. It does not enable remote component downloads. See the official [yt-dlp EJS installation guide](https://github.com/yt-dlp/yt-dlp/wiki/EJS).

### Run locally

```bash
git clone https://github.com/Sylapho/media-downloader.git
cd media-downloader
pnpm install --frozen-lockfile
pnpm tauri dev
```

The first Rust build may take a while.

### Frontend checks and builds

```bash
pnpm format:check
pnpm lint
pnpm check
pnpm test
pnpm build
```

`pnpm format` applies Prettier formatting, including Svelte components. `pnpm lint` runs ESLint for JavaScript, TypeScript and Svelte with zero warnings allowed. `pnpm check` synchronizes SvelteKit and checks Svelte and TypeScript with strict mode enabled. `pnpm test` runs Vitest once; `pnpm test:watch` and `pnpm check:watch` are available while developing.

The frontend tests render the desktop shell, media preview and dependency view in jsdom and mock only the Tauri IPC boundary. They check navigation, focus transfer, URL persistence/validation, metadata and format rendering, nullable fields, broken thumbnails, loading, error guidance, retries, concurrent-submit prevention and stale-result/error suppression. Metadata tests use the same normalized JSON fixture as Rust. They do not require yt-dlp, network access or a running desktop application.

### Desktop shell and browser checks

The shell uses Svelte 5/TypeScript and Tailwind CSS 4 through its Vite plugin. Semantic color/radius tokens live in `src/app.css`; the initial theme stays dark regardless of the OS preference and can be extended for light mode later. Only the native Button and Input patterns from [shadcn-svelte](https://shadcn-svelte.com/) are adopted, in `src/lib/components/ui`, with `clsx` and `tailwind-merge`. No component suite, icon library, animation package or remote font is loaded. Their source license is included beside the components.

The generated demo logos and favicon are removed. `static/favicon.svg` is the source for the shell mark and the native application icons, generated with Tauri's built-in icon command.

**New media**, **Downloads** and **Settings** are local views. The URL and current preview are preserved when switching views. Analysis runs only on form submission, through the typed `get_media_info` command; validation and request state live outside the visual components in `media-preview.svelte.ts`. Basic URL validation happens locally and Rust remains authoritative for supported/public URLs. Errors are translated from known codes into guidance; raw IPC/process messages are never rendered.

Only one extraction can be active. The input and navigation remain usable while it loads. Editing the URL immediately clears the old preview and invalidates the active request, even if the user later returns to the same URL. Its late success/error is ignored; after the current check finishes (Rust enforces a 30-second maximum), submit the new URL. Repeated clicks/Enter cannot launch concurrent extractions. There is no automatic analysis on typing or polling.

Unknown fields remain explicit (untitled media, unknown uploader/duration/streams, unavailable thumbnail or no format information). A failed thumbnail falls back without losing metadata; only HTTP(S) thumbnail URLs without credentials are loaded, with no referrer. Format labels distinguish audio-only, video-only, combined and unknown streams and show the source container/codecs. Exact and estimated file sizes are distinguished. They do not promise downloadable MP3/MP4 output, merging or conversion. Download controls remain unavailable.

Opening Settings runs one local dependency check; rechecks are manual. Its IPC/state controller lives outside the visual component in `dependency-check.svelte.ts`. There are no app polling loops, timers or idle animations.

To run the browser checks against a fresh production build:

```bash
pnpm exec playwright install chromium --only-shell
pnpm test:ui
```

Playwright is a development-only dependency. The tests cover six viewport sizes (800×600, 640×480, 1024×768, 480×640, 320×640 and 400×300), all three views, populated metadata/formats, long version strings, Enter submission, loading/editing, stale results, setup errors, keyboard navigation, the skip link, visible focus, a consistent dark theme and idle network/IPC behavior. The smaller layouts and populated previews scroll vertically when needed; controls and content must not overflow horizontally. Tests launch their own preview server on `127.0.0.1:4173`, mock the Tauri IPC boundary and intercept fixture thumbnails. Workspace and populated-preview screenshots are saved under ignored `test-results/`; failed checks retain screenshots and traces there.

### Rust checks

```bash
cd src-tauri
cargo fmt --all -- --check
cargo check --locked
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
```

Run `cargo fmt --all` in `src-tauri` to apply Rust formatting. Rust tests cover `greet`, version parsing, configured/PATH resolution, structured errors, registered Tauri IPC commands and the mockable runner. Metadata fixtures cover audio-only, video-only, combined, direct and unknown formats, malformed responses and unsupported playlists. A small native fixture is compiled with the installed Rust compiler to exercise paths containing spaces, literal URL arguments, large JSON, broken/missing executables, failed exits, output limits, timeouts and cancellation. The default suite needs no network or installed media tools.

On Windows MSVC, `build.rs` embeds the Common Controls v6 manifest in the application and test targets so the Tauri IPC test harness loads correctly. This preserves Tauri's default manifest dependency and avoids [the known manifest-less test loading failure](https://github.com/tauri-apps/tauri/issues/13419).

If FFmpeg and ffprobe are already installed locally, run the optional real-tool smoke test from `src-tauri`:

```bash
cargo test --locked locally_installed_ffmpeg_and_ffprobe_are_detected -- --ignored --nocapture
```

For an optional real yt-dlp integration check, choose a public single-media URL you are authorized to access. With yt-dlp and its runtime installed, run from the repository root in PowerShell:

```powershell
$env:MEDIA_DOWNLOADER_YT_DLP_PATH = 'C:\Media Tools\yt-dlp.exe' # optional when on PATH
$env:MEDIA_DOWNLOADER_TEST_URL = 'https://www.youtube.com/watch?v=jNQXAC9IVRw'
cargo test --manifest-path src-tauri/Cargo.toml --locked authorized_public_url_integration -- --ignored --nocapture
```

On Unix, export the same environment variables with your local executable path. This check asserts a nonempty media ID and format list and prints a metadata summary. It performs no media download. Both real-tool checks are ignored by the default test suite and CI; fixture tests remain independent of network access and media-site availability.

With the same authorized URL configured, the optional `authorized_public_url_ipc_integration` test exercises the actual registered `get_media_info` command through Tauri's IPC test harness and checks the serialized preview contract:

```bash
cargo test --manifest-path src-tauri/Cargo.toml --locked authorized_public_url_ipc_integration -- --ignored --nocapture
```

This real-tool check is also ignored in the default suite and CI and downloads no media.

### Desktop build and smoke check

From the repository root:

```bash
pnpm tauri build --debug --no-bundle -- --locked
pnpm tauri dev
```

The first command compiles the frontend and desktop executable without generating installers. In the development window, paste an authorized public URL and press Enter or click **Preview media**. Check its title/uploader/duration/thumbnail and source format labels. During a request, edit the URL and verify that the previous response is ignored, then analyze the new URL after the request ends. Check a malformed URL and retry after any error. Switch to Downloads, then Settings and verify the three tool statuses and **Check again**. Navigate with Tab/Shift+Tab and Enter, verify focus and the skip link, and resize the window: the layout should reflow without horizontal clipping. Downloads remain unavailable. Close the application and stop the development command when finished.

### Continuous integration

[`.github/workflows/ci.yml`](.github/workflows/ci.yml) runs on pull requests and pushes to `main`. It installs dependencies from `pnpm-lock.yaml`, checks frontend formatting, lint and types, runs Vitest, builds the frontend, installs Chromium's headless shell and runs the layout tests, checks Rust formatting, runs Cargo check, Clippy and tests with `Cargo.lock`, and compiles the desktop application without installers.

The workflow targets `windows-latest`, which supplies Visual Studio/MSVC and the Windows SDK. It uses Node 24, the pinned pnpm version and stable Rust with rustfmt/Clippy. Tests do not open a WebView; interactive local development requires WebView2. Linux and macOS runners are not configured by this ticket and would need their Tauri native prerequisites.

Keep `pnpm-lock.yaml` and `src-tauri/Cargo.lock` under version control. Generated frontend output (`.svelte-kit`, `build`, `package`, `coverage`), browser test artifacts (`test-results`, `playwright-report`), dependencies, Cargo `target` and Tauri generated schemas are ignored. Update lockfiles intentionally when changing dependencies; CI uses frozen/locked installations.

## Architecture direction

```text
Svelte 5 / TypeScript
        |
        | Tauri commands and events
        v
    Tauri 2 / Rust
        |
        +-- Media extraction / download services
        |       +-- yt-dlp (external process)
        |       +-- FFmpeg (external process)
        |
        +-- Settings and download state
```

The frontend will never be given a generic shell-execution API. Rust will validate input and invoke external tools with explicit arguments, bounded lifetimes, and structured errors.

## Contributing

Contributions and bug reports are welcome through [GitHub Issues](https://github.com/Sylapho/media-downloader/issues). For now, keep changes scoped to a single issue and explain how you validated them.

## License

The package manifest currently declares **MIT**, but verify that a corresponding license file is present before treating the repository as formally licensed for redistribution. The licenses of yt-dlp, FFmpeg, and any bundled third-party components must also be respected.

## Inspiration

Inspired by [Logidownload by La Digitale](https://codeberg.org/ladigitale/logidownload). Media Downloader is an independent project and is not affiliated with La Digitale.
