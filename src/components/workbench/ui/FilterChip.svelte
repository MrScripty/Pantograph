<script lang="ts">
  interface Props {
    /** Applied filter in words, for example "Device cuda:0". */
    label: string;
    onclear: () => void;
  }

  let { label, onclear }: Props = $props();
</script>

<span class="pg-fchip">
  {label}
  <button type="button" aria-label={`Clear ${label}`} title={`Clear ${label}`} onclick={onclear}>×</button>
</span>
