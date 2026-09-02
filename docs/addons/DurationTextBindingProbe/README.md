# DurationTextBindingProbe

Captures bounded retail observations for `C_DurationUtil.CreateDurationTextBinding` on interface `120100`.

## Purpose

The probe records the live client's public factory and binding surface without assuming that the returned object is a table or userdata. It is intended to compare retail behavior with simulator/API models.

Each run captures:

- build metadata and capture time;
- the documented no-argument factory call;
- exploratory factory calls with `nil`, `false`, and a table;
- bounded `type`, `tostring`, equality, `rawequal`, `getmetatable`, `rawget`, `rawset`, and `pairs` observations;
- candidate method presence;
- identity of two separately created bindings;
- usability of a retained reference after `collectgarbage("collect")`;
- getter state before and after safe setter calls and `SetToDefaults`;
- `Duration`, `ManualClock`, and `FontString` resource observations when those APIs are available.

Arbitrary tables and userdata are never recursively serialized. Values are stored as bounded primitive/type summaries, bounded `tostring` results, or bounded error strings.

## Install

Copy the folder to:

```text
World of Warcraft/_retail_/Interface/AddOns/DurationTextBindingProbe/
```

The folder must contain:

```text
DurationTextBindingProbe.toc
DurationTextBindingProbe.lua
```

Enable the addon in the AddOns list. The target client interface is `120100`.

## Run

The probe runs automatically on `PLAYER_LOGIN`. Run it again manually with:

```text
/dtbprobe
```

The long alias is also available:

```text
/durationtextbindingprobe
```

After each run, `/reload` or log out so WoW flushes SavedVariables. The expected artifact is:

```text
WTF/Account/<ACCOUNT>/SavedVariables/DurationTextBindingProbe.lua
```

The newest observation is stored in:

```lua
DurationTextBindingProbeDB.latest
```

Up to ten recent runs are retained in:

```lua
DurationTextBindingProbeDB.runs
```

## Limitations

- This is an observation probe, not a correctness claim about any implementation.
- Exploratory factory arguments only record what the live client does; they do not establish supported API usage.
- `pairs`, raw access, metatable access, and `tostring` are individually `pcall`-wrapped and bounded. A failure is evidence about that operation only.
- The retained-reference section checks whether a still-referenced object remains callable after an explicit collection request. It does **not** prove garbage-collection, finalization, ownership, invalidation, or native lifetime semantics.
- Getter/setter transitions run only with public methods present and use `Duration`, `ManualClock`, and `FontString` objects created by documented APIs when available.
- No recursive object dump, metatable dump, arbitrary table traversal, or rendering claim is made.
