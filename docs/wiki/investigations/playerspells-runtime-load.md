# PlayerSpells runtime load

## Summary

Retail `PlayerSpellsUtil.ToggleSpellBookFrame()` and `ToggleClassTalentFrame()` can demand-load `Blizzard_PlayerSpells` from keybindings. The simulator must preserve the active rilua call frame while `C_AddOns.LoadAddOn()` runs nested addon loads and `ADDON_LOADED`; otherwise the caller sees a bare `not a function` even though the addon finished loading. A live retail 12.1.0 capture (build 69497, interface 120100) also confirms that opening PlayerSpells replaces CharacterFrame through both direct and real toggle paths.

## Findings

- `C_AddOns.LoadAddOn("Blizzard_PlayerSpells")` reached `event Blizzard_PlayerSpells` but still raised after returning to the original Lua call. Preserving `top`, `base`, and `ci` around runtime addon loading keeps the native function ABI stable before pushing `LoadAddOn` return values.
- `PlayerSpellsFrame_LoadUI` is needed before the real addon is loaded, and the fallback should prefer modeled `C_AddOns.LoadAddOn` over `UIParentLoadAddOn`.
- The talents tab can show before some child OnLoad work has run. The temporary PlayerSpells backfill seeds ModelScene camera tables and PvP talent slot indices before showing the ClassTalents tab.
- PvP talent slot defaults must leave `selectedTalentID` nil for empty slots. Lua treats `0` as truthy, causing Blizzard code to query talent ID 0 and index a nil talent info record.

## Live retail panel contract (2026-08-28)

The live client capture targets WoW 12.1.0, build 69497, interface 120100. Both panel paths replace CharacterFrame with PlayerSpells:

- `ShowUIPanel(CharacterFrame)` followed by `ShowUIPanel(PlayerSpellsFrame)` leaves CharacterFrame closed and PlayerSpells open.
- `ToggleCharacter("PaperDollFrame")` followed by `PlayerSpellsUtil.ToggleSpellBookFrame()` leaves CharacterFrame closed and PlayerSpells open.
- Every recorded call in both scenarios, including reset hides, completed successfully (`ok = true`).
- CharacterFrame starts at width 338, expands to 540 when opened, and returns to width 338 when PlayerSpells replaces it.
- Captured non-nil `UIPanelWindows`/`UIPanelLayout-*` attributes: CharacterFrame `whileDead=1`, `pushable=3`; PlayerSpellsFrame `area="centerOrLeft"`, `pushable=3`, `whileDead=1`, `allowOtherPanels=1`, `checkFit=1`, `yoffset=75`.
- The probe recorded empty panel-slot tables. This is inconclusive: `GetUIPanel` belongs to the private `FramePositionDelegate`, not `UIParent`, so the probe's `UIParent:GetUIPanel(...)` lookup cannot establish active slot occupancy.

The production simulator panel behavior was unchanged. Commit `38bc75892` changed the test fixture to load `Blizzard_PlayerSpells` through dependency-aware `C_AddOns.LoadAddOn` rather than directly loading its TOC.

## Verification

- `keybind_n_loads_blizzard_player_spells_and_shows_talents`
- `raw_toggle_spellbook_frame_loads_blizzard_player_spells_and_shows_spellbook`
- `installs_pvp_talent_default_shapes`
- `installs_playerspells_util_bootstrap_defaults`

## 2026-08-27 SpellBook fallback completion

The global `ToggleSpellBook()` path must not treat a successful Lua call as proof that `PlayerSpellsUtil.ToggleSpellBookFrame()` handled the toggle. The temporary bootstrap wrapper can return `nil` without error while a failed `Blizzard_PlayerSpells` load leaves a partially initialized, hidden `PlayerSpellsFrame`; treating that return as handled skipped the simulator fallback and left `SimState.open_panels["SpellBook"]` closed in a bare environment.

The fix checks whether `PlayerSpellsFrame` reached the expected toggled visibility after the helper call. If not, `ToggleSpellBook()` uses its existing `open_panels`/frame-visibility fallback. Real full-UI helper behavior remains Blizzard-owned; the fallback covers only the helper no-op or failure path.

## Sources

- `/tmp/CharacterPlayerSpellsProbe-live-2026-08-28.lua` — live retail SavedVariables capture; SHA-256 `40dcf028acd5605d675810abbfc9eb8aa63147425ed5f8bb8b3b54b78c997595`
- [test_showuipanel_toggles.rs](../../../tests/test_showuipanel_toggles.rs) — commit `38bc75892` fixture using dependency-aware `C_AddOns.LoadAddOn`
- [panel_toggle_verbs.rs](../../../src/lua_api/globals/panel_toggle_verbs.rs) — SpellBook helper/fallback decision
- [panel_toggle_verbs.rs tests](../../../tests/panel_toggle_verbs.rs) — bare-environment regression coverage
- [player_spells_onload_backfill.rs](../../../src/lua_api/workarounds/temporary/player_spells_onload_backfill.rs) — temporary helper bootstrap
