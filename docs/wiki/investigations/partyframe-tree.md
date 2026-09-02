# PartyFrame Tree and Startup-Hang Investigation

PartyFrame now loads, settles, and renders a four-member party at `120x244`. Current retail Edit Mode selection frames use raw frame level 1000; older LOW:3/4/5 dump expectations were stale.

## Current Retail Shape

At 1024×768 with four simulated party members, the relevant tree is:

```text
PartyFrame                         [Frame]  (120x244) visible LOW:1
  .Selection                       [Frame]  (120x244) hidden  LOW:1000 [stored=1x1]
    .MouseOverHighlight            [Frame]  (120x244) hidden  LOW:1001
      .TopLeftCorner               [Texture] (16x16) hidden  LOW:1002
  .Background                      [Frame]  (144x250) hidden  LOW:2
  .MemberFrame1                    [Button] (120x53)  visible LOW:2
  .MemberFrame2                    [Button] (120x53)  visible LOW:2
  .MemberFrame3                    [Button] (120x53)  visible LOW:2
  .MemberFrame4                    [Button] (120x53)  visible LOW:2
```

`Blizzard_EditMode/Shared/EditModeSystemTemplates.xml` declares `EditModeSystemSelectionBaseTemplate` with `frameLevel="1000"`. `MouseOverHighlight` is its child and NineSlice corner textures are one level deeper, producing 1001 and 1002 respectively. The simulator's current dump follows that inheritance. The historical master dump pinned Selection at LOW:3 and no longer represents current retail source.

## Member Region Ordering Contract

`PartyMemberFrameTemplate` declares `Portrait` in `BACKGROUND` and `Flash` and `Name` in `ARTWORK`. Focused coverage verifies those exact `GetDrawLayer()` results at sublevel 0 and verifies each region's semantic parent key, type, size, and visibility in the tree.

Texture and FontString regions do not expose the Frame-only `GetFrameStrata()` or `GetFrameLevel()` client APIs. Their raw strata and level values in simulator dumps are diagnostic widget fields, not the client ordering contract. Effective region ordering follows the owning `MemberFrame1` frame and each region's draw layer.

## Portrait Texture Contract

The current retail runtime does not expose the class-circle portrait as an atlas. After the settled PartyFrame update, both the player and first party portrait report `GetAtlas() == nil`, `GetTexture() == 237669`, and `GetTextureFilePath() == Interface\\TargetingFrame\\UI-Classes-Circles`. The numeric fileDataID is the `GetTexture()` contract; the authored path is recovered through `GetTextureFilePath()`.

The retail source used by the simulator is `~/.cache/wow-ui-sim/blizzard-ui/retail/AddOns/Blizzard_UnitFrame/Mainline/UnitFrame.lua`. Its class-atlas branch calls `SetAtlas()` only when `UnitFrame_ShouldReplacePortrait(self)` is true. The default player and party path uses `SetPortraitTexture`, which leaves the class-circle texture as a fileDataID-backed texture.

## Startup Hang Root Cause

The original investigation found several independent blockers while restoring the tree:

1. Static and content-interned registry keys could diverge, so frame metatable lookup lost the shared metatable during bootstrap. The rilua static-intern fix restored stable identity.
2. Missing party state globals made `PartyFrame:ShouldShow()` false. State-backed group queries and `A_Admin.SetPartySize()` now populate members and dispatch `GROUP_ROSTER_UPDATE`.
3. `SetParentKey` did not synchronize the Lua parent field. Parent-key assignment now updates the frame's Lua-visible child mapping without redundant rewrites.
4. `C_UnitAuras.GetAuraSlots` returned a truthy empty table as continuation token. `AuraUtil.ForEachAura` therefore looped forever while Edit Mode updated TargetFrame auras. Returning nil for the unmodeled continuation ends the iteration.

## Resolved Follow-ups

- Anchored `PartyFrame.Selection:GetWidth()` now agrees with its resolved `120`-pixel registry width while preserving the XML stored size `1x1` in dumps.
- Member frame vertical checks compare absolute stride because Lua geometry uses Y-up coordinates while dump coordinates are top-down. The four members retain the retail 63-pixel stride.
- The dump excludes the orphaned builtin ghost frame and exposes one visible `PartyFrame` owned by `Blizzard_UnitFrame`.
- Runtime lowercase aliases such as `self.portrait` and `self.name` remain in `children_keys` without replacing canonical XML `Portrait` and `Name` parent keys.
- Member children retain semantic names and font size; player and party portraits retain the current numeric class-circle texture identity (`237669`) with no atlas, while `PowerBarAlt` and its `BG`/`BGL`/`BGR` descendants remain present and hidden in the settled tree.

## Verification

Focused coverage in `tests/party_frame_tree.rs` proves:

- frame shape and member offsets
- Selection/background attachment and resolved sizing
- current retail Selection frame levels 1000/1001/1002
- semantic child names and one visible PartyFrame root
- member region `GetDrawLayer()` values and sublevels
- member hover tooltip, current portrait texture identity, and name font

## Sources

- [party_frame_tree.rs](../../../tests/party_frame_tree.rs) — current behavioral and dump-tree coverage
- [group_queries.rs](../../../src/lua_api/globals/group_queries.rs) — state-backed party query surface
- [event system](../../event-system.md) — event dispatch context used by party updates
- Current retail runtime source: `~/.cache/wow-ui-sim/blizzard-ui/retail/AddOns/Blizzard_UnitFrame/Mainline/UnitFrame.lua`

## See Also

- [[partyframe-portrait-composition]] — portrait and frame-art composition
- [[partyframe-statusbar-textures]] — party health/mana texture handling
- [frame data flow](../../frame-data-flow.md) — Rust/Lua frame synchronization
