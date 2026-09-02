# Test Runtime Optimization Boundaries

Commit `e1f9b210b1790258a6f865cc3df0e85f6364e8e0` verified the `Blizzard_SharedMapDataProviders` exact dependency-closure fixture. The measured bottleneck is repeated `WowLuaEnv` and addon setup, not timeout polling, process launch, prefork scheduling, or libtest target topology. Exact closure fixtures are safe for ordinary addon surface/load tests when full-game ordering is not the behavior under test; mutable environments must remain isolated per test.

## Content

### Baseline and scope

The 18-test `blizzard_shared_map_data_providers_loads` module previously ran a full game-screen Blizzard load. Controlled measurements compare that baseline with the verified closure fixture at `e1f9b210b1790258a6f865cc3df0e85f6364e8e0`:

| Run | Baseline | Fixed closure | Change |
|---|---|---|---|
| Parallel | 18/18; 6.01s libtest, 6.083s wall, 34.03s user, 4.60s system, 6,412,848 KiB RSS | 18/18; 0.83s libtest, 0.85s wall, 3.25s user, 0.36s system, 1,067,108 KiB RSS | wall −86.03%; RSS −83.36% |
| Sequential | 18/18; 20.10s libtest, 20.140s wall, 19.06s user, 0.90s system, 1,246,460 KiB RSS | 18/18; 2.96s libtest, 2.97s wall, 2.78s user, 0.17s system, 535,624 KiB RSS | wall −85.25%; RSS −57.03% |

The fixed runs retain the same fresh-environment and direct-root-load test boundary; only the dependency closure is narrowed.

### Root cause and verified fix

The bare root closure was invalid: only 16/18 tests passed. Top-level Lua relied on normal eager ambient order, although the target TOC declares no dependencies. The closure therefore lacked SharedXML color helpers and `CVarMapCanvasDataProviderMixin::Init`; the downstream failure surfaced as a nil `DelveEntrancePinMixin`.

`e1f9b210b1790258a6f865cc3df0e85f6364e8e0` supplies ordered `BlizzardAddonOverride` implicit dependencies: `Blizzard_SharedXML`, then `Blizzard_MapCanvas`. The module still uses `build_blizzard_addon_closure_env` with a fresh game-screen environment and direct `Blizzard_SharedMapDataProviders` root load, followed by its existing post-load workaround step. It does not load panel/full UI state and does not share mutable environments.

The shared helpers in `tests/common/blizzard_addon_harness.rs` remain the fixture boundary:

- `new_blizzard_addon_env` creates a fresh game-screen environment;
- `build_blizzard_addon_closure_env` discovers and loads the root's transitive TOC dependencies plus explicit implicit roots in order;
- `load_blizzard_addon_closure_into_env` applies the same closure to an existing fresh environment.

This is the correct fixture for addon API, TOC, registration, and load-state assertions that do not require the complete game startup sequence. It preserves dependency order without paying for unrelated Blizzard addons. It is not a replacement for full-startup tests: `Blizzard_SharedMapDataProviders` remains ineligible for the finalized prefork parent because its fixture loads before post-load workarounds and omits startup events.

### Rejected alternatives

| Candidate | Finding | Decision |
|---|---|---|
| Timeout re-exec polling | `tests/common/timeout_reexec.rs` polls `Child::try_wait()` every 10 ms. Focused timeout-reexec scenarios cover panic aggregation, exact selection, nested guards, bounded child concurrency, permit release after timeout, and process-tree cleanup; polling contributes at most one interval per normal child. | Keep the 10 ms poll. It is negligible beside addon setup and is required for timeout and process-tree cleanup semantics. |
| Timeout child launch, pipes, and handshake | Exact named launch, two drain threads, and the handshake add sub-second overhead, while startup traces show about 0.25 s Lua initialization, 2.8–3.2 s addon loading, and about 0.27 s post-load layout per child. | Do not change the wrapper. It preserves panic aggregation, sibling continuation, output forwarding, and timeout isolation. |
| Prefork worker tuning | Prefork runs are already roughly 6–10 seconds for representative slices. More workers overlap child setup but do not remove it and increase copy-on-write process-tree memory. Persistent workers would also violate mutable-environment isolation. | Keep bounded workers and one isolated child per case; optimize the fixture graph instead. |
| Libtest wrappers | `test_timeout!`/`with_timeout` isolate ordinary tests by exact re-exec, but each ordinary test still constructs its own environment and loads its selected addon graph. | Retain wrappers for failure and timeout semantics; do not treat them as a startup optimization. |
| Grouped integration targets or target splitting | `build.rs` generates one integration harness from roughly 1,077 top-level test modules. Splitting it would duplicate simulator/test dependency compilation without removing per-test full-UI setup. | Do not split targets for this problem. Any future grouped target needs measured closure and compile evidence first. |

### Isolation boundary

Never share a mutable `WowLuaEnv` across tests. Tests mutate Lua globals, frame trees, addon-loaded state, registries, SavedVariables, and seeded domain state. The safe optimization narrows each test's dependency closure while still creating a fresh environment per test. Reusing a live environment or persistent worker would make sibling order and state leakage part of the test contract.

## Sources

- [SharedMapDataProviders test](../../../tests/blizzard_shared_map_data_providers_loads.rs) — commit `e1f9b210b1790258a6f865cc3df0e85f6364e8e0`, implicit dependency override, and retained assertions
- [Blizzard addon closure harness](../../../tests/common/blizzard_addon_harness.rs) — fresh environments and dependency-ordered closure loading
- [Blizzard addon override type](../../../src/loader/mod.rs) — ordered implicit dependency contract
- [timeout re-exec implementation](../../../tests/common/timeout_reexec.rs) — exact child launch, polling, output drains, handshake, and cleanup
- [timeout re-exec tests](../../../tests/timeout_reexec.rs) — panic, timeout, sibling, descendant, and nested-guard proofs
- [prefork runner](../../../tests/common/prefork.rs) — bounded workers and isolated child execution
- [generated integration harness](../../../build.rs) — top-level module discovery and target generation
- [Blizzard UI test lanes](../reference/blizzard-ui-test-lanes.md) — unit versus addon-bootstrap test intent
- [Prefork test harness](../systems/prefork-test-harness.md) — full-UI preload contract and eligibility boundary
- `/tmp/pi-shared-map-baseline-sequential.result.json` — ephemeral controlled sequential baseline
- `/tmp/pi-shared-map-fixed-*` — ephemeral fixed parallel/sequential measurements; repository claims remain understandable without these artifacts

## See Also

- [[prefork-test-harness]] — immutable full-UI parent, child isolation, and finalized eligibility
- [[blizzard-ui-test-lanes]] — test intent and startup-shape boundaries
- [[development-phases]] — broader performance-regression roadmap
