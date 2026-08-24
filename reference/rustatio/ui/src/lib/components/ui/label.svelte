<script>
  import { cn } from '$lib/utils.js';

  let { class: className = '', for: htmlFor = undefined, children, ...props } = $props();
</script>

<label
  for={htmlFor}
  class={cn(
    'text-sm font-medium leading-none peer-disabled:cursor-not-allowed peer-disabled:opacity-70',
    className
  )}
  {...props}
>
  {@render children?.()}
</label>
