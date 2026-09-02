# DISPLAY_SIZE_CHANGED / UI_SCALE_CHANGED Firing Conditions

A live-client probe (retail 12.0.5.67823) disproved the simulator's assumption that window resizes fire `DISPLAY_SIZE_CHANGED` without `UI_SCALE_CHANGED`. Retail fires both events as an ordered pair — `DISPLAY_SIZE_CHANGED` first, then `UI_SCALE_CHANGED` — on **every** display/scale recalculation; neither event ever fires alone.

## Ground Truth (ScaleEventProbe, 2026-06-12)

Captured via `docs/addons/ScaleEventProbe/` on the live desktop client (~37 event pairs across all scenarios):

| Trigger | Result |
|---|---|
| Client startup | Pair fires **twice**, both **before `PLAYER_LOGIN`**: once at nominal scale (effective 1.0, `GetScreenHeight()=768`), once after the UI scale applies |
| Drag resize, fixed manual scale (`useUiScale=1`) | Repeated ordered pairs during a continuous drag, not one pair per gesture. Captured fixed-scale drag produced five `DISPLAY_SIZE_CHANGED` → `UI_SCALE_CHANGED` pairs in ~1.1s |
| Drag resize, auto scale (`useUiScale=0`) | Repeated ordered pairs during drag; effective scale tracks `768 / GetScreenHeight()` continuously |
| Maximize / restore | Ordered pair per transition; some transitions produced two ordered pairs, including same-size snapshots (no dedupe) |
| UI Scale slider change (no resize) | Ordered pair fires (UI-unit screen size changes even though physical size doesn't), then `CVAR_UPDATE` |
| `useUiScale` toggle | Pair fires, then `CVAR_UPDATE` |
| Resolution change | Pair fires |

Additional invariants observed in every snapshot:

- Startup fires the ordered pair twice before `PLAYER_LOGIN`:
  `DISPLAY_SIZE_CHANGED`, `UI_SCALE_CHANGED`, then
  `DISPLAY_SIZE_CHANGED`, `UI_SCALE_CHANGED` again.
- Some discrete window/display transitions also emit two ordered pairs,
  including cases where the visible UI dimensions are unchanged. Continuous
  drag resize emits repeated ordered pairs during the drag, not a single event
  burst at the end.
- **Scale events fire before `CVAR_UPDATE`**: for `uiScale`/`useUiScale` changes, `UIParent:GetEffectiveScale()` already returns the new scale during `DISPLAY_SIZE_CHANGED` and `UI_SCALE_CHANGED`, while `GetCVar()` still reads the old value. The CVar update follows the pair. Handlers can trust `GetEffectiveScale()`, not the CVar, during the pair.
- `GetScreenHeight() × UIParent:GetEffectiveScale() = 768` always holds, including the case where a manual scale is overridden because the window is too small (e.g. fixed 0.8 scale but effective 0.8581 at 895 UI height).
- `UI_SCALE_CHANGED` is not exclusively CVar-driven — it fires for display/scale recalculations even when `uiScale` does not change. Explicit `uiScale`/`useUiScale` writes also trigger the ordered pair before `CVAR_UPDATE`.

Corroborating community evidence: ElvUI registers **only** `UI_SCALE_CHANGED` for pixel-perfect rescaling (`PixelScaleChanged` re-reads `GetPhysicalScreenSize()`), and ElvUI commit `2934c29c` ("add option to ignore the UI Scale changed popup when changing the window size") plus tukui issue #1066 document the popup firing on resize/maximize — only reachable via `UI_SCALE_CHANGED`.

## Simulator Fix

- `WowLuaEnv::set_screen_size` now fires `UI_SCALE_CHANGED` immediately after `DISPLAY_SIZE_CHANGED`, so window resizes emit the retail pair. The regression test was inverted: it previously asserted resize must **not** fire `UI_SCALE_CHANGED`; it now asserts the ordered pair.
- `SetCVar` now dispatches successful `CVAR_UPDATE` notifications after storing ordinary CVar values. For `uiScale` and `useUiScale`, it applies the modeled UIParent scale where available, emits `DISPLAY_SIZE_CHANGED` then `UI_SCALE_CHANGED` before storage, and dispatches `CVAR_UPDATE` last while handlers observe the old CVar value during the pair.
- **Startup ordering matched**: the post-login pair in `fire_post_login_events` was removed and the pair now fires in `fire_login_sequence` after `VARIABLES_LOADED`, before `PLAYER_LOGIN`. Combined with the `set_screen_size` pair fired during GUI canvas startup, the simulator emits two pre-login pairs and none post-login, matching the retail capture. Verified: lib test failure set unchanged, `lua-errors` clean both Blizzard-only and with all third-party addons.

## Known Observability Limit (not a behavior choice)

Retail fires the pair on same-size window transitions (maximize/restore landing on identical dims). The simulator's only window-size input is the per-frame canvas bounds from iced (`sync_draw_bounds` → `sync_screen_size_to_state`); there is no OS window-state signal, and iced reports no event for a transition that doesn't change the size. A same-size transition is therefore unobservable — the <0.5px guard distinguishes "transition" from "per-frame poll", and every *observable* size change (including each step of a continuous drag) fires the pair, matching the per-step retail capture.

## Probe Notes

The probe initially crashed in the simulator (`attempt to index global 'ScaleEventProbeDB' (a nil value)`) because both registered events fire **before** `PLAYER_LOGIN` — in the sim *and* in retail. Probe addons must buffer event entries until SavedVariables bind on `ADDON_LOADED`. The fixed probe's pre-login buffer is what captured the retail startup ordering.

## Sources

- [ScaleEventProbe](../../addons/ScaleEventProbe/README.md) — live-client probe addon and scenario list
- [env_runtime.rs](../../../src/lua_api/env_runtime.rs) — `set_screen_size` event pair
- [resize_event_tests.rs](../../../src/iced_app/resize_event_tests.rs) — ordered-pair regression test
- [startup.rs](../../../src/startup.rs) — pre-login pair in `fire_login_sequence`
- [create-and-install-wow-addon.md](../../addons/create-and-install-wow-addon.md) — probe install/round-trip workflow

## See Also

- [[event-system]] — event queue and dispatch flow
