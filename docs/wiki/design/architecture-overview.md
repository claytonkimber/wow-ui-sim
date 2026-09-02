# Architecture Overview

The wow-ui-sim project runs WoW addon Lua outside the game for testing and visual preview. It embeds Lua 5.1 (WoW's exact version) via **rilua** — a pure-Rust Lua VM with WoW-style taint tracking built in — inside a Rust host that also handles rendering through the iced GUI framework.

## Goals and Non-Goals

**Primary goal**: run WoW addons headlessly for CI testing and development.
**Secondary goal**: visual preview of addon UI through iced.

**Non-goals**:

- Full game emulation (combat, spells, inventory, network, audio).
- 3D model rendering. `Model`, `ModelScene`, `PlayerModel`, and `DressUpModel` use permanent stub method tables (~38 stubs in `widget_model.rs`); the simulator is 2D-UI only.
- Full protected-frame enforcement. Taint **is** tracked; protected `SetAttribute` writes are gated through `protected_write_blocked()` / `can_change_protected_state_for()`. Retail 12.0.5 probes model absent `CreateForbiddenFrame`, absent legacy `Protect`/`SetProtected`, and normal-frame `SetForbidden(true)` as a no-op. Broader protected-frame combat and forbidden-object restrictions remain incomplete.

## Lua + Rust Dual System

WoW frames live in two parallel systems that must stay in sync:

- **Rust side** (`widget::Frame` + `WidgetRegistry`) — owns layout computation, rendering, and the frame tree. Each frame is a `u64`-keyed entry in a `HashMap`.
- **Lua side** (`FrameHandle` userdata + metatables) — exposes the WoW API to addon Lua code. Each `FrameHandle` carries the same `u64` ID pointing at the Rust frame.

Method calls like `:SetText()` use the ID to update Rust state directly. Child assignments (`parent.Child = frame`) are intercepted by `__newindex` to sync `children_keys` on the Rust side, enabling fast HashMap lookups without round-tripping through Lua.

## Module Layout

```
src/
├── lua_api/    — WoW Lua globals, frame methods, environment bootstrap (rilua)
├── c_api/      — C_* namespace surface (peer of lua_api, not a sub-module)
├── widget/     — Frame struct, WidgetRegistry, anchor/layout primitives
├── render/     — wgpu pipeline, shaders, GPU texture atlas, glyph atlas
├── iced_app/   — iced canvas, strata emit, quad builders, masking, dirty rebuild
├── loader/     — TOC parser, XML template loader, addon discovery
├── event/      — Event queue, dispatch, registered handlers
├── xml/        — XML parsing for widgets, scripts, fonts, animations
├── lua_bridge/ — Rilua ↔ widget plumbing (handle ABI, method dispatch glue)
├── texture/    — Texture path resolution, BLP/WebP loading, cache
└── sound/      — Audio file loading (CLI builds disable with --no-default-features)
```

The C API surface is treated as a first-class subsystem at the same architectural level as `lua_api` — it is a compatibility contract exposed to Lua, not a sub-module of it.

### State-backed PTR service models

Patch 12.1 `C_PlayerChoice` is implemented as a deterministic local service model. `SimState.player_choice` owns an optional current choice, documented nested option/reward payloads, query state (`numRerolls`, remaining time, response waiting), and mutator-intent markers for `SendPlayerChoiceResponse`, `RequestRerollPlayerChoice`, and `OnUIClosed`. The model is available only on the PTR retail epoch surface and does not claim retail timing, server validation, reroll economics, or live service values.

## Design Decisions

- **Lua 5.1 via rilua**. Pure-Rust Lua 5.1 VM matching WoW's Lua version, with Elune-style taint tracking native to the interpreter (no C runtime).
- **Taint tracking**. rilua exposes `debug.setobjecttaint` / `debug.getstacktaint` and propagates stack taint through `CallInfo`. The simulator stamps each addon's compiled chunks with its addon name, so insecure variables are correctly attributed. `issecure()` / `issecurevariable()` work; `SetAttribute` and friends do not block tainted callers (known gap).
- **Auto-generated C_* stubs**. Missing `C_*` methods are filled in by a generated stub layer (returns nil/false/0) so addons don't crash on unrecognized namespaces. Hand-written Rust implementations win when present.
- **XML templates** are loaded before user addons. The Blizzard UI tree (`Blizzard_SharedXMLBase`, `Blizzard_SharedXML`, `Blizzard_SharedXMLGame`, `Blizzard_FrameXMLBase`, `Blizzard_FrameXMLUtil`, `Blizzard_FrameXML`, plus per-feature `Blizzard_*` addons) is loaded from the CASC-synced cache at `~/.cache/wow-ui-sim/blizzard-ui`.
- **rilua hot paths**. Frame methods and globals dispatch directly through rilua's stack-level API; do not assume "Lua bridge cost" when handlers are slow — measure the handler body and downstream Rust work.

## See Also

- [[scaling-coordinates]] — coordinate system and canvas sizing
- [[addon-compatibility]] — addon loading scope and Wowless integration
- [[api-coverage]] — C_* stub coverage and missing namespaces
- [[development-phases]] — current active work
- [[../systems/taint-system]] — taint stamping, dispatch, and enforcement gaps
- [[../systems/lua-api]] — globals, C_* namespaces, frame methods
