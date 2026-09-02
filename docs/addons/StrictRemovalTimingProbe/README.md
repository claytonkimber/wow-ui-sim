# StrictRemovalTimingProbe

Captures addon-visible timing for the pinned 12.1 strict-removal surface on
interface `120100`. Interpret channel-specific results separately: retail 12.1
retains `C_RecruitAFriend.IsEnabled`, while PTR hides it after startup.

## Files and load order

The TOC lists:

```text
Bootstrap.lua [Bootstrap]
StrictRemovalTimingProbe.lua
```

`[Bootstrap]` does **not** reorder TOC files. The probe therefore records the
actual addon-facing sequence at:

- Bootstrap file execution
- normal file execution
- this addon's `ADDON_LOADED`
- subsequent `ADDON_LOADED` events when the observed state changes
- `VARIABLES_LOADED`
- `PLAYER_LOGIN`
- `PLAYER_ENTERING_WORLD`
- manual snapshots

This proves only phases visible after the addon begins loading. It cannot prove
earlier Blizzard-internal wrapper retirement or publication timing.

## Captured surface

The database records exact client version/build/date/interface metadata. Each bounded record contains direct and raw presence/type observations for:

- `C_DyeColor.GetDyeColorForItemLocation`
- `C_DyeColor.GetDyeColorForItem`
- `C_Housing.IsInsideOwnHouse`
- `C_Ping.GetContextualPingTypeForUnit`
- `C_RecruitAFriend.IsEnabled`
- `C_SuperTrack.GetNextWaypointForMap`
- `C_UnitAuras.TriggerPrivateAuraShowDispelType`
- `Enum.EditModeUnitFrameSetting.IconSize`
- `GetInventorySlotInfo`
- `GetCVar`
- `GetCVarDefault`
- `C_CVar.GetCVar`

For these CVars, the probe also calls each available accessor and stores only
primitive results, result types/presence, or bounded error strings:

- `lastLockedDelvesCompanionAbilities`
- `slugSuperSampling`

Functions and tables are never stored as observed values. Records are capped at
128 entries and captured strings at 160 characters. Subsequent `ADDON_LOADED`
records are retained only when the observed removal/CVar state changes, preserving
space for later lifecycle phases. Pre-SavedVariables records are buffered and
flushed when the addon's variables bind.

The probe attempts a non-mutating `hooksecurefunc` hook on
`C_AddOns.LoadAddOn`. The database records whether installation succeeded,
bounded error text if it failed, call count, and the last addon name observed.
The hook never replaces or wraps the API.

## Commands

- `/srtp snapshot [label]` — capture a manual snapshot.
- `/srtp status` — print record count, dropped-record count, and hook status.
- `/srtp reset` — clear captured records.
- `/strictremovalprobe` is an alias for `/srtp`.

Run `/reload` or log out after capture so `StrictRemovalTimingProbeDB` is
written to SavedVariables.

## Run

Install the directory as:

```text
Interface/AddOns/StrictRemovalTimingProbe/
```

Enable it on the 12.1 client, capture startup and manual states, then retrieve
`WTF/.../SavedVariables/StrictRemovalTimingProbe.lua`.
