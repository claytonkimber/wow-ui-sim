# Generated Stubs: Startup-Critical Audit

Date: 2026-04-09

This note narrows the remaining generated-stub risk surface to the functions
that still sit on startup-sensitive or panel-load-sensitive Blizzard paths.

## Method

- Exclude namespaces already promoted to handwritten implementations after the
  earlier broad stub audit, such as `C_PerksActivities`, `C_EncounterTimeline`,
  `C_FriendList`, `C_Macro`, and the `C_TooltipInfo` getters that used to be
  generated.
- Prefer call sites hit during UI startup, settings registration, objective
  tracker layout, or common top-level panels.
- Treat a generated stub as high risk when it returns `false`, `0`, `()`, or an
  empty table where Blizzard code expects typed fields or meaningful state.
- The direct `wow-sim lua-errors` scan did not yield a stable JSON result in the
  current harness before timeout, so this audit is based on current source call
  paths plus existing regression evidence already in-tree.

## Highest-Priority Findings

| Priority | Namespace / functions | Current generated behavior | Blizzard load path | Why it matters |
|---|---|---|---|---|
| 1 | `C_EncounterWarnings.GetEditModeWarningInfo`, `GetColorForSeverity`, `IsFeatureAvailable`, `IsFeatureEnabled`, `PlaySound` | Empty tables, `false`, `0` | `Blizzard_SettingsDefinitions_Frame/AdvancedOptions.lua`, `Blizzard_EncounterWarnings/EncounterWarnings.lua`, `Blizzard_EncounterWarnings/EncounterWarningsView.lua` | Boss-warning settings stay disabled/hidden and edit-mode preview uses an empty warning shape instead of real warning data. |
| 2 | `C_InstanceEncounter.IsEncounterInProgress`, `IsEncounterLimitingResurrections`, `IsEncounterSuppressingRelease`, `ShouldShowTimelineForEncounter` | Always `false` | `Blizzard_EncounterWarnings/EncounterWarnings.lua`, `Blizzard_EncounterTimeline/EncounterTimeline.lua`, `Blizzard_StaticPopup_Game/GameDialogDefsUtil.lua`, `Blizzard_StaticPopup_Game/Mainline/GameDialogDefs.lua` | Encounter-state dependent UI never transitions into its real active mode, so warnings, timeline gating, and death/release popup behavior stay on the safe stub path. |
| 3 | `C_WeeklyRewards.GetActivityEncounterInfo`, `GetSortedProgressForActivity`, `HasInteraction` | Empty tables, `false` | `Blizzard_WeeklyRewards/Blizzard_WeeklyRewards.lua` | Great Vault already has partial handwritten state, but the panel still falls back to read-only and loses encounter/progress detail because these panel-facing methods remain generated. |
| 4 | `C_LFGList.GetPremadeGroupFinderStyle`, `GetApplicationInfo`, `CanCreateScenarioGroup` | `0`, `()`, `false` | `Blizzard_UIParent/Shared/UIParent.lua`, `Blizzard_UIParent/Mainline/UIParent.lua`, `Blizzard_ObjectiveTracker/Blizzard_ScenarioObjectiveTracker.lua` | Startup LFG style selection and scenario “Find Group” affordances are still pinned to stub values. `GetApplications()` is handwritten, but the paired per-application lookup is still generated. |
| 5 | `C_Garrison.GetMissionEncounterIconInfo`, `GetFollowerLink`, `GetBasicMissionInfo`, `GetCompleteTalent`, `GetTalentInfo`, `HasGarrison`, `IsOnGarrisonMap` | Empty tables or no values | `Blizzard_FrameXML/Mainline/AlertFrameSystems.lua`, `Blizzard_FrameXML/Mainline/AlertFrames.lua`, `Blizzard_UIParent/Mainline/UIParent.lua` | Garrison alerts and landing-page/minimap decisions are loaded into the baseline UI, but they remain data-starved because several mission/follower helpers still come from generated stubs. |

## Notes By Area

### 1. Encounter warnings

The risk here is shape, not just truthiness.

- The current generated/stub registration surfaces still give `C_EncounterWarnings.GetEditModeWarningInfo` an empty table and `IsFeatureAvailable` / `IsFeatureEnabled` hardcoded `false`.
- [`Interface/BlizzardUI/Blizzard_EncounterWarnings/EncounterWarnings.lua`](../Interface/BlizzardUI/Blizzard_EncounterWarnings/EncounterWarnings.lua) feeds `GetEditModeWarningInfo(...)` directly into `ShowWarning(...)`, which expects fields like `severity`, `shouldShowWarning`, and formatted text inputs.
- [`Interface/BlizzardUI/Blizzard_SettingsDefinitions_Frame/AdvancedOptions.lua`](../Interface/BlizzardUI/Blizzard_SettingsDefinitions_Frame/AdvancedOptions.lua) uses `C_EncounterWarnings.IsFeatureAvailable()` and `IsFeatureEnabled()` when registering the boss-warning settings.

This is still one of the most direct generated-stub paths into visible UI state.

### 2. Instance encounter state

These methods are pure gating flags today.

- The current generated/stub registration surfaces hardcode all four `C_InstanceEncounter` methods to `false`.
- [`Interface/BlizzardUI/Blizzard_EncounterTimeline/EncounterTimeline.lua`](../Interface/BlizzardUI/Blizzard_EncounterTimeline/EncounterTimeline.lua) uses `IsEncounterInProgress()` and `ShouldShowTimelineForEncounter()` to decide whether to show the live timeline path.
- [`Interface/BlizzardUI/Blizzard_StaticPopup_Game/GameDialogDefsUtil.lua`](../Interface/BlizzardUI/Blizzard_StaticPopup_Game/GameDialogDefsUtil.lua) and [`Interface/BlizzardUI/Blizzard_StaticPopup_Game/Mainline/GameDialogDefs.lua`](../Interface/BlizzardUI/Blizzard_StaticPopup_Game/Mainline/GameDialogDefs.lua) use the suppress-release and limited-resurrection checks in gameplay dialogs.

Nothing crashes here, but the simulator is forced onto the “never in encounter”
branch everywhere.

### 3. Weekly rewards

`C_WeeklyRewards` is only partially promoted today.

- [`src/lua_api/globals/c_misc_api_ui.rs`](../src/lua_api/globals/c_misc_api_ui.rs) already supplies `GetActivities`, `HasAvailableRewards`, and `CanClaimRewards`.
- The current generated/stub registration surfaces still supply `GetActivityEncounterInfo`, `GetSortedProgressForActivity`, and `HasInteraction`.
- [`Interface/BlizzardUI/Blizzard_WeeklyRewards/Blizzard_WeeklyRewards.lua`](../Interface/BlizzardUI/Blizzard_WeeklyRewards/Blizzard_WeeklyRewards.lua) uses those generated methods for read-only mode, encounter grouping, and world-tier progress rendering.

This is the clearest remaining “partially implemented namespace still broken on
panel load” case.

### 4. LFG list

`C_LFGList` already has a handwritten core search path, but several baseline UI
consumers still fall through to generated results.

- [`src/lua_api/globals/c_stubs_api.rs`](../src/lua_api/globals/c_stubs_api.rs) provides `GetApplications()` and the search-result APIs.
- The current generated/stub registration surfaces still provide `GetApplicationInfo`, `GetPremadeGroupFinderStyle`, and `CanCreateScenarioGroup`.
- [`Interface/BlizzardUI/Blizzard_UIParent/Mainline/UIParent.lua`](../Interface/BlizzardUI/Blizzard_UIParent/Mainline/UIParent.lua) iterates applications and calls `GetApplicationInfo(...)`.
- [`Interface/BlizzardUI/Blizzard_UIParent/Shared/UIParent.lua`](../Interface/BlizzardUI/Blizzard_UIParent/Shared/UIParent.lua) checks `GetPremadeGroupFinderStyle()`.
- [`Interface/BlizzardUI/Blizzard_ObjectiveTracker/Blizzard_ScenarioObjectiveTracker.lua`](../Interface/BlizzardUI/Blizzard_ObjectiveTracker/Blizzard_ScenarioObjectiveTracker.lua) gates the “Find Group” button on `CanCreateScenarioGroup(...)`.

### 5. Garrison alert helpers

These are not the entire garrison subsystem, but they are baseline UI call
sites that can surface as obviously empty toasts or disabled branches.

- [`src/lua_api/globals/c_misc_api_game.rs`](../src/lua_api/globals/c_misc_api_game.rs) only handwrites a small `C_Garrison` subset such as `GetLandingPageGarrisonType`.
- The current generated/stub registration surfaces still provide mission/follower/talent helpers as empty returns.
- [`Interface/BlizzardUI/Blizzard_FrameXML/Mainline/AlertFrameSystems.lua`](../Interface/BlizzardUI/Blizzard_FrameXML/Mainline/AlertFrameSystems.lua) and [`Interface/BlizzardUI/Blizzard_FrameXML/Mainline/AlertFrames.lua`](../Interface/BlizzardUI/Blizzard_FrameXML/Mainline/AlertFrames.lua) read those helpers to populate alerts and tooltips.

### Resolved empty-state slice: loot history

[`src/c_api/c_loot_history.rs`](../src/c_api/c_loot_history.rs) now owns the
retail/PTR `C_LootHistory` read surface. Empty state returns:

- `GetAllEncounterInfos()` → `{}`
- `GetInfoForEncounter(...)` → `nil`
- `GetSortedDropsForEncounter(...)` → `{}`
- `GetSortedInfoForDrop(...)` → `nil`
- `GetLootHistoryTime()` → `0.0`

This is bounded to loading and showing the real `GroupLootHistoryFrame` empty
state with zero new Lua errors during `Show()`.

Populated encounters, drops, rolls, event producers, persistence, and timer
progression remain unmodeled.

## Recommended Order

1. Promote `C_EncounterWarnings` out of the generated stub surfaces.
2. Seed `C_InstanceEncounter` state in tandem so warning/timeline features can
   actually become active.
3. Finish the missing `C_WeeklyRewards` panel-facing methods.
4. Replace the remaining startup-visible `C_LFGList` generated methods.
5. Seed the garrison alert helper methods that baseline UI already touches.

## Out of Scope For This Note

- Large generated namespaces that are already under separate diff-sweep work.
- Tooltip getters that were already promoted out of generated stubs.
- Readability-only refactors inside generated stub surfaces.
