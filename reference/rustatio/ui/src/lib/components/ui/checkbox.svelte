<script>
  import { cn } from '$lib/utils.js';

  let {
    class: className = '',
    checked = $bindable(false),
    disabled = false,
    id = undefined,
    onchange = undefined,
    ...props
  } = $props();

  function handleChange(event) {
    checked = event.target.checked;
    if (onchange) {
      onchange(checked);
    }
  }
</script>

<input
  type="checkbox"
  {id}
  {checked}
  onchange={handleChange}
  {disabled}
  class={cn(
    'peer h-4 w-4 shrink-0 rounded-sm border border-primary ring-offset-background focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50 accent-primary',
    className
  )}
  {...props}
/>
