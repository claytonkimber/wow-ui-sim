# PartyFrame portrait composition

The party member class icon is a standalone `Portrait` texture sized `37x37`. In the current retail runtime, it reports no atlas, exposes numeric fileDataID `237669` through `GetTexture()`, and resolves to the authored path `Interface\\TargetingFrame\\UI-Classes-Circles` through `GetTextureFilePath()`. The visible surround is not a second ring widget: it comes from the larger `Texture` frame-art atlas `UI-HUD-UnitFrame-Party-PortraitOn`, which renders at `120x49` and already contains the portrait border artwork.

## Content

## Question

When the live party frame shows a circular class icon with a ring around it, is that ring a separate texture with its own size, or is it baked into the surrounding frame art?

## Result

- `PartyFrame.MemberFrame1.Portrait` is `37x37`
- `PartyFrame.MemberFrame1.PortraitMask` is also `37x37`
- `Portrait:GetAtlas()` is nil
- `Portrait:GetTexture()` is the numeric fileDataID `237669`
- `Portrait:GetTextureFilePath()` resolves to `Interface\\TargetingFrame\\UI-Classes-Circles`
- there is no separate portrait-ring child in the `PartyMemberFrameTemplate`
- the visible surround is part of `PartyFrame.MemberFrame1.Texture`
- that frame-art texture uses atlas `UI-HUD-UnitFrame-Party-PortraitOn`
- in the live master GUI tree, that `Texture` renders at `120x49`

So the portrait ring should be treated as part of the full party-member frame art, not as an independently sized icon border.

## Evidence

### XML template structure

The current retail cache file `~/.cache/wow-ui-sim/blizzard-ui/retail/AddOns/Blizzard_UnitFrame/Mainline/PartyFrameTemplates.xml` defines:

- `Portrait` as a `37x37` texture anchored at `TOPLEFT (7, -6)`
- `PortraitMask` anchored directly to `Portrait`
- `Texture` as a separate child with atlas `UI-HUD-UnitFrame-Party-PortraitOn`
- no dedicated `PortraitRing`, `PortraitBorder`, or equivalent child

That means the template itself already answers the structural question: icon and mask are separate, but the surround is folded into the larger frame-art texture.

### Live master GUI query

Connected `wow-cli dump-tree` against the running master GUI showed the same structure in rendered output:

```text
.portrait [Texture] (37x37) visible LOW:3 x=29, y=153
.PortraitMask [Texture] (37x37) visible LOW:3 MASK x=29, y=153
```

and the adjacent frame-art node:

```text
.Texture [Texture] (120x49) visible LOW:3 x=23, y=149
[atlas] UI-HUD-UnitFrame-Party-PortraitOn
```

The connected master CLI only supports `--filter`, not `--filter-key`, so the portrait lines were extracted from the `PartyFrame` dump via `wow-cli dump-tree --filter Portrait` and `wow-cli dump-tree --filter PartyFrame`.

### Current retail runtime query

The settled current-retail probe in `tests/party_frame_tree.rs` reports this identity for both `PlayerFrame...PlayerPortrait` and `PartyFrame.MemberFrame1.Portrait`:

```text
GetAtlas()          -> nil
GetTexture()        -> 237669
GetTextureFilePath()-> Interface\\TargetingFrame\\UI-Classes-Circles
```

The simulator's runtime source is the profile-scoped cache at `~/.cache/wow-ui-sim/blizzard-ui/retail/AddOns/Blizzard_UnitFrame/Mainline/UnitFrame.lua`. The current source contains no literal `UI-Classes-Circles`; that path is the authored texture resolved from fileDataID `237669`. This is why the path must be asserted through `GetTextureFilePath()`, not `GetTexture()`.

## Implication

If a future bug looks like "the ring is the wrong size" or "the ring is offset from the portrait," the likely fix target is not a missing child widget. The first thing to inspect is the larger `Texture` atlas placement, atlas crop, or the party-frame art texture itself.

## Sources

- Current retail runtime template: `~/.cache/wow-ui-sim/blizzard-ui/retail/AddOns/Blizzard_UnitFrame/Mainline/PartyFrameTemplates.xml` — structure for `Portrait`, `PortraitMask`, and `Texture`
- [party_frame_tree.rs](../../../tests/party_frame_tree.rs) — current retail portrait identity probe
- [atlas.rs](../../../src/lua_api/frame/methods/widgets/texture/atlas.rs) — `GetTexture()` and `GetTextureFilePath()` resolution
- Current retail runtime logic: `~/.cache/wow-ui-sim/blizzard-ui/retail/AddOns/Blizzard_UnitFrame/Mainline/UnitFrame.lua` — portrait update path

## See Also

- [[partyframe-tree]] — master-reference PartyFrame structure and dimensions
- [[partyframe-statusbar-textures]] — adjacent party-frame texture investigation
- [[texture-atlas]] — atlas-backed texture rendering path
