"""Explicit full-worker qualification of the local synthetic CPU fixtures."""
import json
from pathlib import Path
import sys

import torch

ROOT = Path(__file__).parent
sys.path.insert(0, str(ROOT.parents[2] / "torch"))
import worker


def qualify(model_seed):
    directory = ROOT / f"model-{model_seed}"
    worker.load_model(str(directory), device="cpu", trust_remote_code=False, local_files_only=True)
    assert type(worker._model).__name__ == "GPT2LMHeadModel"
    assert next(worker._model.parameters()).device.type == "cpu"
    observed = []

    def forward_inputs(_model, args, kwargs):
        ids = args[0] if args else kwargs["input_ids"]
        observed.append(ids[0].tolist())

    hook = worker._model.register_forward_pre_hook(forward_inputs, with_kwargs=True)
    try:
        for case in json.loads((directory / "golden.json").read_text()):
            assert worker._format_prompt(case["prompt"], case["system_prompt"]) == case["formatted_prompt"]
            for streaming in [False, True]:
                observed.clear()
                before = torch.get_rng_state().clone()
                result = (worker.generate_tokens if streaming else worker.generate)(
                    case["prompt"], system_prompt=case["system_prompt"], max_tokens=4,
                    temperature=0.8, top_p=1.0, top_k=4, min_new_tokens=4,
                    repetition_penalty=1.0, seed=case["seed"], stop_strings=["NEVER_STOP"],
                )
                text = "".join(chunk["text"] for chunk in result) if streaming else result
                assert text == case["text"], (model_seed, case, streaming, text)
                assert observed and observed[0] == case["prompt_token_ids"]
                assert torch.equal(before, torch.get_rng_state())
        print(f"model-{model_seed}: 16 full-worker CPU requests match native oracles, actual prompt forwards and private RNG")
    finally:
        hook.remove()
        worker.unload_model()


if __name__ == "__main__":
    torch.set_num_threads(1)
    for model_seed in [11, 37]:
        qualify(model_seed)
