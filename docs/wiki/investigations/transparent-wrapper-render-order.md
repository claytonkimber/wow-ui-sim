# Transparent Wrapper Render Order

Same-strata child frames retain frame-level ordering. Only regionless `Frame` / `ScrollFrame` containers are transparent to region collection; wrappers that own visible regions remain real render boundaries.

## Current Contract

`build_strata_buckets()` sorts same-strata child frames by:

1. raw frame level
2. raise order within a frame-level tie
3. frame ID as the stable final key

A frame's textures and font strings render with that owning frame. Draw layer and texture sublevel order regions within the applicable frame/hoisted region group; they do not let a lower-level sibling render above a higher-level sibling.

Regionless `Frame` and `ScrollFrame` children are the exception. Their descendant regions can be hoisted through the otherwise renderless wrapper so grouping-only frames do not create false z-order boundaries. A wrapper with a visible same-strata region is not hoisted because its own region establishes an observable render boundary.

## Quest Log Evidence

Current retail `Blizzard_UIPanels_Game/Mainline/QuestMapFrame.xml` declares `QuestLogBorderFrameTemplate` with `frameLevel="100"`. Its `Border` texture uses draw layer `BORDER`, and its `TopDetail` texture uses `ARTWORK`. The high frame level intentionally places that border frame after lower-level quest content; the atlas supplies edge/filigree art rather than an opaque full-panel fill.

The earlier generic regression modeled the border as an opaque texture covering the entire panel, then required lower-level content to render after it. That expectation contradicted frame-level semantics. The simulator's observed ordering—lower-level texture first, level-100 texture second—was correct.

## Historical Correction

The first investigation described both the content texture and border texture as peer hoisted regions. That stopped being true when transparent-wrapper handling was restricted to regionless wrappers. Keeping wrappers with owned regions intact prevents their regions from escaping the owning frame's z-order.

The corrected regression now asserts that a texture owned by frame level 100 renders after a texture owned by frame level 1, even when the lower-level texture uses `ARTWORK` and the higher-level texture uses `BORDER`.

## Verification

- `higher_level_frame_texture_renders_after_lower_level_content` proves region ownership preserves frame-level ordering.
- `late_set_frame_level_invalidates_cached_strata_buckets` proves a later frame-level change rebuilds that ordering.
- `world_map_tiles_render_after_tiled_background` and `world_quest_pin_icon_renders_after_world_map_tiles` retain concrete world-map coverage for transparent wrappers and pin ordering.
- `test_raise_lower_do_not_cross_raw_frame_levels_for_hit_order` independently proves lower raw frame levels do not cross higher levels during hit ordering.

## Sources

- [state_render_buckets.rs](../../../src/lua_api/state_render_buckets.rs) — strata bucket construction, regionless-wrapper detection, and child-frame sorting
- [render_order.rs](../../../tests/render_order.rs) — render-order regressions
- [frame_level.rs](../../../tests/frame_level.rs) — independent raw frame-level hit-order coverage

## See Also

- [[rendering-pipeline]] — strata buckets feed quad emission
- [[action-bar-spell-icons]] — draw-layer and region-order investigations
