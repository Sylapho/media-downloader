<!-- Adapted from shadcn-svelte's MIT-licensed native Button. Only used variants are retained. -->
<script lang="ts">
  import type { HTMLButtonAttributes } from "svelte/elements";
  import { cn } from "$lib/utils";

  let {
    ref = $bindable(null),
    class: className,
    variant = "primary",
    type = "button",
    children,
    ...rest
  }: HTMLButtonAttributes & {
    ref?: HTMLButtonElement | null;
    variant?: "primary" | "secondary" | "ghost";
  } = $props();

  const variants = {
    primary: "bg-primary text-primary-foreground hover:bg-primary/90",
    secondary: "border border-border bg-secondary hover:bg-secondary/70",
    ghost:
      "bg-transparent text-muted-foreground hover:bg-secondary hover:text-foreground",
  };
</script>

<button
  bind:this={ref}
  data-slot="button"
  class={cn(
    "inline-flex min-h-10 shrink-0 items-center justify-center gap-2 rounded-lg px-4 py-2 text-sm font-medium disabled:cursor-not-allowed disabled:opacity-50",
    variants[variant],
    className,
  )}
  {type}
  {...rest}
>
  {@render children?.()}
</button>
