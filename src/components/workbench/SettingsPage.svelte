<script lang="ts">
  import { onMount } from 'svelte';
  import { Play, RefreshCw, Save, Settings, Trash2, Upload, X } from 'lucide-svelte';
  import DeviceConfig from '../DeviceConfig.svelte';
  import ModelConfig from '../ModelConfig.svelte';
  import RagStatus from '../RagStatus.svelte';
  import SandboxSettings from '../SandboxSettings.svelte';
  import ServerStatus from '../ServerStatus.svelte';
  import type { DiagnosticsRetentionPolicy } from '../../services/diagnostics/types';
  import type {
    WorkflowArtifactFormatCapabilities,
    WorkflowArtifactFormatSettings,
    WorkflowArtifactPolicy,
    WorkflowManagedMediaDependencyId,
    WorkflowManagedMediaDependencyStatus,
    WorkflowMediaFormatOption,
  } from '../../services/workflow/types';
  import { workflowService } from '../../services/workflow/WorkflowService';
  import {
    buildArtifactPolicyRows,
    buildDiagnosticsRetentionPolicyRows,
    buildDiagnosticsRetentionSettingRows,
    buildManagedMediaDependencyRows,
    findFormatOption,
    formatManagedMediaDependencyStatus,
    formatOptionItems,
    formatRangeLabel,
    managedMediaVersionOptions,
    managedMediaVersionStatusLabel,
    optionValuesWithCurrent,
    parseNullableIntegerField,
  } from './settingsPagePresenters';
  import { formatWorkflowCommandError } from './workflowErrorPresenters';

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

  function capabilityCount(options: WorkflowMediaFormatOption[] | undefined): string {
    const count = options?.length ?? 0;
    return `${count} ${count === 1 ? 'format' : 'formats'}`;
  }
</script>

<section class="flex h-full min-h-0 flex-col bg-neutral-950">
  <div class="flex shrink-0 items-center justify-between border-b border-neutral-800 px-4 py-3">
    <div class="min-w-0">
      <h1 class="text-base font-semibold text-neutral-100">Settings</h1>
      <div class="mt-1 truncate text-xs text-neutral-500">
        ArtifactStore policy, media artifact defaults, and managed media dependencies
      </div>
    </div>
    <button
      type="button"
      class="inline-flex items-center gap-2 rounded border border-neutral-700 px-3 py-1.5 text-sm text-neutral-300 transition-colors hover:border-neutral-500 hover:text-neutral-100 focus-visible:outline focus-visible:outline-2 focus-visible:outline-cyan-400 disabled:opacity-50"
      onclick={() => refreshSettingsPage()}
      disabled={loading}
    >
      <RefreshCw size={14} aria-hidden="true" class={loading ? 'animate-spin' : ''} />
      Refresh
    </button>
  </div>

  {#if pageError}
    <div class="border-b border-red-900 bg-red-950/50 px-4 py-2 text-sm text-red-200" role="alert">
      {pageError}
    </div>
  {/if}

  <div class="grid min-h-0 flex-1 grid-cols-1 overflow-hidden xl:grid-cols-[1fr_24rem]">
    <div class="min-h-0 overflow-auto">
      <section class="border-b border-neutral-900 px-4 py-4">
        <div class="mb-4">
          <h2 class="text-sm font-semibold text-neutral-100">Runtime And App Settings</h2>
          <div class="mt-1 text-xs text-neutral-500">
            Server connection, model paths, device policy, RAG, and sandbox configuration
          </div>
        </div>
        <div class="grid gap-4 2xl:grid-cols-2">
          <div class="space-y-4">
            <ServerStatus />
            <ModelConfig />
          </div>
          <div class="space-y-4">
            <DeviceConfig />
            <RagStatus />
            <SandboxSettings />
          </div>
        </div>
      </section>

      <form
        class="border-b border-neutral-900 px-4 py-4"
        onsubmit={(event) => {
          event.preventDefault();
          void saveArtifactPolicy();
        }}
      >
        <div class="flex flex-wrap items-start justify-between gap-3">
          <div>
            <h2 class="text-sm font-semibold text-neutral-100">ArtifactStore Policy</h2>
            <div class="mt-1 text-xs text-neutral-500">
              {policy ? `${policy.policy_id} v${policy.policy_version}` : 'Policy unavailable'}
            </div>
          </div>
          <button
            type="submit"
            class="inline-flex items-center gap-2 rounded border border-cyan-800 bg-cyan-950 px-3 py-1.5 text-sm text-cyan-100 transition-colors hover:border-cyan-600 focus-visible:outline focus-visible:outline-2 focus-visible:outline-cyan-400 disabled:opacity-50"
            disabled={loading || savingPolicy || !policy}
          >
            <Save size={14} aria-hidden="true" />
            Save Policy
          </button>
        </div>

        {#if policyError}
          <div class="mt-3 rounded border border-red-900 bg-red-950/40 px-3 py-2 text-sm text-red-200" role="alert">
            {policyError}
          </div>
        {:else if policyMessage}
          <div class="mt-3 rounded border border-emerald-900 bg-emerald-950/30 px-3 py-2 text-sm text-emerald-200" role="status">
            {policyMessage}
          </div>
        {/if}

        <div class="mt-4 grid gap-3 md:grid-cols-2 xl:grid-cols-3">
          <div>
            <label for="settings-artifact-ttl" class="mb-2 block text-xs uppercase text-neutral-500">
              TTL seconds
            </label>
            <input
              id="settings-artifact-ttl"
              type="number"
              min="1"
              inputmode="numeric"
              bind:value={policyDraft.ttl_seconds}
              class="w-full rounded border border-neutral-700 bg-neutral-900 px-3 py-1.5 font-mono text-xs text-neutral-100 placeholder:text-neutral-600 focus:border-cyan-500 focus:outline-none"
              placeholder="unset"
            />
          </div>
          <div>
            <label for="settings-artifact-disk" class="mb-2 block text-xs uppercase text-neutral-500">
              Max disk bytes
            </label>
            <input
              id="settings-artifact-disk"
              type="number"
              min="0"
              inputmode="numeric"
              bind:value={policyDraft.max_disk_bytes}
              class="w-full rounded border border-neutral-700 bg-neutral-900 px-3 py-1.5 font-mono text-xs text-neutral-100 placeholder:text-neutral-600 focus:border-cyan-500 focus:outline-none"
              placeholder="unlimited"
            />
          </div>
          <div>
            <label for="settings-artifact-memory" class="mb-2 block text-xs uppercase text-neutral-500">
              Max memory bytes
            </label>
            <input
              id="settings-artifact-memory"
              type="number"
              min="0"
              inputmode="numeric"
              bind:value={policyDraft.max_memory_bytes}
              class="w-full rounded border border-neutral-700 bg-neutral-900 px-3 py-1.5 font-mono text-xs text-neutral-100 placeholder:text-neutral-600 focus:border-cyan-500 focus:outline-none"
              placeholder="unlimited"
            />
          </div>
          <div>
            <label for="settings-artifact-single" class="mb-2 block text-xs uppercase text-neutral-500">
              Max single artifact bytes
            </label>
            <input
              id="settings-artifact-single"
              type="number"
              min="0"
              inputmode="numeric"
              bind:value={policyDraft.max_single_artifact_bytes}
              class="w-full rounded border border-neutral-700 bg-neutral-900 px-3 py-1.5 font-mono text-xs text-neutral-100 placeholder:text-neutral-600 focus:border-cyan-500 focus:outline-none"
              placeholder="unlimited"
            />
          </div>
          <div>
            <label for="settings-artifact-spill" class="mb-2 block text-xs uppercase text-neutral-500">
              Spill threshold bytes
            </label>
            <input
              id="settings-artifact-spill"
              type="number"
              min="0"
              inputmode="numeric"
              bind:value={policyDraft.spill_threshold_bytes}
              class="w-full rounded border border-neutral-700 bg-neutral-900 px-3 py-1.5 font-mono text-xs text-neutral-100 placeholder:text-neutral-600 focus:border-cyan-500 focus:outline-none"
              placeholder="unset"
            />
          </div>
          <label class="flex items-center gap-3 rounded border border-neutral-800 bg-neutral-900/40 px-3 py-2 text-sm text-neutral-200">
            <input
              type="checkbox"
              bind:checked={policyDraft.delete_on_consume}
              class="h-4 w-4 rounded border-neutral-700 bg-neutral-900 text-cyan-500 focus:ring-cyan-500"
            />
            Delete payload body after consume acknowledgement
          </label>
        </div>
      </form>

      <form
        class="border-b border-neutral-900 px-4 py-4"
        onsubmit={(event) => {
          event.preventDefault();
          void saveDiagnosticsRetentionPolicy();
        }}
      >
        <div class="flex flex-wrap items-start justify-between gap-3">
          <div>
            <h2 class="text-sm font-semibold text-neutral-100">Diagnostics Retention Policy</h2>
            <div class="mt-1 text-xs text-neutral-500">
              {diagnosticsRetentionPolicy ? `${diagnosticsRetentionPolicy.policy_id} v${diagnosticsRetentionPolicy.policy_version}` : 'Policy unavailable'}
            </div>
          </div>
          <button
            type="submit"
            class="inline-flex items-center gap-2 rounded border border-cyan-800 bg-cyan-950 px-3 py-1.5 text-sm text-cyan-100 transition-colors hover:border-cyan-600 focus-visible:outline focus-visible:outline-2 focus-visible:outline-cyan-400 disabled:opacity-50"
            disabled={loading || savingDiagnosticsRetention || !diagnosticsRetentionPolicy}
          >
            <Save size={14} aria-hidden="true" />
            Save Retention
          </button>
        </div>

        {#if diagnosticsRetentionError}
          <div class="mt-3 rounded border border-red-900 bg-red-950/40 px-3 py-2 text-sm text-red-200" role="alert">
            {diagnosticsRetentionError}
          </div>
        {:else if diagnosticsRetentionMessage}
          <div class="mt-3 rounded border border-emerald-900 bg-emerald-950/30 px-3 py-2 text-sm text-emerald-200" role="status">
            {diagnosticsRetentionMessage}
          </div>
        {/if}

        <div class="mt-4 grid gap-3 md:grid-cols-[12rem_1fr]">
          <div>
            <label for="settings-diagnostics-retention-days" class="mb-2 block text-xs uppercase text-neutral-500">
              Days
            </label>
            <input
              id="settings-diagnostics-retention-days"
              type="number"
              min="1"
              inputmode="numeric"
              bind:value={diagnosticsRetentionDays}
              class="w-full rounded border border-neutral-700 bg-neutral-900 px-3 py-1.5 font-mono text-xs text-neutral-100 placeholder:text-neutral-600 focus:border-cyan-500 focus:outline-none"
            />
          </div>
          <div>
            <label for="settings-diagnostics-retention-explanation" class="mb-2 block text-xs uppercase text-neutral-500">
              Explanation
            </label>
            <textarea
              id="settings-diagnostics-retention-explanation"
              rows="3"
              bind:value={diagnosticsRetentionExplanation}
              class="w-full resize-none rounded border border-neutral-700 bg-neutral-900 px-3 py-1.5 text-xs text-neutral-100 placeholder:text-neutral-600 focus:border-cyan-500 focus:outline-none"
            ></textarea>
          </div>
        </div>

        {#if diagnosticsRetentionSettingRows.length > 0}
          <dl class="mt-4 grid gap-2 text-xs md:grid-cols-2">
            {#each diagnosticsRetentionSettingRows as row (row.label)}
              <div class="rounded border border-neutral-800 bg-neutral-900/40 px-3 py-2">
                <dt class="text-neutral-500">{row.label}</dt>
                <dd class={`mt-1 truncate text-neutral-200 ${row.mono ? 'font-mono' : ''}`} title={row.value}>
                  {row.value}
                </dd>
              </div>
            {/each}
          </dl>
        {/if}
      </form>

      <form
        class="px-4 py-4"
        onsubmit={(event) => {
          event.preventDefault();
          void saveArtifactFormats();
        }}
      >
        <div class="flex flex-wrap items-start justify-between gap-3">
          <div>
            <h2 class="text-sm font-semibold text-neutral-100">Artifact Format Defaults</h2>
            <div class="mt-1 text-xs text-neutral-500">
              Image {capabilityCount(capabilities?.image_formats)} / Audio {capabilityCount(capabilities?.audio_formats)} / Video {capabilityCount(capabilities?.video_formats)} / 3D {capabilityCount(capabilities?.three_d_formats)}
            </div>
          </div>
          <button
            type="submit"
            class="inline-flex items-center gap-2 rounded border border-cyan-800 bg-cyan-950 px-3 py-1.5 text-sm text-cyan-100 transition-colors hover:border-cyan-600 focus-visible:outline focus-visible:outline-2 focus-visible:outline-cyan-400 disabled:opacity-50"
            disabled={loading || savingFormats || !capabilities}
          >
            <Save size={14} aria-hidden="true" />
            Save Formats
          </button>
        </div>

        {#if formatError}
          <div class="mt-3 rounded border border-red-900 bg-red-950/40 px-3 py-2 text-sm text-red-200" role="alert">
            {formatError}
          </div>
        {:else if formatMessage}
          <div class="mt-3 rounded border border-emerald-900 bg-emerald-950/30 px-3 py-2 text-sm text-emerald-200" role="status">
            {formatMessage}
          </div>
        {/if}

        <div class="mt-4 grid gap-3 2xl:grid-cols-2">
          <section class="rounded border border-neutral-800 bg-neutral-900/40 p-3">
            <div class="flex items-center justify-between gap-3">
              <h3 class="text-sm font-medium text-neutral-100">Image</h3>
              <span class="text-xs text-neutral-500">{formatRangeLabel(selectedImageFormat?.quality_min_percent, selectedImageFormat?.quality_max_percent, '%')}</span>
            </div>
            <div class="mt-3 grid gap-3 md:grid-cols-3">
              <div>
                <label for="settings-image-format" class="mb-2 block text-xs uppercase text-neutral-500">
                  Format
                </label>
                <select
                  id="settings-image-format"
                  bind:value={formatDraft.image.format_id}
                  onchange={normalizeImageFormatDraft}
                  class="w-full rounded border border-neutral-700 bg-neutral-900 px-3 py-1.5 text-xs text-neutral-100 focus:border-cyan-500 focus:outline-none"
                >
                  {#each imageFormatOptions as option (option.value)}
                    <option value={option.value}>{option.label}</option>
                  {/each}
                </select>
              </div>
              <div>
                <label for="settings-image-quality" class="mb-2 block text-xs uppercase text-neutral-500">
                  Quality percent
                </label>
                <input
                  id="settings-image-quality"
                  type="number"
                  min={selectedImageFormat?.quality_min_percent ?? 0}
                  max={selectedImageFormat?.quality_max_percent ?? undefined}
                  inputmode="numeric"
                  bind:value={formatDraft.image.quality_percent}
                  class="w-full rounded border border-neutral-700 bg-neutral-900 px-3 py-1.5 font-mono text-xs text-neutral-100 focus:border-cyan-500 focus:outline-none"
                />
              </div>
              <div>
                <label for="settings-image-profile" class="mb-2 block text-xs uppercase text-neutral-500">
                  Color profile
                </label>
                <select
                  id="settings-image-profile"
                  bind:value={formatDraft.image.color_profile_id}
                  class="w-full rounded border border-neutral-700 bg-neutral-900 px-3 py-1.5 text-xs text-neutral-100 focus:border-cyan-500 focus:outline-none"
                >
                  {#each imageColorProfileOptions as profile (profile)}
                    <option value={profile}>{profile}</option>
                  {/each}
                </select>
              </div>
            </div>
          </section>

          <section class="rounded border border-neutral-800 bg-neutral-900/40 p-3">
            <div class="flex items-center justify-between gap-3">
              <h3 class="text-sm font-medium text-neutral-100">Audio</h3>
              <span class="text-xs text-neutral-500">{formatRangeLabel(selectedAudioFormat?.bitrate_min_kbps, selectedAudioFormat?.bitrate_max_kbps, ' kbps')}</span>
            </div>
            <div class="mt-3 grid gap-3 md:grid-cols-3">
              <div>
                <label for="settings-audio-container" class="mb-2 block text-xs uppercase text-neutral-500">
                  Container
                </label>
                <select
                  id="settings-audio-container"
                  bind:value={formatDraft.audio.container_id}
                  onchange={normalizeAudioFormatDraft}
                  class="w-full rounded border border-neutral-700 bg-neutral-900 px-3 py-1.5 text-xs text-neutral-100 focus:border-cyan-500 focus:outline-none"
                >
                  {#each audioFormatOptions as option (option.value)}
                    <option value={option.value}>{option.label}</option>
                  {/each}
                </select>
              </div>
              <div>
                <label for="settings-audio-codec" class="mb-2 block text-xs uppercase text-neutral-500">
                  Codec
                </label>
                <select
                  id="settings-audio-codec"
                  bind:value={formatDraft.audio.codec_id}
                  class="w-full rounded border border-neutral-700 bg-neutral-900 px-3 py-1.5 text-xs text-neutral-100 focus:border-cyan-500 focus:outline-none"
                >
                  {#each audioCodecOptions as codec (codec)}
                    <option value={codec}>{codec}</option>
                  {/each}
                </select>
              </div>
              <div>
                <label for="settings-audio-bitrate" class="mb-2 block text-xs uppercase text-neutral-500">
                  Bitrate kbps
                </label>
                <input
                  id="settings-audio-bitrate"
                  type="number"
                  min={selectedAudioFormat?.bitrate_min_kbps ?? 0}
                  max={selectedAudioFormat?.bitrate_max_kbps ?? undefined}
                  inputmode="numeric"
                  bind:value={formatDraft.audio.bitrate_kbps}
                  class="w-full rounded border border-neutral-700 bg-neutral-900 px-3 py-1.5 font-mono text-xs text-neutral-100 focus:border-cyan-500 focus:outline-none"
                />
              </div>
            </div>
          </section>

          <section class="rounded border border-neutral-800 bg-neutral-900/40 p-3">
            <div class="flex items-center justify-between gap-3">
              <h3 class="text-sm font-medium text-neutral-100">Video</h3>
              <span class="text-xs text-neutral-500">{formatRangeLabel(selectedVideoFormat?.crf_min, selectedVideoFormat?.crf_max)}</span>
            </div>
            <div class="mt-3 grid gap-3 md:grid-cols-4">
              <div>
                <label for="settings-video-container" class="mb-2 block text-xs uppercase text-neutral-500">
                  Container
                </label>
                <select
                  id="settings-video-container"
                  bind:value={formatDraft.video.container_id}
                  onchange={normalizeVideoFormatDraft}
                  class="w-full rounded border border-neutral-700 bg-neutral-900 px-3 py-1.5 text-xs text-neutral-100 focus:border-cyan-500 focus:outline-none"
                >
                  {#each videoFormatOptions as option (option.value)}
                    <option value={option.value}>{option.label}</option>
                  {/each}
                </select>
              </div>
              <div>
                <label for="settings-video-codec" class="mb-2 block text-xs uppercase text-neutral-500">
                  Codec
                </label>
                <select
                  id="settings-video-codec"
                  bind:value={formatDraft.video.codec_id}
                  class="w-full rounded border border-neutral-700 bg-neutral-900 px-3 py-1.5 text-xs text-neutral-100 focus:border-cyan-500 focus:outline-none"
                >
                  {#each videoCodecOptions as codec (codec)}
                    <option value={codec}>{codec}</option>
                  {/each}
                </select>
              </div>
              <div>
                <label for="settings-video-crf" class="mb-2 block text-xs uppercase text-neutral-500">
                  CRF
                </label>
                <input
                  id="settings-video-crf"
                  type="number"
                  min={selectedVideoFormat?.crf_min ?? 0}
                  max={selectedVideoFormat?.crf_max ?? undefined}
                  inputmode="numeric"
                  bind:value={formatDraft.video.crf}
                  class="w-full rounded border border-neutral-700 bg-neutral-900 px-3 py-1.5 font-mono text-xs text-neutral-100 focus:border-cyan-500 focus:outline-none"
                />
              </div>
              <div>
                <label for="settings-video-depth" class="mb-2 block text-xs uppercase text-neutral-500">
                  Bit depth
                </label>
                <select
                  id="settings-video-depth"
                  bind:value={formatDraft.video.bit_depth}
                  class="w-full rounded border border-neutral-700 bg-neutral-900 px-3 py-1.5 text-xs text-neutral-100 focus:border-cyan-500 focus:outline-none"
                >
                  {#each videoBitDepthOptions as bitDepth (bitDepth)}
                    <option value={bitDepth}>{bitDepth}</option>
                  {/each}
                </select>
              </div>
            </div>
          </section>

          <section class="rounded border border-neutral-800 bg-neutral-900/40 p-3">
            <h3 class="text-sm font-medium text-neutral-100">3D</h3>
            <div class="mt-3 max-w-sm">
              <label for="settings-three-d-format" class="mb-2 block text-xs uppercase text-neutral-500">
                Format
              </label>
              <select
                id="settings-three-d-format"
                bind:value={formatDraft.three_d.format_id}
                class="w-full rounded border border-neutral-700 bg-neutral-900 px-3 py-1.5 text-xs text-neutral-100 focus:border-cyan-500 focus:outline-none"
              >
                {#each threeDFormatOptions as option (option.value)}
                  <option value={option.value}>{option.label}</option>
                {/each}
              </select>
            </div>
          </section>
        </div>
      </form>

      <section class="border-t border-neutral-900 px-4 py-4">
        <div class="flex flex-wrap items-start justify-between gap-3">
          <div>
            <h2 class="text-sm font-semibold text-neutral-100">Managed Media Dependencies</h2>
            <div class="mt-1 text-xs text-neutral-500">
              FFmpeg, ocioconvert, oiiotool, and OpenColorIO status/actions from backend commands
            </div>
          </div>
          <button
            type="button"
            class="inline-flex items-center gap-2 rounded border border-neutral-700 px-3 py-1.5 text-sm text-neutral-300 transition-colors hover:border-neutral-500 hover:text-neutral-100 focus-visible:outline focus-visible:outline-2 focus-visible:outline-cyan-400 disabled:opacity-50"
            onclick={() => refreshSettingsPage()}
            disabled={loading || managedMediaActionKey !== null}
          >
            <RefreshCw size={14} aria-hidden="true" class={loading ? 'animate-spin' : ''} />
            Refresh Dependencies
          </button>
        </div>

        {#if managedMediaError}
          <div class="mt-3 rounded border border-red-900 bg-red-950/40 px-3 py-2 text-sm text-red-200" role="alert">
            {managedMediaError}
          </div>
        {:else if managedMediaMessage}
          <div class="mt-3 rounded border border-emerald-900 bg-emerald-950/30 px-3 py-2 text-sm text-emerald-200" role="status">
            {managedMediaMessage}
          </div>
        {/if}

        <div class="mt-4 grid gap-3">
          {#each managedMediaDependencies as dependency (dependency.id)}
            {@const presentation = formatManagedMediaDependencyStatus(dependency)}
            {@const selectedVersionOptions = managedMediaVersionOptions(dependency)}
            {@const actionBusy = managedMediaActionInProgress(dependency.id)}
            <section class="rounded border border-neutral-800 bg-neutral-900/40 p-3">
              <div class="flex flex-wrap items-start justify-between gap-3">
                <div class="min-w-0">
                  <div class="flex flex-wrap items-center gap-2">
                    <h3 class="text-sm font-medium text-neutral-100">{dependency.display_name}</h3>
                    <span class={`inline-flex rounded border px-2 py-0.5 text-xs ${presentation.statusClass}`}>
                      {presentation.readinessLabel}
                    </span>
                    <span class="inline-flex rounded border border-neutral-800 bg-neutral-950 px-2 py-0.5 text-xs text-neutral-300">
                      {presentation.categoryLabel}
                    </span>
                  </div>
                  <div class="mt-1 text-xs text-neutral-500">
                    {dependency.catalog.source.owner}/{dependency.catalog.source.project} / {presentation.packageLabel} / {dependency.catalog.platform_key}
                  </div>
                </div>
                <button
                  type="button"
                  class="inline-flex items-center gap-2 rounded border border-neutral-700 px-2.5 py-1.5 text-xs text-neutral-300 transition-colors hover:border-neutral-500 hover:text-neutral-100 focus-visible:outline focus-visible:outline-2 focus-visible:outline-cyan-400 disabled:opacity-50"
                  onclick={() => refreshManagedMediaDependency(dependency.id)}
                  disabled={loading || actionBusy}
                >
                  <RefreshCw size={13} aria-hidden="true" class={actionBusy ? 'animate-spin' : ''} />
                  Refresh
                </button>
              </div>

              <dl class="mt-3 grid gap-2 text-xs md:grid-cols-3 xl:grid-cols-4">
                {#each buildManagedMediaDependencyRows(dependency) as row (row.label)}
                  <div class="min-w-0 border-b border-neutral-800 pb-2">
                    <dt class="text-neutral-500">{row.label}</dt>
                    <dd class:font-mono={row.mono} class="mt-1 truncate text-neutral-200" title={row.value}>
                      {row.value}
                    </dd>
                  </div>
                {/each}
              </dl>

              {#if dependency.missing_files.length > 0}
                <div class="mt-3 rounded border border-amber-900 bg-amber-950/30 px-3 py-2 text-xs text-amber-200">
                  Missing: {dependency.missing_files.join(', ')}
                </div>
              {/if}

              <div class="mt-3 grid gap-3 xl:grid-cols-[minmax(0,1fr)_minmax(18rem,24rem)]">
                <div class="grid gap-3 md:grid-cols-2">
                  <div>
                    <label for={`managed-media-version-${dependency.id}`} class="mb-2 block text-xs uppercase text-neutral-500">
                      Version
                    </label>
                    <input
                      id={`managed-media-version-${dependency.id}`}
                      type="text"
                      bind:value={managedMediaDrafts[dependency.id].version}
                      class="w-full rounded border border-neutral-700 bg-neutral-900 px-3 py-1.5 font-mono text-xs text-neutral-100 placeholder:text-neutral-600 focus:border-cyan-500 focus:outline-none"
                      placeholder={dependency.catalog.version}
                    />
                  </div>
                  <div>
                    <label for={`managed-media-staging-${dependency.id}`} class="mb-2 block text-xs uppercase text-neutral-500">
                      Staging directory
                    </label>
                    <input
                      id={`managed-media-staging-${dependency.id}`}
                      type="text"
                      bind:value={managedMediaDrafts[dependency.id].staging_dir}
                      class="w-full rounded border border-neutral-700 bg-neutral-900 px-3 py-1.5 font-mono text-xs text-neutral-100 placeholder:text-neutral-600 focus:border-cyan-500 focus:outline-none"
                      placeholder="/path/to/staged/package"
                    />
                  </div>
                </div>

                <div>
                  <label for={`managed-media-selected-${dependency.id}`} class="mb-2 block text-xs uppercase text-neutral-500">
                    Installed version
                  </label>
                  <select
                    id={`managed-media-selected-${dependency.id}`}
                    bind:value={managedMediaDrafts[dependency.id].selected_version}
                    class="w-full rounded border border-neutral-700 bg-neutral-900 px-3 py-1.5 text-xs text-neutral-100 focus:border-cyan-500 focus:outline-none"
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
                </div>
              </div>

              <div class="mt-3 flex flex-wrap gap-2">
                <button
                  type="button"
                  class="inline-flex items-center gap-2 rounded border border-cyan-800 bg-cyan-950 px-2.5 py-1.5 text-xs text-cyan-100 transition-colors hover:border-cyan-600 focus-visible:outline focus-visible:outline-2 focus-visible:outline-cyan-400 disabled:opacity-50"
                  onclick={() => installManagedMediaDependency(dependency)}
                  disabled={loading || actionBusy}
                >
                  <Upload size={13} aria-hidden="true" />
                  Install From Staging
                </button>
                <button
                  type="button"
                  class="inline-flex items-center gap-2 rounded border border-neutral-700 px-2.5 py-1.5 text-xs text-neutral-300 transition-colors hover:border-neutral-500 hover:text-neutral-100 focus-visible:outline focus-visible:outline-2 focus-visible:outline-cyan-400 disabled:opacity-50"
                  onclick={() => selectManagedMediaDependency(dependency)}
                  disabled={loading || actionBusy || selectedVersionOptions.length === 0}
                >
                  <Save size={13} aria-hidden="true" />
                  Select
                </button>
                <button
                  type="button"
                  class="inline-flex items-center gap-2 rounded border border-neutral-700 px-2.5 py-1.5 text-xs text-neutral-300 transition-colors hover:border-neutral-500 hover:text-neutral-100 focus-visible:outline focus-visible:outline-2 focus-visible:outline-cyan-400 disabled:opacity-50"
                  onclick={() => setDefaultManagedMediaDependency(dependency)}
                  disabled={loading || actionBusy || selectedVersionOptions.length === 0}
                >
                  <Save size={13} aria-hidden="true" />
                  Set Default
                </button>
                <button
                  type="button"
                  class="inline-flex items-center gap-2 rounded border border-emerald-800 bg-emerald-950/50 px-2.5 py-1.5 text-xs text-emerald-100 transition-colors hover:border-emerald-600 focus-visible:outline focus-visible:outline-2 focus-visible:outline-cyan-400 disabled:opacity-50"
                  onclick={() => activateManagedMediaDependency(dependency)}
                  disabled={loading || actionBusy || selectedVersionOptions.length === 0}
                >
                  <Play size={13} aria-hidden="true" />
                  Activate
                </button>
                <button
                  type="button"
                  class="inline-flex items-center gap-2 rounded border border-neutral-700 px-2.5 py-1.5 text-xs text-neutral-300 transition-colors hover:border-neutral-500 hover:text-neutral-100 focus-visible:outline focus-visible:outline-2 focus-visible:outline-cyan-400 disabled:opacity-50"
                  onclick={() => clearManagedMediaDependencySelection(dependency)}
                  disabled={loading || actionBusy || !dependency.selection.selected_version}
                >
                  <X size={13} aria-hidden="true" />
                  Clear Selection
                </button>
                <button
                  type="button"
                  class="inline-flex items-center gap-2 rounded border border-red-900 bg-red-950/40 px-2.5 py-1.5 text-xs text-red-100 transition-colors hover:border-red-700 focus-visible:outline focus-visible:outline-2 focus-visible:outline-cyan-400 disabled:opacity-50"
                  onclick={() => removeManagedMediaDependency(dependency)}
                  disabled={loading || actionBusy || selectedVersionOptions.length === 0}
                >
                  <Trash2 size={13} aria-hidden="true" />
                  Remove Version
                </button>
              </div>
            </section>
          {/each}
        </div>
      </section>
    </div>

    <aside class="min-h-0 overflow-auto border-t border-neutral-800 bg-neutral-950 xl:border-l xl:border-t-0">
      <section class="border-b border-neutral-900 px-4 py-4">
        <div class="flex items-center gap-2">
          <Settings size={15} aria-hidden="true" class="text-cyan-300" />
          <h2 class="text-sm font-semibold text-neutral-100">ArtifactStore Policy</h2>
        </div>
        <dl class="mt-3 space-y-2">
          {#each policyRows as row (row.label)}
            <div class="grid grid-cols-[9rem_1fr] gap-2 text-xs">
              <dt class="text-neutral-500">{row.label}</dt>
              <dd class:font-mono={row.mono} class="min-w-0 truncate text-neutral-200" title={row.value}>
                {row.value}
              </dd>
            </div>
          {/each}
        </dl>
      </section>

      <section class="border-b border-neutral-900 px-4 py-4">
        <h2 class="text-sm font-semibold text-neutral-100">Diagnostics Retention</h2>
        <dl class="mt-3 space-y-2">
          {#each diagnosticsRetentionPolicyRows as row (row.label)}
            <div class="grid grid-cols-[9rem_1fr] gap-2 text-xs">
              <dt class="text-neutral-500">{row.label}</dt>
              <dd class:font-mono={row.mono} class="min-w-0 truncate text-neutral-200" title={row.value}>
                {row.value}
              </dd>
            </div>
          {/each}
        </dl>
      </section>

      <section class="px-4 py-4">
        <h2 class="text-sm font-semibold text-neutral-100">Capabilities</h2>
        <div class="mt-3 grid gap-2 text-xs">
          <div class="flex items-center justify-between border-b border-neutral-900 pb-2">
            <span class="text-neutral-500">Image</span>
            <span class="font-mono text-neutral-200">{capabilityCount(capabilities?.image_formats)}</span>
          </div>
          <div class="flex items-center justify-between border-b border-neutral-900 pb-2">
            <span class="text-neutral-500">Audio</span>
            <span class="font-mono text-neutral-200">{capabilityCount(capabilities?.audio_formats)}</span>
          </div>
          <div class="flex items-center justify-between border-b border-neutral-900 pb-2">
            <span class="text-neutral-500">Video</span>
            <span class="font-mono text-neutral-200">{capabilityCount(capabilities?.video_formats)}</span>
          </div>
          <div class="flex items-center justify-between">
            <span class="text-neutral-500">3D</span>
            <span class="font-mono text-neutral-200">{capabilityCount(capabilities?.three_d_formats)}</span>
          </div>
        </div>
      </section>
    </aside>
  </div>
</section>
