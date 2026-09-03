# Editor-To-Artifact Image Generation Issues

Current authority: [plan.md](plan.md)

## IMG-I01

**Status:** blocking

Diffusers currently enables model-supplied code without a Rust-owned trust
decision. Resolution belongs to Security Milestone 0 in the current-standards
portfolio.

## IMG-I02

**Status:** blocking, requires current reproduction

The saved `tiny-sd-turbo-diffusion` workflow last reached the real editor but
was rejected by backend inference validation. Capture and classify the complete
typed diagnostics after IMG-I01 closes.

## IMG-I03

**Status:** blocking final acceptance

The real workflow requires a recorded local model/package identity, Python and
Diffusers runtime, WebKit/Tauri driver environment, device capability, and
isolated project state. Missing prerequisites remain unavailable rather than
selecting a lower-fidelity test.

## IMG-I04

**Status:** coordination

Image, architecture, security, and frontend remediation share runtime-host,
worker, Tauri, and editor paths. The current-standards portfolio must hand off
each shared write set before this plan resumes.
