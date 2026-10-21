import { readFile, readlink } from 'node:fs/promises';
import path from 'node:path';

export const MAX_BROWSER_STDERR_BYTES = 1_048_576;

const checkpoints = new Set(['launch_deadline', 'cleanup_observation']);
const portErrors = new Set(['ENOENT', 'EACCES', 'EPERM', 'ENOTDIR', 'EISDIR', 'EINVAL', 'EIO', 'EMFILE', 'ENFILE']);
const signals = new Set([
  'SIGHUP', 'SIGINT', 'SIGQUIT', 'SIGILL', 'SIGTRAP', 'SIGABRT', 'SIGIOT',
  'SIGBUS', 'SIGFPE', 'SIGKILL', 'SIGUSR1', 'SIGSEGV', 'SIGUSR2', 'SIGPIPE',
  'SIGALRM', 'SIGTERM', 'SIGSTKFLT', 'SIGCHLD', 'SIGCONT', 'SIGSTOP', 'SIGTSTP',
  'SIGTTIN', 'SIGTTOU', 'SIGURG', 'SIGXCPU', 'SIGXFSZ', 'SIGVTALRM', 'SIGPROF',
  'SIGWINCH', 'SIGIO', 'SIGPOLL', 'SIGPWR', 'SIGSYS',
]);
const processStates = new Set(['R', 'S', 'D', 'Z', 'T', 't', 'X', 'x', 'K', 'W', 'P', 'I']);
const shells = new Set(['sh', 'bash', 'dash', 'ash', 'ksh', 'zsh', 'fish']);
const chromeExecutables = new Set(['chrome', 'google-chrome', 'google-chrome-stable', 'chromium', 'chromium-browser']);

function field(object, key) {
  try { return object?.[key]; }
  catch { return undefined; }
}

// The caller already owns the exit listener and its original absolute deadline.
// This observes completion only; it neither signals nor acquires cleanup authority.
export async function waitForBrowserParentExit(exitedPromise, absoluteDeadline) {
  const remaining = Number.isFinite(absoluteDeadline) ? Math.max(0, absoluteDeadline - Date.now()) : 0;
  if (remaining === 0) return;
  let timer;
  try {
    await Promise.race([
      exitedPromise,
      new Promise(resolve => { timer = setTimeout(resolve, remaining); }),
    ]);
  } finally {
    clearTimeout(timer);
  }
}

function parseProcessStat(text, expectedPid) {
  if (text.length > 4096) return null;
  const prefix = /^(\d+) \(/.exec(text);
  const end = text.lastIndexOf(')');
  if (!prefix || Number(prefix[1]) !== expectedPid || end < prefix[0].length) return null;
  // comm can contain whitespace and closing parentheses; never split or emit it.
  const fields = text.slice(end + 1).trim().split(/\s+/);
  if (!processStates.has(fields[0]) || !/^\d+$/.test(fields[2] ?? '')) return null;
  const group = Number(fields[2]);
  if (!Number.isSafeInteger(group) || group <= 0) return null;
  return { state: fields[0], groupMatches: group === expectedPid };
}

// Observe only the known direct child's PID. These snapshots cannot prove that
// a process group has drained, that its parent was reaped, or that cleanup is safe.
export async function observeBrowserProcess(browser, facts) {
  const checkpoint = field(facts, 'checkpoint');
  const spawned = field(facts, 'spawned');
  const portError = field(facts, 'portError');
  const stderrBytes = field(facts, 'stderrBytes');
  const stderrSaturated = field(facts, 'stderrSaturated');
  const rawExitCode = field(browser, 'exitCode');
  const rawSignal = field(browser, 'signalCode');
  const exitCode = Number.isInteger(rawExitCode) && rawExitCode >= 0 && rawExitCode <= 255 ? rawExitCode : null;
  const signal = signals.has(rawSignal) ? rawSignal : null;
  const validStderrBytes = Number.isSafeInteger(stderrBytes) && stderrBytes >= 0;
  let procState = 'unavailable';
  let groupMatches = null;
  let executableKind = 'unavailable';
  const pid = field(browser, 'pid');
  if (Number.isSafeInteger(pid) && pid > 0 && pid <= 2_147_483_647) {
    try {
      const stat = parseProcessStat(await readFile(`/proc/${pid}/stat`, 'utf8'), pid);
      if (stat) {
        procState = stat.state;
        groupMatches = stat.groupMatches;
      }
    } catch { /* Missing or inaccessible observations carry no process authority. */ }
    try {
      const executable = path.basename(await readlink(`/proc/${pid}/exe`));
      executableKind = shells.has(executable) ? 'shell' : chromeExecutables.has(executable) ? 'chrome' : 'other';
    } catch { /* Never expose paths or filesystem exception payloads. */ }
  }
  return {
    event: 'image_browser_process',
    checkpoint: checkpoints.has(checkpoint) ? checkpoint : 'unavailable',
    spawned: spawned === true,
    parent_exit_observed: exitCode !== null || signal !== null,
    exit_code: exitCode,
    signal,
    proc_state: procState,
    group_matches: groupMatches,
    executable_kind: executableKind,
    port_read_error: portError === null ? 'none' : portErrors.has(portError) ? portError : 'unavailable',
    stderr_bytes: validStderrBytes ? Math.min(stderrBytes, MAX_BROWSER_STDERR_BYTES) : 'unavailable',
    stderr_saturated: stderrSaturated === true || (validStderrBytes && stderrBytes > MAX_BROWSER_STDERR_BYTES),
  };
}
