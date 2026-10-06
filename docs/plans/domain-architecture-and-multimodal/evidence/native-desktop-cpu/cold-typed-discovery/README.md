# Typed cold Candle discovery successor

Parent source is `7fcd990bf6800defcb4f1eae5e07a2d30cadd799`, tree
`6e1a0b50d139886cd5a8f68d93a89801a25ded1b`. The patch withdraws generic
Candle startup, fixes the selector test using backend-local `none`, preserves
the CPU-device projection repair, and integrates cold compiled-owner discovery.

Hosted composition registers a missing owner-advertised `candle.cpu` as stopped,
without readiness/model/instance/reservation claims. Existing records are kept.
Descriptor availability recognizes only the matched Candle family and CPU owner
variant. Other lifecycle gates remain. Selected package, executable target,
dependency readiness, scheduler/resource admission and the typed loader remain
responsible for actual execution. Resource accounting is unchanged.

548 affected-runtime tests pass; one optional test is ignored. The real Pumas
owner test resolves a valid embedding descriptor over the committed synthetic
fixture while the registry remains stopped and unreserved. Full effective Cargo
graphs, gates and source patch are preserved. This is controlled cold-discovery
coverage; actual native CPU output qualification requires the hosted GUI result.
