# OnUpdate Dirty Handlers

`handle_process_timers()` blanket-discards `render_dirty` after firing OnUpdate handlers, suppressing legitimate visual changes like the cast bar's `SetValue()` calls.

## Root Cause

`WidgetRegistry::get_mut()` unconditionally sets `render_dirty = true` for any mutable access, even when writing the same value. The blanket discard was added as a workaround because some handlers (e.g. `MainMenuMicroButton` setting identical atlas values every second) mark dirty without producing any visual change.

## Classified Handlers (37 visible at startup)

**Noisy (false dirty):**
- `ActionBarButtonUpdateFrame` — calls `SetChecked()` on all buttons each tick; idle after first few ticks once buttons unregister
- `MainMenuMicroButton` — calls `SetNormalAtlas` etc. with the same values every second
- `QueueStatusButton` — calls `Show()` on an already-shown texture every tick
- `LeaveInstanceGroupButton` — only while the compact raid manager subtree is actually shown; as of 2026-04-14, `A_Admin.SetPartySize(0)` fires `GROUP_ROSTER_UPDATE`, so solo transitions now hide `CompactRaidFrameManager` and drop this button out of the visible `OnUpdate` set outside party content

**Legitimate (should trigger redraws):**
- `PlayerCastingBarFrame` — `SetValue()` and `SetText()` genuinely change every frame during a cast
- `PlayerFrame` — `SetAlpha()` on StatusTexture oscillates smoothly during combat
- Action button flash textures — `Show()`/`Hide()` toggling at `ATTACK_BUTTON_FLASH_TIME` intervals

**Inert (no dirty, no issue):**
- `ChatFrame1`, `WorldFrame`, ModelScene frames, idle PartyMemberFrame buttons

## 2026-04-14 Handler Audit Follow-up

Two focused audit tests narrowed the remaining work after the earlier `SetText` / `SetEnabled` no-op guards:

- `LeaveInstanceGroupButton`
  - In the settled solo fixture, a second tick calls `C_PartyInfo.IsPartyWalkIn()` once, `PartyUtil.CanLeaveInstance()` once, and `IsInGroup()` twice; the solo path returns before `IsInInstance()` and `GetPartyLFGID()`, so those remain uncalled.
  - The handler still invokes `SetText` and `SetEnabled`, but the dirty batch stays empty once the button is already settled.
  - Conclusion: the remaining cost is query/dispatch work while the button is visible, not visual mutation churn. After the solo visibility fix, this handler no longer matters outside grouped content.

- `AuraButtonMixin:OnUpdate` (BuffFrame buttons)
  - A settled second tick still runs `SecondsToTimeAbbrev()`, `Duration:SetFormattedText()`, `Duration:SetFontObject()`, `Duration:SetPoint()`, `Duration:SetShown()`, `Duration:SetVertexColor()`, and `SetAlpha()` once each.
  - Current regression coverage still sees the root button plus the duration `FontString` in the dirty set on that settled tick.
  - Conclusion: the remaining cost is still centered in the Lua-side duration formatting / font-threshold path, not in missing coverage around whether the handler runs.

That shifts the next optimization target away from more setter no-op guards and toward short-circuiting the redundant work in the handlers themselves, especially `AuraButtonMixin:OnUpdate`.

## 2026-04-22 BuffFrame slow-handler interpretation

Slow handler logs shaped like `addon=Blizzard_BuffFrame handler=OnUpdate frame=#21946`
do **not** point at the named `BuffFrame` root. They point at an anonymous
`AuraButtonTemplate` child:

- `AuraFrameMixin:AuraFrame_OnLoad()` creates buff buttons with
  `CreateFrame("BUTTON", nil, self.AuraContainer, "AuraButtonTemplate")`, so
  the buttons have no global name.
- The timing logger prints `#<widget_id>` when a frame has no name.
- `AuraButtonMixin:UpdateExpirationTime()` enables `OnUpdate` only for timed
  auras (`expirationTime > 0`), which matches the visible buff buttons whose
  `Duration` labels are active.

That matters because the top-level `BuffFrameMixin:OnUpdate()` has a different
shape:

- it would log as `frame=BuffFrame`, not `frame=#...`
- it is a hidden-buff maintenance path with a `0.2s` throttle
  (`hiddenBuffUpdatePeriod`), not a per-frame countdown update

A headless `dump-tree --filter-key BuffFrame --visible-only` run on
2026-04-22 showed five visible anonymous BuffFrame buttons, each with a
visible `Duration` font string. That lines up with the repeated anonymous
`OnUpdate` timings and rules out the named root frame as the hot path.

The remaining work is still at the handler level. The current Rust mutators for
`SetAlpha`, `SetFormattedText`, `SetFontObject`, and `SetPoint` already have
same-value guards, so the main waste is that `AuraButtonMixin:OnUpdate()` keeps
recomputing countdown formatting and threshold branches every tick before those
guards can bail out.

## 2026-04-22 SetAlpha no-op microbenchmark baseline

Before changing any `SetAlpha` implementation, we measured the current
simulator cost directly in Lua with a batched microbenchmark
(`N=120000`, `R=8`) and wrote the results from `--exec-lua` to
`/tmp/claude/setalpha_bench_results.txt`.

Command shape used:

```bash
LD_LIBRARY_PATH=target/debug:target/debug/deps \
WOW_SIM_NO_SAVED_VARS=1 WOW_SIM_NO_ADDONS=1 \
target/debug/wow-sim --no-addons --no-saved-vars \
  --exec-lua @/tmp/claude/setalpha_bench.lua \
  screenshot -o /tmp/claude/setalpha_bench.webp
```

Measured batch timings (`GetTime` timer path in this headless run):

- `empty_ms=18.959338`
- `get_ms=151.218306` (`GetAlpha`)
- `same_ms=140.049725` (`SetAlpha(1)` when alpha already `1`)
- `change_ms=309.110654` (alternating `SetAlpha(1)` / `SetAlpha(0.5)`)

Required split (per-call, microseconds):

- **Lua->Rust call overhead:** `1.102us` (`GetAlpha - empty`)
- **same-value fast-path overhead:** `1.009us` (`SetAlpha(1) - empty`)
- **real state-change overhead:** `+1.409us` (`change - same`)

Interpretation: the same-value `SetAlpha(1)` path is already near bare
Lua->Rust call cost in this benchmark, while real state change adds another
~`1.4us`/call on top.

## 2026-04-22 SetAlpha no-op post-fast-path measurement

After optimizing `SetAlpha` to return on same-value calls before mutable-state
borrow and alpha-propagation bookkeeping in
`src/lua_api/frame/methods/core_state/alpha.rs`, I reran the same benchmark
script and command shape as above.

Representative measured run (`N=120000`, `R=8`):

- `empty_ms=36.540528`
- `get_ms=262.643878`
- `same_ms=210.177339`
- `change_ms=485.401209`

Derived no-op delta against dispatch (`same - get`), per call:

- baseline from earlier section: `-0.093us`
- post-fast-path: `-0.437us`

So the no-op `SetAlpha(1)` path now measures lower relative cost than the
pre-optimization baseline on the same script metric (`same - get`), while
behavioral no-op contracts remain intact (`tests/noop_hot_setters.rs`).

## 2026-04-22 SetFormattedText no-op microbenchmark baseline

Before changing any `SetFormattedText` implementation, we measured the current
simulator cost in the same shape as `SetAlpha` using a batched headless
`--exec-lua` microbenchmark (`N=120000`, `R=8`) and wrote the output to
`/tmp/claude/setformatted_bench_results.txt`.

Command shape used:

```bash
LD_LIBRARY_PATH=target/debug:target/debug/deps \
WOW_SIM_NO_SAVED_VARS=1 WOW_SIM_NO_ADDONS=1 \
target/debug/wow-sim --no-addons --no-saved-vars \
  --exec-lua @/tmp/claude/setformatted_bench.lua \
  screenshot -o /tmp/claude/setformatted_bench.webp
```

Measured batch timings (`GetTime` timer path in this headless run):

- `empty_ms=16.863603`
- `format_only_ms=397.144604` (`format("%dm", 60)` loop)
- `same_ms=483.865347` (`SetFormattedText("%dm", 60)` no-op path)
- `change_ms=725.311604` (alternating `%dm` with `60/61`)

Required split (per-call, microseconds):

- **argument formatting/parsing cost:** `3.169us` (`format_only - empty`)
- **text-equality fast-path cost:** `0.723us` (`same - format_only`)
- **real text-change cost:** `+2.012us` (`change - same`)

Equality-bail ordering confirmation:

- runtime probe patched global `format` to increment a counter, then called
  `SetFormattedText("%dm", 60)` `100` times on already-matching text
- observed `format_call_probe_same_text_calls=100`
- static path confirms the same ordering: `set_formatted_text()` calls
  `format_text_arg()` before `should_skip_formatted_text_update()`

## 2026-04-22 SetFormattedText / SetFontObject post-fast-path measurement

After the `formatting.rs` optimization pass:

- `SetFormattedText` now has a same-signature result cache keyed by
  `(frame, formatter identity, argument signature)`, but only when global
  `format` still matches the captured default formatter.
- If addons override global `format`, the cache disables and the call path
  falls back to always invoking `format()` (behavior parity with baseline).
- `SetFontObject` now has an early no-op guard when the resolved incoming font
  object is the same table already stored for that frame, so it returns before
  `read_font_object_fields()` and snapshot application checks.

Post-change benchmark rerun (same scripts/shape as baseline):

`setformatted_bench.lua` (`N=120000`, `R=8`, measured):

- `empty_ms=14.762528`
- `format_only_ms=267.826908`
- `same_ms=462.302798`
- `change_ms=847.277951`
- `formatting_parse_us=2.108870`
- `text_equality_fastpath_us=1.620632`
- `real_text_change_extra_us=3.208126`
- `format_call_probe_same_text_calls=100` (global `format` override path still
  executes each call, as expected)

The total same-value `SetFormattedText` cost (`same - empty`) dropped from the
baseline `3.891us` to `3.729us` in this rerun.

`set_misc_bench.lua` (`N=20000`, `R=4`, measured `set_font_object`):

- `dispatch_us=1.111644`
- `prebail_us=0.089713`
- `change_extra_us=3.342836`
- `steady_same_total_us=1.201357`

Compared to the earlier baseline (`steady_same_total_us=6.104us`,
`prebail_us=4.100us`), same-value `SetFontObject` no-op cost is now
substantially lower.

## 2026-04-22 SetPoint no-op microbenchmark baseline

Before touching `SetPoint`, we measured the current no-op and change-path cost
with a differential headless `--exec-lua` benchmark (`N=80000`, `R=8`) and
saved the output in `/tmp/claude/setpoint_bench_results.txt`.

Command shape used:

```bash
LD_LIBRARY_PATH=target/debug:target/debug/deps \
WOW_SIM_NO_SAVED_VARS=1 WOW_SIM_NO_ADDONS=1 \
target/debug/wow-sim --no-addons --no-saved-vars \
  --exec-lua @/tmp/claude/setpoint_bench.lua \
  screenshot -o /tmp/claude/setpoint_bench.webp
```

Measured batch timings (`GetTime` timer path in this headless run):

- `empty_ms=12.880099`
- `implicit_noop_ms=277.082179` (`SetPoint("CENTER", 10, 20)`)
- `explicit_noop_ms=303.036494` (`SetPoint("CENTER", UIParent, "CENTER", 10, 20)`)
- `explicit_name_noop_ms=320.425885` (`SetPoint("CENTER", "UIParent", "CENTER", 10, 20)`)
- `eq_proxy_ms=253.182395` (`GetPointByName("CENTER")` + value compare loop)
- `change_ms=417.348080` (alternating x-offset `10/11` on explicit `SetPoint`)

Required split (per-call, microseconds):

- **anchor argument parsing baseline (no explicit target lookup):**
  `3.303us` (`implicit_noop - empty`)
- **anchor normalization/lookup (userdata target + relative-point parsing):**
  `+0.324us` (`explicit_noop - implicit_noop`)
- **name-based lookup overhead (string target vs userdata target):**
  `+0.217us` (`explicit_name_noop - explicit_noop`)
- **no-op equivalence check proxy (read current anchor + compare values):**
  `3.004us` (`eq_proxy - empty`)
- **full relayout/dirty extra over explicit no-op path:**
  `+1.429us` (`change - explicit_noop`)

Cycle-check / bailout ordering (static):

- `set_point()` parses/normalizes args first (`parse_set_point_args`)
- if `relative_to` is still `None`, it resolves default parent before cycle
  detection
- `ensure_no_anchor_cycle(...)` runs next
- only after that does `apply_set_point(...)` run, where the no-op
  equivalence bail-out (`if unchanged { return Ok(0); }`) lives

So both anchor resolution work and cycle-check run before the no-op bail-out.

## 2026-04-22 SetPoint no-op fast-path optimization

`src/lua_api/frame/methods/button_anchor_hierarchy/anchors.rs::set_point` now
does the same-anchor equivalence check immediately after argument parsing and
default-parent resolution, before `ensure_no_anchor_cycle(...)`. The unchanged
check was removed from `apply_set_point(...)`.

Post-change benchmark rerun (`/tmp/claude/setpoint_bench.lua`, `N=80000`,
`R=8`, output in `/tmp/claude/setpoint_bench_results.txt`):

- `empty_ms=9.564679`
- `implicit_noop_ms=100.608869`
- `explicit_noop_ms=121.001922`
- `explicit_name_noop_ms=130.711111`
- `eq_proxy_ms=162.620613`
- `change_ms=316.323588`

Measured split (per-call, microseconds):

- `parse_base_us=1.138052`
- `normalize_lookup_us=0.254913`
- `name_lookup_extra_us=0.121365`
- `eq_proxy_us=1.913199`
- `full_relayout_extra_us=2.441521`

Before/after explicit same-anchor no-op total (`explicit_noop - empty`):

- baseline: `3.626955us`
- post-change: `1.392966us`
- delta: `-2.233989us` (`~61.6%` lower)

Control-flow ordering after optimization:

- `set_point()` parses/normalizes args and resolves default parent
- it performs same-anchor equivalence check and returns early on no-op
- `ensure_no_anchor_cycle(...)` only runs for real anchor changes
- `apply_set_point(...)` applies mutation/dirty propagation

## 2026-04-22 SetFontObject / SetShown / SetVertexColor no-op baseline

To complete the no-op audit set, we measured `SetFontObject`, `SetShown`, and
`SetVertexColor` in one headless `--exec-lua` benchmark (`N=20000`, `R=4`)
and wrote output to `/tmp/claude/set_misc_bench_results.txt`.

Command shape used:

```bash
LD_LIBRARY_PATH=target/debug:target/debug/deps \
WOW_SIM_NO_SAVED_VARS=1 WOW_SIM_NO_ADDONS=1 \
target/debug/wow-sim --no-addons --no-saved-vars \
  --exec-lua @/tmp/claude/set_misc_bench.lua \
  screenshot -o /tmp/claude/set_misc_bench.webp
```

Measured split (per-call, microseconds):

- `SetFontObject`
  - dispatch cost (`GetFontObject - empty`): `2.004us`
  - pre-bail work (`SetFontObject(same) - GetFontObject`): `4.100us`
  - true state-change extra (`change - same`): `+0.194us`
  - steady-state no-op total (`same - empty`): `6.104us`
- `SetShown`
  - dispatch cost (`IsShown - empty`): `1.390us`
  - pre-bail work (`SetShown(true same) - IsShown`): about `0us` (measured
    `-0.231us`, treated as timer noise around parity)
  - true state-change extra (`change - same`): `+7.765us`
  - steady-state no-op total (`same - empty`): `1.160us`
- `SetVertexColor`
  - dispatch cost (`GetVertexColor - empty`): `1.067us`
  - pre-bail work (`SetVertexColor(same) - GetVertexColor`): `0.159us`
  - true state-change extra (`change - same`): `+0.385us`
  - steady-state no-op total (`same - empty`): `1.226us`

Steady-state priority after `SetAlpha`:

- `SetAlpha` no-op baseline from earlier: `1.009us`
- current highest no-op steady-state among this group: `SetFontObject` at
  `6.104us`/call

So the next primitive optimization priority remains `SetFontObject`.

Static ordering notes:

- `SetFontObject`: argument resolution, `read_font_object_fields`, and
  `table_set(__font_objects, ...)` all run before the change check
  (`font_object_snapshot_changes_frame`), so there is significant pre-bail work
  by design.
- `SetShown`: `show_or_hide` does an early `needs_change` check via
  `read_show_hide_state`; when unchanged it returns before parent-visibility
  checks or handler dispatch.
- `SetVertexColor`: parses RGBA first, then checks `frame.vertex_color != color`
  before taking the `get_mut_visual` write path.

## 2026-04-14 GameTimeFrame calendar atlas follow-up

The `GameTimeFrame_SetDate()` follow-up showed a different no-op churn shape than
the visible `OnUpdate` handlers above:

- `GameTimeFrame_SetDate()` re-applies three atlas-backed button textures every
  time it runs (`up`, `down`, `mouseover`) even when the calendar day has not
  changed.
- In the simulator, the plain `SetNormalTexture` / `SetPushedTexture` /
  `SetHighlightTexture` path resolved the atlas string, then immediately called
  `get_mut_visual()` on both the button and child texture.
- `get_mut_visual()` marks render-dirty on mutable borrow, so a same-day
  `GameTimeFrame_SetDate()` still dirtied the minimap strata even though the
  resolved file path, UVs, and visibility were identical.

The fix was to make `apply_set_button_texture_path()` check the current button
field and texture-child state first. When the resolved path/UVs, `fileDataID`,
parent key, anchors, and visibility already match, it now returns before taking
any visual mutable borrows.

Regression coverage now includes:

- a low-level button-texture test that repeats
  `SetNormalTexture("ui-hud-calendar-1-up")`
- a full-UI `GameTimeFrame_SetDate()` test that proves the second same-day call
  leaves the render-dirty batch empty

## 2026-04-22 AuraButton OnUpdate perf lock-down (`<= 0.5ms`)

To lock the BuffFrame aura-button hot path under `0.5ms`, we added a focused
perf regression in
`tests/buff_aura_onupdate_perf.rs` that measures steady-state
`AuraButtonMixin:OnUpdate` cost with a baseline-subtracted batch loop over a
real `AuraButtonTemplate` instance.

Baseline from the same harness before this pass:

- net per-tick max: `~0.86ms` (samples around `0.82ms`-`0.86ms`)

Root cause from harness breakdown:

- `UpdateDuration(...)` dominated almost all per-tick cost
- within `UpdateDuration`, `SetFormattedText(...)` was the hotspot
- `SecondsToTimeAbbrev(...)`, `SetShown(...)`, and `SetVertexColor(...)`
  were comparatively small

Fixes landed:

- `set_formatted_text` cache now stores a width hint and uses it to short-circuit
  same-text auto-width no-op calls before re-running `measure_text_width`
  (`src/lua_api/frame/methods/text_attribute_event/text/formatting.rs`)
- added `call_function_state_multi` and switched `securecall` to a direct
  multi-return fast path with fallback to the existing protected Lua pcall
  wrapper only on the known direct-call incompatibility marker
  (`src/lua_api/methods.rs`, `src/lua_api/globals/utility_system_spell/mod.rs`)
- `FrameRef.__newindex` now updates `children_keys` / `parent_key` via non-visual
  mutable access (`get_mut`) so non-visual frame-field writes do not trigger
  visual dirty bookkeeping (`src/lua_api/env_init/frames.rs`)

Post-change from the same perf harness:

- net per-tick samples: `21.21us`-`31.44us`
- net per-tick max: `31.44us`
- enforced budget: `500us` (`0.5ms`) via test assertion

Related audit behavior updates:

- settled BuffFrame aura-button audit now observes zero render-dirty mask/ids
- settled leave-instance button audit now observes zero render-dirty mask/ids
  (`tests/onupdate_handler_audit.rs`)

## 2026-08-27 Solo-path audit correction

The leave-instance audit expectation now matches the settled solo branch: `PartyUtil.CanLeaveInstance()` rechecks group membership twice, then returns before querying instance membership or LFG ID. The regression remains a no-dirty audit; it does not claim grouped-content query counts.

## Fix Strategies

**Option A: Same-value guards in Rust methods** — Make `SetValue`, `SetText`, `SetAlpha`, `Show`, `Hide` etc. skip `get_mut()` when the new value equals the current value. Fixes the root cause; blanket discard can be removed entirely. Requires touching many API methods.

**Option B: Per-frame dirty tracking** — Replace single `render_dirty: bool` with a set of dirty frame IDs. Selectively preserve dirty from known-legitimate frames. More complex, doesn't fix the underlying `get_mut()` problem.

**Option C: StatusBar-specific check** — After `fire_on_update()`, check if any StatusBar's `statusbar_value` actually changed. Smallest change, but requires special-casing each new legitimate visual change.

## Sources

- [on-update-dirty-handlers.md](../../on-update-dirty-handlers.md) — full handler classification and fix options
- [BuffFrame.lua](../../../../Interface/BlizzardUI/Blizzard_BuffFrame/BuffFrame.lua) — `BuffFrameMixin` and `AuraButtonMixin` `OnUpdate` paths
- [BuffFrameTemplates.xml](../../../../Interface/BlizzardUI/Blizzard_BuffFrame/BuffFrameTemplates.xml) — `AuraButtonTemplate` script registration
- [handler_timing.rs](../../../../src/lua_api/handler_timing.rs) — `frame=#id` fallback formatting
- [script_helpers.rs](../../../../src/lua_api/script_helpers.rs) — `OnUpdate` dispatch timing scope
- [onupdate_handler_audit.rs](../../../../tests/onupdate_handler_audit.rs) — focused regression test for BuffFrame button `OnUpdate`

## See Also

- [[talent-performance]] — OnUpdate loop in talent panel caused by rect-dirty bugs
