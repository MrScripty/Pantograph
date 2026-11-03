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
        if let Err(error) = py.run(&source, None, None) {
            error.print(py);
            panic!("actual CPU denoising must preserve guidance semantics");
        }
    });
}

#[test]
#[ignore = "explicit native qualification requires real CPU Torch and Diffusers"]
fn actual_cpu_diffusers_multiple_images_match_worker_batch_shape_and_seed_order() {
    Python::with_gil(|py| {
        let locals = pyo3::types::PyDict::new(py);
        locals
            .set_item("worker_source", include_str!("../torch/worker.py"))
            .unwrap();
        let source = CString::new(r#"
import ast
import torch
import diffusers
from diffusers import DDIMScheduler, StableDiffusionPipeline, UNet2DConditionModel
functions = {'_batch_shared_generation_value', '_batch_pipeline_call_kwargs'}
module = ast.parse(worker_source)
module.body = [node for node in module.body if isinstance(node, ast.FunctionDef) and node.name in functions]
exec(compile(module, '<actual-worker-batch-kwargs>', 'exec'))
torch.set_num_threads(1)
torch.manual_seed(42)
unet = UNet2DConditionModel(sample_size=4, in_channels=4, out_channels=4, layers_per_block=1,
    block_out_channels=(4,), down_block_types=('CrossAttnDownBlock2D',), up_block_types=('CrossAttnUpBlock2D',),
    cross_attention_dim=4, attention_head_dim=2, norm_num_groups=1)
pipeline = StableDiffusionPipeline(vae=None, text_encoder=None, tokenizer=None, unet=unet,
    scheduler=DDIMScheduler(num_train_timesteps=10, steps_offset=1, clip_sample=False),
    safety_checker=None, feature_extractor=None, requires_safety_checker=False)
pipeline.set_progress_bar_config(disable=True)
for count in [1, 3]:
    members = [{'planned': {'generation_kwargs': {'prompt': str(seed), 'seed': seed,
        'num_images_per_prompt': count, 'num_inference_steps': 1, 'guidance_scale': 1.0}}} for seed in [42,43]]
    kwargs = _batch_pipeline_call_kwargs(members)
    generators = kwargs.pop('generator')
    assert [generator.initial_seed() for generator in generators] == [42]*count+[43]*count
    kwargs.pop('prompt')
    # Supplied embeddings avoid all tokenizer, text-model and weight dependencies.
    result = pipeline(prompt_embeds=torch.ones(2,2,4), generator=generators, output_type='latent', **kwargs)
    assert result.images.shape == (2*count,4,4,4)
    assert torch.isfinite(result.images).all()
    for group, seed in enumerate([42,43]):
        assert all(generators[group*count] is generator for generator in generators[group*count:(group+1)*count])
        solo = pipeline(prompt_embeds=torch.ones(1,2,4), generator=torch.Generator(device='cpu').manual_seed(seed),
            num_images_per_prompt=count, num_inference_steps=1, guidance_scale=1.0, output_type='latent')
        torch.testing.assert_close(result.images[group*count:(group+1)*count], solo.images, rtol=1e-5, atol=1e-6)
        if count > 1:
            assert not torch.equal(result.images[group*count], result.images[group*count+1])
    assert not torch.equal(result.images[0], result.images[count])
print(f'native_image_count_torch={torch.__version__}; diffusers={diffusers.__version__}; device=cpu; counts=1,3; pretrained_models=0', flush=True)
"#).unwrap();
        if let Err(error) = py.run(&source, Some(&locals), Some(&locals)) {
            error.print(py);
            panic!("actual worker kwargs and CPU pipeline preserve image count and seed order");
        }
    });
}

#[test]
#[ignore = "explicit native qualification requires real CPU Torch and Diffusers"]
fn actual_cpu_diffusers_scheduler_override_matches_explicit_pipeline_without_resident_mutation() {
    Python::with_gil(|py| {
        let module_source = CString::new(include_str!("../torch/worker_diffusion.py")).unwrap();
        let module = pyo3::types::PyModule::from_code(
            py,
            &module_source,
            c"worker_diffusion_native.py",
            c"worker_diffusion_native",
        )
        .unwrap();
        let locals = pyo3::types::PyDict::new(py);
        locals
            .set_item(
                "call_diffusion_pipeline",
                module.getattr("call_diffusion_pipeline").unwrap(),
            )
            .unwrap();
        let source = CString::new(r#"
import copy
import sys
import threading
from concurrent.futures import ThreadPoolExecutor
import torch
import diffusers
from diffusers import DDIMScheduler, EulerDiscreteScheduler, StableDiffusionPipeline, UNet2DConditionModel
torch.set_num_threads(1)
torch.manual_seed(42)
unet = UNet2DConditionModel(sample_size=4, in_channels=4, out_channels=4, layers_per_block=1,
    block_out_channels=(4,), down_block_types=('CrossAttnDownBlock2D',), up_block_types=('CrossAttnUpBlock2D',),
    cross_attention_dim=4, attention_head_dim=2, norm_num_groups=1)
pipeline = StableDiffusionPipeline(vae=None, text_encoder=None, tokenizer=None, unet=unet,
    scheduler=DDIMScheduler(num_train_timesteps=10, steps_offset=1, clip_sample=False),
    safety_checker=None, feature_extractor=None, requires_safety_checker=False)
pipeline.set_progress_bar_config(disable=True)
original = pipeline.scheduler
original_config = dict(pipeline.config)
outputs = {}
for choice, scheduler_type in [('ddim', DDIMScheduler), ('euler', EulerDiscreteScheduler)]:
    seen = []
    owned_schedulers = []
    foreign_calls = []
    step_code = scheduler_type.step.__code__
    owner_thread = threading.get_ident()
    def foreign_steps():
        scheduler = scheduler_type.from_config(original.config)
        scheduler.set_timesteps(2)
        sample = torch.ones(1,4,4,4)
        calls = 0
        for timestep in scheduler.timesteps:
            scheduler.scale_model_input(sample, timestep)
            sample = scheduler.step(torch.zeros_like(sample), timestep, sample).prev_sample
            calls += 1
        assert torch.isfinite(sample).all()
        return threading.get_ident(), calls
    def observe_step(frame, event, arg):
        if event == 'call' and frame.f_code is step_code:
            scheduler = frame.f_locals.get('self')
            if type(scheduler) is scheduler_type:
                assert threading.get_ident() == owner_thread
                assert scheduler is not original
                seen.append(type(scheduler).__name__)
                owned_schedulers.append(scheduler)
                if len(seen) == 1:
                    # Finish actual same-class steps in another thread while
                    # observation is installed. A class patch would count them.
                    with ThreadPoolExecutor(max_workers=1) as executor:
                        foreign_calls.append(executor.submit(foreign_steps).result())
    previous_profile = sys.getprofile()
    kwargs = dict(prompt_embeds=torch.ones(2,2,4), latents=torch.ones(4,4,4,4), num_images_per_prompt=2,
        num_inference_steps=2, guidance_scale=1.0, output_type='latent')
    sys.setprofile(observe_step)
    try:
        actual = call_diffusion_pipeline(pipeline, kwargs, choice)
    finally:
        sys.setprofile(previous_profile)
    assert sys.getprofile() is previous_profile
    assert seen == [scheduler_type.__name__]*2
    assert len(foreign_calls) == 1 and foreign_calls[0][0] != owner_thread
    assert foreign_calls[0][1] == 2
    assert owned_schedulers[0] is owned_schedulers[1]
    assert pipeline.scheduler is original and dict(pipeline.config) == original_config
    oracle = copy.copy(pipeline)
    oracle.scheduler = scheduler_type.from_config(original.config)
    assert oracle.unet is pipeline.unet
    expected = oracle(**kwargs)
    torch.testing.assert_close(actual.images, expected.images, rtol=0, atol=0)
    assert torch.isfinite(actual.images).all()
    outputs[choice] = actual.images
assert not torch.equal(outputs['ddim'], outputs['euler'])
kwargs = dict(prompt_embeds=torch.ones(1,2,4), latents=torch.ones(1,4,4,4),
    num_inference_steps=2, guidance_scale=1.0, output_type='latent')
implicit = call_diffusion_pipeline(pipeline, kwargs)
explicit_default = pipeline(**kwargs)
torch.testing.assert_close(implicit.images, explicit_default.images, rtol=0, atol=0)
assert pipeline.scheduler is original and dict(pipeline.config) == original_config
for choice in ['flow_match_euler', 'EulerDiscreteScheduler', 'diffusers.EulerDiscreteScheduler', '']:
    try: call_diffusion_pipeline(pipeline, kwargs, choice)
    except ValueError: pass
    else: raise AssertionError('unsupported scheduler accepted')
    assert pipeline.scheduler is original and dict(pipeline.config) == original_config
try: call_diffusion_pipeline(pipeline, {}, 'euler')
except (ValueError, TypeError): pass
else: raise AssertionError('invalid pipeline call unexpectedly succeeded')
assert pipeline.scheduler is original and dict(pipeline.config) == original_config
print(f'native_scheduler_torch={torch.__version__}; diffusers={diffusers.__version__}; choices=ddim,euler; device=cpu; pretrained_models=0', flush=True)
"#).unwrap();
        if let Err(error) = py.run(&source, Some(&locals), Some(&locals)) {
            error.print(py);
            panic!(
                "actual scheduler override preserves resident defaults and equals the native oracle"
            );
        }
    });
}
