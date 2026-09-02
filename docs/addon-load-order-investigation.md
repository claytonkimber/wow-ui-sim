# Addon Load Order Investigation

> **Status: RESOLVED (verified 2026-06-19).** This symptom no longer reproduces.
> `PaperDollItemSlotButton_OnLoad` is defined by the time the bag buttons load,
> their `OnLoadInternal` runs to completion (e.g.
> `CharacterBag0Slot:IsEventRegistered("ITEM_LOCK_CHANGED")` is `true`, which is
> set after the OnLoad call), and `lua-errors` reports no PaperDoll errors. The
> `workarounds_bags.rs` re-run referenced below has been removed (`70fca4e25`
> dropped the replay, `d4f1287f9` deleted the file). The analysis below is kept
> for historical context. See the wiki version:
> [investigations/addon-load-order.md](wiki/investigations/addon-load-order.md).

## Problem

`Blizzard_MainMenuBarBagButtons` calls `PaperDollItemSlotButton_OnLoad(self)` during OnLoad, but that function is defined in `Blizzard_UIPanels_Game/Mainline/PaperDollFrame.lua` which historically loaded later.

## Current ItemButton runtime ordering

`WowLuaEnv::new` does not publish `ItemButtonUtil`. Loading `Blizzard_UIParent` instantiates the visible UIParent frame, whose `UIParent_OnShow` calls `C_AddOns.LoadAddOn("Blizzard_AccountStore")`. Runtime `C_AddOns` loads the game foundation lane first, including `Blizzard_FrameXMLUtil`; `ItemUtil.lua` publishes `ItemButtonUtil` before eager discovery reaches `Blizzard_ItemButton`.

Commit `9b1ba9bcd` removed the obsolete test that expected `ItemButtonMixin` to exist while `ItemButtonUtil` remained unavailable. The production load-order snapshot is unchanged and green. This intermediate global state is not a valid load-order contract.

## Load Order Source

Blizzard ships `Interface/ui-toc-list.txt` — a manifest that defines the exact addon load order. Wowless uses this file directly. Our loader uses topological sort on TOC dependencies + alphabetical tiebreaking, which produces the same relative order for these addons.

Relevant positions in `ui-toc-list.txt`:
- Line 138: `Blizzard_FrameXMLUtil`
- Line 209: `Blizzard_MainMenuBarBagButtons`
- Line 327: `Blizzard_UIPanels_Game`

## TOC Analysis

`Blizzard_MainMenuBarBagButtons_Mainline.toc` has **no dependency declarations at all** — no `Dependencies`, `RequiredDep`, `LoadFirst`, or `LoadWith`. Just `AllowLoad: Game`.

## What Happens During Load

1. `Blizzard_MainMenuBarBagButtons` loads:
   - `MainMenuBarBagButtons.lua` defines `BaseBagSlotButtonMixin.OnLoadInternal` (calls `PaperDollItemSlotButton_OnLoad`)
   - `MainMenuBarBagButtons.xml` creates concrete frames (`MainMenuBarBackpackButton`, `CharacterBag0Slot`, etc.) parented to `UIParent`
   - OnLoad fires immediately → `PaperDollItemSlotButton_OnLoad` is nil → **error**
   - Rest of `OnLoadInternal` is skipped (event registration, slot setup, etc.)

2. `Blizzard_UIPanels_Game` loads later:
   - `PaperDollFrame.lua:1496` defines `PaperDollItemSlotButton_OnLoad`

The ItemButton utility follows a different path: the runtime foundation load
through `Blizzard_FrameXMLUtil` executes `ItemUtil.lua` before eager discovery
reaches `Blizzard_ItemButton`, so `ItemButtonUtil` is already published by then.

## Historical Workaround (removed)

`src/lua_api/workarounds_bags.rs` used to re-run `PaperDollItemSlotButton_OnLoad`, `PaperDollItemSlotButton_Update`, and `UpdateTextures` on each bag button after all addons loaded. This replay was dropped (`70fca4e25`) and the file later deleted (`d4f1287f9`) once OnLoad began completing on its own. The only surviving bag-adjacent workaround is `src/lua_api/workarounds/temporary/character_frame_surface_refresh.rs`, which re-runs `PaperDollItemSlotButton_Update` and icon textures on the character panel — not `_OnLoad`.

## Resolution

**The real client has this same error.** Both WoW and wowless wrap OnLoad script handlers in `xpcall` — the error is caught, the rest of `OnLoadInternal` is skipped, but frame creation continues. Confirmed in wowless: `wowless/modules/security.lua` uses `xpcall(fun, ErrorHandler, ...)` for all script dispatch.

Our loader does the same (`loader/xml_lifecycle.rs` catches OnLoad errors and continues). In the current build OnLoad no longer errors, so no recovery is needed; historically, later events (`PLAYER_ENTERING_WORLD`, `BAG_UPDATE_DELAYED`) and the now-removed replay re-ran any skipped initialization.

**There is no missing load order mechanism.** The architectural conclusion still holds — relying on `xpcall` recovery mirrors the real client — but the specific partial-OnLoad failure no longer occurs.

### Why it no longer fails: transitive LoadFirst

Although `Blizzard_UIPanels_Game` is not itself `LoadFirst` on Mainline, the two-pass eager loader (`topological_sort_addons` → `emit_early_addons` in `src/loader/addon_order.rs`) pulls it into the early pass anyway:

- `Blizzard_EnvironmentCleanup` is `## LoadFirst: 1` and lists `Blizzard_UIPanels_Game` in its `## Dependencies`.
- `emit_addon_recursive` emits a LoadFirst addon's dependencies first, recursively, so `Blizzard_UIPanels_Game` (and thus `PaperDollItemSlotButton_OnLoad`) loads before any non-LoadFirst addon.
- `Blizzard_MainMenuBarBagButtons` (no deps) loads in the later "remaining" pass, by which point the function exists, so its XML `OnLoad` completes.

So the original "Addons With LoadFirst (Mainline)" note below is correct that UIPanels_Game lacks `LoadFirst` directly — but `LoadFirst` still resolves this transitively through `EnvironmentCleanup`'s dependency edge, combined with the eager recursive-dependency emission that the loader didn't do at the time of the original investigation.

## Other Functions Called Before Defined

Same pattern — `Blizzard_MainMenuBarBagButtons` also uses:
- `PaperDollItemSlotButton_Update` (from `Blizzard_UIPanels_Game`)
- `PaperDollItemSlotButton_OnShow` (from `Blizzard_UIPanels_Game`)
- `ItemButtonUtil.*` (from `Blizzard_FrameXMLUtil/ItemUtil.lua`) — not a valid intermediate-state assertion; it is published by the runtime foundation lane before eager discovery reaches `Blizzard_ItemButton`

## Addons With LoadFirst (Mainline)

Only 6 Mainline addons have `LoadFirst: 1`:
- Blizzard_EnvironmentCleanup
- Blizzard_FrameXML
- Blizzard_GlueMenuFrame
- Blizzard_GlueParent
- Blizzard_GlueXMLBase
- Blizzard_GlueXML

Plus shared (non-flavor) TOCs: Blizzard_ClassTrial, Blizzard_Flyout, Blizzard_MapCanvasSecureUtil, Blizzard_Menu, Blizzard_CatalogShopSharedTemplates, Blizzard_CatalogShopSharedUtil.

`Blizzard_UIPanels_Game` has `LoadFirst` only on Classic TOC, not Mainline.

## ui-toc-list.txt

Located at `vendor/wow-ui-source/Interface/ui-toc-list.txt`. This is the authoritative static load order from the real WoW client. Runtime `C_AddOns.LoadAddOn` dependency handling is an additional ordering path and is why the removed ItemButton intermediate-state test was obsolete.
