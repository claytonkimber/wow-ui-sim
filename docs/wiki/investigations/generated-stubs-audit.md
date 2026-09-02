# Generated Stubs Audit

Audit of generated-stub functions that still sit on startup-sensitive or panel-load-sensitive Blizzard paths. Date: 2026-04-09. Updated: 2026-08-23.

## Priority Findings

| Priority | Namespace / Functions | Risk |
|---|---|---|
| 1 | `C_EncounterWarnings.GetEditModeWarningInfo`, `IsFeatureAvailable`, `IsFeatureEnabled` | Boss-warning settings disabled; edit-mode preview uses empty shape |
| 2 | `C_InstanceEncounter.IsEncounterInProgress`, `ShouldShowTimelineForEncounter`, etc. | UI always on "never in encounter" branch; warnings/timeline/death dialogs never activate |
| 3 | `C_WeeklyRewards.GetActivityEncounterInfo`, `GetSortedProgressForActivity`, `HasInteraction` | Great Vault panel loses encounter/progress detail (namespace is partially promoted) |
| 4 | `C_LFGList.GetPremadeGroupFinderStyle`, `GetApplicationInfo`, `CanCreateScenarioGroup` | Startup LFG style and scenario "Find Group" pinned to stub values |
| 5 | `C_Garrison` mission/follower/talent helpers | Garrison alerts and minimap decisions remain data-starved |

## Key Pattern

A generated stub is high-risk when it returns `false`, `0`, `()`, or an empty table where Blizzard code expects typed fields or meaningful state — particularly on paths that run at startup, settings registration, objective tracker layout, or common panel load.

## Partially Promoted Namespaces

`C_WeeklyRewards` is the clearest "partially implemented, still broken on panel load" case: `GetActivities`, `HasAvailableRewards`, and `CanClaimRewards` are handwritten; `GetActivityEncounterInfo`, `GetSortedProgressForActivity`, and `HasInteraction` remain generated.

`C_LFGList` similarly has a handwritten search core but `GetApplicationInfo` and `GetPremadeGroupFinderStyle` are still generated, affecting `UIParent.lua` iteration on startup.

## Resolved Empty-State Slice

`C_LootHistory` is no longer an unresolved generated-only priority on retail/PTR. The state-backed surface returns `GetAllEncounterInfos()` and `GetSortedDropsForEncounter(...)` as empty tables, keyed lookups as `nil`, and `GetLootHistoryTime()` as `0.0`. The real `GroupLootHistoryFrame` loads and shows its empty state with zero new Lua errors during `Show()`.

Populated encounters, drops, rolls, event producers, persistence, and timer progression remain unmodeled.

## Recommended Order

1. Promote `C_EncounterWarnings` out of the generated stub surfaces
2. Seed `C_InstanceEncounter` state so warning/timeline features can activate
3. Finish missing `C_WeeklyRewards` panel-facing methods
4. Replace remaining startup-visible `C_LFGList` generated methods
5. Seed garrison alert helper methods

## Sources

- [generated-stubs-startup-audit.md](../../generated-stubs-startup-audit.md) — full findings with call site references

## See Also

- [[addon-load-order]] — startup load sequence context
