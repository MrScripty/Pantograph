"""Generate tiny untrained local GPT-2 fixtures and native CPU text oracles.

Uses installed official Transformers/Tokenizers classes; no from_pretrained,
network access, external weights or executable model code are required.
"""
import hashlib
import json
from pathlib import Path
import string

import torch
import transformers
from tokenizers import Tokenizer, decoders, models, pre_tokenizers
from transformers import GPT2Config, GPT2LMHeadModel, PreTrainedTokenizerFast


ROOT = Path(__file__).parent
TEMPLATE = (
    "{% for message in messages %}{{ message['role'] }}:{{ message['content'] }};"
    "{% endfor %}{% if add_generation_prompt %}assistant:{% endif %}"
)


def generate(model_seed):
    directory = ROOT / f"model-{model_seed}"
    directory.mkdir(exist_ok=True)
    chars = list(string.ascii_lowercase + " :;\n")
    vocabulary = {token: index for index, token in enumerate(
        ["[PAD]", "[EOS]", "[BOS]", "[UNK]", *chars])}
    raw = Tokenizer(models.WordLevel(vocabulary, unk_token="[UNK]"))
    raw.pre_tokenizer = pre_tokenizers.Split("", behavior="isolated")
    raw.decoder = decoders.Fuse()
    tokenizer = PreTrainedTokenizerFast(
        tokenizer_object=raw, pad_token="[PAD]", eos_token="[EOS]",
        bos_token="[BOS]", unk_token="[UNK]", chat_template=TEMPLATE,
        model_input_names=["input_ids", "attention_mask"],
    )
    with torch.random.fork_rng(devices=[]):
        torch.manual_seed(model_seed)
        model = GPT2LMHeadModel(GPT2Config(
            vocab_size=len(vocabulary), n_positions=96, n_ctx=96, n_embd=16,
            n_layer=1, n_head=2, resid_pdrop=0, embd_pdrop=0, attn_pdrop=0,
            bos_token_id=2, eos_token_id=1, pad_token_id=0,
        )).eval()
    model.save_pretrained(directory, safe_serialization=True)
    tokenizer.save_pretrained(directory)
    cases = []
    for prompt, system in [("hello world", "be brief"), ("alternate question", None)]:
        messages = ([{"role": "system", "content": system}] if system else [])
        messages.append({"role": "user", "content": prompt})
        formatted = tokenizer.apply_chat_template(
            messages, tokenize=False, add_generation_prompt=True)
        inputs = tokenizer(formatted, return_tensors="pt")
        for seed in [0, 42, 43, (1 << 64) - 1]:
            # Independent installed native generate oracle, with ambient RNG
            # isolated here. Production requests use their own generators.
            with torch.random.fork_rng(devices=[]), torch.no_grad():
                torch.manual_seed(seed)
                output = model.generate(
                    **inputs, max_new_tokens=4, min_new_tokens=4,
                    do_sample=True, temperature=0.8, top_p=1.0, top_k=4,
                    repetition_penalty=1.0,
                )
            generated = output[0, inputs.input_ids.shape[-1]:].tolist()
            cases.append({"prompt": prompt, "system_prompt": system, "seed": seed,
                          "formatted_prompt": formatted,
                          "prompt_token_ids": inputs.input_ids[0].tolist(),
                          "generated_token_ids": generated,
                          "text": tokenizer.decode(generated, skip_special_tokens=True)})
    (directory / "golden.json").write_text(json.dumps(cases, indent=2) + "\n")
    manifest = {
        "model_seed": model_seed, "synthetic_untrained": True,
        "torch": torch.__version__, "transformers": transformers.__version__,
        "parameter_count": sum(parameter.numel() for parameter in model.parameters()),
        "chat_template": TEMPLATE,
        "sha256": {file.name: hashlib.sha256(file.read_bytes()).hexdigest()
                   for file in sorted(directory.iterdir()) if file.name != "manifest.json"},
    }
    (directory / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
    print(directory.name, manifest["parameter_count"], "untrained parameters;", len(cases), "CPU oracles")


if __name__ == "__main__":
    torch.set_num_threads(1)
    for model_seed in [11, 37]:
        generate(model_seed)
