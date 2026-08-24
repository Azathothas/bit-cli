<script>
  import { cn } from '$lib/utils.js';

  let {
    variant = 'default',
    size = 'default',
    class: className = '',
    onclick = undefined,
    disabled = false,
    type = 'button',
    children,
    ...props
  } = $props();

  const variants = {
    default:
      'bg-primary text-primary-foreground shadow-lg shadow-primary/25 hover:bg-primary/90 hover:shadow-xl hover:shadow-primary/30 hover:-translate-y-0.5',
    destructive:
      'bg-destructive text-destructive-foreground shadow-lg shadow-destructive/25 hover:bg-destructive/90 hover:shadow-xl hover:shadow-destructive/30 hover:-translate-y-0.5',
    outline:
      'border-2 border-primary/20 bg-background hover:bg-primary/5 hover:border-primary/40 hover:-translate-y-0.5',
    secondary:
      'bg-secondary text-secondary-foreground hover:bg-secondary/80 hover:-translate-y-0.5',
    ghost: 'hover:bg-accent hover:text-accent-foreground hover:-translate-y-0.5',
    link: 'text-primary underline-offset-4 hover:underline',
  };

  const sizes = {
    default: 'h-11 px-6 py-2.5 text-sm',
    sm: 'h-9 px-3 py-2 text-xs',
    lg: 'h-12 px-8 py-3 text-base',
    icon: 'h-10 w-10',
  };
</script>

<button
  {type}
  class={cn(
    'inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-lg font-semibold ring-offset-background transition-all duration-200 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:pointer-events-none disabled:opacity-50 active:scale-95',
    variants[variant],
    sizes[size],
    className
  )}
  {disabled}
  {onclick}
  {...props}
>
  {@render children?.()}
</button>
