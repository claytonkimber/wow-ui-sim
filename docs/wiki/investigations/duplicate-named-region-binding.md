# Duplicate Named Region Binding

Duplicate named sibling regions keep the first global binding while remaining distinct objects. This matches current retail XML that anchors a later same-name texture to the earlier sibling.

## Symptom

`Blizzard_GMChatUI.xml` defines two `GMChatTabBG` textures. The second texture anchors to `$parentBG`. The simulator replaced `_G.GMChatTabBG` as soon as it created the second texture, so the anchor resolved to the second texture itself. `SetPoint` rejected the cycle and XML processing stopped before creating `GMChatStatusFrame`; Behavioral Messaging later failed through `AddGMChatStatusFrameToStatusFrames`.

## Root Cause

Named child regions used unconditional last-writer-wins global registration. That behavior differs from the duplicate sibling pattern required by current Blizzard XML. Frame recreation remains separate: duplicate named frames still receive fresh identity and replace the global binding.

## Fix

`bind_named_child_global` preserves an existing binding only when both objects are Texture/FontString regions with the same parent. Both regions remain attached to the parent, but the first region continues to own the global name. Unrelated globals, frames, and regions under different parents retain existing registration behavior.

## Coverage

- `duplicate_named_textures_keep_first_global_binding` proves distinct texture identity, first-binding ownership, anchoring to the first texture, and two retained parent regions.
- `blizzard_gm_chat_ui_publishes_gm_chat_frame_global` proves both current GMChat XML roots publish.
- Full prefork coverage proves Behavioral Messaging dispatch no longer fails through the GM chat status-frame bootstrap.

## Sources

- [shared.rs](../../../src/lua_api/frame/methods/button_anchor_hierarchy/shared.rs) — named child-region global registration
- [methods_texture.rs](../../../tests/methods_texture.rs) — duplicate-region behavioral coverage
- [blizzard_gm_chat_ui_loads.rs](../../../tests/blizzard_gm_chat_ui_loads.rs) — current GMChat XML publication coverage
- [addon-loading-pipeline.md](../../addon-loading-pipeline.md) — XML/addon loading context

## See Also

- [[global-frame-index]] — named frame and region lookup
- [[xml-template-system]] — XML frame/template creation pipeline
