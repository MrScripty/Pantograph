<script lang="ts">
  /*
   * Title for a typed node's BaseNode header snippet. BaseNode draws the
   * status glyph to its left; this keeps the title in the same token type as
   * BaseNode's default header and holds the 20px line the header height
   * (37px, so handles sit at 52px + 20px per port) depends on.
   */
  interface Props {
    label: string;
  }

  let { label }: Props = $props();
</script>

<span class="node-header-title" title={label}>{label}</span>

<style>
  .node-header-title {
    display: block;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 12.5px;
    line-height: 20px;
    font-weight: 700;
    color: var(--pg-text-bright);
  }
</style>
