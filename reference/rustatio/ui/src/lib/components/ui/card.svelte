<script>
  import { cn } from '$lib/utils.js';

  let { class: className = '', children, ...props } = $props();
</script>

<div
  class={cn(
    'rounded-lg border border-border/40 bg-card text-card-foreground shadow-sm transition-all duration-200',
    className
  )}
  {...props}
>
  {@render children?.()}
</div>
