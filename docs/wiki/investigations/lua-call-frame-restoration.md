# Lua Call-Frame Restoration After Errors

Commit `ff01991aa` fixed stale `LuaState` call-frame state after direct Lua errors. Commit `6ac9e32ee` then removed the obsolete exact-string retry that could invoke a failing closure twice. Commits `430ac3cb8`, `f32ae4470`, and `326bcf335` preserve nested Lua tracebacks during Rust-driven widget dispatch. After the retail EditMode epoch correction in `7d6051797`, `cb75dd007`, and explicit alias modeling in `20f141534`, the bounded default-retail startup proof at that revision was clean: `target/debug/wow-sim --no-addons --no-saved-vars lua-errors` emitted `[]` (0 unique errors, 0 occurrences).

## Content

### Root cause

`call_function_state` and `call_function_state_multi` temporarily changed the active Lua state to invoke a function. On Lua failure, the direct-call path returned the error without restoring the saved call-frame state. `LuaState.ci` could remain nonzero, with stale stack/frame state visible to the next call.

### Fix

`call_function_state` now delegates to the multi-return implementation. `call_function_state_multi` saves `base` and `ci` before `precall`. On error it restores:

- `top` to the temporary function slot,
- `base` to the caller frame,
- `ci` to the saved call frame,
- `ci_overflow` when the frame stack is no longer full.

Successful calls retain their existing result collection and caller-state restoration.

### Proof

Focused test: `src/lua_api/script_helpers/tests.rs::direct_state_call_restores_call_frame_after_lua_error`.

The regression test:

1. directly calls a Lua function that raises `direct-state boom`,
2. asserts the error is returned and `lua.state().ci == 0`,
3. performs a subsequent direct call returning `42`.

The RED state reproduced stale-frame behavior; the GREEN state passed after `ff01991aa`.

The retry-removal tests also prove that a genuine error containing `expected Lua closure in execute` runs once, propagates once, restores the caller state, and does not enter the protected retry path. Protected execution remains separate and tested.

### Widget-dispatch traceback root cause

`call_widget_handler` originally used the direct protected-call boundary. By the time Rust routed its error to the configured WoW error handler, the failing Lua frames had already unwound, so a nested handler failure reported the message but omitted the local `inner` call path.

Commit `430ac3cb8` switched widget dispatch to `protected_lua_pcall_state`, whose Lua bridge uses `xpcall(..., debug.traceback)`. That exposed a second boundary error: simulator initialization replaced rilua's native `xpcall` with a Rust implementation. The replacement called `protected_call_state`, restored the caller frame, and only then invoked the supplied error handler. `debug.traceback` therefore still saw only the wrapper stack.

Commits `f32ae4470` and `326bcf335` replace that override with a variadic Lua wrapper around the native rilua `xpcall` captured during initialization. The native implementation invokes the error handler before restoring the failed call frame. The wrapper retains WoW's extra-argument behavior and returns the original error if the supplied handler also fails. Handled `pcall`/`xpcall` failures remain excluded from `lua_error_counts`, matching commit `75e829f0`; commit `9bcfa511a` corrected stale tests that still expected collection.

Focused proof:

- `tests/system_api.rs::test_xpcall_error_handler_sees_failing_lua_call_path` verifies initialized-runtime `xpcall` includes `inner`.
- `tests/error_handler.rs::test_error_handler_receives_event_dispatch_traceback` verifies Rust-fired `OnEvent` dispatch preserves the same nested path through the configured error handler.
- The complete `system_api::`, `error_handler::`, and `game_menu::` focused modules pass after the change.

### EditMode epoch root cause

After call-frame recovery, the remaining retail startup cascade localized to two missing enum members at the retail 12.0.7 load boundary:

- `Enum.EditModeEncounterEventsSetting.TooltipAnchor = 8`
- `Enum.DamageMeterVisibility.InGroup = 3`

The corresponding metadata is `EditModeEncounterEventsSettingMeta = 0/13/14` and `DamageMeterVisibilityMeta = 0/3/4`. The modern table preserves `ShowTooltips = 8` and publishes `TooltipAnchor = 8` as an alias. The historical 12.0.0 override retains only `ShowTooltips = 8`; `TooltipAnchor` and `InGroup` are absent there.

Those missing members aborted `Blizzard_EditMode` initialization. After correcting enum publication, every remaining recorded family—including EditMode, CooldownViewer, chat, MicroMenu, compact-raid, WorldFrame, and ExtraAbility—also disappeared without local fixes. The post-fix baseline therefore identifies them as startup cascades rather than separate fixes.

### Final startup proof

At HEAD `20f141534`, the bounded default-retail command produces `[]` with 0 unique errors and 0 occurrences. Artifact: `/tmp/claude/wow-sim-lua-errors-after-editmode.out`. The result covers the call-frame recovery, direct-call retry removal, retail enum epoch correction, and explicit alias modeling together; it does not claim unrelated GUI or full-LoD behavior beyond this command.

## Sources

- [src/lua_api/methods.rs](../../../src/lua_api/methods.rs) — direct Lua call state setup and restoration
- [src/lua_api/script_helpers/tests.rs](../../../src/lua_api/script_helpers/tests.rs) — focused call-frame and exactly-once dispatch tests
- [src/lua_api/env_events.rs](../../../src/lua_api/env_events.rs) — Rust-driven widget dispatch and error routing
- [src/lua_api/globals/utility_system_spell/mod.rs](../../../src/lua_api/globals/utility_system_spell/mod.rs) — variadic native-`xpcall` wrapper installation
- [tests/error_handler.rs](../../../tests/error_handler.rs) — event-dispatch traceback and error-handler integration proof
- [tests/system_api.rs](../../../tests/system_api.rs) — initialized-runtime `xpcall` behavior proof
- [src/lua_api/globals/enum_data/edit_mode.rs](../../../src/lua_api/globals/enum_data/edit_mode.rs) — retail EditMode enum values, aliases, and metadata
- [src/lua_api/env_init/enums.rs](../../../src/lua_api/env_init/enums.rs) — epoch-specific enum overrides
- [playerspells-runtime-load](playerspells-runtime-load.md) — related nested addon-load call-frame preservation
- [rilua-mlua-gap-audit](rilua-mlua-gap-audit.md) — rilua migration gap context

## See Also

- [[playerspells-runtime-load]] — nested addon loading must preserve the active caller frame
- [[rilua-mlua-gap-audit]] — broader rilua migration gaps
- [[retail-ptr-full-startup-lua-errors]] — startup error investigations and bounded fixes
- [[micro-menu-click-offset]] — separate MicroMenu interaction/layout investigation; no local startup fix was needed here
