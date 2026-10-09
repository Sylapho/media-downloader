<!-- Adapted from shadcn-svelte's MIT-licensed Input for text/URL entry. -->
<script lang="ts">
  import type { HTMLInputAttributes } from "svelte/elements";
  import { cn } from "$lib/utils";

  let {
    ref = $bindable(null),
    value = $bindable(""),
    class: className,
    type = "text",
    ...rest
  }: Omit<HTMLInputAttributes, "value" | "type"> & {
    ref?: HTMLInputElement | null;
    value?: string;
    type?: "text" | "url";
  } = $props();
</script>

<input
  bind:this={ref}
  bind:value
  data-slot="input"
  {type}
  class={cn(
    "min-h-11 w-full min-w-0 rounded-lg border bg-background px-3 py-2 text-sm placeholder:text-muted-foreground disabled:opacity-50",
    className,
  )}
  {...rest}
/>
