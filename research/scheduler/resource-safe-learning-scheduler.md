# A Resource Safe Learning Scheduler for Pantograph

## A completion oriented algorithm for heterogeneous execution plans

Revised research paper | 2 October 2026 | Puma  
Repository basis: Pantograph main at `4938e405c7f656365eefdca492774ccae110c90d`  
Evidence: source inspection, an exact toy calculation, and audited v1 simulation; v2 is a proposed design

## Abstract

Pantograph must finish queued workflows efficiently while retaining useful model state across RAM and VRAM, adapting to measured host speeds, and responding to an app or workflow speed boost without blocking background progress. The dispatch unit remains an eligible node. Its execution choice, however, must be a capability-defined plan containing compute stages, memory allocations, transfer edges, batch rules, and ownership boundaries. This revision develops that model for independent CPU/GPU replicas, host-weight offload, sequential CPU/GPU partitions, supported microbatch pipelines, operator-role partitions, and explicitly supported sharding. Static, dynamic whole-request, and continuous iteration-level batching receive separate semantics.

The first proof of concept exposed a policy-quality problem despite passing its represented safety tests. In one independently reproduced synthetic case, the repaired proposed policy took 56.642415 simulated seconds while FIFO took 45.694653. The candidate set contained a faster cold GPU option, yet the shaped finite-horizon score preferred a slow warm CPU. Matched follow-up controls also retained a randomized-workload loss. These findings motivate replacing partial-service reward with a completion-oriented, multi-event comparison whose unfinished work remains liable in a common continuation.

The revised scheduler combines authoritative physical leases, stage- and host-conditioned timing distributions, bounded workflow-owner forecasts, mode-diverse candidate coverage, protected service opportunities, and receding-horizon search. It executes only the first revalidated action and never frees capacity on a prediction or cancellation request. The paper gives formal variables, units, constraints, pseudocode, conditional invariants, and a generated-workload evaluation contract with matched baselines and sealed test families. The contribution is a falsifiable algorithmic design and a better-defined experimental scope, not a globally optimal policy or a measured Pantograph hardware speedup. No v2 implementation or v2 performance result is claimed.

## 1 The scheduling problem and its objective

At a decision event, choose which eligible nodes or legal iteration continuations can start, which supported execution plan and batch membership they should use, and which model preparation or eviction improves subsequent completion. Completing a node can expose more nodes, but the workflow owner alone certifies their readiness. The scheduler may examine bounded submitted structure or owner forecasts without taking over graph correctness or assigning a start time to every node in every workflow.

Physical feasibility, performance prediction, and preference are separate. A sound resource contract determines what can coexist. Conditional profiles estimate how fast it will run. A declared completion objective determines whether that use of resources is valuable. Neither a high utilization percentage nor accurate isolated timings establishes that a chosen schedule finishes useful work sooner.

### 1.1 Completion is the primary outcome

For a finite submitted cohort, the default target is batch makespan: the wall-clock time until its required workflows finish. Workflow goodput is accepted completed workflows per unit time at a fixed workload mix. For a speed-boosted app or workflow, use a disclosed weighted flow-time profile under the same physical and background-service protections. Iterative serving additionally needs time to first token, inter-token delay, accepted output tokens, and final request completion. These are distinct profiles and metrics, not interchangeable notions of efficiency.

Let a_w and C_w denote release and accepted completion of workflow w. Two finite-cohort losses are

$$J_{\rm batch}=\mathbb{E}[\max_{w\in U} C_w-t],\qquad J_{\rm speed}=\mathbb{E}[\sum_{w\in U}h_w(C_w-a_w)].$$

U is the same evaluated workflow population for every candidate; h_w is a policy weight frozen during a decision. Both losses have units of seconds, with dimensionless weights in the second. A workflow completes only after all required outputs and branches in its actual contract resolve. A represented prefix is not a completed workflow. These finite losses assume the evaluated obligations can complete under the declared retry contract. An unserved or terminally failed required workflow has no finite accepted-completion time; fault cohorts must report that outcome explicitly and use a declared failure penalty or separate reliability criterion, never reward fast failure as fast completion.

For open arrivals, additionally measure accepted workflow goodput, backlog, and response distributions over a declared observation window. The finite-cohort objective is the initial v2 quality experiment, not a stability theorem for an unbounded stream. If offered work exceeds available service, scheduling alone cannot ensure bounded queues; admission limits, backpressure, or an explicit overload policy are required.

Raw node count is sensitive to splitting one operation into several nodes. Allocated compute units, partial nonpreemptible progress, and CPU/GPU resource-seconds are not accepted outputs. They are useful state or diagnostic quantities, but the revised policy does not make them its headline reward. Setup, transfer, interference, and eviction-induced reloads already delay completion in the simulator; adding a separate byte penalty requires an explicit secondary objective rather than an unexplained preference for a slower warm path.

### 1.2 Productive residency and intentional idle time

The requested efficiency includes keeping the right models available, loading a critical model before it is needed, and permitting CPU and GPU copies of the same model when their combined service helps the queue. It does not mean filling every memory domain or eliminating every idle interval. Early residency consumes memory-time; a load can interfere with a predecessor; an idle replica may still avoid a valuable reload. A CPU co-runner can keep another device busy while slowing the whole cohort.

Measure accepted completion together with categorized idle time: no work, dependency wait, model preparation, capacity fragmentation, protected-service drain, and external contention. An explicit wait or a temporarily idle device can be the best feasible action. The policy must explain that choice using the completion model and current obligations, not a universal utilization target.


## 2 What the inspected Pantograph contract actually provides

This proposal is based on the pinned repository revision above. The source audit distinguishes current behavior from research extensions. The commit is the reproducibility boundary; later code may differ. Repository references identify the relevant source surfaces in Appendix A.

Pantograph requires public workflow execution through execution sessions and scheduler-backed admission. ADR-011 makes queue controls and diagnostics part of that boundary. The scheduler's node intent is path-free and identifies one ready task using workflow, run, node, task, and fairness identity, task type, model reference, hard runtime/device constraints, trait overrides, and estimate hints. These are suitable foundations for a ready-frontier scheduler [P1, P2].

The inspected dispatch selection policy is deliberately conservative: it selects the sole eligible candidate and reports ambiguity when multiple candidates remain. The embedded candidate provider requires an explicit selected device until runtime capability facts expose device candidates. Therefore learned ranking and unconstrained automatic CPU/GPU choice are proposed extensions, not existing behavior [P3].

The inference gateway owns one active boxed backend behind a read-write lock. Backend switching stops the old backend. The selected text path holds the exclusive backend lock through model loading, output collection, and cancellation or producer drain. The PyTorch backend has a single optional loaded-model field. For that owner, the safe initial action set is serialized. A mathematical policy must not infer simultaneous models or CPU/GPU execution merely because the machine contains both devices [P4].

Runtime registry accounting is keyed by runtime identity rather than a complete shared physical-resource ledger. Missing budgets or resource kinds can leave admission unconstrained. The current inference resource estimator uses a static model-byte multiplier and fixed addition for RAM and VRAM. Such estimates are useful input hints, but they are not sufficient as simultaneous residency and phase-overlap proofs [P5]. A separate technical-fit selection path already ranks residency, warmup, queue/headroom, and historical performance once all candidates meet sample thresholds. The existing run queue also has priority, bypass aging, and limited warm-reuse behavior. This proposal extends those narrower mechanisms into joint placement and anticipation; it does not claim that Pantograph has no history-aware selection [P7]. Current candidate gathering acquires resource reservations before final ranking; the proposed non-mutating candidate evaluation followed by selected-bundle atomic commitment is therefore a necessary integration change [P3, P5]. Timing contracts cover load, unload, warmup, and related attempts; terminal resource summaries exist. This is not equivalent to continuous device-level phase telemetry [P6].

Existing session/admin queue controls can reprioritize queued runs, but the inspected mutation rejects active runs. No live app-wide speed flag or protected background floor is established by that API. The network API reports LocalOnly and no peers, so the network-worker design remains a future extension [P9].

Pumas remains the authority for package and load-target facts. The inspected Pantograph dependency pins Pumas at `f87c3da8276a914a54c6f4f36d617bef9d9f424e`; current full-facts and load-target dispatch require Owner access, while LocalClient/ReadOnly integration is planned. A fingerprint field exists but the inspected producer leaves it unpopulated. Immutable model identity below is a proposed stronger contract, not a guarantee of the current adapter. Scheduling must not bypass license, trust, dependency, or artifact-freshness checks [P10].

### 2.1 The proposed extension boundary

The scheduler should consume authoritative capabilities, a physical-resource ledger, and richer observations from runtime adapters. It should not take over model-file resolution, graph correctness, or backend internals. These are source-level findings at the pinned revision, not a claim that the subsequent standalone simulator changed Pantograph. The initial deployment can improve ranking, CPU versus GPU decisions where supported, and timing estimates while retaining the current ownership lock. Existing readiness logic tests materialized task-input bindings and then requires dependency and admission proof; the proposal preserves those authorities rather than replacing them with forecasts [P8]. Parallel residency, overlapping loads, or hybrid execution become candidates only after the adapter explicitly supports and validates them.

The following are distinct capabilities: running separate nodes on CPU and GPU at the same time; executing one node with a backend-supported CPU/GPU split; retaining several model instances; overlapping load with inference; overlapping two loads; preempting a kernel or generation; and migrating a running task. Each requires its own capability fact. Support for one does not imply another.


## 3 What the first proof of concept established

V1 is an implementation of a selected abstract scheduling contract, not a validation of every mechanism in this paper. It exercised model residency, physical reservations, some contention, priority/protection, forecasts, and lifecycle faults. It did not implement the full staged split-compute and iterative-batching model developed below. All times in this section are simulated batch makespans, not hardware inference times or mean per-workflow latency. The delivered v1 loss analysis and post-revision independent audit are the evidence sources [E1].

### 3.1 A repaired omission did not repair the objective

In randomized seed 7, FIFO finished at 45.694653 seconds and the proposed default at 56.642415, about 24.0% longer. A candidate helper initially stopped considering eviction whenever any mode already fit. This could hide a cold GPU alternative behind a feasible warm CPU. The repair preserved replacement candidates for other modes and added a known-cost regression. Crucially, the repaired seed-7 outcome remained 56.642415 seconds.

At its final decision, the repaired policy scored the warm CPU at 0.712417 and the GPU replacement at 0.597209. Its partial-service term nearly rewarded completion of the long CPU run inside or near a 16-second horizon, while transition penalties and a low normalized-delay weight outweighed the earlier finish on GPU. A one-time forced final-GPU diagnostic finished at 47.038627. This isolated a mismatch between the shaped score and faster batch completion in that state. It was not an exact oracle or a general replacement policy, and it still lost to FIFO by 1.343974 seconds because earlier scheduling choices differed.

Disabling protection yielded 56.665930 seconds, so this loss cannot be explained away as a necessary fairness cost. Earlier scheduling differences and the remaining aggregate loss were not fully isolated. The correct response is to improve the objective and comparison design, not silently retune a coefficient until the known trace wins.

### 3.2 Matched controls showed both benefits and losses

Follow-up seeds 59, 83, and 101 applied the same eight-second protected-turn rule and the same boost inputs to every policy. Each baseline kept its own ranking. The table gives means of batch makespan across those three seeds, in simulated seconds.

| Matched workload | FIFO | Residency greedy | V1 proposed |
| --- | ---: | ---: | ---: |
| Randomized | 47.351844 | 40.342836 | 47.084326 |
| Priority stress | 54.165873 | 54.165873 | 54.165873 |
| Contention | 40.903679 | 40.903679 | 34.755009 |
| Lookahead | 14.637694 | 14.637694 | 12.820722 |

The proposed policy remained about 16.7% slower than residency-greedy on the randomized family. Contention and lookahead benefits survived matched controls, while priority stress tied. A separate predeclared delay-weight ablation improved the randomized mean to about 43.41 seconds but still lost to 40.34. Neither three seeds nor that small ablation establishes a generally superior default.

### 3.3 Correctness evidence has a narrower meaning

The post-revision audit reran 41 core tests, 36 independent adversarial tests, and 180 randomized stress runs with no represented correctness failures. It independently reproduced four seed-7 diagnostic variants. It checked stored result accounting for 117 main, 72 follow-up, and 12 ablation runs; it did not independently rerun all those policy experiments. The original main comparison used unmatched service configurations and cannot isolate fairness, boosts, or search.

These checks support the audited simulator contract and report integrity. They do not establish real allocator safety, calibrated host predictors, durable distributed ownership, universal policy quality, or the richer v2 mechanisms. Safety tests and terminal runs can all pass while a policy chooses a substantially worse schedule. The revision therefore makes candidate coverage, completion-oriented continuation, multi-event search, and matched held-out evaluation explicit release requirements.

## 4 State and authority at node and stage boundaries

Let t be an event time. The workflow owner publishes eligible nodes R_t and versioned optional lookahead Q_t. The runtime publishes executing plans X_t, replicas and shards Z_t, active sequence state I_t, authoritative leases L_t, and measured physical capacities B_t. Application service debt and protected-turn state are D_t; predictor state is Theta_t. A useful scheduler observation is

$$s_t=(R_t,Q_t,X_t,Z_t,I_t,L_t,B_t,D_t,\Theta_t).$$

This is an approximate partially observed semi-Markov control state: elapsed service and latent contention matter, and decision intervals vary. Sufficient-state or Markov claims need additional assumptions. A bounded planner does not solve the full stochastic control problem exactly.

### 4.1 Capability defined execution plans

For node i, a legal mode k is a finite stage plan P_ik. Its stages declare operator class, assigned host/device, work, input/output location, additional allocations, owner tokens, and predecessor stage IDs. Transfer edges declare bytes and physical paths. Model state declares complete replicas or the exact shards and offloaded copies needed. Batch and iteration rules specify membership changes and safe interruption points.

A stage graph inside one node describes backend execution mechanics. It is not the workflow DAG. The outer scheduler starts only a ready node or valid continuation; the adapter or simulated runtime advances its legal internal stages. A CPU shard cannot masquerade as a complete CPU model, and a mode requiring several devices must acquire the full protected episode envelope or use a separately justified deadlock-free staged protocol.

The legal mode set K_i comes from actual capability facts and semantic constraints, including revision, dtype, quantization, adapter, operation, shape, output bounds, and quality. A scheduler cannot silently change model quality, precision, or output semantics to improve time. Independent copies, CPU storage, CPU computation, pipeline overlap, sharding, and kernel preemption are separate capabilities.

### 4.2 Immediate actions and atomic commitment

An action selects eligible node-to-mode starts, a compatible batch, a legal next iteration, model preparation, unpinned eviction, or bounded waiting. At most one current attempt of a node can be started. A combined action is checked against one snapshot and committed atomically; individually fitting placements can conflict when selected together.

Non-mutating candidate construction must precede real reservation. Revalidate graph/attempt generations, capability facts, allocations, owner locks, and offers during commit. Stale facts cause replanning, not partial admission. Eviction request and replacement load are normally separate events: bytes become available only after release acknowledgement. A hypothetical plan may schedule that future dependency but cannot spend the bytes early.

Running work stays committed unless the mode defines a safe boundary or preemption protocol. A cancellation request prevents new participation, but in-flight kernels, transfers, sequence state, and output buffers remain leased until their owners confirm cleanup. A forecasted finish and a coordinator timeout are not physical release events.

### 4.3 Readiness and output versions

The graph owner checks required inputs, branch predicates, attempt identity, and cancellation before publishing readiness. For a static DAG it can update only affected dependency edges, costing O(outdegree) on a completion and O(V+E) across a run, apart from indexing. It need not re-solve the graph at every dispatch.

Results are committed to the current valid attempt before successors are unblocked. Outputs can remain allocated after producer computation ends, until their consumers or retention policy release them. Shared model instances do not imply shared request state. Run-local inputs, KV state, cancellation, and accepted outputs must retain independent ownership and attribution.

## 5 Six meanings of CPU plus GPU

The scheduler must select a declared mechanism rather than an arbitrary CPU/GPU percentage. The following mechanisms have different dependency and memory consequences. None is inferred from aggregate RAM plus VRAM.

### 5.1 Independent complete replicas

A CPU copy and a GPU copy each execute a complete request. Different requests can run simultaneously if they have independent permitted owners. Their resident allocations are distinct unless physically shared storage is explicitly identified. A retained checkpoint file may be shared without implying shared RAM or VRAM bytes. Loading a second copy buys service capacity and locality, not an automatic speedup for one request.

Compare the extra replica against the best no-extra-replica continuation under the same queue, fairness, and interference. Even when a warm CPU fits, retain the cold GPU replacement candidate. CPU inference can help a busy GPU or harm tokenization, staging, or memory bandwidth. The comparison is workload dependent.

### 5.2 CPU resident weights with GPU computation

Offload is a storage and transfer plan. It does not necessarily execute model operations on the CPU. Accelerate documents hooks that move CPU-offloaded weights to the execution device around forward work [11]. A faithful abstract mode therefore contains host weights, device staging, recurring transfer stages, and GPU computation. A layer window or microbatch traversal can amortize transfers but requires explicit buffers and ordering.

A model that fits across tiers can still be too transfer-bound to help the queue. Repeated transfers cannot be replaced by a one-time load penalty. Full host retention, transient device copies, and any decoded checkpoint buffers coexist according to their actual lifetimes.

### 5.3 Sequential CPU and GPU layer partitions

A layer partition assigns some actual computation to CPU and the rest to GPU. One microbatch must respect those dependencies. For a simple CPU-then-GPU plan, isolated latency is

$$T=T_{\rm load}+T_{\rm input}+T_{\rm CPU}+T_{\rm boundary}+T_{\rm GPU}+T_{\rm output}.$$

It is not total work divided by the sum of nominal device rates. A GPU-then-CPU partition can have different transfer and output costs. More device-resident weights can be slower if the partition adds expensive activation boundaries. llama.cpp provides runtime-specific hybrid inference and distinguishes layer and tensor mechanisms; its live documentation is mechanism evidence, not certification of Pantograph's adapters or arbitrary partitions [12, 13].

### 5.4 Supported microbatch pipelines

Different microbatches may overlap across independent stages when the engine supports it. With fixed stage times t_j, independent stage resources, sufficient buffers, and b microbatches, the ideal pipeline time is

$$T_{\rm pipe}=\sum_{j=1}^{k}t_j+(b-1)\max_j t_j.$$

The formula includes fill and drain. It does not apply unchanged to reentrant devices, a shared bus, varying shapes, or coupled compute/transfer service. Schedule the actual stages and resource edges in those cases. A single dependent microbatch cannot earn fictitious CPU/GPU overlap.

Transfer/compute overlap requires suitable device, stream, asynchronous-operation, and host-memory support. CUDA documentation describes such prerequisites [14]. A capability flag permits the modeled arrangement; it does not guarantee that overlap improves completion. Bound in-flight activation buffers and stop upstream stages when downstream capacity is unavailable.

### 5.5 Operator role partitions

A split can place different operator roles on different devices rather than consecutive layer groups. FastDecode motivates separating attention/KV-associated work from other computation in a heterogeneous pipeline [15]. FlexGen studies tensor placement, traversal order, and delegated computation across memory tiers for throughput-oriented generation [16]. These examples motivate explicit weight, activation, and KV placement; they do not promise that CPU attention helps every model.

An abstract simulator can give CPU-attention and GPU-dense stages declared work and transfer profiles. It should not claim to reproduce those systems' kernels, numerical behavior, or hardware gains. Sequence-length-dependent attention work and shared CPU coordination are part of the chosen profile, not incidental noise.

### 5.6 Tensor or operator sharding

Concurrent shards compute portions of one operation and then communicate or synchronize. Even an ideal two-shard stage requires the slower shard's time plus gather/reduction and serial work. Imbalance can leave a device idle. AlpaServe illustrates the broader tradeoff between model-parallel overhead and multiplexing opportunities [17].

Arbitrary CPU/GPU tensor sharding remains outside the initial v2 subset unless a specific operator, split rule, communication pattern, numerical contract, and joint reservation are added. Unsupported must be reported as such. Neither a fractional placement field nor adding device rates demonstrates split computation.


## 6 Separate compute memory and transfer resources

A resource-safe action needs an overlap model, not just a model-weight size. For physical resource r, capacity C_r is reduced by an operating margin S_r. Let M_r include all existing committed envelopes: running workspace and unconsumed KV growth, loading and evicting transitions, retained outputs, resident shared state counted once, and external/system allowances. Let b_ar be only the additional nonduplicated conservative reservation for action a. Components must be disjoint, so a whole-task envelope that already contains weights cannot be added to those same weights again. A conservative admission test is

$$M_r+\sum_{a\in A_{\rm new}}b_{ar}\le C_r-S_r\qquad\hbox{for every hard resource }r.$$

This form reserves each action's worst relevant overlapping envelope until a confirmed phase transition permits a smaller reservation. A richer implementation can reserve phase-indexed intervals, but projected future availability is not a present lease. If a forecasted completion is late, its reservation remains live and later starts wait or are recomputed.

### 6.1 Memory is more than model weights

For GPU d, account for resident weights; loading destination weights; device loading or conversion scratch; active workspaces and activations; bounded KV-cache growth [18]; input/output buffers; context and allocator overhead; and fragmentation margin. RAM additionally needs retained checkpoints, mapped-memory accounting policy, decoded host copies, pinned transfer staging, CPU backend state, and retained intermediate results. Count shared weights once only under an explicit engine sharing guarantee; count each distinct copy otherwise. Map every logical RAM/VRAM allocation to its actual physical backing domain. On unified-memory or shared-DRAM hardware, CPU and GPU cannot each spend the full capacity as independent budgets. The same verified allocation ID is charged once in that domain; distinct host/device copies are charged separately even when their contents match. Sharing a pool does not remove migration, bandwidth pressure, or placement-specific overhead.

Resident footprint and active work are distinct. A sparse model can activate fewer expert weights per token while retaining the inactive experts. Fewer active weights do not imply free memory unless a supported cache/offload plan actually relocates them and pays its transfers.

The distinction between current observed allocation and permitted future growth is essential. A generation that has used 2 GiB of KV cache may still be entitled to grow to its admitted context and output limit. Sampling current free memory without reserving that growth admits unsafe neighbors. If an adapter cannot supply a justified enforceable envelope, reject that configuration for hard-safe admission or explicitly classify it as best effort. A serialized fallback is safe only when its isolated configuration has its own justified bound; serialization does not make an unknown peak safe. A learned p99 memory estimate is not a hard memory guarantee.

A model can be ABSENT, LOADING, READY, IN_USE, EVICTING, or FAILED. READY means that the backend has acknowledged the usable instance, including required warmup or readiness probing. IN_USE carries reference counts. EVICTING still consumes resources until confirmed release. A load failure or cancellation can leave an allocated instance requiring cleanup; the state machine must represent that possibility.

### 6.2 A concrete overlap trap

Suppose a GPU has 16 GiB and a 1 GiB safety margin. Model A occupies 7 GiB and its current inference workspace needs 2 GiB. Model B would occupy 6 GiB, and its loader needs 3 GiB of device scratch. The two weight sets total only 13 GiB, but overlapping A inference and B loading needs 18 GiB. It is infeasible under the 15 GiB usable budget. Once A's inference ends, retaining A while loading B still needs 16 GiB, so merely waiting for computation is insufficient. The policy must evict A, choose another device or mode, or use a backend load path with a different validated envelope.

If the 3 GiB scratch belongs in host RAM rather than device RAM, the answer changes. That is why the phase contract must come from the specific loader and backend. Simultaneous loads require the sum of their overlapping staging and destination allocations; serializing loads can be preferable even when inference itself is parallel.

### 6.3 Compute and bandwidth constraints

CPU threads, GPU owner slots, engine request slots, and loader concurrency tokens can be hard constraints if the runtime enforces them. GPU compute utilization fractions are generally not additive hard capacities. Two kernels can interfere through occupancy, memory bandwidth, cache, and launch behavior. Use measured co-execution modes or a conservative concurrency cap rather than treating utilization percentages as exact packing weights.

Storage, PCIe, host memory, and interconnect bandwidth create coupling. An enforced bandwidth quota can be a hard reservation. Without enforcement, bandwidth demand belongs in the interference model and conservative load-concurrency policy. A memory-safe action can still be catastrophically slow. Clockwork, ServerlessLLM, and MuxServe demonstrate different deliberate choices about serialization, loading, and sharing; their capabilities should not be transplanted as assumptions into Pantograph [1-3].

### 6.4 Backpressure and output lifetime

Admission must account for inputs waiting in RAM, retained outputs awaiting consumption, and queues of loaded-but-not-running models. Bound pending bytes and in-flight loads, not just task counts. The workflow owner determines when an output version is no longer referenced; the scheduler should receive that release event. A GPU optimizer that fills all RAM with preload copies can prevent the CPU preparation step that would feed it.


### 6.5 Normalized compute units with explicit classes

Use memory in GiB, transfer demand in GiB on named paths, and stage work in normalized compute units CU_c for operator class c. A class may represent dense prefill, memory-bound decode, an image operator, or CPU preparation. Define each class's reference work convention and profile its host/device service rate in CU_c per second. A universal scalar CU does not make all operations have the same CPU/GPU speed ratio; do not add unlike class work and call it useful output.

For admitted compute allocation q, one possible rate model is

$$r_{s,d}=q_{s,d}\,\rho_{c,d}(q,\mathrm{shape},\mathrm{batch},\mathrm{mode})\,g_{s,d}(\mathrm{corunners}).$$

If q uses normalized processing units PU, rho has units CU_c/(PU second), and g is dimensionless. Allow rho to depend on q so scaling is not forced to be linear. A measured table or saturation curve can be simpler than this factorization. A sum of PU reservations may be an enforced compute quota or only an abstract concurrency token; without runtime isolation it is not a physical GPU partition or latency guarantee. Preserve a conservative owner-serialized mode.

For remaining stage work W, advance by the integral of its current service rate and recompute at every arrival, stage start/end, or contention change. Do not freeze a duration at launch and ignore later co-runners. Vary weights and work independently enough in generated cases that large memory footprint does not automatically mean slow compute or a fixed GPU advantage.

### 6.6 Named links and changing transfer rates

Represent storage-to-RAM, RAM-to-GPU, GPU-to-RAM, host-to-host, and supported device-to-device paths explicitly. An isolated transfer costs fixed latency plus bytes divided by path bandwidth. Simultaneous flows share bottleneck links and any modeled host-memory service; each must not receive the full link rate. Independent links should not accidentally share one global bottleneck. Half-duplex and full-duplex are distinct capabilities.

A conservative initial simulator can use processor sharing on named links plus explicit host-contention factors. This is a coarse abstraction, not a PCIe transaction, NUMA, cache, or kernel-occupancy model. Integrate remaining bytes across events. Offload, model loading, and result movement must all pay the relevant transfers, including recurrent copies and latency. Sensitivity to link rates is required because the preferred CPU/GPU partition can reverse as transfer cost changes.

### 6.7 Episode envelopes and phase growth

The simplest auditable admission reserves each episode's maximum additional live allocation across all its declared phases, including batch workspace and admitted sequence growth. Persistent shared weights are accounted once under their allocation IDs, not once per member. This can be conservative but prevents several tasks from each holding resources while waiting for unreserved growth.

A phase-specific allocator can improve packing only if it atomically obtains the next phase's growth before allocation and provides a deadlock-free progress rule. Blockwise KV admission is a separate policy: a sequence cannot execute its next step without its new blocks. If swapping or recomputation is unsupported, stop new admissions before all resident sequences become unable to advance. More detailed accounting is not automatically safer than a conservative episode peak.

### 6.8 Retained output can cause a resource deadlock

Capacity safety alone does not imply workflow progress. Suppose RAM capacity is 10 units. Each of two independent producers leaves a retained output of 4 units and needs 2 additional scratch while running. After the first finishes, the second fits because 4+6=10. Once both finish, retained outputs total 8. Each consumer needs its own retained input plus 3 additional scratch, so neither can start under the remaining 2 units. The DAG is acyclic and a producer-then-consumer serial schedule was feasible, but locally safe producer admission has created a stalled resource state.

A production contract therefore needs an explicit output-progress rule. Options include an owner-supplied conservative output/consumer-headroom reservation, a feasible drain certificate for the bounded admitted dependency closure, or supported spill/recompute with its own space, transfer, ownership, and correctness budget. A certificate is a resource-feasible continuation under sound envelopes, not a prediction that memory will soon be free. Aggregate fork/join requirements must be included; reserving scratch for one arbitrary consumer is not sufficient in general. The owner can publish bounded progress facts so dispatch does not repeatedly solve every workflow graph.

The initial v2 implementation must declare which rule it enforces. Admission that produces retained data must preserve that rule; otherwise it is capacity-safe only and may deadlock. If no certificate or supported reclamation path is available, reject or backpressure the new production before committing, according to the declared overload contract. Also detect a nonterminal state with no possible enabled progress and report a deadlock/incomplete run explicitly. A fallback must not wait forever or report the stalled state as completion. Test the 10-unit example and join variants against the declared rule.

## 7 Three batching contracts

Batching is an execution mechanism as well as a queue policy. All batch members need compatible immutable model/configuration, operation, mode, shape or ragged-input support, output bounds, and state isolation. Same model names alone are insufficient. The runtime must scatter outputs and charge resource/service use to the right current attempts. Cross-tenant batching needs an isolation and attribution contract.

### 7.1 Static batching

A static batch has fixed membership before execution. Every member must be released and eligible before a non-speculative start. It is useful for isolating a known batch's execution profile from the online choice to wait. A reference with future membership unavailable to other policies is an offline or privileged comparison, not an ordinary matched baseline.

Batch work and memory come from a supported profile for the actual membership. A padded batch may execute every member at the maximum shape, increasing both work and workspace; a ragged backend needs an explicit representation and kernel contract. Nonmonotonic batch profiles are legal. The largest fitting batch is not necessarily the fastest completion policy.

### 7.2 Dynamic whole request batching

Dynamic batching selects membership from currently eligible compatible requests and may wait a bounded assembly interval for observed arrivals. Membership stays fixed during the whole-request execution unless the runtime says otherwise. Triton's documented dynamic batcher, delay controls, and ragged-input mechanism illustrate why these are separate concerns [19, 20].

The decision is membership plus mode, placement, allocation, and start/wait. Use a bounded delay menu, such as no wait, a small interval, or remaining assembly budget. Record when each request first became eligible; new arrivals must not reset its age. An assembly-delay bound does not bound model-load or capacity wait under overload. End assembly waiting when protected service is due; compatible batchmates may join only without violating that service obligation.

Reserve batch activation/workspace, all member output buffers, and persistent state. A cancellation request does not shrink an in-flight shared kernel's footprint. Under a conservative whole-request contract, the kernel drains, canceled members' results are discarded, and their private memory is released only after cleanup. A bulk duration multiplier without one actual modeled batched execution is not evidence of dynamic batching.

### 7.3 Continuous iteration level batching

Continuous batching changes membership at safe iteration boundaries. ORCA demonstrates iteration-level and selective batching, which requires execution-engine cooperation [21]. A faithful abstract request retains prompt progress, current decode step/context length, allocated KV blocks and location, last output time, cancellation state, compatible model signature, and owner identity.

The scheduler chooses supported prefill chunks and/or next decode steps under a token or work budget, memory limits, and service policy. Requests at different positions can share an iteration only if the mode profile supports that composition. A completed sequence may leave, and a newly eligible sequence may join the next legal boundary. Existing KV and pinned model state persist between iterations; changing membership does not grant free migration or release.

The declared maximum output length bounds memory, but actual stopping remains hidden until observed. A simulator that hands the policy the final sampled token count is clairvoyant. Prefill and decode need separate profiles. Sarathi-Serve motivates chunking prefill to manage interference with ongoing decode, but its throughput/token-latency tradeoff is workload and mechanism dependent [22]. Report time to first token and inter-token delay, not only final request latency.

### 7.4 Outer and inner service accounting

The outer scheduler assigns app opportunities and retains model state. An inner iterative scheduler distributes token/iteration service. Charge a mixed batch to its actual participants under a declared unit; one app must not gain entitlement by splitting a request into many workflows or tokens. VTC uses input/output token service under a specific LLM-serving fairness model [23]. Its guarantee does not automatically transfer to multi-model workflow DAGs, setup, heterogeneous resources, or this protected-turn policy.

An already-admitted sequence waiting for its next legal iteration is still backlogged service, even though it is no longer a newly ready workflow node. Its age and app entitlement persist. Define a protected iterative episode as a bounded prefill/decode quantum ending at a safe boundary; the general-workflow episode can remain one complete bounded node plus setup. A pause at an iteration boundary does not release pinned model or KV state.

The iterative cohort should have an ordinary FIFO iteration baseline and a declared fairness-oriented iteration baseline with the same token/KV budget. Whole-request and iterative cohorts need separate reporting. Calling a fixed-batch throughput multiplier continuous batching would erase the very state and interruption semantics being tested.

## 8 Workflow lookahead with an explicit information boundary

The dispatcher continues to act on eligible nodes. The graph owner may expose bounded submitted structure or forecast summaries that influence ordering, model residency, batch assembly, split choice, and host placement. This is useful lookahead without requiring a global start-time optimization for every workflow.

### 8.1 Serial parallel and mixed graphs

A serial queue is a chain of dependencies or an explicit order contract. A parallel queue consists of independent eligible work. A fork/join graph can expose several branches, retain a shared input, and then wait for every required predecessor. Mixed graphs contain chain segments, fan-out, joins, and possibly unresolved conditional branches. Generated tests must preserve these differences rather than flatten every graph into independent tasks.

Known structure is not known future outcome. Submitted edges and possible branches can be visible while branch choice, data-dependent shape, external release, failures, and runtime noise remain hidden. The owner alone resolves eligibility. Completing a forecasted predecessor in a rollout does not authorize a real descendant to run.

### 8.2 Three declared information levels

A ready-frontier control uses only eligible descriptors and observed running state. A known-structure control additionally sees submitted nodes/edges and disclosed static descriptors. A forecast-enabled control receives bounded downstream demand, estimated need intervals or distributions, remaining-work/criticality summaries, confidence, and expiry. Every comparison must name its level and give matched baselines the same information.

HEFT-style ranks can be an information-rich reference when the corresponding graph and cost estimates are available [4]. StarPU provides a precedent for dependency-aware heterogeneous tasks, implementation variants, data movement, and performance models [24]. Neither source establishes the proposed combined policy or removes the need for explicit memory and runtime ownership.

### 8.3 Bounded versioned forecast records

A forecast contains workflow/owner identity, graph version/generation, model signature, permitted modes, count or probability-weighted work, need-time distribution or interval, confidence, expiry, and invalidation identity. Optional criticality or remaining-work summaries are labeled graph-derived. Distinguish a known submitted release from a guessed arrival process. Do not expose generator seeds, actual future durations, sampled output lengths, unrevealed failures, or hidden branch draws.

Need time depends on placement and contention; a fork/join uses a maximum over required predecessor completions. The planner can update a small local forecast under candidate scenarios, or use frozen hints as an explicit approximation. Branch alternatives are not all certain future demand. Expired, canceled, or wrong-generation hints cannot authorize new preparation. Already loaded state remains physically allocated until legally evicted.

### 8.4 Time preparation against need

For load start b, random setup S, and forecast need time T_need, late availability and early idle residency are

$$L_{\rm late}=(b+S-T_{\rm need})_+,\qquad I_{\rm early}=(T_{\rm need}-b-S)_+.$$

Both are diagnostics and can affect completion through blocked work or capacity. A separate cost per byte-second of early residency is optional and must be an explicit objective. A useful idle replica may avoid a later reload; preloading everything can delay the current critical path.

A probability target Pr(b+S <= T_need) >= 1-delta uses the joint distribution. Under suitable continuity it can be expressed with the lower delta quantile of T_need-S, not a difference between a setup p95 and a mean readiness time. Atoms require a threshold that directly satisfies the inequality. A downstream completion target also includes transfer, actual readiness, and execution. No preload policy guarantees exact just-in-time readiness under arbitrary uncertainty.

### 8.5 A structural nonanticipation test

Give the policy an immutable sanitized observation, not a reference to the execution world's hidden state. In two counterfactual worlds with identical observations up to t but different future arrivals, durations, branches, or stop conditions, candidate scores and actions at t must match for the same policy random seed. They may diverge only after an observation differs. Rollout worlds may sample hypothetical futures, but later decisions see only events revealed by that simulated time.


## 9 Learn host and phase behavior without future leakage

The prediction target is a conditional distribution, not a universal average. Let T_i,k denote service duration and L_m,d denote load-to-ready duration for a particular instance and device. Their conditional distributions depend on immutable model identity, input shape, output limit, mode, hardware and software revision, cold or warm state, current co-runners, storage locality, transfer contention, and recent system load.

### 9.1 Record the measured speed of each host

A hardware profile is versioned by host, physical domain, backend build, precision, and relevant operating condition. Store capacities separately from measured rates: CPU throughput for representative operators and thread counts; GPU prefill, decode, image or other operation rates by shape/batch; sustained DRAM bandwidth; storage read and decode rates; host-device transfer bandwidth and latency; and, for remote work, network latency and payload transfer rates. A GPU product name or advertised peak FLOPS is only a feature, not a completion-time estimate.

For a simple phase, an interpretable starting model is fixed overhead plus workload amount divided by a measured effective rate. Rates need units such as bytes/second, tokens/second at a specified context, or images/second at a specified shape. A pipelined load may depend on its slowest stage, whereas serialized decode and transfer stages add. Fit the actual path rather than blindly summing bytes divided by nominal bandwidths. Compute throughput under a sustained batch and latency of an individual request are different measurements.

Use a small versioned calibration profile to seed estimates, then update from real phase observations, retaining sample count, age, uncertainty, and provenance. Condition on co-runners, thermal or power regime where observable, background load, input length, output limit, active work, resident bytes, and backend configuration. Do not run disruptive calibration on a user's machine without operational permission. This paper records no measured hardware profile; it specifies what must be measured.

### 9.2 Observe phase boundaries

Record enqueue, eligibility, reservation commit, actual phase start, checkpoint-open, host materialization, transfer start and finish, warmup finish, ready acknowledgement, compute start, first output where meaningful, compute finish, producer termination, and resource release. Record attempts and outcomes, including failure, cancellation, and retry identity. An asynchronous launch returning is not GPU execution completion: use appropriate device events plus phase wall-clock timestamps, and avoid introducing global synchronization that destroys the overlap being measured. Not every backend exposes every phase; unavailable observations stay missing rather than becoming zero-duration events.

Queue delay must not be mislabeled as intrinsic model-load time. Separate time waiting for an owner or loader from active loading. Likewise, last output and producer termination can differ. The latter determines when capacity can safely be reused. Correlate all observations with the chosen action, model revision, mode, co-runners, and resource sample window so policy-induced selection bias can be studied.

### 9.3 A practical predictor family

Begin with stratified empirical distributions and hierarchical shrinkage: a well-sampled exact configuration gets its own estimate; sparse configurations borrow from compatible node, model, and device families. A regression or survival model can then condition on shape and co-execution features. There is no requirement that times be Gaussian. Retain upper-tail behavior, outliers, and confidence about sample support.

For a running operation with elapsed time e and survival function S, the residual-duration distribution is

$$\Pr(T-e>x\mid T>e)=\frac{S(e+x)}{S(e)}.$$

This is preferable to subtracting elapsed time from an unconditional mean and clipping at zero. It assumes the conditioning features and survival estimate are appropriate. Cancellation produces right-censored observations when true completion time is unknown; it is not a successfully completed short run. User- or policy-driven cancellation can be informative: ordinary survival estimation requires appropriate conditional non-informative censoring assumptions, or an explicit model of the cancellation process. Hangs and failures may need a separate competing-risk or failure model.

For deadline d_i and candidate start t, estimate the probability of completion by the deadline from the joint setup and execution distribution:

$$p_i(P)=\Pr(C_i(P)\le d_i\mid s_t,\Theta_t).$$

The completion C_i(P) includes planned waiting, input transfer, setup/warmup, and service. Only for an immediate sequential setup-and-run action with no other delay does it reduce to t+L_i+T_i; intrinsic loader time remains separate from queueing. Do not add marginal p95 values and call the result a p95 total. Loads sharing a storage channel and jobs sharing memory bandwidth are correlated. Sample common latent system conditions or measured joint residuals when scoring scenarios. Under a calibrated joint predictor, an estimated chance constraint is a useful service-risk control, but it remains a probabilistic claim about performance, not a physical safety proof.

### 9.4 Calibration and drift

Report coverage and sharpness by device, model family, shape, warm/cold state, and concurrency condition. Track whether an advertised p90 completion bound is exceeded roughly 10% of the time in the population to which it is applied, alongside finite-sample uncertainty. Monitor rolling calibration, residual change, and support. New model revisions, driver changes, runtime updates, or thermal regimes can invalidate old estimates.

Use uncertainty to become more conservative when evidence is thin. Evaluate out-of-distribution configurations in shadow mode first. Adaptive conformal methods can help adjust empirical coverage under changing conditions, but their guarantees depend on the particular method and assumptions; they do not supply per-request conditional coverage or physical memory safety automatically [8].

### 9.5 Exploration that does not spend safety

Log which feasible alternatives were available and which action was selected. Limited randomized exploration among already resource-safe, policy-permitted configurations can reduce selection bias and identify better CPU/GPU or co-execution choices. Cap exploration by a service-risk budget and exclude protected deadline work, unsupported modes, and unbounded allocations. Unknown speed is explorable; unknown memory safety is not. Off-policy evaluation must account for propensity support and cannot establish the value of modes never observed.


### 9.6 Separate latent streams and evaluation samples

Generate request mix, stage-time variation, transfer conditions, and control-flow outcomes through independently named or keyed random streams. A branch draw must not alter later runtime draws simply by consuming another random number. Key exogenous conditions by scenario/request/attempt/stage where appropriate, while keeping them hidden from policy observations. Common underlying conditions enable paired comparisons; policy-dependent contention still changes realized completion legitimately.

Planning samples come from a declared conditional distribution, not the execution world's actual future draws. Residual runtime must be inferred from elapsed observations; a policy may not read true remaining simulated work. Include shared host slow periods and storage states so uncertainty is not only independent noise per request. Initial synthetic profiles and observed-sample updates are not calibrated deployment predictors. Report coverage and bias separately by split mode, batch/shape class, stage family, and host.

## 10 Completion oriented multi event planning

The revised policy is a resource-safe, completion-oriented receding-horizon search over capability-defined plans. It evaluates consequences across successive decision events and executes only the first revalidated action. A larger simultaneous first bundle is not multi-step planning. The change directly addresses v1's demonstrated score mismatch while retaining its physical admission and protection boundaries.

### 10.1 Freeze the population and objective

At t, fix a common comparison universe U_t and graph/forecast information, priority weights, service obligations, and scenario samples for all candidates. Unselected work remains in the continuation; a plan cannot lower its score by dropping inconvenient requests. For a closed represented cohort, evaluate actual predicted workflow/request completions under the batch or speed profile in Section 1. When only a prefix is represented, either retain a disclosed downstream completion estimate or label the loss as represented-request completion, not workflow completion.

A default finite-cohort search minimizes expected remaining makespan, with expected completion sum as a declared secondary tie-break. A speed profile minimizes weighted workflow flow time while preserving protected app progress. Optional tail loss may use CVaR of a specified response or lateness variable. If both the base loss and tail loss use seconds, its coefficient is dimensionless. Define the quantile level, coefficient, and population before evaluation. CVaR is min_z {z + E[(L-z)_+]/(1-alpha)} for 0 < alpha < 1.

Load delay, transfer, interference, and reload consequences enter the simulated C_w. Do not add an arbitrary byte penalty that reverses a faster finish unless bytes, energy, or another resource is itself an authorized optimization objective. Report such a profile separately. Small numerical score differences need a declared tolerance or uncertainty comparison; finite Monte Carlo estimates are not exact policy-value orderings.

### 10.2 Continue every unfinished obligation

For v2's small closed cohorts, prefer a bounded population whose continuations are simulated to terminal completion by a fixed observation-conditioned rollout policy. The bound is on selected population, branch representation, and event work, not a reward cutoff that makes unfinished work disappear. If a rollout reaches a nonterminal resource deadlock or lacks its required output-progress contract, mark the candidate inadmissible or incomplete under the declared policy. If it hits its event or planning limit, mark its value incomplete and use a fully evaluated candidate or the baseline fallback. An in-progress job does not earn almost-complete output merely because the horizon expires.

For a larger open stream, a finite horizon needs an explicit terminal drain schedule or residual completion-cost estimate covering every unfinished represented obligation, retained allocation, and forecast probability. Capacity occupied beyond the horizon cannot be used again in that estimate. Report terminal-model sensitivity and compare against small exact fixtures. This is an approximation; no finite prefix model provides globally optimal completion for arbitrary unseen descendants.

A protected cold or long episode may commit beyond the planning horizon. Neither lack of in-horizon completion nor a low speculative score can erase its turn. If a system cannot obtain a complete candidate score within budget, it still has the physical-safe earliest-predicted-finish/protected fallback rather than an excuse to starve long operations.

### 10.3 Candidate coverage and event depth

Build a bounded shortlist preserving protected/oldest targets, important ready work, representative models, and capability/mode diversity. For each selected request or compatible membership set, consider all legal shortlisted placements, including cold eviction/load alternatives when a warm CPU already fits. Include supported batch sizes, bounded assembly waits, discrete split/offload plans, legal preload, retain/evict, and intentional wait. Budget some alternatives by mode/model class so warm affinity does not prune away the intended comparison.

A beam explores stage/load/transfer completions, arrivals visible within its declared information, legal iteration boundaries, and subsequent dispatch choices. All scenarios share the same immediate action. Later choices may depend on their observed simulated histories, not latent eventual durations. After search depth d, the same documented continuation rule completes the represented cohort or evaluates its terminal estimate. Record which part is optimized and which is fixed continuation.

Candidate bundles use a copied ledger, canonicalize shared allocations, and respect owner locks. Atomic real admission rechecks the chosen first action. An eviction-and-load sequence cannot be collapsed into immediate reuse unless an adapter supplies an actual atomic replacement operation with a validated peak. Real lifecycle truth wins over the plan at every event.

### 10.4 Pseudocode

```text
ON_EVENT(event):
  update owner-certified readiness and observed stage/sequence state
  reconcile leases only from acknowledged allocations and releases
  update timing evidence; preserve protected-turn position and timer
  snapshot sanitized state, valid forecasts, capabilities, and epochs
  apply current objective profile and app/workflow boost scope

  target <- protected_oldest_feasible_target_if_due(snapshot)
  U <- fixed_comparison_population(snapshot)
  modes <- capability_valid_stage_and_batch_plans(U)
  seeds <- mode_diverse_start_load_evict_batch_wait_candidates(modes)
  scenarios <- conditional_samples_independent_of_actual_future()
  beam <- physically_and_progress_valid_nonmutating(seeds)

  for each event depth while planning budget remains:
      expand each branch only after its simulated observations
      preserve nonanticipation across scenarios
      continue every obligation using the common rollout rule
      mark truncated/incomplete scores; never credit dropped work
      keep bounded diverse best complete candidates

  choice <- protected_episode_or_best_complete_first_action(beam)
  if none: choice <- matched_safe_earliest_finish_fallback(snapshot)
  if waiting is required: preserve debt and await real event
  else if atomic_revalidate_all_leases_and_epochs(choice):
      dispatch with attempt IDs and exact allocation ownership
  else: replan from fresh observations
```

The fallback has the same capabilities, batch limits, information, boost inputs, and protection obligations as the search. It is not a privileged recovery path. A protected episode can first stop conflicting discretionary admission and await actual drain; it need not be immediately runnable.

### 10.5 Computational cost and runtime budgets

Let N be eligible descriptors, K supported modes, F the shortlist, A expansions per branch, B beam width, d event depth, S scenarios, and E simulated events per continuation. A direct mode scan costs O(NK); shortlist selection can cost O(N log F). A straightforward bounded search costs O(d B A S E log E), plus resource and graph-update checks. These are structural costs, not measured dispatch performance. Indexed or budgeted retrieval is needed if the full queue is unbounded.

Batch subsets and stage/placement sequences are combinatorial. The search is approximate; even simpler assignment/packing cases are hard. Log candidate counts, complete/incomplete evaluations, simulated events, peak beam, and real planner wall time. A simulator's clock cannot hide scheduler overhead. Short tasks may favor cached configurations or the simple baseline. A wider beam or deeper continuation can improve or worsen choices under model error, so it must earn its cost on held-out families.


## 11 Priority progress and conditional guarantees

### 11.1 Dynamic app and workflow speed priority

Use hierarchical policy: applications receive progress entitlements; within an app, a workflow speed flag changes discretionary ranking. This prevents an app from increasing its entitlement just by submitting more workflows. A boost raises the cost of delaying the selected app or workflow and can change replica retention, preload timing, and placement. Product controls should specify scope and expiry explicitly rather than inferring them from activity indefinitely.

A priority-change event replans uncommitted starts and transitions at the next decision boundary. It does not revoke resource ownership, discard another app's inputs, evict pinned weights, or invent kernel preemption. A foreground request may wait for an existing non-preemptible inference or load. Report that residual blocking separately from scheduler delay. Cancel/restart or checkpoint preemption is available only if the backend exposes safe semantics and its lost-work cost is accepted.

### 11.2 Protected progress for other applications

Positive weights or a small age bonus alone do not guarantee progress. Keep an explicit protected-opportunity queue for admitted apps with continuously waiting, individually feasible ready work or legal active-sequence continuation. Each fairness round gives every such app a turn, including any required cold load; new apps cannot repeatedly jump ahead. In a protected turn, choose the oldest continuously eligible, individually feasible node or continuation within that app, so repeated workflow boosts cannot starve an older background workflow in the same app. Between turns, the throughput/speed optimizer uses discretionary capacity. Maintain a persistent round counter and elapsed discretionary-time budget G; when the budget is exhausted or the current protected appointment arrives, begin the protected drain. New arrivals, replanning, and boost toggles do not reset this timer or the queue position.

When a turn is due, stop admitting new work that conflicts with the target's full setup-and-execution envelope, let existing leases drain, legally evict unpinned state if needed, then reserve and launch the target. Nonconflicting work can continue. During a cold target's drain, existing sequences that pin blocking KV or model state must retain the iterations needed to finish and release it; block new conflicting sequence admissions, not those required continuations. Otherwise the protection mechanism could deadlock on pins it prevents from draining. Any KV growth must already be covered by the episode envelope or the validated staged-growth protocol. Include the blockers' remaining bounded service, safe cleanup, and any declared resumable offload in the drain bound. Candidate pruning must preserve this target, even if its model is cold, its duration exceeds the planning horizon, or the foreground model is always warm. Failed episodes use a bounded retry policy rather than holding the turn forever.

If at most N apps are admitted, each preceding protected episode, required drain, and resource-reacquisition outage is bounded by B_u, discretionary gaps per round total at most G, and the full episode is individually feasible, an app already queued reaches its protected activation after at most the current residual episode plus G plus the sum of preceding B_u. Its actual load/execution start additionally includes its own bounded drain, reacquisition outage, and preparation; a conservative bound including the full target episode is the current residual plus G plus the sum of B_u over all apps in that round. This follows from a finite ordered queue of bounded episodes. The first bound is for protected activation, and the conservative second bound covers the target's bounded service episode. Neither bounds completion of an arbitrarily large workflow. Unbounded runtimes, infeasible models, unlimited admitted apps, perpetual external occupancy, or endlessly failing loads invalidate a finite bound.

For a quantitative service floor rather than occasional progress, admit only a jointly feasible set of recurring entitlements. Account service debt in declared common units; CPU/GPU resource-seconds are not automatically equivalent useful output. Dominant-resource fairness and deficit round robin are useful baselines or inspirations, but their original guarantees do not automatically transfer to model-loading workflows [5, 10]. The protected mechanism can cost throughput. Measure that cost alongside the foreground latency benefit instead of promising that every objective improves simultaneously.

### 11.3 Conditional physical safety

Assume every action has a sound enforceable resource envelope; all allocation paths acquire leases before use; resident sharing is correctly accounted; admission is atomic; external consumers are represented by reservations or operating margins; and leases are released only after actual acknowledgement. If the initial ledger is feasible, the admission test preserves the capacity invariant by induction over commits and releases. A commit adds only a checked envelope; a valid release removes usage already ended. The argument is elementary but operationally demanding. It proves capacity preservation, not deadlock freedom: the output-progress rule in Section 6.8 and any staged-growth/drain premises are separate liveness obligations.

The proof does not apply when an estimate is mistaken, allocations bypass admission, a driver retains unaccounted memory, or an external process exceeds its allowance. Reconciliation can detect divergence but cannot retroactively make an unsafe allocation safe. On divergence, stop new admission for the affected resource, retain uncertain leases, and recover through the adapter's failure and cleanup protocol. Use smaller validated configurations, or validated isolated modes, when envelopes are not trustworthy; otherwise do not claim hard-safe admission.

### 11.4 Information limits of a ready frontier

Consider two ready nodes with identical visible runtime and resource descriptors. In one hidden workflow state, completing node a ends a high-value request while b reveals a long chain; in another, their downstream roles are reversed. A policy with the same visible state must choose the same first-action distribution in both worlds, although the end-to-end preferred decision can differ. Consequently, frontier-only scheduling cannot guarantee globally optimal workflow completion value or latency over arbitrary unseen graphs.

The limitation does not invalidate local scheduling. It defines the contract: optimize the visible work with explicit value metadata, measure user-visible outcomes, and accept that graph-global bounds require additional graph information or assumptions. A critical-path algorithm such as HEFT is a useful contrasting baseline when the whole DAG is available, not a drop-in policy for this information constraint [4].

### 11.5 No inherited optimality theorem

MaxWeight and queueing-control theory motivate backlog or service-debt terms, but throughput-optimality results require specific arrival, service, state, and exact-action assumptions. Setup, non-preemption, hidden descendants, learned durations, approximate beam search, and state-dependent interference require a new argument. This paper does not provide that argument [6, 7]. Likewise, a calibrated timing predictor is not proof of optimal scheduling, and a good toy result is not evidence of a production speedup.

The revised falsifiable thesis is narrower: given trustworthy capability and reservation contracts, completion-oriented multi-event candidate evaluation over staged modes and batches, with conditional timing and interference predictions will improve selected goodput and delay metrics against specified baselines on representative Pantograph workloads, at acceptable planning cost. Each component should earn its place through an ablation.


## 12 Worked examples and falsification cases

This example isolates placement and sequence-dependent setup. It is intentionally small, deterministic, and synthetic. All six independent nodes are ready at time zero in order A1, B1, A2, B2, A3, B3. Each CPU execution takes 5 time units. Each GPU execution takes 1 unit. The GPU starts with model A resident, can hold only one model, and requires 4 units to switch models. Loading and inference are serialized on the GPU. The CPU has one execution slot; its complete cost is already included in the 5 units. CPU and GPU do not interfere. Nodes are non-preemptible and have equal value.

Every assignment and per-device order is enumerated: choose a permutation of the six nodes and a split between the CPU prefix and GPU suffix. There are 6! times 7 = 5,040 such schedules, with each pair of ordered device queues represented once. For this stated model, the enumeration is an exact oracle for both makespan and sum of completion times. It is not an oracle for arbitrary Pantograph scheduling.

| Policy | Makespan | Mean completion time | Main choice |
| --- | ---: | ---: | --- |
| GPU FIFO | 26 | 13.50 | Alternates models and pays five switches |
| GPU model grouping | 10 | 5.50 | Three A nodes then three B nodes |
| Cold aware isolated choice | 15 | 6.00 | A on GPU and B on CPU |
| Earliest predicted finish | 10 | 5.50 | Greedy queue tail placement |
| Exact makespan oracle | 9 | 4.67 | One B on CPU and five nodes on GPU |
| Exact completion sum oracle | 9 | 4.67 | Same optimum on this instance |

The cold-aware isolated rule compares each node against the initial resident state, choosing CPU on a tie. Thus B appears to cost 5 units on either device and goes to the CPU, but three B nodes then serialize for 15 units. It misses both setup amortization and the joint device allocation. The earliest-finish rule accounts for its assigned queue tails and current planned GPU residency, yet its irreversible early choices still miss the optimum.

One optimal schedule sends B1 to the CPU during [0,5]. The GPU executes A1, A2, A3 during [0,3], loads B during [3,7], then executes B2 and B3 during [7,9]. Completion times are 1, 2, 3, 5, 8, and 9; their sum is 28 and mean is 28/6. Makespan is 9. The CPU becomes idle after time 5, and the GPU is non-computing while loading. Those idle categories do not mean this schedule is suboptimal: it is optimal for the stated finite-batch objective.

The revised search should include this joint pattern through a warm-model seed, a compatible CPU start, and a later load. Whether its beam and continuation find it is a planned test; this table does not benchmark v2 or its richer batching/split model. Its purpose is to reject formulations that ignore setup, optimize each node independently, or equate every idle interval with waste. Reproducibility details are in Appendix B.


### 12.1 Completion and split reversal fixtures

A warm CPU needs 12 seconds, while a cold GPU needs 2 seconds of setup and 3 seconds of execution. With no competing work or extra objective, the 5-second completion must beat 12 seconds. This is a direct score regression, not just a candidate-existence test. Then add transfer contention with an important current request and verify that the preferred plan can legitimately reverse.

For a declared hybrid mode, choose weights that cannot fit full VRAM but whose CPU/GPU shards plus staging/workspace fit both domains. The single request must traverse actual CPU and GPU stages. CPU-only may remain legal but slower. Increase boundary bytes or reduce link rate until the split loses. A CPU storage-only mode must still execute its declared GPU work and recurring transfers; it cannot be credited with CPU compute.

For two independent pipeline stages of 3 and 2 seconds, three microbatches finish in 3+2+2 times 3 = 11 seconds under the ideal assumptions in Section 5.4. One microbatch takes 5 seconds. Disabling overlap makes the three take 15 seconds. Shared-link or reentrant-resource cases must be evaluated by stage events, not forced to match this ideal formula. These are hand-reasoned planned fixtures, not v2 results.

### 12.2 Batch and forecast reversal fixtures

Four compatible short requests can benefit from a favorable batch-four profile. A long padded request can instead increase work and memory enough that a smaller membership wins. A request that finishes after two decode steps should leave an iterative batch; a later request can enter only at the next legal boundary and after KV admission. A fixed whole-request batch cannot silently acquire that ability.

A correct successor forecast can hide setup behind nonconflicting predecessor work. A wrong branch or shared transfer bottleneck can turn the same preload into wasted residency or delayed completion. Show both cases. A permanently warm foreground stream must not reset the protected cold request's age. Long output beyond its median must remain memory-safe without revealing its actual remaining length. These cases test semantic mechanisms and reversals, not a predetermined winning policy.

## 13 A research contract for the second proof of concept

V2 should follow paper review and implement a declared subset in a standalone simulator. It is not a Pantograph repository integration, real model engine, or hardware benchmark. The research deliverable must separate faithful mechanism representation from evidence that a policy improves completion.

### 13.1 Two explicitly reported cohorts

The general workflow cohort includes static and dynamic whole-request batching; independent replicas; staged CPU/GPU layer partitions; CPU-resident weight offload with recurring transfers; and optional declared microbatch overlap. Generate serial chains, ordered queues, wide independent work, fork/join DAGs, and mixed graphs. Include known-structure and forecast information regimes with hidden duration and branch variation.

The iterative-serving cohort has abstract prefill chunks and decode steps, changing membership at legal boundaries, persistent/growing KV state, hidden stopping, cancellation, and token/work budgets. It measures request completion plus first-token/inter-token behavior. It studies scheduling semantics without producing text. A smaller model set and bounded stage counts keep this cohort tractable without erasing iteration state.

Arbitrary tensor collectives, unrestricted live repartitioning, real allocator enforcement, kernel preemption, content correctness, and durable distributed failure recovery are outside the initial subset unless their contracts are added and independently tested. Remote ownership may be a protocol simulation. If a proposed cohort or mechanism is omitted, mark it partial or omitted and narrow the claim; an adjacent feature does not substitute for it.

### 13.2 Procedural families and independent axes

Vary graph depth, width, join structure, model popularity, weights, stage work, device speed ratios, batch compatibility, input/output shape, arrivals, cache capacity, transfer bandwidth, and correlated host conditions. Include warm hot sets, rare cold models, memory oversubscription, transfer-heavy chains, incorrect/expired hints, failures, cancellations, and heterogeneous sequence lengths. Reject generated cycles and preserve input/output lifetime.

Sample model size independently enough from work and speed ratio to avoid encoding the policy's answer in the generator. Some modes should fit only with a split; some should be harmed by splitting. Some requests should be unbatchable or differ in revision, dtype, adapter, or shape. Label intentionally infeasible requests and test rejection rather than dropping them from reported runs.

### 13.3 Seal development validation and test populations

Before tuning, write and hash a manifest containing generator version, parameter ranges, family/configuration splits, profile assumptions, development seeds, validation seeds, and locked test seeds. Use development for correctness and bounded-search design, validation for profile selection, and the final test only after freezing code and parameters. New seeds from one narrow distribution are not broad generalization.

If test results motivate a change, keep those outcomes, version the policy, and use a fresh test population. Do not select defaults by repeatedly examining the same losing trace. Key latent random streams independently so a branch or a policy-dependent event order does not silently change unrelated exogenous conditions. Policies may share paired workload conditions without receiving them as future information.

### 13.4 Strong matched baselines

Ordinary comparisons require FIFO with feasible placement and compatible batching; residency-greedy with legal replacement; earliest predicted finish including setup, transfer, and occupancy; graph-aware earliest finish/criticality when the same graph information is available; and the bounded multi-event policy. Iterative serving additionally needs FIFO and a disclosed fairness-oriented iteration baseline with equal token and KV budgets.

Every mechanism comparison matches capabilities, forecasts, profiles, batch limits, priority inputs, and service obligations. Baselines differ in ranking and planning, not in whether they are allowed to use the GPU or ignore background work. A capability-disabled control is an ablation; a future-aware static batch or clairvoyant runtime schedule is a privileged reference. Label those differences explicitly.

An exact small oracle must specify its finite action space, including whether it enumerates batch groupings, stage ordering, cache transitions, deliberate waits, shared-link overlap, and iteration state. An oracle omitting a mechanism cannot certify optimality for it. Independently verify hand-solvable cases and report solver or enumeration bounds, not just a plausible best schedule.

### 13.5 Safety nonanticipation and quality are separate gates

Test the retained-output deadlock and join fixtures in Section 6.8, including that all matched baselines use the same output-progress rule. Test allocation identity and full envelopes under simultaneous bundles, load failure, cancellation during transfer/iteration, duplicate events, stale generations, owner restart, external pressure, and retained output. Verify that candidate search is non-mutating, pinned state cannot be evicted, and acknowledged release is required. In a continuous batch, one member's cancellation must not prematurely free another member's workspace or its own in-flight KV use. An adversarial protected-drain test must keep a foreground sequence active with pinned KV, make a cold background target due, allow the existing sequence's required continuations, reject new conflicting admissions, and verify eventual release and target service under bounded output and service assumptions.

Counterfactual observation tests must cover unseen future arrivals, branch outcomes, exact remaining runtime, output stop length, and injected failures. Use identical policy randomness until observations differ. Test expiry/invalidation of hints and old remote offers. Safety within synthetic envelopes and in-memory fencing remains a simulator claim, not production isolation or durable exactly-once behavior.

Then separately test policy quality: whether faster cold alternatives are compared, unfinished liabilities persist, depth crosses actual decision events, and defaults improve the chosen completion profile against strong matched controls. Passing lifecycle tests or completing every scenario does not establish those results.

### 13.6 Metrics ablations and uncertainty

Report per-family paired makespan and workflow flow-time differences, workflow goodput, request delay, background/protected wait, load/reload count, bytes per path, RAM/VRAM peaks, compute/transfer busy time, failures, and planner cost. Iterative runs add first-token/inter-token delay, accepted tokens, and aborted-token work. State how cancellations and rejected work affect each denominator. Report wins, ties, losses, spread, worst cases, and confidence intervals with independent scenario/run as the resampling unit.

Ablate forecasts, preload, split modes, batching, assembly wait, iterative mixing, contention, prediction noise, continuation depth, and fairness/boost controls without relabeling unmatched comparisons. No-contention and perfect-information controls are sensitivity references. Explore interactions: split ratio versus link rate; batch size versus KV/workspace pressure; lookahead error versus cache capacity; boost intensity versus rare-model arrivals. Keep negative outcomes.

Validate the simulator against event traces before any hardware policy claim. Shadow-score real events before an authorized canary. A meaningful improvement must survive equal resources, compatible output semantics, safety/progress obligations, and planning overhead. Small-sample tail percentiles are descriptive, not latency guarantees. A valid result may be that a simple baseline matches or beats the richer search.


## 14 Extension to networked Pantograph workers

The local policy should make machine and memory-domain identity explicit now, while distributed execution remains a separate extension. A render worker is not another slice of one global VRAM pool. Candidate placement includes host, device, backend, model replica, and data location; each host owns its physical resource authority. Ray provides a relevant dynamic-task and locality precedent, but its architecture is not a proof of this proposed protocol [9].

### 14.1 Price data movement and host speed

A remote completion prediction includes input discovery and transfer, model fetch or replica preparation, worker queueing, actual execution under that host's measured profile, output materialization, and return or next-consumer transfer. Use topology and cache locality: model files, host-resident weights, device copies, inputs, and outputs are different locations. Add shared-link contention, serialization, compression where supported, and local storage pressure. A faster remote GPU can lose to a slower local device when transfer dominates.

A coordinator can rank fresh worker offers and prepare bounded placement plans; it must not promise capacity from stale heartbeat snapshots. The selected worker atomically revalidates capability, artifact identity, resource envelopes, owner locks, and lease generation before accepting. A rejection triggers another candidate or a fresh plan. If a node genuinely needs several workers simultaneously, use an explicit gang protocol with timeout and rollback; the initial extension should prefer single-worker node execution to avoid this extra distributed acquisition problem.

### 14.2 Ownership and failure semantics

Every attempt carries a workflow/run/node identity, monotonic ownership generation or fencing token, input/output versions, and an idempotent submission key. The worker persists accepted ownership and reports lifecycle events idempotently. The workflow owner commits an output only from the current valid attempt and only after artifact validation; stale attempts cannot unblock successors.

A disconnect means uncertainty, not completion. Remote leases must not free physical allocations merely because the coordinator's timer expired. The worker retains local resource ownership until it confirms producer termination and cleanup. Fencing prevents stale attempts from committing an accepted result; it does not stop a GPU kernel or undo an external side effect. Retrying after a lost acknowledgement can produce multiple execution attempts. Effectively-once result commitment requires a durable compare-and-set or equivalent commit protocol, while arbitrary external side effects need idempotency, transactional integration, or user/application recovery. Do not claim exactly-once execution from retries alone.

Cancellation is a versioned request to the owner, followed by acknowledged termination and release. A reconnect reconciles attempts, replicas, outputs, and remaining leases before new work is trusted. Workers must authenticate peers and enforce allowed models, trust policies, app isolation, and permitted data destinations; LAN proximity is not an authorization boundary. Input transfer and model licensing are eligibility constraints, not score penalties that may be traded away.

### 14.3 Keep fairness coherent across hosts

The coordinator owns application-level entitlements and speed flags, while each worker enforces physical admission and local protection against overload. Define whether an app's share is global across the cluster or reserved per worker; the recommended user-facing policy is global entitlement with local feasibility gates. Record service once per accepted attempt/result according to the declared accounting rule, so retries and replicated placement do not accidentally multiply fair-share credit. A partition can reduce available capacity and invalidate a promised floor; report degraded service rather than silently treating unreachable resources as available.

Validate the local scheduler before this extension. Then test delayed offers, duplicate submissions, worker restart, network partition, lost completion acknowledgement, stale output, cancellation during transfer, and model-revision mismatch. Distributed policy benefits and correctness should be reported separately from local concurrency gains.


## 15 Conclusion and implementation order

The revised thesis is a capability-constrained, completion-oriented scheduler for eligible workflow nodes whose execution modes expose stages, memory lifetimes, transfer paths, and batch semantics. It anticipates model demand using bounded owner information, records measured host behavior, permits beneficial CPU/GPU replicas or supported split plans, and prioritizes active work while preserving background service. Physical safety stays independent of timing optimism.

The v1 evidence makes the next priority concrete. Preserve the repaired mode coverage, replace the demonstrated partial-progress/byte-cost bias with explicit completion comparisons, and search across successive events with a common continuation. Implement and audit the two proposed cohorts only after this paper is reviewed. Freeze workload families and baselines before policy tuning, report both benefits and losses, and separate simulator fidelity from real Pantograph capability.

No global throughput, latency, or fairness theorem follows from combining these mechanisms. Hard guarantees depend on sound envelopes and ownership; progress bounds depend on admitted feasibility and bounded episodes; policy quality depends on the stated workload and information. The desired research outcome is an honest, reproducible improvement under those conditions, or an equally useful explanation of why a simpler policy is preferable.


## References

[1] Gujarati, A. et al. Serving DNNs like Clockwork: Performance Predictability from the Bottom Up. OSDI 2020. https://www.usenix.org/system/files/osdi20-gujarati.pdf

[2] Fu, Y. et al. ServerlessLLM: Low-Latency Serverless Inference for Large Language Models. OSDI 2024. https://www.usenix.org/system/files/osdi24-fu.pdf

[3] Duan, J. et al. MuxServe: Flexible Spatial-Temporal Multiplexing for Multiple LLM Serving. ICML 2024. https://proceedings.mlr.press/v235/duan24a.html

[4] Topcuoglu, H., Hariri, S., and Wu, M.-Y. Performance-Effective and Low-Complexity Task Scheduling for Heterogeneous Computing. IEEE TPDS 13(3), 2002. https://disco.ethz.ch/courses/fs14/seminar/paper/Jochen/4.pdf

[5] Ghodsi, A. et al. Dominant Resource Fairness: Fair Allocation of Multiple Resource Types. NSDI 2011. https://people.eecs.berkeley.edu/~matei/papers/2011/nsdi_drf.pdf

[6] Tassiulas, L. and Ephremides, A. Stability Properties of Constrained Queueing Systems and Scheduling Policies for Maximum Throughput in Multihop Radio Networks. IEEE TAC 37(12), 1992. https://drum.lib.umd.edu/items/571fda52-aefb-4497-9a2d-69d8c7c907b9

[7] Neely, M. J. Dynamic Optimization and Learning for Renewal Systems. 2010. https://arxiv.org/abs/1011.5942

[8] Gibbs, I. and Candes, E. Adaptive Conformal Inference Under Distribution Shift. NeurIPS 2021. https://papers.neurips.cc/paper_files/paper/2021/hash/0d441de75945e5acbc865406fc9a2559-Abstract.html

[9] Moritz, P. et al. Ray: A Distributed Framework for Emerging AI Applications. OSDI 2018. https://www.usenix.org/conference/osdi18/presentation/moritz

[10] Shreedhar, M. and Varghese, G. Efficient Fair Queueing Using Deficit Round-Robin. IEEE/ACM Transactions on Networking 4(3), 375-385, 1996; SIGCOMM 1995 precursor. https://web.stanford.edu/class/ee384x/EE384X/papers/DRR.pdf


[11] Hugging Face. Loading big models into memory. Accelerate official documentation, accessed 2 October 2026. https://huggingface.co/docs/accelerate/en/concept_guides/big_model_inference

[12] ggml-org. llama.cpp official project. Accessed 2 October 2026. https://github.com/ggml-org/llama.cpp

[13] ggml-org. llama.cpp multi-GPU documentation. Mutable master documentation, accessed 2 October 2026. https://github.com/ggml-org/llama.cpp/blob/master/docs/multi-gpu.md

[14] NVIDIA. CUDA C++ Best Practices Guide, version 12.6.3. https://docs.nvidia.com/cuda/archive/12.6.3/cuda-c-best-practices-guide/index.html

[15] He, J. and Zhai, J. FastDecode: High-Throughput GPU-Efficient LLM Serving using Heterogeneous Pipelines. 2024 preprint. https://arxiv.org/abs/2403.11421

[16] Sheng, Y. et al. FlexGen: High-Throughput Generative Inference of Large Language Models with a Single GPU. ICML 2023. https://proceedings.mlr.press/v202/sheng23a.html

[17] Li, Z. et al. AlpaServe: Statistical Multiplexing with Model Parallelism for Deep Learning Serving. OSDI 2023. https://www.usenix.org/conference/osdi23/presentation/li-zhouhan

[18] Kwon, W. et al. Efficient Memory Management for Large Language Model Serving with PagedAttention. SOSP 2023. https://arxiv.org/abs/2309.06180

[19] NVIDIA. Triton Inference Server Batchers. Official documentation, accessed 2 October 2026. https://docs.nvidia.com/deeplearning/triton-inference-server/user-guide/docs/user_guide/batcher.html

[20] NVIDIA. Triton Inference Server Ragged Batching. Official documentation, accessed 2 October 2026. https://docs.nvidia.com/deeplearning/triton-inference-server/user-guide/docs/user_guide/ragged_batching.html

[21] Yu, G.-I. et al. Orca: A Distributed Serving System for Transformer-Based Generative Models. OSDI 2022. https://www.usenix.org/system/files/osdi22-yu.pdf

[22] Agrawal, A. et al. Taming Throughput-Latency Tradeoff in LLM Inference with Sarathi-Serve. OSDI 2024. https://www.usenix.org/conference/osdi24/presentation/agrawal

[23] Sheng, Y. et al. Fairness in Serving Large Language Models. OSDI 2024. https://www.usenix.org/system/files/osdi24-sheng.pdf

[24] StarPU project. Features. Official project documentation, accessed 2 October 2026. https://starpu.gitlabpages.inria.fr/features.html

[E1] Pantograph scheduler proof of concept v1. Delivered standalone research companion, revision audited 2 October 2026. Evidence: “Why the scheduler loses and what the revision establishes” and “Post-revision independent audit”; exact reproducibility filenames and audit scope are summarized in Appendix B. These are local synthetic experimental records, not a peer-reviewed systems benchmark.


## Appendix A Repository evidence

The following evidence is tied to Pantograph commit `4938e405c7f656365eefdca492774ccae110c90d`. The linked source is authoritative for the inspection claims; no running deployment or hardware performance is implied.

P1. [ADR-011 lines 1-80](https://github.com/MrScripty/Pantograph/blob/4938e405c7f656365eefdca492774ccae110c90d/docs/adr/ADR-011-scheduler-only-workflow-execution.md#L1-L80). Public workflow-session admission and scheduler ownership.

P2. [Ready-node intent lines 100-212](https://github.com/MrScripty/Pantograph/blob/4938e405c7f656365eefdca492774ccae110c90d/crates/pantograph-scheduler/src/intent.rs#L100-L212). One-ready-task identity and descriptor fields.

P3. [Dispatch selector lines 16-64](https://github.com/MrScripty/Pantograph/blob/4938e405c7f656365eefdca492774ccae110c90d/crates/pantograph-scheduler/src/dispatch_selection_policy.rs#L16-L64); [candidate provider lines 217-335](https://github.com/MrScripty/Pantograph/blob/4938e405c7f656365eefdca492774ccae110c90d/crates/pantograph-embedded-runtime/src/runtime_dispatch_candidate_provider.rs#L217-L335). Sole-candidate selection, explicit-device limitation, and candidate-stage reservation side effects.

P4. [Gateway ownership lines 130-157](https://github.com/MrScripty/Pantograph/blob/4938e405c7f656365eefdca492774ccae110c90d/crates/inference/src/gateway.rs#L130-L157); [backend switching lines 502-541](https://github.com/MrScripty/Pantograph/blob/4938e405c7f656365eefdca492774ccae110c90d/crates/inference/src/gateway.rs#L502-L541); [selected execution lines 1084-1176](https://github.com/MrScripty/Pantograph/blob/4938e405c7f656365eefdca492774ccae110c90d/crates/inference/src/gateway.rs#L1084-L1176); [PyTorch state lines 107-130](https://github.com/MrScripty/Pantograph/blob/4938e405c7f656365eefdca492774ccae110c90d/crates/inference/src/backend/pytorch.rs#L107-L130). Single active backend, exclusive execution, observed drain, and optional loaded-model field.

P5. [Registry admission lines 706-916](https://github.com/MrScripty/Pantograph/blob/4938e405c7f656365eefdca492774ccae110c90d/crates/pantograph-runtime-registry/src/lib.rs#L706-L916); [resource estimator lines 8-193](https://github.com/MrScripty/Pantograph/blob/4938e405c7f656365eefdca492774ccae110c90d/crates/pantograph-embedded-runtime/src/inference_resource_estimator.rs#L8-L193). Runtime-scoped accounting and static peak hints.

P6. [Timing contracts lines 107-208](https://github.com/MrScripty/Pantograph/blob/4938e405c7f656365eefdca492774ccae110c90d/crates/pantograph-timing-contracts/src/lib.rs#L107-L208); [terminal telemetry lines 1-55](https://github.com/MrScripty/Pantograph/blob/4938e405c7f656365eefdca492774ccae110c90d/crates/inference/src/execution_telemetry.rs#L1-L55); [resource observations lines 14-59](https://github.com/MrScripty/Pantograph/blob/4938e405c7f656365eefdca492774ccae110c90d/crates/inference/src/resource_observation.rs#L14-L59). Attempt timing and terminal observation contracts.

P7. [Technical-fit ranking lines 493-608](https://github.com/MrScripty/Pantograph/blob/4938e405c7f656365eefdca492774ccae110c90d/crates/pantograph-runtime-registry/src/runtime_selection_policy.rs#L493-L608); [history wiring lines 111-173](https://github.com/MrScripty/Pantograph/blob/4938e405c7f656365eefdca492774ccae110c90d/crates/pantograph-embedded-runtime/src/technical_fit.rs#L111-L173); [run queue policy lines 261-355](https://github.com/MrScripty/Pantograph/blob/4938e405c7f656365eefdca492774ccae110c90d/crates/pantograph-workflow-service/src/scheduler/policy.rs#L261-L355). Existing history-aware fit and run-queue aging/warm reuse.

P8. [Readiness checks lines 3527-3556](https://github.com/MrScripty/Pantograph/blob/4938e405c7f656365eefdca492774ccae110c90d/crates/pantograph-workflow-service/src/scheduler/task_orchestrator.rs#L3527-L3556); [session runner lines 756-871](https://github.com/MrScripty/Pantograph/blob/4938e405c7f656365eefdca492774ccae110c90d/crates/pantograph-workflow-service/src/workflow/session_scheduler_runner.rs#L756-L871). Materialized-input readiness and current ready-task scan.

P9. [Queued-only reprioritization lines 582-636](https://github.com/MrScripty/Pantograph/blob/4938e405c7f656365eefdca492774ccae110c90d/crates/pantograph-workflow-service/src/scheduler/store_queue.rs#L582-L636); [local network API lines 21-105](https://github.com/MrScripty/Pantograph/blob/4938e405c7f656365eefdca492774ccae110c90d/crates/pantograph-workflow-service/src/workflow/local_network_api.rs#L21-L105). Active-run reprioritization rejected; LocalOnly network status and empty peers.

P10. [Pumas dependency pin](https://github.com/MrScripty/Pantograph/blob/4938e405c7f656365eefdca492774ccae110c90d/Cargo.toml#L111); [package-facts access lines 103-160](https://github.com/MrScripty/Pantograph/blob/4938e405c7f656365eefdca492774ccae110c90d/crates/pantograph-embedded-runtime/src/pumas_dispatch_package_facts.rs#L103-L160); [load-target access lines 98-190](https://github.com/MrScripty/Pantograph/blob/4938e405c7f656365eefdca492774ccae110c90d/crates/pantograph-embedded-runtime/src/runtime_dispatch_load_target_facts.rs#L98-L190); [Pumas fingerprint construction lines 314-336](https://github.com/MrScripty/Pumas-Library/blob/f87c3da8276a914a54c6f4f36d617bef9d9f424e/rust/crates/pumas-core/src/model_library/artifact_load_target.rs#L314-L336). Owner-only dispatch and the unpopulated fingerprint field.


## Appendix B Experimental evidence and reproducibility

The original six-node exact calculation enumerates every permutation of (A1, B1, A2, B2, A3, B3) and every split into ordered CPU and GPU queues. CPU nodes take five units; GPU nodes take one, with four units per model change and A initially resident. Its 5,040 schedules give minimum makespan 9 and minimum completion sum 28. The separately written verifier independently enumerated the optimum and checked every reported row; the revised-paper review also reconstructed the baseline values. This oracle contains no split modes, batching, unknown arrivals, iterative state, or contention and cannot certify their schedules.

The delivered v1 package includes `reports/loss_analysis.md`, `review/post-revision-audit.md`, `reports/loss_diagnosis.json`, `reports/holdout_matched_evaluation.json`, and `reports/objective_ablation.json`. The corresponding diagnostic traces and scripts preserve the legacy omission, repaired policy, protection-disabled run, and one-time forced-GPU intervention as separate cases. The package's code fingerprint and post-revision audit fingerprint matched; the independent audit reran the core/adversarial/stress suites and four diagnosis variants, while checking the other report totals from stored results.

The 117-run main comparison used the original unmatched control configuration. The 72 follow-up runs contain matched/unprotected controls; the 12 ablation runs change one delay coefficient. Section 3 uses the matched rows and reports means of batch makespan, not mean individual-workflow latency. Those distinctions are part of the evidence, not interchangeable labels. The revised v2 design has no execution results at paper publication; every v2 fixture, family, and release gate remains planned.

## Appendix C Minimum evidence behind feature labels

A CPU/GPU split requires one request to traverse actual modeled CPU and GPU compute stages, with shard locations and transfers. Offload requires another storage tier and every recurring transfer in its plan. Independent replicas require separate complete-serving copies; a shard is not one.

Dynamic batching requires compatible requests to share one modeled batch execution with a batch-specific work and memory profile. Continuous batching requires legal iteration boundaries, changing membership, persistent per-sequence state, and hidden remaining output length. A fixed-batch multiplier demonstrates neither contract by itself.

Workflow lookahead requires disclosed future structure or an owner forecast to affect a decision without granting early execution or revealing hidden outcomes. Multi-step search requires decisions across successive events, not only a larger simultaneous first bundle. Candidate coverage must retain cold placements even when a warm option fits.

Memory safety means preservation of sound declared envelopes and authoritative ownership transitions. Synthetic allocation checks do not certify a real allocator. Better performance names a metric, matched controls, workload population, and uncertainty; mechanism presence and passing correctness tests are insufficient.

The implementation review should map every declared requirement to variables, an actual decision path, and executable tests, then label it implemented, partial, or omitted. Required categories are physical plans/resources, batch/sequence service, owner lookahead/nonanticipation, search/lifecycle, and held-out evaluation. Scope reductions must be explicit before results are interpreted.
