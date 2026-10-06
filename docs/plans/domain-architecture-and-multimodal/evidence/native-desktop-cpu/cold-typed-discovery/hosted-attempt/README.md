# Native cold-owner discovery and submission timeout

[Run 37502222511](https://github.com/MrScripty/Pantograph/actions/runs/37502222511)
executes `42155032ef88c009f5fcd7680a3f10ea89c0bbd1`, tree
`39ac261c877d0f66b9e4bda3b49a8ec5611e802f`. Official setup, effective
no-download graphs, native binary build and all eight startup tests pass.
The actual app opens, reports the real compiled Candle owner stopped with no
model/instance/reservation, saves/reopens the graph and displays both wires.
The configured screenshot precedes the normal interface Apply/Save.

The real Submit gate admits the graph and the test clicks Submit at
17:29:18.924 UTC. IO Inspector never appears within the observed interval.
The harness used a 300000ms inspector wait under a 240000ms Mocha timeout;
the global timeout kills the session before its failure hook can capture owner
projection, UI error or run state. Execution and CPU output are unobserved;
there is no retained output artifact. Neither successful execution nor a
production loader failure can be inferred from this timeout.

All fifteen artifact members, original PNG/JSON, inner startup log and masked
job log are preserved. ZIP artifact 11431521224 SHA256 is
`01fdb11c5a7ba496e0d27e4821fb791c555a92b9d8f9a81a28cf9416b1f8c028`.
The successor bounds the inspector/error wait to 30 seconds, records the actual
pre-submit owner projection, and captures visible GUI errors and scoped run state
before failure. It preserves production gates, execution and global timeout.
