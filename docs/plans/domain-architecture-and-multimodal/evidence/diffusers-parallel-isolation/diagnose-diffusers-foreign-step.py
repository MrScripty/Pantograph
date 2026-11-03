"""Reproduce the frozen native scheduler test's global observer contamination."""
import hashlib
import re
import subprocess
import sys
import traceback
import types

import diffusers
import torch

FROZEN = '9e8cd64cc64fa7ccb8dfd6cd85d37bff2282dfb6'
TEST_PATH = 'crates/inference/tests/diffusers_guidance_native.rs'
WORKER_PATH = 'crates/inference/torch/worker_diffusion.py'
test_source = subprocess.check_output(['git', 'show', f'{FROZEN}:{TEST_PATH}']).decode()
worker_source = subprocess.check_output(['git', 'show', f'{FROZEN}:{WORKER_PATH}']).decode()
print(f'frozen_source={FROZEN}; native_test_sha256={hashlib.sha256(test_source.encode()).hexdigest()}', flush=True)
print(f'python={sys.version.split()[0]}; torch={torch.__version__}; diffusers={diffusers.__version__}; device=cpu; pretrained_models=0', flush=True)
worker = types.ModuleType('frozen_worker_diffusion')
sys.modules[worker.__name__] = worker
exec(compile(worker_source, WORKER_PATH, 'exec'), worker.__dict__)
section = test_source.split('fn actual_cpu_diffusers_scheduler_override_matches_explicit_pipeline_without_resident_mutation()', 1)[1]
script = re.search(r'CString::new\(r#"(.*?)"#\)', section, re.S).group(1)
# Preserve the actual helper, global hook, native forwards and original assertion.
# Force another thread's legitimate scheduler step while the global hook is live.
foreign = '''    if choice == 'ddim':
        from concurrent.futures import ThreadPoolExecutor
        foreign_scheduler = scheduler_type.from_config(original.config)
        foreign_scheduler.set_timesteps(2)
        def foreign_step():
            return foreign_scheduler.step(torch.zeros(1,4,4,4), foreign_scheduler.timesteps[0], torch.ones(1,4,4,4))
        with ThreadPoolExecutor(max_workers=1) as foreign_executor:
            foreign_executor.submit(foreign_step).result(timeout=10)
        print('foreign-thread DDIM step completed while frozen global observer was installed', flush=True)
'''
needle = '    scheduler_type.step = observed_step\n'
assert script.count(needle) == 1
script = script.replace(needle, needle + foreign)
needle = '        assert seen == [scheduler_type.__name__]*2\n'
assert script.count(needle) == 1
script = script.replace(needle, "        print('observed step calls:', seen, 'expected:', [scheduler_type.__name__]*2, flush=True)\n" + needle)
try:
    exec(compile(script, 'frozen-native-scheduler-with-controlled-foreign-step.py', 'exec'), {'call_diffusion_pipeline': worker.call_diffusion_pipeline})
except AssertionError:
    traceback.print_exc()
    print('Confirmed: the frozen exact-two-calls assertion counts an unrelated thread. Production helper and source are unchanged.', flush=True)
    raise SystemExit(1)
else:
    raise AssertionError('frozen contamination was not reproduced')
