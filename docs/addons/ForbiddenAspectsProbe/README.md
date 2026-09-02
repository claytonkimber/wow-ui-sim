# ForbiddenAspectsProbe

Captures addon-visible WoW 12.1 behavior for every `Enum.ForbiddenScriptObjectAspect` value on isolated probe frames.

## Purpose

The simulator stores forbidden-aspect masks and models some parent/layout propagation, but exact retail enforcement is unknown. This probe records, through bounded `pcall` observations:

- enum values, direct masks, and parent/layout inheritable masks;
- `UntrustedScriptExecution`: `SetScript`, `GetScript`, `HasScript`, `HookScript`, show/hide dispatch, and `ClearScripts`;
- `UntrustedLayoutScriptExecution`: parent assignment, anchor assignment, size changes, and behavior before/after the destination owns the aspect;
- `EventRegistrations`: event and unit-event registration/unregistration;
- `AlwaysPropagateInput`: keyboard-propagation getters/setters;
- `ScriptedInput`: mouse/keyboard handler registration, hooks, enable calls, and programmatic `Button:Click()`;
- `QueryFocus`: available global and frame focus-query methods.

Fixtures are unnamed, hidden, one-pixel frames under `UIParent`. The probe does not use protected templates, enter combat, manipulate Blizzard-owned UI, synthesize real keyboard/mouse events, or fabricate Blizzard-secure execution.

## Install and run

Copy `ForbiddenAspectsProbe` into the retail or PTR AddOns directory and enable it. The TOC targets interface `120100`.

The probe runs once after `PLAYER_LOGIN`. Commands:

```text
/fasp
/fasp status
/fasp reset
```

The long alias `/forbiddenaspectsprobe` is also available. Run `/reload` or log out after capture so SavedVariables flush.

## Artifact

Retain:

```text
WTF/Account/<ACCOUNT>/SavedVariables/ForbiddenAspectsProbe.lua
```

The database stores build metadata and up to ten bounded runs in `ForbiddenAspectsProbeDB.runs`; the newest is `ForbiddenAspectsProbeDB.latest`.

## Interpretation limits

The probe distinguishes mask storage from registration, invocation, and dispatch rejection. It can establish only addon-tainted behavior.

It cannot prove:

- Blizzard-secure caller exemptions;
- secure layout-script execution;
- actual keyboard/mouse propagation without real input;
- protected focus visibility under a secure caller;
- event-delivery restrictions for events that cannot be safely generated on demand.

Rows remain open until the raw retail/PTR SavedVariables capture is retained and interpreted. Missing methods and unsupported handler names must be distinguished from forbidden-aspect enforcement.
