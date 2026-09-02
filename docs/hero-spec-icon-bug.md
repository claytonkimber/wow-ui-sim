# Hero Spec Icon Position Bug

Status: retired as stale evidence in the current build.

## Original Report

The original report claimed the hero talent spec icon (large circular paladin fire icon with ring
border) rendered at the **bottom-right** of the talent panel instead of **top-center** where it
belongs.

## Frame Hierarchy

```
PlayerSpellsFrame (scale=0.88)
  └─ ClassTalentsFrame (TalentsFrame)
      ├─ ButtonsParent (__tpl_25748, 1418x681, clipChildren=true)
      │   └─ [talent node buttons positioned via ApplyPosition]
      └─ HeroTalentsContainer (__tpl_25774, 200x800)
          │  anchor: TOP -> ButtonsParent:TOP offset(0,0)
          ├─ HeroSpecLabel (.HeroSpecLabel) "TEMPLAR"
          │    anchor: BOTTOM -> parent:TOP offset(0,23)
          ├─ HeroSpecButton (__tpl_25780, 108x108)
          │    anchor: TOP -> parent:TOP offset(0,-102)
          │    ├─ Icon1 (talentsheroclassicons) setAllPoints
          │    ├─ IconMask (common-mask-circle) setAllPoints
          │    └─ Border (talents-heroclass-ring-mainpane, 192x192)
          │         anchor: CENTER -> parent:CENTER offset(0,-2)
          └─ CurrencyFrame (30x30) "PV" / "0"
               anchor: CENTER -> HeroSpecButton:BOTTOM offset(0,-3)
```

Key: HeroTalentsContainer is a **sibling** of ButtonsParent (not a child), anchored to its TOP.

## Lua Positioning

`ClassTalentsFrameMixin:UpdateSpecBackground()` (Blizzard_ClassTalentsFrame.lua:204):
```lua
local heroContainerOffset = specVisuals and specVisuals.heroContainerOffset or 0;
self.HeroTalentsContainer:SetPoint("TOP", self.ButtonsParent, heroContainerOffset, 0);
```

4-arg SetPoint: `(point, relativeTo, offsetX, offsetY)` — relativePoint defaults to "TOP".

## Investigation Findings

### dump-tree shows correct position

```
__tpl_25774 [Frame] (176x704) x=711, y=61   anchor: TOP -> __tpl_25748:TOP -> (800,61)
  __tpl_25780 [Button] (95x95) x=752, y=151  anchor: TOP -> parent:TOP -> (800,163)
    .Icon1 [Texture] (95x95) x=752, y=151
    .Border [Texture] (168x168) x=715, y=115
```

### historical screenshot annotation marked a bottom-right point (~x=1000, y=610)

### No stale layout_rect

The `layout_rect` cached on every frame in this subtree **matches** the freshly computed rect (no `[layout_rect=...]` stale annotations in dump). This holds true even when dumping AFTER `ensure_layout_rects()` runs in the screenshot path.

### Identical loading sequence

Both dump-tree and screenshot follow the same startup:
1. `fire_startup_events` (OnShow -> UpdateSpecBackground -> SetPoint)
2. `apply_post_event_workarounds`
3. `rebuild_anchor_index`
4. `process_pending_timers`
5. `fire_one_on_update_tick`
6. `sleep(2s)` + `run_extra_update_ticks(3)`

### Rendering pipeline

`collect_sorted_frames` (frame_collect.rs:102) reads `f.layout_rect` directly — confirmed correct.
`emit_all_frames` (render.rs:190-192) converts to screen bounds with `UI_SCALE=1.0` — no transform.

### Quad emission matches layout rect

`tests/hero_talents_render.rs::hero_spec_icon_and_mask_quads_match_layout_rect` now verifies the
actual `QuadBatch` output for the active hero spec button:

- `HeroSpecButton.Icon1` emits exactly one textured quad request
- that request's vertex bounds exactly match `Icon1`'s computed layout rect
- `HeroSpecButton.IconMask` emits exactly one mask request
- that mask request's vertex bounds also exactly match the same layout rect

Observed values in the full UI harness:

```
Icon1 layout rect: x=481.58 y=111.41 w=60.83 h=60.83
Icon1 textured quad bounds: (481.58, 111.41) -> (542.42, 172.24)
IconMask quad bounds:       (481.58, 111.41) -> (542.42, 172.24)
```

That rules out divergence in:

- anchor/layout resolution
- visible-frame collection
- texture quad emission
- mask quad clipping setup

### Atlas crop request matches atlas metadata

`tests/hero_talents_render.rs::hero_spec_icon_crop_request_matches_atlas_entry` also verifies that
the emitted cropped texture request for `HeroSpecButton.Icon1` matches the atlas database entry for
`talents-heroclass-paladin-lightsmith` exactly.

Current emitted request:

```
Interface\talentframe\talentsheroclassicons@crop:0.395020,0.492676,0.790039,0.985352
```

Atlas DB entry:

```
talents-heroclass-paladin-lightsmith
  file: Interface\talentframe\talentsheroclassicons
  left/right/top/bottom: 0.395020 / 0.492676 / 0.790039 / 0.985352
```

That rules out divergence in:

- `SetAtlas()` atlas lookup for `Icon1`
- `atlas_tex_coords` storage on the frame
- `remap_atlas_crop()` crop-key generation

### Loaded crop content and GPU sampling still match

`tests/hero_talents_render.rs::hero_spec_icon_full_ui_render_matches_isolated_crop_render`
pushes the investigation one step further down the pipeline:

- it loads the exact emitted crop request for `HeroSpecButton.Icon1` via `load_texture_or_crop()`
- it verifies several stable interior sample points in that crop stay substantially opaque
- it renders the exact same crop request in isolation through `render_to_image()`
- it renders the full class-talent UI through that same headless GPU path
- it compares several stable interior sample points inside the circular icon (`top-center`,
  `center-left`, `center`, `center-right`, `bottom-center`)

The full UI render matches the isolated crop render at those interior points, while the loaded crop
itself is also non-empty and opaque there. That rules out divergence in:

- the extracted `talentsheroclassicons` sub-region content for the active Lightsmith icon
- CPU-side crop extraction via `TextureManager::load_sub_region()`
- downstream GPU texture upload / sampling for the actual `Icon1` quad at those interior points

### Hiding the entire hero subtree only changes the top-center region

`tests/hero_talents_render.rs::hiding_hero_talents_container_only_changes_top_center_region`
renders the full class-talent UI twice at the screenshot size (`1600x1200`):

- once normally
- once after hiding `HeroTalentsContainer`

It then computes the pixel-difference bounds between those two renders. The diff stays confined to
the expanded hero-panel region near the top center, and the old historical artifact point
`(1000, 610)` does not change at all.

That rules out a whole class of false leads:

- the bottom-right visual is not emitted by `HeroTalentsContainer`
- the bottom-right visual is not emitted by `HeroSpecButton.Icon1`
- the bottom-right visual is not emitted by another child of the hero-spec subtree

### The historical bottom-right point is just marble background in the current render

The current batch inspection
checks the exact old screenshot coordinate from this investigation, `(1000, 610)`, against the
current `1600x1200` screenshot-path batch. In the current build, that point intersects exactly one
textured request:

```
framegeneral/ui-background-marble
```

So the old bottom-right point is not only outside the hero subtree, it is not intersecting any hero
icon-like request at all in the current render. That means the original screenshot annotation is
stale or misidentified rather than evidence that `HeroSpecButton.Icon1` is still rendering there.

## What's Ruled Out

- **Stale layout_rect**: Confirmed correct after ensure_layout_rects
- **SetPoint parsing**: 4-arg form correctly parsed (anchor shows TOP->TOP offset 0,0)
- **UI_SCALE**: Is 1.0, no transform
- **Duplicate frames**: Only 3 instances of talentsheroclassicons texture, all within HeroSpecButton
- **Pan offset**: Pan system moves individual talent buttons, not ButtonsParent position
- **clipChildren**: ButtonsParent clips, but HeroTalentsContainer is a sibling not a child

## Conclusion

The current build does not reproduce a hero-spec icon positioning bug:

- the hero icon renders at the expected top-center location in the current screenshot output
- the old bottom-right point intersects only marble background in the current screenshot-path batch
- hiding `HeroTalentsContainer` does not affect that old bottom-right area at all

So this is no longer an active hero-icon bug. The evidence now points to the original report being
stale or misidentified.

Decision: retire the hero-spec bottom-right report in this codebase unless someone produces a fresh
current-build repro that actually places `HeroSpecButton.Icon1` away from the top-center hero slot.

There may still be a separate bottom-right visual oddity elsewhere in the class-talent UI, but that
is not a `HeroTalentsContainer` / `HeroSpecButton.Icon1` issue and should be tracked separately.
That follow-up now lives in
[`docs/class-talents-right-side-artifact.md`](/syncthing/Sync/Projects/wow/wow-ui-sim/docs/class-talents-right-side-artifact.md).

## Debug Tools Added

`--dump-tree` flag on `screenshot` subcommand:
```bash
wow-sim screenshot --dump-tree __tpl_25774   # dump subtree after ensure_layout_rects
wow-sim screenshot --dump-tree               # dump all (no filter)
```

## Next Steps

- If the bottom-right class-talent oddity still matters, open a separate non-hero visual bug for it
- Otherwise, leave this investigation closed
