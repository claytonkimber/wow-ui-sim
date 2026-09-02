# PrivateScriptObjectProbe

Addon-scope probe for the retail/PTR 12.1 private-script-object surface.

## What it does

- Loads `Blizzard_AuraContainer` through `C_AddOns.LoadAddOn`.
- Instantiates Blizzard's existing `CustomAuraContainerTemplate` on the XML fixture host.
- Compares the public object with `GetForbiddenObjectTable(publicObject)`.
- Records bounded, `pcall`-wrapped observations for:
  - object type, bounded `tostring`, self equality, `rawequal`, metatable access, and selected raw/direct keys;
  - selected public/private keys from the CustomAuraContainer mixins;
  - bounded `GetChildren()` traversal after creating one ordinary child;
  - `GetScript`, `SetScript`, `HookScript`, and `hooksecurefunc` calls;
  - an extracted `GetAuraFrameCount` delegate called with public, `UIParent`, and forbidden receivers;
  - `issecure()` and `issecretvalue()` metadata when those APIs are available.

The capture keeps at most 12 runs and at most 12 returned values per operation. It never recursively serializes arbitrary tables/objects and never stores raw values identified as secret. Object `tostring` results are bounded and secret values are redacted.

## Install

Copy `PrivateScriptObjectProbe` into the retail or PTR AddOns directory and enable it. The TOC targets interface `120100` and declares `PrivateScriptObjectProbeDB`.

## Commands

```text
/psop capture
/psop reset
/psop status
```

A capture also runs after `PLAYER_LOGIN`. Use `/psop capture` for a fresh run after changing the controlled UI state. Use `/psop reset` before a clean session. Run `/psop status` before leaving the client.

Use `/reload` or log out after capture so WoW flushes `PrivateScriptObjectProbeDB` to SavedVariables.

## Artifact

Retain the raw file from:

```text
WTF/Account/<ACCOUNT>/SavedVariables/PrivateScriptObjectProbe.lua
```

Record the client build, whether `Blizzard_AuraContainer` was already loaded, and the exact commands used. The database contains bounded `captures` entries with build metadata and stop reasons.

## What addon code can prove

This is addon-tainted observation only. It can establish what the addon can read, call, compare, hook, or receive from the public and forbidden object values exposed to it. It can record successful calls, errors, Lua types, bounded object strings, and callable secrecy metadata.

## What addon code cannot prove

The probe cannot establish Blizzard-secure caller privileges, secure delegate internals, inaccessible values that addon code cannot obtain, implementation details hidden behind the client, or behavior that requires a missing fixture/template/global. It does not recreate secure mixins or fabricate secure code. When the public loader API, fixture, template, or forbidden-object helper is unavailable, it records the stop reason and exits cleanly.
