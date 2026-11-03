<script lang="ts">
  interface Props {
    /** One sentence summarising the page, for example "4 runs waiting on 2 devices". */
    label: string;
  }

  let { label }: Props = $props();
</script>

<div class="pg-dimrow">
  <div class="pg-dim">
    <span class="pg-dim-label" title={label}>{label}</span>
  </div>
</div>
