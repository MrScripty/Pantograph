<script lang="ts">
  interface Props {
    /** First-step label, for example "Cancel run". */
    label: string;
    /** Second-step label, for example "Confirm cancel". */
    confirmLabel: string;
    onconfirm: () => void;
    disabled?: boolean;
    /** Why the action is unavailable; shown as the title when disabled. */
    disabledReason?: string;
    small?: boolean;
    /** Milliseconds before an armed button returns to its first step. */
    resetMs?: number;
    class?: string;
  }

  let {
    label,
    confirmLabel,
    onconfirm,
    disabled = false,
    disabledReason,
    small = true,
    resetMs = 4000,
    class: className = '',
  }: Props = $props();

  let armed = $state(false);

  // The reset timer lives exactly as long as the armed state: disarming,
  // disabling or unmounting clears it.
  $effect(() => {
    if (!armed || disabled) return;
    const timer = setTimeout(() => {
      armed = false;
    }, resetMs);
    return () => clearTimeout(timer);
  });

  $effect(() => {
    if (disabled) armed = false;
  });

  function handleClick(): void {
    if (disabled) return;
    if (!armed) {
      armed = true;
      return;
    }
    armed = false;
    onconfirm();
  }
</script>

<button
  type="button"
  class="pg-btn {armed ? 'pg-btn-confirm' : 'pg-btn-danger'} {className}"
  class:pg-btn-sm={small}
  aria-disabled={disabled ? 'true' : undefined}
  title={disabled ? disabledReason : undefined}
  onclick={handleClick}
>
  {armed ? confirmLabel : label}
</button>
