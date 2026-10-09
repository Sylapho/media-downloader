# Media Downloader

A lightweight, open-source desktop application for downloading and managing media, built with **Tauri 2, Rust, Svelte 5, SvelteKit and TypeScript**.

> [!IMPORTANT]
> **Early development (v0.1.0).** The repository currently contains the working Tauri + Svelte starter application. Media extraction, downloads, and FFmpeg integration are **planned, not yet implemented**.

## Goals

- Provide a clean, responsive desktop interface for media downloads.
- Keep downloads, process management, and filesystem operations in Rust.
- Use **yt-dlp** for media extraction and downloads and **FFmpeg** for media processing.
- Keep idle RAM and CPU usage low; avoid unnecessary background processes.
- Start with Windows, then work toward Linux and macOS support.
- Design for maintainability, testability, accessibility, and secure process execution.

Only download content you are authorized to access, and respect applicable laws and platform terms.

## Technology stack

| Layer | Technology |
| --- | --- |
| Desktop application | Tauri 2 |
| Backend | Rust |
| Frontend | Svelte 5 + SvelteKit 2 |
| Frontend language | TypeScript |
| Frontend tooling | Vite + pnpm |
| Media extraction (planned) | yt-dlp |
| Media processing (planned) | FFmpeg |
| Asynchronous operations (planned) | Tokio |
| Styling (planned) | Tailwind CSS 4 |

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

- [Node.js](https://nodejs.org/) and [pnpm](https://pnpm.io/)
- [Rust](https://rustup.rs/) with the appropriate target toolchain
- [Tauri 2 system prerequisites](https://v2.tauri.app/start/prerequisites/) for your operating system

On **Windows**, install the Microsoft C++ Build Tools with **Desktop development with C++**, including MSVC and the Windows SDK. The Microsoft WebView2 runtime is also required.

**Note:** yt-dlp and FFmpeg are not used by the current starter app. They will be needed for future media functionality; the project will initially support locally installed executables before bundling them for distribution.

### Run locally

```bash
git clone https://github.com/Sylapho/media-downloader.git
cd media-downloader
pnpm install
pnpm tauri dev
```

The first Rust build may take a while.

### Frontend checks and builds

```bash
pnpm check
pnpm build
```

### Rust checks

```bash
cd src-tauri
cargo fmt --all -- --check
cargo check
cargo test
```

A fuller automated test and CI setup is tracked in [CORE-001](https://github.com/Sylapho/media-downloader/issues/1); not all planned scripts or checks exist yet.

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
