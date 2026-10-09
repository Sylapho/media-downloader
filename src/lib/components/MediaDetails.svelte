<script lang="ts">
  import type { MediaInfo } from "$lib/media-info";
  import {
    durationLabel,
    formatDetails,
    sizeLabel,
    streamLabel,
    thumbnailUrl,
  } from "$lib/media-display";
  import Icon from "./Icon.svelte";

  let { media }: { media: MediaInfo } = $props();
  let imageFailed = $state(false);
  const thumbnail = $derived(thumbnailUrl(media.thumbnail));
</script>

<div class="p-5">
  <div class="grid min-w-0 gap-4 min-[600px]:grid-cols-[160px_minmax(0,1fr)]">
    <div
      class="flex aspect-video items-center justify-center overflow-hidden rounded-lg border bg-background"
    >
      {#if thumbnail && !imageFailed}
        <img
          src={thumbnail}
          alt={`Thumbnail for ${media.title || "this media"}`}
          class="h-full w-full object-cover"
          loading="lazy"
          decoding="async"
          referrerpolicy="no-referrer"
          onerror={() => {
            imageFailed = true;
          }}
        />
      {:else}
        <div
          class="flex flex-col items-center gap-2 p-3 text-center text-xs text-muted-foreground"
        >
          <Icon name="play" /><span>Thumbnail unavailable</span>
        </div>
      {/if}
    </div>
    <div class="min-w-0">
      <h3 class="text-lg leading-snug font-semibold [overflow-wrap:anywhere]">
        {media.title || "Untitled media"}
      </h3>
      <dl class="mt-3 grid gap-2 text-sm">
        <div>
          <dt class="text-xs text-muted-foreground">Uploader</dt>
          <dd class="mt-0.5 [overflow-wrap:anywhere]">
            {media.uploader || "Uploader unknown"}
          </dd>
        </div>
        <div>
          <dt class="text-xs text-muted-foreground">Duration</dt>
          <dd class="mt-0.5">{durationLabel(media.duration)}</dd>
        </div>
      </dl>
    </div>
  </div>

  <div class="mt-6 border-t pt-4">
    <h3 class="font-medium">
      Available source formats <span class="ml-1 text-xs text-muted-foreground"
        >({media.formats.length})</span
      >
    </h3>
    <p class="mt-2 text-xs leading-relaxed text-muted-foreground">
      Containers and streams reported by yt-dlp. Downloading, merging and
      conversion are not available yet.
    </p>
    {#if media.formats.length === 0}
      <p
        class="mt-4 rounded-lg border bg-background p-3 text-sm text-muted-foreground"
      >
        No format information is available for this media.
      </p>
    {:else}
      <table class="mt-4 w-full table-fixed text-left text-xs">
        <caption class="sr-only">Available source formats</caption>
        <thead
          ><tr
            ><th
              scope="col"
              class="w-1/4 pb-3 font-medium text-muted-foreground"
              >Format / container</th
            ><th
              scope="col"
              class="w-1/4 pb-3 font-medium text-muted-foreground">Streams</th
            ><th scope="col" class="pb-3 font-medium text-muted-foreground"
              >Details</th
            ></tr
          ></thead
        >
        <tbody>
          {#each media.formats as format (format.formatId)}
            <tr>
              <th
                scope="row"
                class="border-t py-3 pr-3 align-top font-medium [overflow-wrap:anywhere]"
                >{format.formatId}<span
                  class="mt-1 block font-normal text-muted-foreground"
                  >{format.extension?.toUpperCase() ||
                    "Container unknown"}</span
                ></th
              >
              <td class="border-t py-3 pr-3 align-top [overflow-wrap:anywhere]"
                >{streamLabel(format)}</td
              >
              <td
                class="border-t py-3 align-top leading-relaxed [overflow-wrap:anywhere]"
              >
                <p>{formatDetails(format)}</p>
                {#if format.formatNote}<p class="mt-1 text-muted-foreground">
                    {format.formatNote}
                  </p>{/if}
                <p class="mt-1 font-mono text-muted-foreground">
                  Audio: {format.audioCodec ??
                    (format.hasAudio === false ? "None" : "Unknown")}<br
                  />Video: {format.videoCodec ??
                    (format.hasVideo === false ? "None" : "Unknown")}
                </p>
                <p class="mt-1 text-muted-foreground">{sizeLabel(format)}</p>
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    {/if}
  </div>
</div>
