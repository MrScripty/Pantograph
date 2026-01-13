<!-- Text input field component template -->
<script lang="ts">
  interface Props {
    value?: string;
    placeholder?: string;
    label?: string;
    type?: 'text' | 'email' | 'password' | 'number';
    disabled?: boolean;
    error?: string;
    oninput?: (value: string) => void;
  }

  let {
    value = $bindable(''),
    placeholder = '',
    label,
    type = 'text',
    disabled = false,
    error,
    oninput,
  }: Props = $props();

  const handleInput = (e: Event) => {
    const target = e.target as HTMLInputElement;
    value = target.value;
    oninput?.(value);
  };
</script>

<div class="w-full">
  {#if label}
    <label class="block text-sm font-medium text-neutral-300 mb-1.5">
      {label}
    </label>
  {/if}

  <input
    {type}
    {value}
    {placeholder}
    {disabled}
    oninput={handleInput}
    class="w-full px-4 py-2.5 bg-neutral-800 border rounded-lg text-neutral-100 placeholder:text-neutral-500 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent disabled:opacity-50 disabled:cursor-not-allowed transition-colors
      {error ? 'border-red-500' : 'border-neutral-700 hover:border-neutral-600'}"
  />

  {#if error}
    <p class="mt-1.5 text-sm text-red-400">{error}</p>
  {/if}
</div>
