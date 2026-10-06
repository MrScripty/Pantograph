//! Explicit CPU tensor/pipeline qualification, separate from unit-test stubs.
//! Uses a tiny randomly initialized UNet and supplied embeddings, no model files.
#![cfg(feature = "backend-pytorch")]

use std::ffi::CString;

use pyo3::prelude::*;

#[test]
#[ignore = "explicit native qualification requires real CPU Torch and Diffusers"]
fn actual_cpu_diffusers_guidance_matches_conditional_noise_oracle() {
    Python::with_gil(|py| {
        let source = CString::new(
            r#"
import inspect
import torch
import diffusers
from diffusers import DDIMScheduler, StableDiffusionPipeline, UNet2DConditionModel

torch.set_num_threads(1)
torch.manual_seed(42)
unet = UNet2DConditionModel(
    sample_size=4, in_channels=4, out_channels=4, layers_per_block=1,
    block_out_channels=(4,), down_block_types=("CrossAttnDownBlock2D",),
    up_block_types=("CrossAttnUpBlock2D",), cross_attention_dim=4,
    attention_head_dim=2, norm_num_groups=1,
)
pipeline = StableDiffusionPipeline(
    vae=None, text_encoder=None, tokenizer=None, unet=unet,
    scheduler=DDIMScheduler(num_train_timesteps=10, steps_offset=1, clip_sample=False),
    safety_checker=None, feature_extractor=None, requires_safety_checker=False,
)
pipeline.set_progress_bar_config(disable=True)
recorded = {}
def record_unet(module, inputs, result):
    recorded["raw_noise"] = result[0].detach().clone()
hook = unet.register_forward_hook(record_unet)
original_step = pipeline.scheduler.step
def record_step(noise, *args, **kwargs):
    recorded["guided_noise"] = noise.detach().clone()
    return original_step(noise, *args, **kwargs)
pipeline.scheduler.step = record_step
outputs = {}
try:
    default = inspect.signature(pipeline.__call__).parameters["guidance_scale"].default
    for requested in [None, -1.0, 0.0, 1.0, 2.5, 7.5]:
        recorded.clear()
        kwargs = {} if requested is None else {"guidance_scale": requested}
        result = pipeline(
            prompt_embeds=torch.ones(1, 2, 4), negative_prompt_embeds=torch.zeros(1, 2, 4),
            latents=torch.ones(1, 4, 4, 4), num_inference_steps=1,
            output_type="latent", **kwargs,
        )
        scale = default if requested is None else requested
        raw = recorded["raw_noise"]
        assert raw.device.type == "cpu"
        if scale > 1:
            assert pipeline.do_classifier_free_guidance
            assert raw.shape[0] == 2
            unconditional, conditional = raw.chunk(2)
            assert not torch.equal(unconditional, conditional)
            expected = unconditional + scale * (conditional - unconditional)
        else:
            assert not pipeline.do_classifier_free_guidance
            assert raw.shape[0] == 1
            expected = raw
        torch.testing.assert_close(recorded["guided_noise"], expected, rtol=0, atol=0)
        assert torch.isfinite(result.images).all()
        outputs[requested] = result.images.detach().clone()
    torch.testing.assert_close(outputs[-1.0], outputs[0.0], rtol=0, atol=0)
    torch.testing.assert_close(outputs[0.0], outputs[1.0], rtol=0, atol=0)
    if default == 7.5:
        torch.testing.assert_close(outputs[None], outputs[7.5], rtol=0, atol=0)
    assert not torch.equal(outputs[1.0], outputs[7.5])
finally:
    hook.remove()
print(f"native_guidance_torch={torch.__version__}; diffusers={diffusers.__version__}; device=cpu; cases=6; pretrained_models=0", flush=True)
"#,
        )
        .unwrap();
        py.run(&source, None, None)
            .expect("actual CPU denoising must preserve guidance semantics");
    });
}
