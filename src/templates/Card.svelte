<!-- Container card component template -->
<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    title?: string;
    subtitle?: string;
    variant?: 'default' | 'elevated' | 'outlined';
    padding?: 'none' | 'sm' | 'md' | 'lg';
    children?: Snippet;
  }

  let {
    title,
    subtitle,
    variant = 'default',
    padding = 'md',
    children,
  }: Props = $props();

  const baseClasses = 'rounded-xl';

  const variantClasses = {
    default: 'bg-neutral-800',
    elevated: 'bg-neutral-800 shadow-xl shadow-black/20',
    outlined: 'bg-transparent border border-neutral-700',
  };

  const paddingClasses = {
    none: '',
    sm: 'p-3',
    md: 'p-4',
    lg: 'p-6',
  };
</script>

<div class="{baseClasses} {variantClasses[variant]} {paddingClasses[padding]}">
  {#if title || subtitle}
    <div class="mb-4">
      {#if title}
        <h3 class="text-lg font-semibold text-neutral-100">{title}</h3>
      {/if}
      {#if subtitle}
        <p class="text-sm text-neutral-400 mt-1">{subtitle}</p>
      {/if}
    </div>
  {/if}

  {#if children}
    {@render children()}
  {/if}
</div>
