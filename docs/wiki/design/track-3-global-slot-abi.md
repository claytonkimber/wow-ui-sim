# Track 3: Global-slot fast-path ABI (design)

Design for the `GETGLOBAL_SLOT` / `SETGLOBAL_SLOT` opcode ABI. The
compiler replaces whitelisted global-name constants with slot indexes;
the installed VM mode then chooses the read path. Default/no-shadow
reads query the current root `_G` with the slot's pre-interned key,
while optional live-shadow/freeze reads use a frozen snapshot with
shadow overrides taking precedence. This is sub-item 1 of Track 3 —
the design side; sub-items 2-5 are the bootstrap populator, compiler
integration, bytecode cache version bump, and proof/measurement.

## Motivation

Track 3 originally targeted the `Table::get_str` bucket walk that
hot global dereferences pay after Track 2 removed repeated string
interning. The landed ABI separates that optimization from Lua
semantics: the compiler can precompute a slot index for whitelisted
names, but default/no-shadow reads still query the live root table so
reassignment remains visible. The optional live-shadow/freeze mode can
use the bootstrap vector as the direct indexed fallback because writes
are represented through its shadow.

## Scope

**In scope for the first ABI version:**

- `_G` itself (slot 0 — the bootstrap table reference).
- `Enum`, `Constants` namespaces.
- The 95 `C_*` namespace tables in `HOT_NAMESPACES`.
- The 35 hot bare globals in `HOT_GLOBALS` (`UIParent`, `WorldFrame`,
  `Mixin`, `CreateFrame`, `GetTime`, `GetCVar`/`SetCVar`, etc.).

**Out of scope:**

- Method names / field keys (`SetPoint`, `__index`, etc.). Those are
  table-member lookups, not global lookups; they live in
  `HOT_FRAME_METHODS` / `HOT_METATABLE_KEYS` and stay on Track
  1/2's `intern_string_static` fast path.
- Dynamically-named globals (`_G[name]` where `name` is a variable).
  These flow through the normal string-arena lookup; the slot path
  only kicks in when the compiler statically sees a
  whitelisted name.
- Slot *allocation* (hash-consing new slots as code runs). The
  whitelist is frozen at bootstrap — adding entries requires bumping
  [`WHITELIST_VERSION`] and rebuilding the bytecode cache.

## Slot numbering

The slot index of an entry in the slot vector is determined by its
category + position within that category:

    slot(entry) = category_base[entry.category] + entry.index

With `category_base` a compile-time table:

    _G          = 0
    HOT_GLOBALS = 1
    HOT_NAMESPACES = 1 + HOT_GLOBALS.len()

Total slot count = `1 + HOT_GLOBALS.len() + HOT_NAMESPACES.len()` =
1 + 35 + 97 = **133 slots** in ABI v1.

`_G` gets an explicit slot 0 because it is referenced by multiple
Blizzard globals and its table identity is stable. Slot 0 being `_G`
itself also makes it the canonical fallback target for opcodes that hit
"slot not resolved" (`GETGLOBAL_SLOT 0` == `GETGLOBAL _G`). The other
slots are bootstrap snapshots; whether a read uses those snapshots or
current root-table values is determined by the installed VM mode.

## Global-read modes and mutation parity

The rilua change pinned by wow-ui-sim commit `f0f5312be` makes the
default/no-shadow mode Lua-compatible for slot-optimized reads:

- bare assignment (`Name = value`),
- `_G.Name = value`,
- `rawset(_G, "Name", value)`, and
- assignment to `nil`

all update the root table observed by `GETGLOBAL_SLOT`. A direct read
must therefore agree with `_G.Name` after each operation; the bootstrap
slot vector is not an authoritative frozen value in this mode.

The optional live-shadow/freeze mode preserves the original snapshot
contract. Its read order is:

    slot_read(i) =
      let shadow_val = live_shadow.get(slot_name[i])
      if shadow_val is an override {
        return shadow_val                 // shadow wins
      }
      return frozen_slot_vec[i]           // snapshot fallback

Root `_G` mutations that are not represented in the shadow do not
replace the frozen fallback in this mode. This mode distinction prevents
an optimization-only snapshot from changing ordinary Lua global
reassignment semantics while retaining the freeze-gate design where it
is explicitly installed.

`SETGLOBAL_SLOT` must follow the same installed-mode contract. In the
shadow/freeze mode, writes target the mutable shadow rather than the
frozen bootstrap table; the default/no-shadow mode must retain root-table
write semantics.

## ABI version

`hot_literals::WHITELIST_VERSION` is the canonical slot-ABI version.
Its current value of `1` is the first stable version. Bump rules:

- **Adding entries at the end** of any category slice bumps the
  total slot count but preserves every existing slot's index. Soft
  bump: increment `WHITELIST_VERSION`, old bytecode cache is
  forward-compatible (older indices still point at the same slots)
  but new indices need a recompile. Cache entries keyed on the old
  version can be loaded and extended with the new slots populated
  at the old slot-count boundary.

- **Reordering or removing entries** shifts indices. Hard bump:
  increment `WHITELIST_VERSION` by a larger step (e.g. +10) to make
  the break explicit, and invalidate every cached bytecode hash
  that carries the old version. Avoid if at all possible — it
  triggers a full recompile pass on next startup.

Sub-item 4 (cache version bump) wires `WHITELIST_VERSION` into the
bytecode cache key format so stale slot indexes can't silently
interpret against a new whitelist.

## Interaction with the rilua VM

Track 3 uses two rilua-side mechanisms:

1. **Bytecode ops** `GETGLOBAL_SLOT` and `SETGLOBAL_SLOT`. Argument:
   u16 slot index (133 slots fit easily).

2. **Per-VM slot metadata and bootstrap values.** Wow-ui-sim walks
   `HOT_GLOBALS` + `HOT_NAMESPACES` at bootstrap and passes the resolved
   vector to rilua. The vector remains available for the optional
   live-shadow/freeze mode; default/no-shadow `GETGLOBAL_SLOT` reads must
   resolve the current root `_G` value instead of returning a stale
   bootstrap snapshot.

Wow-ui-sim's job (sub-item 2) is to populate the slot metadata and
bootstrap vector; rilua's installed mode determines whether reads use
that vector or current root-table values.

## Compiler fast-path emission

During `patch_string_constants` (rilua's load-time proto rewrite pass),
whenever the compiler sees a `GETGLOBAL <str>` op whose constant
matches a whitelisted name, it rewrites to `GETGLOBAL_SLOT <idx>`.
The name lookup needs the `&[u8]` → `slot_idx` map built once per VM
(keyed on `WHITELIST_VERSION`).

Non-whitelisted `GETGLOBAL` ops stay untouched — they go through the
normal `Table::get_str` path. This keeps the optimization opt-in per
name, so addon-authored globals (`MyAddonFrame`, `MyAddonConfig`)
never hit the slot fast path (which is correct: those aren't on the
whitelist and the compiler has no way to know their slot).

## Bytecode cache key

Current cache key (in `loader::chunk_cache::tagged_hash`):

    hash(source_bytes) + hash(tag)

New key (sub-item 4):

    hash(source_bytes) + hash(tag) + hash(WHITELIST_VERSION)

This makes every cached bytecode invalidate when the version bumps,
so the slot indexes baked into the cached ops can never point at a
stale whitelist.

## Measurement plan (sub-item 5)

Add a `wow-cli startup-global-slot-stats` command (gated on the
existing `intern-stats` feature or a new one) that reports:

- Slot-lookup count (how often `GETGLOBAL_SLOT` fires).
- Fallback-lookup count (how often a whitelisted name was accessed
  via `GETGLOBAL` because the cache was stale — should be 0 after
  steady state).
- `_G_live`-override count (how often a slot read's shadow check
  fired).

Parity tests must cover a whitelisted global through bare assignment,
`_G.Name = value`, `rawset(_G, "Name", value)`, and `Name = nil`, asserting
that `_G.Name` and precompiled direct-name reads agree in the
default/no-shadow mode. A separate live-shadow/freeze probe must assert
that a shadow override wins while the frozen snapshot remains the
fallback.

Startup wall-time comparison: release build, `--no-addons
--no-saved-vars`, before vs after the slot ABI lands. Rilua's
`intern-stats` feature is not on the critical path here (the slot
path bypasses `intern_string` entirely for whitelisted names), so
the comparison is plain wall-time.

## Implementation status and remaining proof

The compiler rewrite and slot metadata path are implemented. Commit
`f0f5312be` updates the wow-ui-sim rilua pin so default/no-shadow reads
retain root-table reassignment semantics. The pinned rilua revision
covers bare, `_G`, `rawset`, nil, custom-environment, taint-propagation,
shadow-override, and empty-shadow snapshot behavior. Remaining Track 3
work is startup measurement and any end-to-end freeze-mode proof beyond
the VM contract.

## Sources

- [Cargo.lock](../../../Cargo.lock) — commit `f0f5312be` pins the rilua revision containing the slot-read coherence fix
- [global_slots.rs](../../../src/lua_api/global_slots.rs) — simulator slot metadata, mode selection, and shadow-aware read contract
- [Track 2 intern audit](../investigations/track-2-intern-audit.md) — preceding hot-literal and intern-path work

## See Also

- [[bytecode-cache-growth]] — bounded cache storage for generated chunks using the slot ABI
- [[architecture-overview]] — rilua and simulator architecture context
