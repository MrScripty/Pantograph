# Native startup regression failure

[Run 37498074275](https://github.com/MrScripty/Pantograph/actions/runs/37498074275)
executed `7fcd990bf6800defcb4f1eae5e07a2d30cadd799`, tree
`6e1a0b50d139886cd5a8f68d93a89801a25ded1b`. The actual native binary
built. Added startup tests compiled and ran: eight passed, one failed because
the llama.cpp comparison reused canonical `cpu`; the backend-local CPU selector
is `none`. Production selector validation correctly rejected that fixture.

The regression command exited 101 before launcher creation. WebDriver's missing
launcher is downstream. No app session, public submission or CPU output occurred.
All eleven artifact members and the masked job log are preserved losslessly.
ZIP artifact 11428389456 has SHA256
`62b97222090a939b0b6671125d87c0c92a10d946f9354ef77b6b4a01c7b2590b`.

Independent source review also found that generic `CandleBackend::start`
unconditionally requires scheduler-selected package and executable target.
Request-field tests did not exercise that rejection. The generic startup
proposal is withdrawn; the typed authorization boundary remains unchanged.
A focused selector-contract regression now uses `none` for llama.cpp and checks
that canonical `cpu` remains rejected. The valid owner CPU-device projection
repair remains. The successor adds owner-backed cold discovery so the existing typed scheduler
can authorize loading. Actual native CPU qualification remains pending.
