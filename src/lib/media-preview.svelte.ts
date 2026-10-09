import {
  getMediaInfo,
  type ExtractionErrorCode,
  type MediaInfo,
} from "./media-info";

export interface PreviewError {
  code: ExtractionErrorCode | "ipc_unavailable";
  title: string;
  message: string;
}

const errors: Record<PreviewError["code"], Omit<PreviewError, "code">> = {
  invalid_url: {
    title: "Check the media URL",
    message:
      "Enter one public HTTP or HTTPS media URL without credentials or local/private addresses.",
  },
  missing_yt_dlp: {
    title: "yt-dlp is missing",
    message:
      "Install a current yt-dlp release, then open Settings to check its availability and try again.",
  },
  invalid_configuration: {
    title: "Check your yt-dlp setup",
    message:
      "Open Settings and check the configured executable path. It must point to a native yt-dlp executable.",
  },
  permission_denied: {
    title: "yt-dlp cannot run",
    message:
      "Check executable permissions and your yt-dlp setup in Settings, then try again.",
  },
  timeout: {
    title: "Analysis timed out",
    message:
      "The metadata request took too long. Check your connection and try again, or choose another public media URL.",
  },
  output_limit: {
    title: "Too much metadata returned",
    message: "Choose a single media URL and update yt-dlp before trying again.",
  },
  unsupported_content: {
    title: "Media unavailable",
    message:
      "This media or URL is unsupported or unavailable. Check the link and choose a public single media item.",
  },
  authentication_required: {
    title: "This media requires access",
    message:
      "Choose a public media URL. Login, cookies and private media are not supported.",
  },
  runtime_required: {
    title: "Additional runtime required",
    message:
      "Update yt-dlp and install its EJS solver and a supported JavaScript runtime. See the setup guide in the README.",
  },
  network_error: {
    title: "Could not reach the media",
    message:
      "Check your connection and try again. The media site may be temporarily unavailable.",
  },
  extraction_failed: {
    title: "Could not analyze this media",
    message:
      "Check that the link is public and supported, update yt-dlp, and try again.",
  },
  invalid_metadata: {
    title: "Unexpected media metadata",
    message:
      "The tool returned an unreadable response. Update yt-dlp and try again.",
  },
  ipc_unavailable: {
    title: "Desktop connection unavailable",
    message:
      "Could not contact the desktop backend. Open the Tauri desktop app and try again.",
  },
};

function presentError(value: unknown): PreviewError {
  const code =
    typeof value === "object" &&
    value !== null &&
    "code" in value &&
    typeof value.code === "string" &&
    Object.hasOwn(errors, value.code)
      ? (value.code as PreviewError["code"])
      : "ipc_unavailable";
  // Only known codes cross into the view. Never render raw IPC/process messages.
  return { code, ...errors[code] };
}

function validateInput(input: string): string | null {
  if (
    new TextEncoder().encode(input).length > 4096 ||
    [...input].some(
      (character) =>
        character.charCodeAt(0) < 32 || character.charCodeAt(0) === 127,
    )
  )
    return null;
  const trimmed = input.trim();
  if (
    !/^https?:\/\/[^/]/i.test(trimmed) ||
    /\s/.test(trimmed) ||
    trimmed.includes("\\")
  )
    return null;
  try {
    const url = new URL(trimmed);
    return url.hostname && !url.username && !url.password && url.port !== "0"
      ? trimmed
      : null;
  } catch {
    return null;
  }
}

/** Owns validation, IPC and request identity; visual components only render state. */
export function createMediaPreview() {
  let url = $state("");
  let media = $state<MediaInfo | null>(null);
  let error = $state<PreviewError | null>(null);
  let pending = $state(false);
  let revision = $state(0);
  let activeRevision = $state(0);

  return {
    get url() {
      return url;
    },
    set url(value: string) {
      if (value === url) return;
      url = value;
      revision += 1;
      media = null;
      error = null;
    },
    get media() {
      return media;
    },
    get error() {
      return error;
    },
    get pending() {
      return pending;
    },
    get changedWhilePending() {
      return pending && revision !== activeRevision;
    },
    async analyze() {
      if (pending) return;
      media = null;
      error = null;
      const validated = validateInput(url);
      if (!validated) {
        error = { code: "invalid_url", ...errors.invalid_url };
        return;
      }
      const requestRevision = ++revision;
      activeRevision = requestRevision;
      pending = true;
      try {
        const result = await getMediaInfo(validated);
        if (revision === requestRevision) media = result;
      } catch (failure: unknown) {
        if (revision === requestRevision) error = presentError(failure);
      } finally {
        pending = false;
      }
    },
  };
}
