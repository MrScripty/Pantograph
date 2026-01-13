<!-- Basic button component template -->
<script lang="ts">
  interface Props {
    label?: string;
    variant?: 'primary' | 'secondary' | 'danger';
    size?: 'sm' | 'md' | 'lg';
    disabled?: boolean;
    onclick?: () => void;
  }

  let {
    label = 'Button',
    variant = 'primary',
    size = 'md',
    disabled = false,
    onclick,
  }: Props = $props();

  const baseClasses = 'font-medium rounded-lg transition-colors focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-offset-neutral-900';

  const sizeClasses = {
    sm: 'px-3 py-1.5 text-sm',
    md: 'px-4 py-2 text-base',
    lg: 'px-6 py-3 text-lg',
  };

  const variantClasses = {
    primary: 'bg-blue-600 text-white hover:bg-blue-700 focus:ring-blue-500 disabled:bg-blue-600/50',
    secondary: 'bg-neutral-700 text-neutral-100 hover:bg-neutral-600 focus:ring-neutral-500 disabled:bg-neutral-700/50',
    danger: 'bg-red-600 text-white hover:bg-red-700 focus:ring-red-500 disabled:bg-red-600/50',
  };
</script>

<button
  class="{baseClasses} {sizeClasses[size]} {variantClasses[variant]}"
  {disabled}
  onclick={onclick}
>
  {label}
</button>
