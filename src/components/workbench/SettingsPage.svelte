<script lang="ts">
  import { onMount } from 'svelte';
  import DeviceConfig from '../DeviceConfig.svelte';
  import ModelConfig from '../ModelConfig.svelte';
  import RagStatus from '../RagStatus.svelte';
  import ManagedBinaryOverviewPanel from '../runtime-manager/ManagedBinaryOverviewPanel.svelte';
  import ManagedRuntimePanel from '../runtime-manager/ManagedRuntimePanel.svelte';
  import SandboxSettings from '../SandboxSettings.svelte';
  import ServerStatus from '../ServerStatus.svelte';
  import type {
    DiagnosticsRetentionPolicy,
    WorkflowRetentionCleanupResult,
  } from '../../services/diagnostics/types';
  import type {
    WorkflowArtifactFormatCapabilities,
    WorkflowArtifactFormatSettings,
    WorkflowArtifactPolicy,
    WorkflowManagedMediaDependencyId,
    WorkflowManagedMediaDependencyStatus,
  } from '../../services/workflow/types';
  import { workflowService } from '../../services/workflow/WorkflowService';
  import {
    clearSettingsFocus,
    settingsFocus,
  } from '../../stores/workbenchStore';
  import {
    buildArtifactPolicyRows,
    buildCapabilityRows,
    buildDiagnosticsRetentionCleanupRows,
    buildDiagnosticsRetentionPolicyRows,
    buildDiagnosticsRetentionSettingRows,
    buildManagedMediaDependencyRows,
    findFormatOption,
    formatCapabilitySummary,
    formatManagedMediaDependencyStatus,
    formatOptionItems,
    formatRangeLabel,
    managedMediaVersionOptions,
    managedMediaVersionStatusLabel,
    optionValuesWithCurrent,
    parseNullableIntegerField,
  } from './settingsPagePresenters';
  import { formatWorkflowCommandError } from './workflowErrorPresenters';
  import { Chip, KeyValue, Panel } from './ui';

  interface ArtifactPolicyDraft {
    ttl_seconds: string;
    max_disk_bytes: string;
    max_memory_bytes: string;
    max_single_artifact_bytes: string;
    spill_threshold_bytes: string;
    delete_on_consume: boolean;
  }

  interface ArtifactFormatDraft {
    image: {
      format_id: string;
      quality_percent: string;
      color_profile_id: string;
    };
    audio: {
      container_id: string;
      codec_id: string;
      bitrate_kbps: string;
    };
    video: {
      container_id: string;
      codec_id: string;
      crf: string;
      bit_depth: string;
    };
    three_d: {
      format_id: string;
    };
  }

  interface ManagedMediaDependencyDraft {
    version: string;
    staging_dir: string;
    selected_version: string;
  }

  let policy = $state<WorkflowArtifactPolicy | null>(null);
  let diagnosticsRetentionPolicy = $state<DiagnosticsRetentionPolicy | null>(null);
  let diagnosticsRetentionCleanup = $state<WorkflowRetentionCleanupResult | null>(null);
  let capabilities = $state<WorkflowArtifactFormatCapabilities | null>(null);
  let managedMediaDependencies = $state<WorkflowManagedMediaDependencyStatus[]>([]);
  let managedMediaDrafts = $state<Record<string, ManagedMediaDependencyDraft>>({});
  let policyDraft = $state<ArtifactPolicyDraft>(emptyPolicyDraft());
  let diagnosticsRetentionDays = $state('365');
  let diagnosticsRetentionExplanation = $state('');
  let formatDraft = $state<ArtifactFormatDraft>(defaultFormatDraft());
  let loading = $state(false);
  let savingPolicy = $state(false);
  let savingDiagnosticsRetention = $state(false);
  let applyingDiagnosticsRetentionCleanup = $state(false);
  let savingFormats = $state(false);
  let managedMediaActionKey = $state<string | null>(null);
  let pageError = $state<string | null>(null);
  let policyError = $state<string | null>(null);
  let diagnosticsRetentionError = $state<string | null>(null);
  let formatError = $state<string | null>(null);
  let managedMediaError = $state<string | null>(null);
  let policyMessage = $state<string | null>(null);
  let diagnosticsRetentionMessage = $state<string | null>(null);
  let formatMessage = $state<string | null>(null);
  let managedMediaMessage = $state<string | null>(null);

  let policyRows = $derived(buildArtifactPolicyRows(policy));
  let diagnosticsRetentionPolicyRows = $derived(
    buildDiagnosticsRetentionPolicyRows(diagnosticsRetentionPolicy),
  );
  let diagnosticsRetentionSettingRows = $derived(
    buildDiagnosticsRetentionSettingRows(diagnosticsRetentionPolicy),
  );
  let diagnosticsRetentionCleanupRows = $derived(
    buildDiagnosticsRetentionCleanupRows(diagnosticsRetentionCleanup),
  );
  let capabilityRows = $derived(buildCapabilityRows(capabilities));
  let capabilitySummary = $derived(formatCapabilitySummary(capabilities));
  let imageFormatOptions = $derived(
    formatOptionItems(capabilities?.image_formats ?? [], formatDraft.image.format_id),
  );
  let audioFormatOptions = $derived(
    formatOptionItems(capabilities?.audio_formats ?? [], formatDraft.audio.container_id),
  );
  let videoFormatOptions = $derived(
    formatOptionItems(capabilities?.video_formats ?? [], formatDraft.video.container_id),
  );
  let threeDFormatOptions = $derived(
    formatOptionItems(capabilities?.three_d_formats ?? [], formatDraft.three_d.format_id),
  );
  let selectedImageFormat = $derived(
    findFormatOption(capabilities?.image_formats ?? [], formatDraft.image.format_id),
  );
  let selectedAudioFormat = $derived(
    findFormatOption(capabilities?.audio_formats ?? [], formatDraft.audio.container_id),
  );
  let selectedVideoFormat = $derived(
    findFormatOption(capabilities?.video_formats ?? [], formatDraft.video.container_id),
  );
  let imageColorProfileOptions = $derived(
    optionValuesWithCurrent(selectedImageFormat?.color_profile_ids ?? [], formatDraft.image.color_profile_id),
  );
  let audioCodecOptions = $derived(
    optionValuesWithCurrent(selectedAudioFormat?.codec_ids ?? [], formatDraft.audio.codec_id),
  );
  let videoCodecOptions = $derived(
    optionValuesWithCurrent(selectedVideoFormat?.codec_ids ?? [], formatDraft.video.codec_id),
  );
  let videoBitDepthOptions = $derived(
    optionValuesWithCurrent(selectedVideoFormat?.bit_depths ?? [], formatDraft.video.bit_depth),
  );

  onMount(() => {
    void refreshSettingsPage();
  });

  $effect(() => {
    const focus = $settingsFocus;
    if (focus?.target_id !== 'diagnostics_retention') {
      return;
    }

    setTimeout(() => {
      document
        .getElementById('settings-diagnostics-retention-policy')
        ?.scrollIntoView({ behavior: 'smooth', block: 'start' });
      clearSettingsFocus();
    }, 0);
  });

  async function refreshSettingsPage(): Promise<void> {
    loading = true;
    pageError = null;
    policyError = null;
    diagnosticsRetentionError = null;
    formatError = null;
    managedMediaError = null;
    policyMessage = null;
    diagnosticsRetentionMessage = null;
    formatMessage = null;
    managedMediaMessage = null;
    try {
      const [
        loadedPolicy,
        loadedDiagnosticsRetentionPolicy,
        loadedFormats,
        loadedCapabilities,
        loadedDependencies,
      ] = await Promise.all([
        workflowService.artifactPolicy(),
        workflowService.queryRetentionPolicy(),
        workflowService.artifactFormatSettings(),
        workflowService.artifactFormatCapabilities(),
        workflowService.listManagedMediaDependencies(),
      ]);
      policy = loadedPolicy;
      policyDraft = policyToDraft(loadedPolicy);
      applyDiagnosticsRetentionPolicy(loadedDiagnosticsRetentionPolicy.retention_policy);
      formatDraft = settingsToDraft(loadedFormats.settings);
      capabilities = loadedCapabilities;
      managedMediaDependencies = loadedDependencies;
      managedMediaDrafts = buildManagedMediaDrafts(loadedDependencies);
      normalizeAllFormatDrafts();
    } catch (error) {
      pageError = formatWorkflowCommandError(error);
    } finally {
      loading = false;
    }
  }

  async function saveArtifactPolicy(): Promise<void> {
    policyError = null;
    policyMessage = null;
    if (!policy) {
      policyError = 'ArtifactStore policy is not loaded';
      return;
    }

    const parsed = parseArtifactPolicyDraft();
    if (typeof parsed === 'string') {
      policyError = parsed;
      return;
    }

    savingPolicy = true;
    try {
      const updatedPolicy = await workflowService.updateArtifactPolicy({
        ...policy,
        ...parsed,
      });
      policy = updatedPolicy;
      policyDraft = policyToDraft(updatedPolicy);
      policyMessage = 'ArtifactStore policy saved';
    } catch (error) {
      policyError = formatWorkflowCommandError(error);
    } finally {
      savingPolicy = false;
    }
  }

  async function saveDiagnosticsRetentionPolicy(): Promise<void> {
    diagnosticsRetentionError = null;
    diagnosticsRetentionMessage = null;

    const parsedDays = Number.parseInt(diagnosticsRetentionDays, 10);
    if (!Number.isSafeInteger(parsedDays) || parsedDays < 1) {
      diagnosticsRetentionError = 'Retention days must be at least 1';
      return;
    }

    const explanation = diagnosticsRetentionExplanation.trim();
    if (explanation.length === 0) {
      diagnosticsRetentionError = 'Retention explanation is required';
      return;
    }

    savingDiagnosticsRetention = true;
    try {
      const response = await workflowService.updateRetentionPolicy({
        retention_days: parsedDays,
        explanation,
        reason: 'gui_workbench_settings_retention_update',
      });
      applyDiagnosticsRetentionPolicy(response.retention_policy);
      diagnosticsRetentionMessage = 'Diagnostics retention policy saved';
    } catch (error) {
      diagnosticsRetentionError = formatWorkflowCommandError(error);
    } finally {
      savingDiagnosticsRetention = false;
    }
  }

  async function applyDiagnosticsRetentionCleanup(): Promise<void> {
    diagnosticsRetentionError = null;
    diagnosticsRetentionMessage = null;
    applyingDiagnosticsRetentionCleanup = true;
    try {
      const response = await workflowService.applyRetentionCleanup({
        limit: 250,
        reason: 'gui_workbench_settings_retention_cleanup',
      });
      diagnosticsRetentionCleanup = response.cleanup;
      diagnosticsRetentionMessage = `${response.cleanup.expired_artifact_count} artifacts expired`;
    } catch (error) {
      diagnosticsRetentionError = formatWorkflowCommandError(error);
    } finally {
      applyingDiagnosticsRetentionCleanup = false;
    }
  }

  async function saveArtifactFormats(): Promise<void> {
    formatError = null;
    formatMessage = null;
    const parsed = parseArtifactFormatDraft();
    if (typeof parsed === 'string') {
      formatError = parsed;
      return;
    }

    savingFormats = true;
    try {
      const response = await workflowService.updateArtifactFormatSettings({
        settings: parsed,
        reason: 'gui_workbench_settings_update',
      });
      formatDraft = settingsToDraft(response.settings);
      normalizeAllFormatDrafts();
      formatMessage = 'Artifact format defaults saved';
    } catch (error) {
      formatError = formatWorkflowCommandError(error);
    } finally {
      savingFormats = false;
    }
  }

  async function refreshManagedMediaDependency(id: WorkflowManagedMediaDependencyId): Promise<void> {
    await runManagedMediaAction(`${id}:refresh`, async () => {
      const status = await workflowService.managedMediaDependencyStatus(id);
      replaceManagedMediaDependency(status);
      managedMediaMessage = `${status.display_name} status refreshed`;
    });
  }

  async function installManagedMediaDependency(
    dependency: WorkflowManagedMediaDependencyStatus,
  ): Promise<void> {
    const draft = managedMediaDrafts[dependency.id] ?? emptyManagedMediaDependencyDraft(dependency);
    const version = draft.version.trim();
    const stagingDir = draft.staging_dir.trim();
    if (!version) {
      managedMediaError = `${dependency.display_name} version is required`;
      return;
    }
    if (!stagingDir) {
      managedMediaError = `${dependency.display_name} staging directory is required`;
      return;
    }

    await runManagedMediaAction(`${dependency.id}:install`, async () => {
      const status = await workflowService.installManagedMediaDependencyFromStaging({
        id: dependency.id,
        version,
        staging_dir: stagingDir,
      });
      replaceManagedMediaDependency(status);
      managedMediaMessage = `${status.display_name} ${version} installed from staging`;
    });
  }

  async function selectManagedMediaDependency(
    dependency: WorkflowManagedMediaDependencyStatus,
  ): Promise<void> {
    const selectedVersion = selectedManagedMediaVersion(dependency);
    await runManagedMediaAction(`${dependency.id}:select`, async () => {
      const status = await workflowService.selectManagedMediaDependencyVersion({
        id: dependency.id,
        version: selectedVersion,
      });
      replaceManagedMediaDependency(status);
      managedMediaMessage = selectedVersion
        ? `${status.display_name} selected version ${selectedVersion}`
        : `${status.display_name} selection cleared`;
    });
  }

  async function clearManagedMediaDependencySelection(
    dependency: WorkflowManagedMediaDependencyStatus,
  ): Promise<void> {
    await runManagedMediaAction(`${dependency.id}:clear-select`, async () => {
      const status = await workflowService.selectManagedMediaDependencyVersion({
        id: dependency.id,
        version: null,
      });
      replaceManagedMediaDependency(status);
      managedMediaMessage = `${status.display_name} selection cleared`;
    });
  }

  async function setDefaultManagedMediaDependency(
    dependency: WorkflowManagedMediaDependencyStatus,
  ): Promise<void> {
    const selectedVersion = selectedManagedMediaVersion(dependency);
    await runManagedMediaAction(`${dependency.id}:default`, async () => {
      const status = await workflowService.setDefaultManagedMediaDependencyVersion({
        id: dependency.id,
        version: selectedVersion,
      });
      replaceManagedMediaDependency(status);
      managedMediaMessage = selectedVersion
        ? `${status.display_name} default version ${selectedVersion}`
        : `${status.display_name} default cleared`;
    });
  }

  async function activateManagedMediaDependency(
    dependency: WorkflowManagedMediaDependencyStatus,
  ): Promise<void> {
    const selectedVersion = selectedManagedMediaVersion(dependency);
    if (!selectedVersion) {
      managedMediaError = `${dependency.display_name} needs an installed version to activate`;
      return;
    }

    await runManagedMediaAction(`${dependency.id}:activate`, async () => {
      const status = await workflowService.activateManagedMediaDependencyVersion({
        id: dependency.id,
        version: selectedVersion,
      });
      replaceManagedMediaDependency(status);
      managedMediaMessage = `${status.display_name} activated version ${selectedVersion}`;
    });
  }

  async function removeManagedMediaDependency(
    dependency: WorkflowManagedMediaDependencyStatus,
  ): Promise<void> {
    const selectedVersion = selectedManagedMediaVersion(dependency);
    if (!selectedVersion) {
      managedMediaError = `${dependency.display_name} needs an installed version to remove`;
      return;
    }

    await runManagedMediaAction(`${dependency.id}:remove`, async () => {
      const status = await workflowService.removeManagedMediaDependencyVersion({
        id: dependency.id,
        version: selectedVersion,
      });
      replaceManagedMediaDependency(status);
      managedMediaMessage = `${dependency.display_name} removed version ${selectedVersion}`;
    });
  }

  async function runManagedMediaAction(actionKey: string, action: () => Promise<void>): Promise<void> {
    managedMediaActionKey = actionKey;
    managedMediaError = null;
    managedMediaMessage = null;
    try {
      await action();
    } catch (error) {
      managedMediaError = formatWorkflowCommandError(error);
    } finally {
      managedMediaActionKey = null;
    }
  }

  function replaceManagedMediaDependency(status: WorkflowManagedMediaDependencyStatus): void {
    managedMediaDependencies = managedMediaDependencies.map((dependency) =>
      dependency.id === status.id ? status : dependency,
    );
    managedMediaDrafts = {
      ...managedMediaDrafts,
      [status.id]: managedMediaStatusToDraft(status, managedMediaDrafts[status.id]),
    };
  }

  function buildManagedMediaDrafts(
    dependencies: WorkflowManagedMediaDependencyStatus[],
  ): Record<string, ManagedMediaDependencyDraft> {
    return Object.fromEntries(
      dependencies.map((dependency) => [dependency.id, managedMediaStatusToDraft(dependency)]),
    );
  }

  function managedMediaStatusToDraft(
    dependency: WorkflowManagedMediaDependencyStatus,
    current?: ManagedMediaDependencyDraft,
  ): ManagedMediaDependencyDraft {
    return {
      version: current?.version.trim() || dependency.catalog.version,
      staging_dir: current?.staging_dir ?? '',
      selected_version:
        dependency.selection.selected_version ??
        dependency.selection.active_version ??
        dependency.versions[0]?.version ??
        '',
    };
  }

  function emptyManagedMediaDependencyDraft(
    dependency: WorkflowManagedMediaDependencyStatus,
  ): ManagedMediaDependencyDraft {
    return {
      version: dependency.catalog.version,
      staging_dir: '',
      selected_version: '',
    };
  }

  function selectedManagedMediaVersion(
    dependency: WorkflowManagedMediaDependencyStatus,
  ): string | null {
    const selectedVersion = managedMediaDrafts[dependency.id]?.selected_version.trim() ?? '';
    return selectedVersion.length > 0 ? selectedVersion : null;
  }

  function managedMediaActionInProgress(id: WorkflowManagedMediaDependencyId): boolean {
    return managedMediaActionKey?.startsWith(`${id}:`) ?? false;
  }

  function parseArtifactPolicyDraft():
    | string
    | Omit<WorkflowArtifactPolicy, 'policy_id' | 'policy_version'> {
    const fields = [
      ['TTL seconds', policyDraft.ttl_seconds, { min: 1 }, 'ttl_seconds'],
      ['Max disk bytes', policyDraft.max_disk_bytes, { min: 0 }, 'max_disk_bytes'],
      ['Max memory bytes', policyDraft.max_memory_bytes, { min: 0 }, 'max_memory_bytes'],
      [
        'Max single artifact bytes',
        policyDraft.max_single_artifact_bytes,
        { min: 0 },
        'max_single_artifact_bytes',
      ],
      ['Spill threshold bytes', policyDraft.spill_threshold_bytes, { min: 0 }, 'spill_threshold_bytes'],
    ] as const;
    const parsed: Record<string, number | null> = {};

    for (const [label, rawValue, options, key] of fields) {
      const result = parseNullableIntegerField(label, rawValue, options);
      if (result.error) {
        return result.error;
      }
      parsed[key] = result.value;
    }

    return {
      ttl_seconds: parsed.ttl_seconds,
      max_disk_bytes: parsed.max_disk_bytes,
      max_memory_bytes: parsed.max_memory_bytes,
      max_single_artifact_bytes: parsed.max_single_artifact_bytes,
      spill_threshold_bytes: parsed.spill_threshold_bytes,
      delete_on_consume: policyDraft.delete_on_consume,
    };
  }

  function parseArtifactFormatDraft(): WorkflowArtifactFormatSettings | string {
    const quality = parseRequiredIntegerField(
      'Image quality',
      formatDraft.image.quality_percent,
      selectedImageFormat?.quality_min_percent ?? 0,
      selectedImageFormat?.quality_max_percent ?? null,
    );
    if (typeof quality === 'string') {
      return quality;
    }

    const bitrate = parseRequiredIntegerField(
      'Audio bitrate',
      formatDraft.audio.bitrate_kbps,
      selectedAudioFormat?.bitrate_min_kbps ?? 0,
      selectedAudioFormat?.bitrate_max_kbps ?? null,
    );
    if (typeof bitrate === 'string') {
      return bitrate;
    }

    const crf = parseRequiredIntegerField(
      'Video CRF',
      formatDraft.video.crf,
      selectedVideoFormat?.crf_min ?? 0,
      selectedVideoFormat?.crf_max ?? null,
    );
    if (typeof crf === 'string') {
      return crf;
    }

    return {
      image: {
        format_id: formatDraft.image.format_id,
        quality_percent: quality,
        color_profile_id: formatDraft.image.color_profile_id,
      },
      audio: {
        container_id: formatDraft.audio.container_id,
        codec_id: formatDraft.audio.codec_id,
        bitrate_kbps: bitrate,
      },
      video: {
        container_id: formatDraft.video.container_id,
        codec_id: formatDraft.video.codec_id,
        crf,
        bit_depth: formatDraft.video.bit_depth,
      },
      three_d: {
        format_id: formatDraft.three_d.format_id,
      },
    };
  }

  function parseRequiredIntegerField(
    label: string,
    rawValue: string,
    min: number,
    max: number | null,
  ): number | string {
    const result = parseNullableIntegerField(label, rawValue, { min, max });
    if (result.error) {
      return result.error;
    }
    if (result.value === null) {
      return `${label} is required`;
    }
    return result.value;
  }

  function policyToDraft(nextPolicy: WorkflowArtifactPolicy): ArtifactPolicyDraft {
    return {
      ttl_seconds: nullableNumberToInput(nextPolicy.ttl_seconds),
      max_disk_bytes: nullableNumberToInput(nextPolicy.max_disk_bytes),
      max_memory_bytes: nullableNumberToInput(nextPolicy.max_memory_bytes),
      max_single_artifact_bytes: nullableNumberToInput(nextPolicy.max_single_artifact_bytes),
      spill_threshold_bytes: nullableNumberToInput(nextPolicy.spill_threshold_bytes),
      delete_on_consume: nextPolicy.delete_on_consume,
    };
  }

  function applyDiagnosticsRetentionPolicy(nextPolicy: DiagnosticsRetentionPolicy): void {
    diagnosticsRetentionPolicy = nextPolicy;
    diagnosticsRetentionDays = String(nextPolicy.retention_days);
    diagnosticsRetentionExplanation = nextPolicy.explanation;
  }

  function settingsToDraft(settings: WorkflowArtifactFormatSettings): ArtifactFormatDraft {
    return {
      image: {
        format_id: settings.image.format_id,
        quality_percent: String(settings.image.quality_percent),
        color_profile_id: settings.image.color_profile_id,
      },
      audio: {
        container_id: settings.audio.container_id,
        codec_id: settings.audio.codec_id,
        bitrate_kbps: String(settings.audio.bitrate_kbps),
      },
      video: {
        container_id: settings.video.container_id,
        codec_id: settings.video.codec_id,
        crf: String(settings.video.crf),
        bit_depth: settings.video.bit_depth,
      },
      three_d: {
        format_id: settings.three_d.format_id,
      },
    };
  }

  function emptyPolicyDraft(): ArtifactPolicyDraft {
    return {
      ttl_seconds: '',
      max_disk_bytes: '',
      max_memory_bytes: '',
      max_single_artifact_bytes: '',
      spill_threshold_bytes: '',
      delete_on_consume: false,
    };
  }

  function defaultFormatDraft(): ArtifactFormatDraft {
    return {
      image: {
        format_id: 'jpg',
        quality_percent: '75',
        color_profile_id: 'srgb',
      },
      audio: {
        container_id: 'ogg',
        codec_id: 'opus',
        bitrate_kbps: '96',
      },
      video: {
        container_id: 'ivf',
        codec_id: 'svt_av1',
        crf: '32',
        bit_depth: '8bit',
      },
      three_d: {
        format_id: 'glb',
      },
    };
  }

  function nullableNumberToInput(value: number | null | undefined): string {
    return value === null || value === undefined ? '' : String(value);
  }

  function normalizeAllFormatDrafts(): void {
    normalizeImageFormatDraft();
    normalizeAudioFormatDraft();
    normalizeVideoFormatDraft();
  }

  function normalizeImageFormatDraft(): void {
    const selected = selectedImageFormat;
    formatDraft.image.color_profile_id = firstAvailableValue(
      selected?.color_profile_ids ?? [],
      formatDraft.image.color_profile_id,
    );
  }

  function normalizeAudioFormatDraft(): void {
    const selected = selectedAudioFormat;
    formatDraft.audio.codec_id = firstAvailableValue(
      selected?.codec_ids ?? [],
      formatDraft.audio.codec_id,
    );
  }

  function normalizeVideoFormatDraft(): void {
    const selected = selectedVideoFormat;
    formatDraft.video.codec_id = firstAvailableValue(
      selected?.codec_ids ?? [],
      formatDraft.video.codec_id,
    );
    formatDraft.video.bit_depth = firstAvailableValue(
      selected?.bit_depths ?? [],
      formatDraft.video.bit_depth,
    );
  }

  function firstAvailableValue(values: string[], currentValue: string): string {
    if (values.length === 0 || values.includes(currentValue)) {
      return currentValue;
    }
    return values[0];
  }
</script>

{#snippet sectionNote(error: string | null, message: string | null)}
  {#if error}
    <p class="pg-muted pg-small set-note" role="alert">{error}</p>
  {:else if message}
    <p class="pg-muted pg-small set-note" role="status">{message}</p>
  {/if}
{/snippet}

<div class="pg-main h-full">
  <div class="set-body">
    <Panel label="Settings">
      {#snippet top()}
        <div class="set-head">
          <div class="set-headtext">
            <h1 class="pg-title set-h1">Settings</h1>
            <p class="pg-muted pg-small set-sub">
              ArtifactStore policy, media artifact defaults, managed inference runtimes, and media dependencies
            </p>
          </div>
          <button type="button" class="pg-btn" onclick={() => refreshSettingsPage()} disabled={loading}>
            {loading ? 'Refreshing' : 'Refresh'}
          </button>
        </div>
        {#if pageError}
          <p class="pg-muted pg-small set-note" role="alert">{pageError}</p>
        {/if}
      {/snippet}

      <section class="pg-sec" aria-labelledby="settings-runtime-heading">
        <h2 id="settings-runtime-heading" class="set-h2">Runtime and app settings</h2>
        <p class="pg-muted pg-small set-sub">
          Server connection, model paths, device policy, RAG, and sandbox configuration
        </p>
        <div class="set-cols">
          <div class="set-stack">
            <ServerStatus showManagedRuntimePanel={false} />
            <ModelConfig />
          </div>
          <div class="set-stack">
            <DeviceConfig />
            <RagStatus />
            <SandboxSettings />
          </div>
        </div>
      </section>

      <form
        class="pg-sec"
        aria-labelledby="settings-artifact-policy-heading"
        onsubmit={(event) => {
          event.preventDefault();
          void saveArtifactPolicy();
        }}
      >
        <div class="set-sechead">
          <div class="set-headtext">
            <h2 id="settings-artifact-policy-heading" class="set-h2">ArtifactStore policy</h2>
            <p class="pg-muted pg-small set-sub">
              {#if policy}
                <span class="pg-mono">{policy.policy_id} v{policy.policy_version}</span>
              {:else}
                Policy unavailable
              {/if}
            </p>
          </div>
          <button type="submit" class="pg-btn pg-btn-primary" disabled={loading || savingPolicy || !policy}>
            Save policy
          </button>
        </div>

        {@render sectionNote(policyError, policyMessage)}

        <div class="set-fields">
          <label class="pg-fl">
            TTL seconds
            <input
              id="settings-artifact-ttl"
              type="number"
              min="1"
              inputmode="numeric"
              bind:value={policyDraft.ttl_seconds}
              class="pg-field pg-mono"
              placeholder="unset"
            />
          </label>
          <label class="pg-fl">
            Max disk bytes
            <input
              id="settings-artifact-disk"
              type="number"
              min="0"
              inputmode="numeric"
              bind:value={policyDraft.max_disk_bytes}
              class="pg-field pg-mono"
              placeholder="unlimited"
            />
          </label>
          <label class="pg-fl">
            Max memory bytes
            <input
              id="settings-artifact-memory"
              type="number"
              min="0"
              inputmode="numeric"
              bind:value={policyDraft.max_memory_bytes}
              class="pg-field pg-mono"
              placeholder="unlimited"
            />
          </label>
          <label class="pg-fl">
            Max single artifact bytes
            <input
              id="settings-artifact-single"
              type="number"
              min="0"
              inputmode="numeric"
              bind:value={policyDraft.max_single_artifact_bytes}
              class="pg-field pg-mono"
              placeholder="unlimited"
            />
          </label>
          <label class="pg-fl">
            Spill threshold bytes
            <input
              id="settings-artifact-spill"
              type="number"
              min="0"
              inputmode="numeric"
              bind:value={policyDraft.spill_threshold_bytes}
              class="pg-field pg-mono"
              placeholder="unset"
            />
          </label>
          <label class="set-check">
            <input type="checkbox" bind:checked={policyDraft.delete_on_consume} />
            Delete payload body after consume acknowledgement
          </label>
        </div>
      </form>

      <form
        id="settings-diagnostics-retention-policy"
        class="pg-sec"
        aria-labelledby="settings-diagnostics-retention-heading"
        onsubmit={(event) => {
          event.preventDefault();
          void saveDiagnosticsRetentionPolicy();
        }}
      >
        <div class="set-sechead">
          <div class="set-headtext">
            <h2 id="settings-diagnostics-retention-heading" class="set-h2">Diagnostics retention policy</h2>
            <p class="pg-muted pg-small set-sub">
              {#if diagnosticsRetentionPolicy}
                <span class="pg-mono">{diagnosticsRetentionPolicy.policy_id} v{diagnosticsRetentionPolicy.policy_version}</span>
              {:else}
                Policy unavailable
              {/if}
            </p>
          </div>
          <button
            type="submit"
            class="pg-btn pg-btn-primary"
            disabled={loading || savingDiagnosticsRetention || !diagnosticsRetentionPolicy}
          >
            Save retention
          </button>
        </div>

        {@render sectionNote(diagnosticsRetentionError, diagnosticsRetentionMessage)}

        <div class="set-fields set-fields-retention">
          <label class="pg-fl">
            Days
            <input
              id="settings-diagnostics-retention-days"
              type="number"
              min="1"
              inputmode="numeric"
              bind:value={diagnosticsRetentionDays}
              class="pg-field pg-mono"
            />
          </label>
          <label class="pg-fl">
            Explanation
            <textarea
              id="settings-diagnostics-retention-explanation"
              rows="3"
              bind:value={diagnosticsRetentionExplanation}
              class="pg-field set-textarea"
            ></textarea>
          </label>
        </div>

        {#if diagnosticsRetentionSettingRows.length > 0}
          <div class="set-block">
            <KeyValue rows={diagnosticsRetentionSettingRows} label="Retention settings" />
          </div>
        {/if}

        <section class="pg-card set-card" aria-labelledby="settings-retention-cleanup-heading">
          <div class="set-sechead">
            <div class="set-headtext">
              <h3 id="settings-retention-cleanup-heading" class="set-h3">Retention cleanup</h3>
              <p class="pg-muted pg-small set-sub">
                Apply backend retention cleanup using the current diagnostics retention policy.
              </p>
            </div>
            <button
              type="button"
              class="pg-btn"
              onclick={() => {
                void applyDiagnosticsRetentionCleanup();
              }}
              disabled={loading || applyingDiagnosticsRetentionCleanup || !diagnosticsRetentionPolicy}
            >
              {applyingDiagnosticsRetentionCleanup ? 'Applying cleanup' : 'Apply cleanup'}
            </button>
          </div>

          {#if diagnosticsRetentionCleanupRows.length > 0}
            <div class="set-block">
              <KeyValue rows={diagnosticsRetentionCleanupRows} label="Last cleanup" />
            </div>
          {/if}
        </section>
      </form>

      <form
        class="pg-sec"
        aria-labelledby="settings-formats-heading"
        onsubmit={(event) => {
          event.preventDefault();
          void saveArtifactFormats();
        }}
      >
        <div class="set-sechead">
          <div class="set-headtext">
            <h2 id="settings-formats-heading" class="set-h2">Artifact format defaults</h2>
            <p class="pg-muted pg-small set-sub">{capabilitySummary}</p>
          </div>
          <button type="submit" class="pg-btn pg-btn-primary" disabled={loading || savingFormats || !capabilities}>
            Save formats
          </button>
        </div>

        {@render sectionNote(formatError, formatMessage)}

        <div class="set-cards">
          <section class="pg-card" aria-labelledby="settings-format-image-heading">
            <div class="set-cardhead">
              <h3 id="settings-format-image-heading" class="set-h3">Image</h3>
              <span class="pg-muted pg-small">
                {formatRangeLabel(selectedImageFormat?.quality_min_percent, selectedImageFormat?.quality_max_percent, '%')}
              </span>
            </div>
            <div class="set-fields set-fields-card">
              <label class="pg-fl">
                Format
                <select
                  id="settings-image-format"
                  bind:value={formatDraft.image.format_id}
                  onchange={normalizeImageFormatDraft}
                  class="pg-field"
                >
                  {#each imageFormatOptions as option (option.value)}
                    <option value={option.value}>{option.label}</option>
                  {/each}
                </select>
              </label>
              <label class="pg-fl">
                Quality percent
                <input
                  id="settings-image-quality"
                  type="number"
                  min={selectedImageFormat?.quality_min_percent ?? 0}
                  max={selectedImageFormat?.quality_max_percent ?? undefined}
                  inputmode="numeric"
                  bind:value={formatDraft.image.quality_percent}
                  class="pg-field pg-mono"
                />
              </label>
              <label class="pg-fl">
                Color profile
                <select id="settings-image-profile" bind:value={formatDraft.image.color_profile_id} class="pg-field">
                  {#each imageColorProfileOptions as profile (profile)}
                    <option value={profile}>{profile}</option>
                  {/each}
                </select>
              </label>
            </div>
          </section>

          <section class="pg-card" aria-labelledby="settings-format-audio-heading">
            <div class="set-cardhead">
              <h3 id="settings-format-audio-heading" class="set-h3">Audio</h3>
              <span class="pg-muted pg-small">
                {formatRangeLabel(selectedAudioFormat?.bitrate_min_kbps, selectedAudioFormat?.bitrate_max_kbps, ' kbps')}
              </span>
            </div>
            <div class="set-fields set-fields-card">
              <label class="pg-fl">
                Container
                <select
                  id="settings-audio-container"
                  bind:value={formatDraft.audio.container_id}
                  onchange={normalizeAudioFormatDraft}
                  class="pg-field"
                >
                  {#each audioFormatOptions as option (option.value)}
                    <option value={option.value}>{option.label}</option>
                  {/each}
                </select>
              </label>
              <label class="pg-fl">
                Codec
                <select id="settings-audio-codec" bind:value={formatDraft.audio.codec_id} class="pg-field">
                  {#each audioCodecOptions as codec (codec)}
                    <option value={codec}>{codec}</option>
                  {/each}
                </select>
              </label>
              <label class="pg-fl">
                Bitrate kbps
                <input
                  id="settings-audio-bitrate"
                  type="number"
                  min={selectedAudioFormat?.bitrate_min_kbps ?? 0}
                  max={selectedAudioFormat?.bitrate_max_kbps ?? undefined}
                  inputmode="numeric"
                  bind:value={formatDraft.audio.bitrate_kbps}
                  class="pg-field pg-mono"
                />
              </label>
            </div>
          </section>

          <section class="pg-card" aria-labelledby="settings-format-video-heading">
            <div class="set-cardhead">
              <h3 id="settings-format-video-heading" class="set-h3">Video</h3>
              <span class="pg-muted pg-small">
                {formatRangeLabel(selectedVideoFormat?.crf_min, selectedVideoFormat?.crf_max)}
              </span>
            </div>
            <div class="set-fields set-fields-card">
              <label class="pg-fl">
                Container
                <select
                  id="settings-video-container"
                  bind:value={formatDraft.video.container_id}
                  onchange={normalizeVideoFormatDraft}
                  class="pg-field"
                >
                  {#each videoFormatOptions as option (option.value)}
                    <option value={option.value}>{option.label}</option>
                  {/each}
                </select>
              </label>
              <label class="pg-fl">
                Codec
                <select id="settings-video-codec" bind:value={formatDraft.video.codec_id} class="pg-field">
                  {#each videoCodecOptions as codec (codec)}
                    <option value={codec}>{codec}</option>
                  {/each}
                </select>
              </label>
              <label class="pg-fl">
                CRF
                <input
                  id="settings-video-crf"
                  type="number"
                  min={selectedVideoFormat?.crf_min ?? 0}
                  max={selectedVideoFormat?.crf_max ?? undefined}
                  inputmode="numeric"
                  bind:value={formatDraft.video.crf}
                  class="pg-field pg-mono"
                />
              </label>
              <label class="pg-fl">
                Bit depth
                <select id="settings-video-depth" bind:value={formatDraft.video.bit_depth} class="pg-field">
                  {#each videoBitDepthOptions as bitDepth (bitDepth)}
                    <option value={bitDepth}>{bitDepth}</option>
                  {/each}
                </select>
              </label>
            </div>
          </section>

          <section class="pg-card" aria-labelledby="settings-format-three-d-heading">
            <div class="set-cardhead">
              <h3 id="settings-format-three-d-heading" class="set-h3">3D</h3>
            </div>
            <div class="set-fields set-fields-card">
              <label class="pg-fl">
                Format
                <select id="settings-three-d-format" bind:value={formatDraft.three_d.format_id} class="pg-field">
                  {#each threeDFormatOptions as option (option.value)}
                    <option value={option.value}>{option.label}</option>
                  {/each}
                </select>
              </label>
            </div>
          </section>
        </div>
      </form>

      <section class="pg-sec" aria-labelledby="settings-runtimes-heading">
        <h2 id="settings-runtimes-heading" class="set-h2">Managed inference runtimes</h2>
        <p class="pg-muted pg-small set-sub">
          Install and select sidecar backends required by scheduler workflows, including llama.cpp.
        </p>
        <div class="set-stack set-block">
          <ManagedRuntimePanel />
          <ManagedBinaryOverviewPanel />
        </div>
      </section>

      <section class="pg-sec" aria-labelledby="settings-media-heading">
        <div class="set-sechead">
          <div class="set-headtext">
            <h2 id="settings-media-heading" class="set-h2">Managed media dependencies</h2>
            <p class="pg-muted pg-small set-sub">
              FFmpeg, ocioconvert, oiiotool, and OpenColorIO status and actions from backend commands
            </p>
          </div>
          <button
            type="button"
            class="pg-btn"
            onclick={() => refreshSettingsPage()}
            disabled={loading || managedMediaActionKey !== null}
          >
            {loading ? 'Refreshing dependencies' : 'Refresh dependencies'}
          </button>
        </div>

        {@render sectionNote(managedMediaError, managedMediaMessage)}

        <div class="set-stack set-block">
          {#each managedMediaDependencies as dependency (dependency.id)}
            {@const presentation = formatManagedMediaDependencyStatus(dependency)}
            {@const selectedVersionOptions = managedMediaVersionOptions(dependency)}
            {@const actionBusy = managedMediaActionInProgress(dependency.id)}
            {@const dependencyRows = buildManagedMediaDependencyRows(dependency)}
            {@const rowSplit = Math.ceil(dependencyRows.length / 2)}
            <section class="pg-card" aria-labelledby={`managed-media-heading-${dependency.id}`}>
              <div class="set-sechead">
                <div class="set-headtext">
                  <div class="set-titleline">
                    <h3 id={`managed-media-heading-${dependency.id}`} class="set-h3">{dependency.display_name}</h3>
                    <Chip glyph={presentation.statusGlyph}>{presentation.readinessLabel}</Chip>
                    <Chip>{presentation.categoryLabel}</Chip>
                  </div>
                  <p class="pg-muted pg-small set-sub">
                    {dependency.catalog.source.owner}/{dependency.catalog.source.project}, {presentation.packageLabel},
                    <span class="pg-mono">{dependency.catalog.platform_key}</span>
                  </p>
                </div>
                <button
                  type="button"
                  class="pg-btn pg-btn-sm"
                  aria-label={`Refresh ${dependency.display_name}`}
                  onclick={() => refreshManagedMediaDependency(dependency.id)}
                  disabled={loading || actionBusy}
                >
                  {managedMediaActionKey === `${dependency.id}:refresh` ? 'Refreshing' : 'Refresh'}
                </button>
              </div>

              <div class="set-kvcols">
                <KeyValue rows={dependencyRows.slice(0, rowSplit)} label={`${dependency.display_name} catalog`} />
                <KeyValue rows={dependencyRows.slice(rowSplit)} label={`${dependency.display_name} versions`} />
              </div>

              {#if dependency.missing_files.length > 0}
                <p class="pg-muted pg-small set-note">Missing: {dependency.missing_files.join(', ')}</p>
              {/if}

              <div class="set-fields set-fields-media">
                <label class="pg-fl">
                  Version
                  <input
                    id={`managed-media-version-${dependency.id}`}
                    type="text"
                    bind:value={managedMediaDrafts[dependency.id].version}
                    class="pg-field pg-mono"
                    placeholder={dependency.catalog.version}
                  />
                </label>
                <label class="pg-fl">
                  Staging directory
                  <input
                    id={`managed-media-staging-${dependency.id}`}
                    type="text"
                    bind:value={managedMediaDrafts[dependency.id].staging_dir}
                    class="pg-field pg-mono"
                    placeholder="/path/to/staged/package"
                  />
                </label>
                <label class="pg-fl">
                  Installed version
                  <select
                    id={`managed-media-selected-${dependency.id}`}
                    bind:value={managedMediaDrafts[dependency.id].selected_version}
                    class="pg-field"
                    disabled={selectedVersionOptions.length === 0}
                  >
                    {#if selectedVersionOptions.length === 0}
                      <option value="">No installed versions</option>
                    {:else}
                      {#each selectedVersionOptions as version (version)}
                        {@const versionStatus = dependency.versions.find((candidate) => candidate.version === version)}
                        <option value={version}>
                          {versionStatus ? managedMediaVersionStatusLabel(versionStatus) : version}
                        </option>
                      {/each}
                    {/if}
                  </select>
                </label>
              </div>

              <div class="set-actions">
                <button
                  type="button"
                  class="pg-btn pg-btn-sm"
                  onclick={() => installManagedMediaDependency(dependency)}
                  title={
                    managedMediaDrafts[dependency.id].staging_dir.trim().length === 0
                      ? 'A local staging directory is required for this manual install path'
                      : 'Install from local staging directory'
                  }
                  disabled={
                    loading ||
                    actionBusy ||
                    managedMediaDrafts[dependency.id].staging_dir.trim().length === 0
                  }
                >
                  Install from staging
                </button>
                <button
                  type="button"
                  class="pg-btn pg-btn-sm"
                  onclick={() => selectManagedMediaDependency(dependency)}
                  disabled={loading || actionBusy || selectedVersionOptions.length === 0}
                >
                  Select
                </button>
                <button
                  type="button"
                  class="pg-btn pg-btn-sm"
                  onclick={() => setDefaultManagedMediaDependency(dependency)}
                  disabled={loading || actionBusy || selectedVersionOptions.length === 0}
                >
                  Set default
                </button>
                <button
                  type="button"
                  class="pg-btn pg-btn-sm"
                  onclick={() => activateManagedMediaDependency(dependency)}
                  disabled={loading || actionBusy || selectedVersionOptions.length === 0}
                >
                  Activate
                </button>
                <button
                  type="button"
                  class="pg-btn pg-btn-sm"
                  onclick={() => clearManagedMediaDependencySelection(dependency)}
                  disabled={loading || actionBusy || !dependency.selection.selected_version}
                >
                  Clear selection
                </button>
                <button
                  type="button"
                  class="pg-btn pg-btn-sm pg-btn-danger"
                  onclick={() => removeManagedMediaDependency(dependency)}
                  disabled={loading || actionBusy || selectedVersionOptions.length === 0}
                >
                  Remove version
                </button>
              </div>
            </section>
          {/each}
        </div>
      </section>
    </Panel>

    <Panel label="Settings summary" width="320px">
      <section class="pg-sec" aria-labelledby="settings-summary-policy-heading">
        <h2 id="settings-summary-policy-heading" class="set-h2">ArtifactStore policy</h2>
        <KeyValue rows={policyRows} />
      </section>

      <section class="pg-sec" aria-labelledby="settings-summary-retention-heading">
        <h2 id="settings-summary-retention-heading" class="set-h2">Diagnostics retention</h2>
        <KeyValue rows={diagnosticsRetentionPolicyRows} />
      </section>

      <section class="pg-sec" aria-labelledby="settings-summary-capabilities-heading">
        <h2 id="settings-summary-capabilities-heading" class="set-h2">Capabilities</h2>
        <KeyValue rows={capabilityRows} />
      </section>
    </Panel>
  </div>
</div>

<style>
  .set-body {
    flex: 1;
    min-height: 0;
    display: flex;
    gap: 14px;
    align-items: stretch;
  }

  .set-head,
  .set-sechead {
    display: flex;
    gap: 16px;
    align-items: flex-start;
    flex-wrap: wrap;
  }

  .set-headtext {
    flex: 1 1 260px;
    min-width: 0;
  }

  .set-h1 {
    margin: 0;
  }

  .set-h2 {
    margin: 0;
    font-size: 13px;
    font-weight: 700;
    color: var(--pg-text-bright);
  }

  .set-h3 {
    margin: 0;
    font-size: 13px;
    font-weight: 700;
    color: var(--pg-text-bright);
  }

  .set-sub {
    margin: 2px 0 0;
  }

  .set-note {
    margin: 8px 0 0;
  }

  .set-block {
    margin-top: 12px;
  }

  /* The panel top already draws a rule above the first section. */
  .pg-sec:first-child {
    border-top: 0;
  }

  .pg-sec .set-h2 + :global(.pg-kv) {
    margin-top: 8px;
  }

  .set-cols {
    margin-top: 12px;
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(min(420px, 100%), 1fr));
    gap: 14px;
    align-items: start;
  }

  .set-stack {
    display: flex;
    flex-direction: column;
    gap: 14px;
    min-width: 0;
  }

  .set-fields {
    margin-top: 12px;
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
    gap: 10px 14px;
    align-items: end;
  }

  .set-fields-retention {
    grid-template-columns: 12rem minmax(0, 1fr);
    align-items: start;
  }

  .set-fields-card {
    grid-template-columns: repeat(auto-fit, minmax(140px, 1fr));
    margin-top: 10px;
  }

  .set-fields-media {
    grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
  }

  /* Outranks the shared `.pg-fl .pg-field` single-line height. */
  .pg-fl textarea.set-textarea {
    height: auto;
    padding: 6px 10px;
    resize: vertical;
    line-height: 1.45;
  }

  .set-check {
    display: flex;
    align-items: center;
    gap: 8px;
    min-height: 28px;
    font-size: 13px;
    color: var(--pg-text);
    cursor: pointer;
  }

  .set-check input {
    width: 15px;
    height: 15px;
    accent-color: var(--pg-text);
  }

  .set-card {
    margin-top: 12px;
  }

  .set-cards {
    margin-top: 12px;
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(min(380px, 100%), 1fr));
    gap: 12px;
  }

  .set-cardhead {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 12px;
  }

  .set-titleline {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
  }

  .set-kvcols {
    margin-top: 12px;
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(min(280px, 100%), 1fr));
    gap: 5px 24px;
    align-items: start;
  }

  .set-actions {
    margin-top: 12px;
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  }
</style>
