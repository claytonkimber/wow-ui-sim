# Addon Load Order

Investigation into historical Blizzard addon load-order assumptions affecting bag-button initialization and ItemButton utilities.

> **Status: RESOLVED (verified 2026-06-19; ItemButton state corrected 2026-08-26).** The symptom no longer reproduces in
> the current build. `PaperDollItemSlotButton_OnLoad` is defined by the time the
> bag buttons load, their `OnLoadInternal` runs to completion, and there are no
> related Lua errors at startup. The old re-run workaround has been removed. The
> historical analysis is kept below for context.

## Verification (current build)

- `lua-errors` (full addon load) reports **no** PaperDoll / ItemSlotButton errors.
- At runtime `type(PaperDollItemSlotButton_OnLoad) == "function"`.
- `CharacterBag0Slot` is fully initialized: `GetID()` = 20, has `icon`,
  `UpdateTextures`, and `IsEventRegistered("ITEM_LOCK_CHANGED")` = `true`. That
  event registration happens in `OnLoadInternal` **after** the
  `PaperDollItemSlotButton_OnLoad` call, proving OnLoad completed rather than
  erroring partway and being swallowed by `xpcall`.

## Current ItemButton runtime ordering

`WowLuaEnv::new` does not publish `ItemButtonUtil`. Loading `Blizzard_UIParent` instantiates the visible UIParent frame, whose `UIParent_OnShow` calls `C_AddOns.LoadAddOn("Blizzard_AccountStore")`. Runtime `C_AddOns` loads the game foundation lane first, including `Blizzard_FrameXMLUtil`; `ItemUtil.lua` publishes `ItemButtonUtil` before eager discovery reaches `Blizzard_ItemButton`.

Commit `9b1ba9bcd` removed the test that asserted the obsolete intermediate state (`ItemButtonMixin` present while `ItemButtonUtil` remained absent). The production load-order snapshot is unchanged and remains green. The retained contract is the resolved addon snapshot and its runtime behavior, not the transient global state observed by that removed test.

## Why it works now (root cause)

`PaperDollItemSlotButton_OnLoad` is defined in
`Blizzard_UIPanels_Game/.../PaperDollFrame.lua`. The bag buttons
(`Blizzard_MainMenuBarBagButtons`, which declares no dependencies) call it during
their XML `OnLoad`, and `OnLoad` fires synchronously in
`src/loader/xml_frame/finalize.rs` during that addon's load. So this only works
if `Blizzard_UIPanels_Game` loads first.

It would not by any naive ordering: alphabetically `M` < `U`, in
`ui-toc-list.txt` the bag addon is line 209 vs UIPanels_Game line 327, and plain
topological sort leaves the dep-less bag addon free to load early. All three give
the historically broken order.

What flips it is the **two-pass eager loader** (`topological_sort_addons` →
`emit_early_addons` in `src/loader/addon_order.rs`), via a **transitive
`LoadFirst`**:

1. `Blizzard_EnvironmentCleanup` has `## LoadFirst: 1` **and**
   `## Dependencies: ... Blizzard_UIPanels_Game ...`.
2. The early pass emits each `LoadFirst` addon through `emit_addon_recursive`,
   which emits that addon's dependencies **first, recursively**.
3. So `Blizzard_UIPanels_Game` is pulled into the early pass — before any
   non-`LoadFirst` addon, including `Blizzard_MainMenuBarBagButtons` in the later
   "remaining" pass.

By the time the bag buttons' XML finalizes and fires `OnLoad`,
`PaperDollItemSlotButton_OnLoad` already exists, so `OnLoadInternal` runs to
completion. Note the irony vs. the historical analysis below: `UIPanels_Game` is
not itself `LoadFirst` on Mainline, but `LoadFirst` still fixes this
**transitively** through `EnvironmentCleanup`'s dependency edge.

## What changed

The OnLoad re-run replay was removed as obsolete:

- `70fca4e25` — "Drop obsolete bag bar workaround replay" (gutted the OnLoad
  re-run from `workarounds_bags.rs`)
- `747cb23b5` — "Skip PaperDollItemSlotButton_OnLoad for backpack button" (the
  backpack's `OnLoadInternal` never calls it; the replay was wrongly forcing it
  and grabbing the head-slot icon)
- `d4f1287f9` — "Remove bag token tracker workaround" (deleted
  `workarounds_bags.rs` entirely)

The only surviving bag-adjacent workaround is
`src/lua_api/workarounds/temporary/character_frame_surface_refresh.rs`, which
re-runs `PaperDollItemSlotButton_Update` and icon textures on the character
panel — **not** `_OnLoad`.

## Historical finding (no longer reproduces)

`Blizzard_MainMenuBarBagButtons` (line 209 in `ui-toc-list.txt`) called
`PaperDollItemSlotButton_OnLoad` during frame creation, but that function is
defined in `Blizzard_UIPanels_Game` (line 327), which loaded later. The bag
buttons' `OnLoadInternal` failed partway through, skipping event registration
and slot setup. The same pattern applied to `PaperDollItemSlotButton_Update` and
`PaperDollItemSlotButton_OnShow`. The old `ItemButtonUtil.*` note was an
invalid intermediate-state claim: `WowLuaEnv::new` starts without that global,
but runtime loading of `Blizzard_UIParent` enters the `C_AddOns` foundation
lane, loads `Blizzard_FrameXMLUtil`, and runs `ItemUtil.lua` before eager
discovery reaches `Blizzard_ItemButton`.

## Historical root cause

`Blizzard_MainMenuBarBagButtons_Mainline.toc` has **no dependency declarations** —
no `Dependencies`, `RequiredDep`, `LoadFirst`, or `LoadWith`. The real WoW client
had the same error; both WoW and wowless wrap OnLoad in `xpcall`, catching the
error and continuing frame creation with the frame partially initialized.

`LoadFirst` was never a fix for this: only 6 Mainline addons carry `LoadFirst: 1`
(FrameXML, Glue frames, etc.), and `Blizzard_UIPanels_Game` is not one of them on
Mainline. Adding it would have been a fabricated ordering the real client doesn't
ship.

## Load Order Source

`vendor/wow-ui-source/Interface/ui-toc-list.txt` is the authoritative load order
from the real WoW client. Our loader uses topological sort on TOC dependencies
with alphabetical tiebreaking, producing the same relative order.

## Sources

- [addon-load-order-investigation.md](../../addon-load-order-investigation.md) — full historical analysis and current ItemButton ordering
- [load_order.rs](../../../tests/load_order.rs) — retained load-order snapshot and eager-order assertion

## See Also

- [[addon-loading]] — TOC discovery and runtime foundation loading
