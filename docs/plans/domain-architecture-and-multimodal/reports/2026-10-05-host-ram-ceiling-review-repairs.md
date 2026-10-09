# Host RAM ceiling review repairs

This bounded successor preserves frozen RAM `d7903010a06acea1561fa2570f093bc7cfbe107b`
and CPU `394d4748be333fade6653aa5665a13129d8f4495`. Exact-model timing work remains
separate. It addresses the two independent RAM correctness findings without
changing PR54/55, configured budgets, claims, generation fencing or ranking.

## Real hierarchy root and namespace-visible root

The previous positive fixture incorrectly supplied `memory.max` at a real
hierarchy root. The [kernel memory interface contract](https://www.kernel.org/doc/html/latest/admin-guide/cgroup-v2.html#memory-interface-files)
defines that interface on non-root cgroups. The [core interface contract](https://www.kernel.org/doc/html/latest/admin-guide/cgroup-v2.html#core-interface-files)
also defines `cgroup.type` on non-root cgroups. A verified real root therefore has
neither interface; its readable `cgroup.controllers` confirms the mounted hierarchy
remains accessible. A namespace-visible root can still expose a non-root group's
interfaces and memory limit.

The probe retains read error kinds. Only a NotFound result for `memory.max` at the
verified conventional mount root can enter the root check. A present `cgroup.type`,
permission/I/O error, unreadable hierarchy marker, or missing intermediate limit
remains unavailable. A readable namespace-root `memory.max=30` is charged even
when `/proc/self/cgroup` reports `/`; the walk also retains that bound beneath
child and parent groups. No blanket ENOENT exemption or mount-root skip exists.

Controlled regressions cover child 80/parent 60/real root without memory.max,
process at a verified real root, namespace root 30/zero/max, absent namespace
limit, and intermediate/root-marker permission and I/O failures.

This environment remains outside the verified mapping: its cgroup2 mount root is
`/..`, `/proc/self/cgroup` reports `0::/`, `cgroup.type` is `domain`, and the visible
`memory.max` is 17179869184. The actual native source still reports unavailable
and fails closed. This is not established as the reviewed real-root failure, and
the repair does not provide positive native capacity qualification here.

## Independent resident pools

Resident publication now validates only backing domains charged by the declaration.
Zero or absent charge in a pool cannot cause an unrelated known resident fact to
be rejected. A zero envelope remains known zero; omitted resource kinds remain
unknown. A declaration adding charge to the over-capacity RAM pool still fails,
and every live task/resident charge remains held.

The portable regression holds PyTorch RAM 60 in its 100-byte pool, lowers the owner
ceiling to 40, declares Candle VRAM 10 in a distinct 100-byte pool, and admits a
20-byte VRAM claim. Further RAM admission remains blocked and the original RAM
lease still holds 60.

The native controlled regression reaches actual CPU candidate admission,
scheduler selection and the selected-text host port. With the complete text peak
still held, it lowers RAM below that peak, publishes a separate controlled Candle
VRAM declaration and admits a real registry VRAM lease. It proves the RAM peak
remains unchanged and new RAM claims fail. The Candle owner/resource facts are
controlled; this does not execute Candle on a GPU or qualify physical VRAM.

## Executed qualification

- All 139 registry tests pass, including the new unrelated-pool regression.
- All 715 inference library tests pass serially; seven host-RAM tests include
  the actual native unavailable observation and controlled root-layout cases.
- All 65 focused native host tests pass. The full embedded suite executes 501:
  499 pass and the same descriptor-count and warmup-timeout baseline failures
  remain. Their assertions and timeouts are unchanged.
- Warning-deny mixed-backend Clippy covers registry/inference/embedded all targets;
  formatting, critical/accessibility/traceability gates, 28 traceability tests and
  nine ONNX no-build-download graphs pass. Dependencies and Pumas pins are unchanged.
- The new regression assertions are replayed on frozen pre-repair production
  source with only test additions in a separate worktree/target. The unrelated
  resident publication and missing real-root interface cases fail there.

Builds use Rust 1.92, locked/offline dependencies, the actual Python 3.12 library,
unset ORT variables, and a dedicated target with local core crates cleaned after
copying dependency artifacts. The separate broader-inference wire-fixture failure
and parallel Python-fixture failure reported at `d7903010` remain outside this
repair. GTK/WebKit, real model quality, physical GPU capacity and cross-platform
native execution remain unqualified.
