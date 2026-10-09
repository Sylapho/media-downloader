<script lang="ts">
  import type { MediaInfo } from "$lib/media-info";
  import type { PreviewError } from "$lib/media-preview.svelte";
  import Button from "./ui/button/Button.svelte";
  import Icon from "./Icon.svelte";
  import MediaDetails from "./MediaDetails.svelte";

  let {
    media,
    pending,
    error,
    changedWhilePending,
    onsettings,
  }: {
    media: MediaInfo | null;
    pending: boolean;
    error: PreviewError | null;
    changedWhilePending: boolean;
    onsettings: () => void;
  } = $props();
</script>

<section
  aria-labelledby="preview-heading"
  class="mt-5 min-w-0 rounded-xl border bg-card"
>
  <div
    class="flex flex-wrap items-center justify-between gap-2 border-b px-5 py-3"
  >
    <h2 id="preview-heading" class="font-medium">Media preview</h2>
    <span class="text-xs text-muted-foreground"
      >{pending
        ? "Analyzing…"
        : media
          ? "Metadata ready"
          : error
            ? "Needs attention"
            : "Waiting for a link"}</span
    >
  </div>
  <div aria-live="polite">
    {#if pending}
      <div role="status" class="px-5 py-8 text-center">
        <Icon name="link" class="mx-auto mb-3 size-6 text-primary" />
        <p class="font-medium">
          {changedWhilePending
            ? "URL changed during analysis"
            : "Analyzing media…"}
        </p>
        <p
          class="mx-auto mt-2 max-w-sm text-sm leading-relaxed text-muted-foreground"
        >
          {changedWhilePending
            ? "Wait for the current check to finish, then analyze the new URL. The previous result will be ignored."
            : "Fetching metadata only. You can keep editing the URL or browse the workspace. This can take up to 30 seconds."}
        </p>
      </div>
    {:else if error}
      <div class="p-5">
        <div
          id="preview-error"
          role="alert"
          class="rounded-lg border border-destructive/40 bg-destructive/5 p-4"
        >
          <h3 class="flex items-center gap-2 font-medium text-destructive">
            <Icon name="alert" class="size-4 shrink-0" />{error.title}
          </h3>
          <p class="mt-2 text-sm leading-relaxed">{error.message}</p>
        </div>
        {#if ["missing_yt_dlp", "invalid_configuration", "permission_denied", "runtime_required"].includes(error.code)}
          <Button variant="secondary" class="mt-4" onclick={onsettings}
            >Open settings</Button
          >
        {/if}
      </div>
    {:else if media}
      {#key media}<MediaDetails {media} />{/key}
    {:else}
      <div class="flex flex-col items-center px-5 py-5 text-center">
        <span
          class="mb-3 flex size-11 items-center justify-center rounded-xl border bg-secondary text-muted-foreground"
          ><Icon name="play" class="size-6" /></span
        >
        <h3 class="font-medium">Your media starts here</h3>
        <p class="mt-2 max-w-sm text-sm leading-relaxed text-muted-foreground">
          Paste a public media URL above and select Preview media to see its
          metadata and source formats.
        </p>
      </div>
    {/if}
  </div>
</section>
