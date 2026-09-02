# Widget Registry Storage

The widget registry stores every UI object as a `Frame`. Model-family state is required for Lua/API compatibility but is rare in the 2D simulator, so keeping it inline in every `Frame` inflated the full-game registry estimate. Commit `2542135be` moves that state into one lazy boxed payload without changing the public method surface or the intentional 3D-rendering boundary.

## Content

### Root cause

`Frame` previously paid inline storage for nine model-family state groups on all widgets: model path/file ID, transform, appearance, rendering, ModelScene state, actor IDs/tags, and PlayerModel state. The registry estimate therefore grew with the total frame count even when most frames could never use those fields.

### Fix

`Frame::model_state` is `Option<Box<ModelWidgetState>>`. Read helpers return the previous defaults while the payload is absent. Mutating model methods allocate it lazily, regardless of `WidgetType`, because frame methods remain globally callable. The storage estimator counts the boxed payload and its owned string/vector capacities only when present; it does not double-count those fields through the inline `Frame` size.

The 3D boundary is unchanged: model methods preserve the Lua-facing compatibility surface, while camera, lighting, mesh, animation, and model-scene rendering remain outside scope.

### Measured result

At the settled full-game fixture's 45,002 frames, the registry estimate fell from **239,185,088** to **213,058,036** bytes. The existing **230,000,000-byte** budget remains unchanged and now passes with **16,941,964 bytes** of margin. Model-family round trips remain covered by `tests/widget_methods_model.rs`; the budget assertion remains in `tests/widget_registry_perf.rs`.

## Sources

- [frame.rs](../../../src/widget/frame.rs) — `Frame::model_state` and lazy accessors
- [frame_types.rs](../../../src/widget/frame_types.rs) — `ModelWidgetState` and model-family state types
- [frame_size.rs](../../../src/widget/frame_size.rs) — inline and boxed storage accounting
- [widget_methods_model.rs](../../../tests/widget_methods_model.rs) — model, ModelScene, and PlayerModel behavior coverage
- [widget_registry_perf.rs](../../../tests/widget_registry_perf.rs) — settled registry frame-count and storage budgets
- [widget-system.md](../../widget-system.md) — maintained widget architecture reference

## See Also

- [[widget-system]] — Frame storage, model-family compatibility, and registry structure
- [[rendering-pipeline]] — 2D rendering boundary and frame traversal
- [[architecture-overview]] — simulator scope and non-goals
