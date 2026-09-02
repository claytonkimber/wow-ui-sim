# Dropdown Intrinsic Script Chain

Reputation filter dropdowns failed to open because intrinsic `DropdownButton` XML scripts were replaced by style-template scripts. Intrinsic handlers now use the simulator's precall binding, so runtime dispatch preserves the intrinsic handler before the derived style handler. Later shared input work also made `RegisterForMouse` and click propagation real simulator state, so dropdown-like parents do not need per-widget click shims when a child or physical mouse registration is involved.

## Content

### Symptoms

`ReputationFrame.filterDropdown` had correct menu data when the generator was intercepted: radios for All, Warband, the character name, a divider, and the legacy reputations checkbox. The visible UI still did not behave like the real menu path because the dropdown click did not open a Blizzard `Menu.GetManager()` menu.

### Root Cause

`DropdownButton.xml` registers `DropdownButton` as an intrinsic template with `OnMouseDown method="OnMouseDown_Intrinsic"`. `WowStyle1DropdownTemplate` then contributes its own `OnMouseDown method="OnMouseDown"`. The runtime template chain applied the intrinsic template first, then treated the derived style script as a normal replacement, so the intrinsic click handler was lost.

The issue was in the simulator XML/template script application, not in the ReputationFrame generator or popup row rendering.

### Fix

When a template chain applies an intrinsic base, its default scripts are installed in the precall binding. A derived style template's ordinary script remains in the normal binding, so dispatch runs the intrinsic handler first and the style handler second. `GetScript("OnMouseDown")` without a binding argument reads only the normal binding; the regression therefore uses `WowLuaEnv::fire_script_handler` to exercise the real dispatch path.

The fake menu path was retired:

- `MENU_DESCRIPTOR_FALLBACK_LUA` and `ensure_menu_descriptor_fallback` were removed.
- The post-load `Blizzard_Menu` fallback install hook was removed.
- Runtime bootstrap dropdown materialization and style mouse-down patches were removed.
- `tests/menu_fallback.rs` was deleted.

### Coverage

- `intrinsic_dropdown_scripts_dispatch_before_style_template_scripts` asserts a minimal intrinsic `DropdownButton` plus style template dispatches both handlers in order.
- `reputation_filter_dropdown_opens_with_blizzard_menu_renderer` loads Blizzard UI, runs `ReputationFrame`'s real `OnShow`, clicks the real dropdown script, and asserts `Menu.GetManager()` tracks the opened menu.
- `register_for_mouse_restricts_physical_mouse_button_events` asserts `RegisterForMouse("LeftButtonDown", "LeftButtonUp")` suppresses right-button `OnMouseDown`/`OnMouseUp` dispatch.
- `propagated_mouse_clicks_fire_parent_mouse_handlers` asserts a child with `SetPropagateMouseClicks(true)` forwards physical mouse scripts to its parent. This covers dropdown-style parents whose base handler is on the parent while the deepest hit target is a child.

### Follow-up Root Cause

The simulator already stored `propagate_mouse_clicks`, but GUI mouse dispatch did not consume it. `RegisterForMouse` was also a stub, even though Blizzard dropdown intrinsics call it from `DropdownButtonMixin:OnLoad_Intrinsic()` and deprecated `DropDownToggleButtonMixin:OnLoad_Intrinsic()`.

The shared fix stores physical mouse registrations on `Frame`, filters physical `OnMouseDown`/`OnMouseUp` by the registered edge, and walks parent frames while the current frame has `propagate_mouse_clicks` enabled. `RegisterForClicks` remains separate and still controls `OnClick`/`PostClick` edge dispatch.

## Sources

- [DropdownButton.xml](../../../Interface/BlizzardUI/Blizzard_Menu/DropdownButton.xml) — intrinsic template scripts
- [MenuTemplates.xml](../../../Interface/BlizzardUI/Blizzard_Menu/Mainline/MenuTemplates.xml) — style dropdown scripts
- [ReputationFrame.lua](../../../Interface/BlizzardUI/Blizzard_UIPanels_Game/Mainline/ReputationFrame.lua) — menu generator
- [template_chain.rs](../../../src/lua_api/globals/create_frame/template_chain.rs) — runtime template script application
- [helpers.rs](../../../src/loader/helpers.rs) — slow-path XML script chaining and binding selection
- [helpers_anim.rs](../../../src/loader/helpers_anim.rs) — animation-group mixin and XML method-script ordering
- [xml_frame_codegen.rs](../../../src/loader/xml_frame_codegen.rs) — XML KeyValues initializer timing
- [parse.rs](../../../src/xml/parse.rs) — inline `<Scripts>` sibling preservation
- [mouse.rs](../../../src/iced_app/mouse.rs) — GUI mouse dispatch, physical edge checks, and click propagation
- [input.rs](../../../src/lua_api/frame/methods/core_state/input.rs) — `RegisterForMouse` state storage
- [startup_api_stubs.rs](../../../tests/startup_api_stubs.rs) — Reputation dropdown regression test
- [mouse_tests.rs](../../../src/iced_app/mouse_tests.rs) — shared mouse registration and propagation regressions
- [registry.rs](../../../tests/xml_templates/registry.rs) — intrinsic/style chaining regression test

## See Also

- [[xml-template-system]] — template registration, inheritance, XML lifecycle order, and script bindings
- [[frame-data-flow]] — Lua/Rust frame state and script dispatch
