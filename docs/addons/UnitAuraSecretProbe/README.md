# UnitAuraSecretProbe

Captures the addon-visible WoW 12.1 `UnitAura` / `C_UnitAuras` secret-value contract without fabricating a Blizzard-secure caller.

## Purpose

The simulator currently exposes ordinary Lua aura tables and event arguments. This probe records what an actual addon can observe:

- legacy `UnitAura`, `UnitBuff`, and `UnitDebuff` return tuples;
- `C_UnitAuras.GetAuraDataByIndex`, instance-ID, and spell-name lookups when available;
- AuraData field reads, `rawget`, iteration, equality, concatenation, arithmetic, pass-through, table storage, `tostring`, `issecretvalue`, and `canaccessvalue` outcomes;
- repeated lookup identity;
- `UNIT_AURA` arguments and known update-info fields;
- exact `pcall` success/error shapes for inaccessible or secret values.

Every operation is bounded and wrapped in `pcall`. Secret values are summarized by type/access metadata and are not deliberately serialized as raw values.

## Install

Copy `UnitAuraSecretProbe` to the retail or PTR AddOns directory and enable it. The TOC targets interface `120100`.

## Capture procedure

1. Log in and wait for the automatic player capture.
2. Run `/uasp status`.
3. Mark each controlled scenario before changing aura state:
   - `/uasp mark player-aura-add`
   - apply or gain a known player buff/debuff;
   - `/uasp capture player`
   - `/uasp mark player-aura-remove`
   - remove or let the aura expire;
   - `/uasp capture player`
4. Target a friendly or hostile unit. Target changes automatically create a target capture; repeat manually with `/uasp capture target` when needed.
5. Use `/uasp mark <label>` before any additional refresh/reapplication scenario.
6. Run `/uasp status`, then `/reload` or log out to flush SavedVariables.

Commands:

```text
/uasp capture <unit>
/uasp mark <phase>
/uasp status
/uasp reset
```

## Artifact

Retain the raw file from:

```text
WTF/Account/<ACCOUNT>/SavedVariables/UnitAuraSecretProbe.lua
```

Record the client build and capture procedure with the artifact. The database contains bounded `captures`, `events`, and `markers` arrays plus build metadata.

## Limitations

This probe runs only as addon-tainted code. It cannot establish Blizzard-internal secure caller access, secure delegate behavior, or privileged AuraData visibility. Those rows must remain open unless a real Blizzard-provided execution path or other authoritative client evidence exists. Event coverage also depends on the controlled aura changes performed during the capture session.
