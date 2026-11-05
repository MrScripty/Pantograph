<script lang="ts">
  import { SegmentedControl, type SegmentOption } from '../workbench/ui';

  type Props = {
    connectionType: 'external' | 'sidecar';
  };

  let { connectionType = $bindable() }: Props = $props();

  const CONNECTION_OPTIONS: SegmentOption<'external' | 'sidecar'>[] = [
    { value: 'external', label: 'External' },
    { value: 'sidecar', label: 'Sidecar' },
  ];
</script>

<SegmentedControl
  label="Connection mode"
  options={CONNECTION_OPTIONS}
  value={connectionType}
  onchange={(next) => (connectionType = next)}
/>
