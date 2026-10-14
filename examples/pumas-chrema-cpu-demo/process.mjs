import { spawn } from 'node:child_process';
import process from 'node:process';
import { clearTimeout, setTimeout } from 'node:timers';
import { closeSync, openSync, readFileSync } from 'node:fs';

// This CPU example is qualified on Linux. Own the process group so deadlines
// cannot leave Cargo compilers or native children behind. Await close after kill.
export async function command(executable, args, { cwd, env = process.env, log, timeoutMs = 60_000 } = {}) {
  const stdout = openSync(`${log}.stdout`, 'wx');
  let stderr;
  let timer;
  let escalation;
  let timedOut = false;
  try {
    stderr = openSync(`${log}.stderr`, 'wx');
    const child = spawn(executable, args, { cwd, env, detached: true, stdio: ['ignore', stdout, stderr] });
    const kill = signal => {
      if (!child.pid) return;
      try { process.kill(-child.pid, signal); } catch (error) {
        if (error.code !== 'ESRCH') throw error;
      }
    };
    timer = setTimeout(() => {
      timedOut = true;
      kill('SIGTERM');
      escalation = setTimeout(() => kill('SIGKILL'), 1_000);
    }, timeoutMs);
    const outcome = await new Promise((resolve, reject) => {
      child.once('error', reject);
      child.once('close', (code, signal) => resolve({ code, signal }));
    });
    if (timedOut || outcome.code !== 0) {
      const error = new Error(timedOut ? 'child_deadline_exceeded' : 'child_execution_failed');
      error.details = { executable, args, ...outcome, timedOut, log };
      throw error;
    }
    return outcome;
  } finally {
    clearTimeout(timer);
    clearTimeout(escalation);
    closeSync(stdout);
    if (stderr !== undefined) closeSync(stderr);
  }
}

export function loggedJson(log) {
  const bytes = readFileSync(`${log}.stdout`);
  if (bytes.length > 8 * 1024 * 1024) throw new Error('child_response_too_large');
  return JSON.parse(bytes.toString('utf8'));
}
