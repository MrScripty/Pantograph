import { spawn } from 'node:child_process';
import process from 'node:process';
import { clearTimeout, setTimeout } from 'node:timers';
import { closeSync, openSync, readFileSync, readdirSync } from 'node:fs';

function executingGroupMembers(group) {
  return readdirSync('/proc').filter(name => /^\d+$/.test(name)).filter(name => {
    try {
      const stat = readFileSync(`/proc/${name}/stat`, 'utf8');
      const fields = stat.slice(stat.lastIndexOf(') ') + 2).split(' ');
      return Number(fields[2]) === group && !['Z', 'X'].includes(fields[0]);
    } catch (error) {
      if (error.code === 'ENOENT' || error.code === 'ESRCH') return false;
      throw error;
    }
  });
}

async function awaitGroupTermination(group) {
  const deadline = Date.now() + 2_000;
  while (executingGroupMembers(group).length) {
    if (Date.now() >= deadline) throw new Error('child_group_cleanup_incomplete');
    await new Promise(resolve => setTimeout(resolve, 10));
  }
}

// This CPU example is qualified on Linux. Own the process group so deadlines
// cannot leave Cargo compilers or native children behind. Await close after kill.
export async function command(executable, args, { cwd, env = process.env, log, timeoutMs = 60_000 } = {}) {
  const stdout = openSync(`${log}.stdout`, 'wx');
  let stderr;
  let timer;
  let escalation;
  let escalationComplete;
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
    const beginTermination = () => {
      if (escalationComplete) return;
      kill('SIGTERM');
      escalationComplete = new Promise(resolve => {
        escalation = setTimeout(() => { kill('SIGKILL'); resolve(); }, 1_000);
      });
    };
    timer = setTimeout(() => {
      timedOut = true;
      beginTermination();
    }, timeoutMs);
    const outcome = await new Promise((resolve, reject) => {
      child.once('error', reject);
      child.once('close', (code, signal) => resolve({ code, signal }));
    });
    clearTimeout(timer);
    const outlivedLeader = executingGroupMembers(child.pid).length > 0;
    if (timedOut || outlivedLeader) {
      // A leader can exit before its descendants. Its close event must not
      // cancel escalation or publish failure while those descendants execute.
      beginTermination();
      await escalationComplete;
      await awaitGroupTermination(child.pid);
    }
    if (timedOut || outcome.code !== 0 || outlivedLeader) {
      const error = new Error(timedOut ? 'child_deadline_exceeded'
        : outcome.code !== 0 ? 'child_execution_failed' : 'child_group_outlived_leader');
      error.details = { executable, args, ...outcome, timedOut, outlivedLeader, log };
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
