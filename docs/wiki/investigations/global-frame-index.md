# Global Frame Index

Lazy frame lookup via `__index` on `_G` to avoid materializing Lua userdata for frames that are never accessed from Lua.

## Problem

Opening the talent panel creates hundreds of frames. Per-frame cost included:
1. `lua.create_userdata(FrameHandle)` — mlua allocation + metatable setup
2. `lua.globals().set(name, ud)` — insert into `_G`
3. `lua.globals().set("__frame_{id}", ud)` — internal event dispatch ref
4. Template child codegen — `_G["childName"] = child` via `lua.load().exec()`

For talent nodes never accessed from Lua, this is entirely wasted work.

## Design

### Phase 1: `__index` on `_G` (IMPLEMENTED)

Set a metatable on `lua.globals()` with an `__index` function. On name lookup:
1. Check `widgets.get_id_by_name(key)`
2. On hit: create FrameHandle userdata, cache via `rawset(_G, key, handle)`
3. On miss: return nil

Handles both named frames and `__frame_{id}` internal refs (by stripping the `__frame_` prefix). All eager `globals.set()` calls removed from `create_frame.rs`, `global_frames.rs`, and template child codegen.

### Phase 2 & 3: Pure Rust template child creation (PLANNED)

Replace `build_create_child_code` (Lua string calling `CreateFrame`) with direct Rust calls via `register_new_frame`. Mixin, KeyValues, and Scripts still require Lua, but can be batched into one chunk per parent instead of one per child.

## Behavior Preserved

- `CreateFrame` still returns a FrameHandle (caller expects it)
- `pairs(_G)` won't see uncached frames — acceptable, WoW addons access frames by name not by iteration
- `rawget(_G, name)` returns nil for uncached frames — rare in addon code

## Test Coverage

`src/loader/tests/global_frame_access.rs` covers: named frames in `_G`, unnamed frames excluded, returned handle is functional, XML-loaded frames and child textures accessible, button child globals (`ButtonNameNormalTexture` etc.), pre-registered globals (UIParent, WorldFrame, Minimap).

## Impact

First talent panel open: 431ms → 263ms (release). GC overhead during creation: 19% → ~0% (debug).

## Sources

- [global-frame-index-plan.md](../../global-frame-index-plan.md) — full design with code samples

## See Also

- [[talent-performance]] — performance results from this optimization
- [[method-dispatch-refactor]] — related `__index` lookup order changes
- [[duplicate-named-region-binding]] — duplicate sibling regions retain the first global binding
