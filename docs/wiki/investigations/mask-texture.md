# Mask Texture

WoW uses MaskTextures to clip child textures to shapes (rounded squares, circles, etc.) using mask coverage. Depending on the asset, coverage comes from the mask alpha channel or RGB intensity.

## How It Works

1. `MaskTexture` is created with `CreateMaskTexture()` or XML `<MaskTexture>`
2. `<MaskedTextures>` block calls `icon:AddMaskTexture(mask)` on each referenced child
3. During rendering, the masked texture's quads carry `mask_tex_index` and `mask_tex_coords` vertex attributes
4. Primitive preparation resolves the mask path from the RGBA atlas first, then the BC1/BC3 atlas, and remaps UVs into the resolved slot
5. The fragment shader applies the selected mask coverage mode — where effective coverage is zero, the pixel is fully transparent

## Atlas Resolution and UV Computation

Mask paths are deferred texture requests. A resolved RGBA mask uses one of the five RGBA atlas tiers; a compressed mask uses the BC1 or BC3 atlas binding. If neither atlas contains the path, the pending mask index is cleared and no mask is applied. This resolution step is required for CircleMask-style BC assets, which previously rendered unmasked because their pending requests were treated as unresolved.

The mask UV maps the icon's screen position into the mask's screen area. Critical: the mask should be **larger** than the icon it clips, so the icon samples only the opaque center of the mask texture.

For a 64×64 mask centered on a 45×45 icon:
- UV range: `(9.5/64, 54.5/64)` = `(0.148, 0.852)` on both axes
- Icon samples only the center 70% of the mask texture

If mask size is 0×0 (broken), the full mask (0–1 UV) maps to the icon, clipping visible area at transparent borders.

## Key Behavior: `useAtlasSize` Default

MaskTextures default to `useAtlasSize=true` when not specified in XML. Without this, the mask frame is 0×0 and the full mask texture (including transparent borders) maps to the icon, shrinking the visible area.

## `SmallActionButtonMixin` Override

For 30×30 small buttons, `SmallActionButtonMixin_OnLoad` explicitly sets IconMask to 45×45. This overrides the atlas-derived 64×64 size, giving similar proportional UV coverage (0.167–0.833).

## Action Bar Icon Chain

1. Icon (45×45, fills button) → masked by IconMask (64×64 rounded square atlas)
2. SlotBackground (dark fill visible at rounded corners)
3. SlotArt (decorative golden border)
4. NormalTexture (frame border, OVERLAY layer)

## Wrap Modes

`CLAMPTOBLACKADDITIVE` on a MaskTexture means areas outside the atlas are fully transparent — important for the sheen animation's mask.

## Key Files

- `src/iced_app/masking.rs` — mask UV computation
- `src/render/shader/primitive.rs` — deferred RGBA/BC mask resolution and UV remapping
- `src/render/shader/quad.wgsl` — fragment shader mask sampling
- `src/loader/xml_texture.rs` — XML MaskTexture creation

## Sources

- [mask-texture-system.md](../../mask-texture-system.md) — full system description

## See Also

- [[action-bar-spell-icons]] — concrete use of IconMask on action buttons
- [[talent-sheen]] — sheen animation uses MaskTexture for button shape clipping
