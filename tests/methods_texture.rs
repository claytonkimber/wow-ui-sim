//! Tests for texture-related Lua methods (methods_texture.rs).
//!
//! Covers: SetTexture, GetTexture, SetTexCoord, SetVertexColor, GetVertexColor,
//! SetColorTexture, SetAtlas, GetAtlas, SetBlendMode, GetBlendMode,
//! SetHorizTile, GetHorizTile, SetVertTile, GetVertTile, SetDrawLayer, GetDrawLayer,
//! SetDesaturated, IsDesaturated, mask textures, pixel grid, texel snapping,
//! and nine-slice stub methods.

#[path = "methods_texture/masks_and_misc.rs"]
mod masks_and_misc;
#[path = "methods_texture/nine_slice.rs"]
mod nine_slice;

use wow_ui_sim::lua_api::WowLuaEnv;

fn env() -> WowLuaEnv {
    WowLuaEnv::new().expect("Failed to create Lua environment")
}

/// Create a frame with a child texture. Returns `(frame_global, texture_global)` names.
fn setup_texture(env: &WowLuaEnv, prefix: &str) -> (String, String) {
    let frame_name = format!("{}Frame", prefix);
    let tex_name = format!("{}Tex", prefix);
    env.exec(&format!(
        r#"
        local f = CreateFrame("Frame", "{frame_name}", UIParent)
        f:CreateTexture("{tex_name}", "BACKGROUND")
    "#
    ))
    .unwrap();
    (frame_name, tex_name)
}

fn assert_coords_close(actual: [f64; 8], expected: [f64; 8]) {
    for (actual, expected) in actual.into_iter().zip(expected) {
        assert!(
            (actual - expected).abs() < 0.0001,
            "expected {expected}, got {actual}"
        );
    }
}

#[test]
fn duplicate_named_textures_keep_first_global_binding() {
    let env = env();
    let result: (bool, bool, bool, i64) = env
        .eval(
            r#"
            local parent = CreateFrame("Frame", "DuplicateTextureParent", UIParent)
            local first = parent:CreateTexture("DuplicateTextureRegion", "BACKGROUND")
            local second = parent:CreateTexture("DuplicateTextureRegion", "ARTWORK")
            second:SetPoint("LEFT", DuplicateTextureRegion, "RIGHT")
            local _, relativeTo = second:GetPoint(1)
            return first ~= second,
                   DuplicateTextureRegion == first,
                   relativeTo == first,
                   parent:GetNumRegions()
            "#,
        )
        .expect("duplicate named texture probe should succeed");

    assert_eq!(result, (true, true, true, 2));
}

// ============================================================================
// SetTexture / GetTexture
// ============================================================================

#[test]
fn test_set_texture_known_path_returns_fdid_and_no_args_clear() {
    let env = env();
    let (_, tex) = setup_texture(&env, "TexPath");
    let (file_data_id, cleared): (i64, bool) = env
        .eval(&format!(
            r#"
            {tex}:SetTexture("Interface\\Buttons\\UI-Panel-Button-Up")
            local fileDataID = {tex}:GetTexture()
            {tex}:SetTexture()
            return fileDataID, {tex}:GetTexture() == nil
            "#
        ))
        .unwrap();
    assert_eq!(file_data_id, 130828);
    assert!(cleared);
}

#[test]
fn test_set_texture_applies_tiling_flags() {
    let env = env();
    let (_, tex) = setup_texture(&env, "TexSetTextureTiles");
    env.exec(&format!(
        r#"{tex}:SetTexture("Interface\\DialogFrame\\UI-DialogBox-Border", true, false)"#
    ))
    .unwrap();

    let (horiz_tile, vert_tile): (bool, bool) = env
        .eval(&format!("return {tex}:GetHorizTile(), {tex}:GetVertTile()"))
        .unwrap();

    assert!(
        horiz_tile,
        "SetTexture(path, true, false) should enable horizontal tiling"
    );
    assert!(
        !vert_tile,
        "SetTexture(path, true, false) should leave vertical tiling disabled"
    );
}

#[test]
fn test_set_texture_nil_clears() {
    let env = env();
    let (_, tex) = setup_texture(&env, "TexNil");
    env.exec(&format!(
        r#"{tex}:SetTexture("Interface\\Buttons\\UI-Panel-Button-Up"); {tex}:SetTexture(nil)"#
    ))
    .unwrap();
    let is_nil: bool = env
        .eval(&format!("return {tex}:GetTexture() == nil"))
        .unwrap();
    assert!(is_nil, "SetTexture(nil) should clear the texture path");
}

#[test]
fn test_get_texture_default_nil() {
    let env = env();
    let (_, tex) = setup_texture(&env, "TexDef");
    let is_nil: bool = env
        .eval(&format!("return {tex}:GetTexture() == nil"))
        .unwrap();
    assert!(is_nil, "Texture path should be nil by default");
}

#[test]
fn test_set_texture_file_data_id_resolves_path_and_round_trips() {
    let env = env();
    let (_, tex) = setup_texture(&env, "TexFileId");
    env.exec(&format!(r#"{tex}:SetTexture(136243)"#)).unwrap();

    let texture_value: i64 = env.eval(&format!("return {tex}:GetTexture()")).unwrap();
    assert_eq!(texture_value, 136243);

    let state = env.state().borrow();
    let id = state.widgets.get_id_by_name(&tex).unwrap();
    let widget = state.widgets.get(id).unwrap();
    let expected_path = format!(
        "Interface\\{}",
        wow_ui_sim::manifest_interface_data::get_texture_path(136243)
            .expect("test fileDataID should exist in the texture manifest")
            .replace('/', "\\")
    );
    assert_eq!(
        widget.texture_file_data_id,
        Some(136243),
        "numeric SetTexture should preserve the fileDataID for GetTexture() round-trips"
    );
    assert_eq!(
        widget.texture.as_deref(),
        Some(expected_path.as_str()),
        "numeric SetTexture should also resolve and store the texture path immediately"
    );
}

#[test]
fn test_set_texture_after_atlas_resets_atlas_tex_coords() {
    let env = env();
    let (_, tex) = setup_texture(&env, "TexAfterAtlas");
    env.exec(&format!(
        r#"
        {tex}:SetAtlas("checkbox-minimal")
        {tex}:SetTexture(134583)
        "#
    ))
    .unwrap();

    let coords: (i64, f64, f64, f64, f64, f64, f64, f64, f64) = env
        .eval(&format!(
            r##"
            return select("#", {tex}:GetTexCoord()), {tex}:GetTexCoord()
        "##
        ))
        .unwrap();
    assert_eq!(coords.0, 8);
    assert_coords_close(
        [
            coords.1, coords.2, coords.3, coords.4, coords.5, coords.6, coords.7, coords.8,
        ],
        [0.0, 0.0, 0.0, 1.0, 1.0, 0.0, 1.0, 1.0],
    );
}

#[test]
fn test_set_texture_after_atlas_preserves_custom_tex_coords() {
    let env = env();
    let (_, tex) = setup_texture(&env, "TexAfterAtlasCustomCoords");
    let coords_preserved: bool = env
        .eval(&format!(
            r#"
        {tex}:SetAtlas("checkbox-minimal")
        {tex}:SetTexCoord(0.25, 0.75, 0.125, 0.875)
        local beforeLeft, beforeRight, beforeTop, beforeBottom = {tex}:GetTexCoord()
        {tex}:SetTexture(134583)
        local afterLeft, afterRight, afterTop, afterBottom = {tex}:GetTexCoord()
        return beforeLeft == afterLeft
            and beforeRight == afterRight
            and beforeTop == afterTop
            and beforeBottom == afterBottom
        "#
        ))
        .unwrap();
    assert!(
        coords_preserved,
        "SetTexture should preserve caller-provided tex coords"
    );
}

// ============================================================================
// SetVertexColor / GetVertexColor
// ============================================================================

#[test]
fn test_set_get_vertex_color() {
    let env = env();
    let (_, tex) = setup_texture(&env, "VC");
    let (r, g, b, a): (f64, f64, f64, f64) = env
        .eval(&format!(
            "{tex}:SetVertexColor(0.5, 0.6, 0.7, 0.8); return {tex}:GetVertexColor()"
        ))
        .unwrap();
    assert!((r - 0.5).abs() < 0.001);
    assert!((g - 0.6).abs() < 0.001);
    assert!((b - 0.7).abs() < 0.001);
    assert!((a - 0.8).abs() < 0.001);
}

#[test]
fn test_vertex_color_default_alpha() {
    let env = env();
    let (_, tex) = setup_texture(&env, "VCDef");
    let (r, g, b, a): (f64, f64, f64, f64) = env
        .eval(&format!(
            "{tex}:SetVertexColor(0.1, 0.2, 0.3); return {tex}:GetVertexColor()"
        ))
        .unwrap();
    assert!((r - 0.1).abs() < 0.001);
    assert!((g - 0.2).abs() < 0.001);
    assert!((b - 0.3).abs() < 0.001);
    assert!((a - 1.0).abs() < 0.001, "Alpha should default to 1.0");
}

#[test]
fn test_set_vertex_color_accepts_create_color_object() {
    let env = env();
    let (_, tex) = setup_texture(&env, "VCColorObject");
    let (r, g, b, a): (f64, f64, f64, f64) = env
        .eval(&format!(
            "{tex}:SetVertexColor(CreateColor(0.9, 0.8, 0.7, 0.6)); return {tex}:GetVertexColor()"
        ))
        .unwrap();
    assert!((r - 0.9).abs() < 0.001);
    assert!((g - 0.8).abs() < 0.001);
    assert!((b - 0.7).abs() < 0.001);
    assert!((a - 0.6).abs() < 0.001);
}

#[test]
fn test_set_vertex_color_object_alpha_argument_overrides_table_alpha() {
    let env = env();
    let (_, tex) = setup_texture(&env, "VCColorObjectAlpha");
    let (r, g, b, a): (f64, f64, f64, f64) = env
        .eval(&format!(
            r#"
            local color = CreateColor(0.25, 0.5, 0.75, 0.9)
            {tex}:SetVertexColor(color, 0.4)
            return {tex}:GetVertexColor()
            "#
        ))
        .unwrap();
    assert!((r - 0.25).abs() < 0.001);
    assert!((g - 0.5).abs() < 0.001);
    assert!((b - 0.75).abs() < 0.001);
    assert!((a - 0.4).abs() < 0.001);
}

#[test]
fn test_vertex_color_default_white() {
    let env = env();
    let (_, tex) = setup_texture(&env, "VCWhite");
    let (r, g, b, a): (f64, f64, f64, f64) =
        env.eval(&format!("return {tex}:GetVertexColor()")).unwrap();
    assert_eq!((r, g, b, a), (1.0, 1.0, 1.0, 1.0));
}

// ============================================================================
// SetColorTexture
// ============================================================================

#[test]
fn test_set_color_texture() {
    let env = env();
    let (_, tex) = setup_texture(&env, "CT");
    env.exec(&format!(
        r#"{tex}:SetTexture("Interface\\Buttons\\something"); {tex}:SetColorTexture(1, 0, 0, 0.5)"#
    ))
    .unwrap();

    let is_nil: bool = env
        .eval(&format!("return {tex}:GetTexture() == nil"))
        .unwrap();
    assert!(is_nil, "GetTexture should return nil after SetColorTexture");

    let state = env.state().borrow();
    let id = state.widgets.get_id_by_name(&tex).unwrap();
    let color = state.widgets.get(id).unwrap().color_texture.unwrap();
    assert!((color.r - 1.0).abs() < 0.001);
    assert!((color.g - 0.0).abs() < 0.001);
    assert!((color.b - 0.0).abs() < 0.001);
    assert!((color.a - 0.5).abs() < 0.001);
}

#[test]
fn test_set_color_texture_default_alpha() {
    let env = env();
    let (_, tex) = setup_texture(&env, "CTDef");
    env.exec(&format!("{tex}:SetColorTexture(0.2, 0.3, 0.4)"))
        .unwrap();

    let state = env.state().borrow();
    let id = state.widgets.get_id_by_name(&tex).unwrap();
    let color = state.widgets.get(id).unwrap().color_texture.unwrap();
    assert!((color.a - 1.0).abs() < 0.001, "Alpha should default to 1.0");
}

// ============================================================================
// SetTexCoord / atlas-relative tex coords
// ============================================================================

#[test]
fn test_set_tex_coord_basic() {
    let env = env();
    env.exec(
        r#"
        local frame = CreateFrame("Frame", "TCFrame", UIParent)
        local tex = frame:CreateTexture("TCTex", "BACKGROUND")
        tex:SetTexCoord(0.1, 0.9, 0.2, 0.8)
    "#,
    )
    .unwrap();

    let state = env.state().borrow();
    let id = state.widgets.get_id_by_name("TCTex").unwrap();
    let widget = state.widgets.get(id).unwrap();
    let coords = widget.tex_coords.unwrap();
    assert!((coords.0 - 0.1).abs() < 0.001);
    assert!((coords.1 - 0.9).abs() < 0.001);
    assert!((coords.2 - 0.2).abs() < 0.001);
    assert!((coords.3 - 0.8).abs() < 0.001);
}

#[test]
fn test_get_tex_coord_returns_wow_corner_form_for_rect_coords() {
    let env = env();
    let (_, tex) = setup_texture(&env, "GetTexCoordCorners");

    let default: (i64, f64, f64, f64, f64, f64, f64, f64, f64) = env
        .eval(&format!(
            r##"
            return select("#", {tex}:GetTexCoord()), {tex}:GetTexCoord()
        "##
        ))
        .unwrap();

    assert_eq!(default.0, 8);
    assert_coords_close(
        [
            default.1, default.2, default.3, default.4, default.5, default.6, default.7,
            default.8,
        ],
        [0.0, 0.0, 0.0, 1.0, 1.0, 0.0, 1.0, 1.0],
    );

    env.exec(&format!("{tex}:SetTexCoord(0.1, 0.9, 0.2, 0.8)"))
        .unwrap();

    let rect: (i64, f64, f64, f64, f64, f64, f64, f64, f64) = env
        .eval(&format!(
            r##"
            return select("#", {tex}:GetTexCoord()), {tex}:GetTexCoord()
        "##
        ))
        .unwrap();

    assert_eq!(rect.0, 8);
    assert_coords_close(
        [
            rect.1, rect.2, rect.3, rect.4, rect.5, rect.6, rect.7, rect.8,
        ],
        [0.1, 0.2, 0.1, 0.8, 0.9, 0.2, 0.9, 0.8],
    );
}

#[test]
fn test_get_tex_coord_round_trips_through_set_tex_coord() {
    let env = env();
    let (_, source) = setup_texture(&env, "TexCoordRoundTripSource");
    let (_, target) = setup_texture(&env, "TexCoordRoundTripTarget");

    let count: i64 = env
        .eval(&format!(
            r##"
            {source}:SetTexCoord(0.1, 0.9, 0.2, 0.8)
            {target}:SetTexCoord({source}:GetTexCoord())
            return select("#", {target}:GetTexCoord())
            "##
        ))
        .unwrap();

    assert_eq!(count, 8);

    let coords: (f64, f64, f64, f64, f64, f64, f64, f64) = env
        .eval(&format!("return {target}:GetTexCoord()"))
        .unwrap();
    assert_coords_close(
        [
            coords.0, coords.1, coords.2, coords.3, coords.4, coords.5, coords.6, coords.7,
        ],
        [0.1, 0.2, 0.1, 0.8, 0.9, 0.2, 0.9, 0.8],
    );
}

#[test]
fn test_set_tex_coord_with_atlas_remaps() {
    let env = env();
    // Manually set up atlas_tex_coords via Rust, then call SetTexCoord
    // The atlas sub-region is (0.25, 0.75, 0.1, 0.9)
    // SetTexCoord(0, 1, 0, 1) should produce the atlas coords themselves
    env.exec(
        r#"
        local frame = CreateFrame("Frame", "TCAtlasFrame", UIParent)
        local tex = frame:CreateTexture("TCAtlasTex", "BACKGROUND")
    "#,
    )
    .unwrap();

    // Set atlas_tex_coords directly in Rust state
    {
        let mut state = env.state().borrow_mut();
        let id = state.widgets.get_id_by_name("TCAtlasTex").unwrap();
        let widget = state.widgets.get_mut(id).unwrap();
        widget.atlas_tex_coords = Some((0.25, 0.75, 0.1, 0.9));
    }

    // Now call SetTexCoord(0, 1, 0, 1) - should remap to atlas sub-region
    env.exec("TCAtlasTex:SetTexCoord(0, 1, 0, 1)").unwrap();

    let state = env.state().borrow();
    let id = state.widgets.get_id_by_name("TCAtlasTex").unwrap();
    let widget = state.widgets.get(id).unwrap();
    let coords = widget.tex_coords.unwrap();
    // 0.25 + 0 * 0.5 = 0.25, 0.25 + 1 * 0.5 = 0.75
    // 0.1 + 0 * 0.8 = 0.1, 0.1 + 1 * 0.8 = 0.9
    assert!((coords.0 - 0.25).abs() < 0.001);
    assert!((coords.1 - 0.75).abs() < 0.001);
    assert!((coords.2 - 0.1).abs() < 0.001);
    assert!((coords.3 - 0.9).abs() < 0.001);
}

#[test]
fn test_set_tex_coord_with_atlas_partial() {
    let env = env();
    env.exec(
        r#"
        local frame = CreateFrame("Frame", "TCPartialFrame", UIParent)
        local tex = frame:CreateTexture("TCPartialTex", "BACKGROUND")
    "#,
    )
    .unwrap();

    // Atlas region: left=0.0, right=1.0, top=0.0, bottom=1.0
    {
        let mut state = env.state().borrow_mut();
        let id = state.widgets.get_id_by_name("TCPartialTex").unwrap();
        let widget = state.widgets.get_mut(id).unwrap();
        widget.atlas_tex_coords = Some((0.0, 1.0, 0.0, 1.0));
    }

    // SetTexCoord(0.5, 1.0, 0.5, 1.0) - should produce (0.5, 1.0, 0.5, 1.0)
    env.exec("TCPartialTex:SetTexCoord(0.5, 1.0, 0.5, 1.0)")
        .unwrap();

    let state = env.state().borrow();
    let id = state.widgets.get_id_by_name("TCPartialTex").unwrap();
    let widget = state.widgets.get(id).unwrap();
    let coords = widget.tex_coords.unwrap();
    assert!((coords.0 - 0.5).abs() < 0.001);
    assert!((coords.1 - 1.0).abs() < 0.001);
    assert!((coords.2 - 0.5).abs() < 0.001);
    assert!((coords.3 - 1.0).abs() < 0.001);
}

// ============================================================================
// SetHorizTile / GetHorizTile / SetVertTile / GetVertTile
// ============================================================================

#[test]
fn test_horiz_tile() {
    let env = env();
    let (_, tex) = setup_texture(&env, "HT");
    let (before, after): (bool, bool) = env
        .eval(&format!("local b = {tex}:GetHorizTile(); {tex}:SetHorizTile(true); return b, {tex}:GetHorizTile()"))
        .unwrap();
    assert!(!before, "HorizTile should default to false");
    assert!(after, "HorizTile should be true after SetHorizTile(true)");
}

#[test]
fn test_vert_tile() {
    let env = env();
    let (_, tex) = setup_texture(&env, "VT");
    let (before, after): (bool, bool) = env
        .eval(&format!(
            "local b = {tex}:GetVertTile(); {tex}:SetVertTile(true); return b, {tex}:GetVertTile()"
        ))
        .unwrap();
    assert!(!before, "VertTile should default to false");
    assert!(after, "VertTile should be true after SetVertTile(true)");
}

// ============================================================================
// SetBlendMode / GetBlendMode
// ============================================================================

#[test]
fn test_blend_mode_default() {
    let env = env();
    let (_, tex) = setup_texture(&env, "BM");
    let mode: String = env.eval(&format!("return {tex}:GetBlendMode()")).unwrap();
    assert_eq!(mode, "BLEND");
}

#[test]
fn test_set_blend_mode_no_error() {
    let env = env();
    let (_, tex) = setup_texture(&env, "BMSet");
    env.exec(&format!(r#"{tex}:SetBlendMode("ADD"); {tex}:SetBlendMode("ALPHAKEY"); {tex}:SetBlendMode("DISABLE"); {tex}:SetBlendMode("MOD")"#)).unwrap();
}

#[test]
fn test_set_blend_mode_persists_raw_mode_on_frame() {
    let env = env();
    let (_, tex) = setup_texture(&env, "BMState");
    env.exec(&format!(r#"{tex}:SetBlendMode("MOD")"#)).unwrap();

    let state = env.state().borrow();
    let id = state.widgets.get_id_by_name(&tex).unwrap();
    let widget = state.widgets.get(id).unwrap();
    assert_eq!(widget.alpha_mode.as_deref(), Some("MOD"));
    assert_eq!(widget.blend_mode, wow_ui_sim::BlendMode::Alpha);
}

// ============================================================================
// SetDesaturated / IsDesaturated
// ============================================================================

#[test]
fn test_desaturated_default() {
    let env = env();
    let (_, tex) = setup_texture(&env, "Desat");
    let desat: bool = env.eval(&format!("return {tex}:IsDesaturated()")).unwrap();
    assert!(!desat, "IsDesaturated should default to false");
}

#[test]
fn test_set_desaturated_no_error() {
    let env = env();
    let (_, tex) = setup_texture(&env, "DesatSet");
    env.exec(&format!(
        "{tex}:SetDesaturated(true); {tex}:SetDesaturated(false)"
    ))
    .unwrap();
}

#[test]
fn test_set_desaturated_updates_texture_state() {
    let env = env();
    let (_, tex) = setup_texture(&env, "DesatState");

    env.exec(&format!("{tex}:SetDesaturated(true)")).unwrap();
    let desat: bool = env.eval(&format!("return {tex}:IsDesaturated()")).unwrap();

    assert!(desat, "SetDesaturated(true) should update IsDesaturated");
}

#[test]
fn test_button_texture_child_desaturation_persists() {
    let env = env();

    env.exec(
        r#"
        local btn = CreateFrame("Button", "DesatButtonTextureChild", UIParent)
        btn:SetDisabledTexture("Interface\\EncounterJournal\\UI-EncounterJournalTextures")
        btn:GetDisabledTexture():SetDesaturated(true)
        "#,
    )
    .unwrap();

    let desat: bool = env
        .eval("return DesatButtonTextureChild:GetDisabledTexture():IsDesaturated()")
        .unwrap();

    assert!(
        desat,
        "desaturation should persist on button texture children"
    );
}

// ============================================================================
// SetAtlas / GetAtlas
// ============================================================================

#[test]
fn test_set_atlas_known() {
    let env = env();
    let (_, tex) = setup_texture(&env, "Atlas");
    env.exec(&format!(r#"{tex}:SetAtlas("checkbox-minimal")"#))
        .unwrap();

    let state = env.state().borrow();
    let id = state.widgets.get_id_by_name(&tex).unwrap();
    let widget = state.widgets.get(id).unwrap();
    assert_eq!(widget.atlas.as_deref(), Some("checkbox-minimal"));
    assert!(
        widget.texture.is_some(),
        "Known atlas should set texture path"
    );
    assert!(
        widget.tex_coords.is_some(),
        "Known atlas should set tex_coords"
    );
    assert!(
        widget.atlas_tex_coords.is_some(),
        "Known atlas should set atlas_tex_coords"
    );
}

#[test]
fn test_set_atlas_tile_slice_uses_direct_atlas_entry() {
    let env = env();
    let (_, tex) = setup_texture(&env, "QuestlogAtlas");
    env.exec(&format!(r#"{tex}:SetAtlas("questlog-frame")"#))
        .unwrap();

    let state = env.state().borrow();
    let id = state.widgets.get_id_by_name(&tex).unwrap();
    let widget = state.widgets.get(id).unwrap();

    assert_eq!(widget.atlas.as_deref(), Some("questlog-frame"));
    assert!(
        widget
            .texture
            .as_ref()
            .is_some_and(|path| path.contains("questlogframe")),
        "questlog-frame should resolve to questlogframe texture, got: {:?}",
        widget.texture
    );
    assert!(
        widget.tex_coords.is_some(),
        "questlog-frame should set tex_coords"
    );
    assert!(
        widget.atlas_tex_coords.is_some(),
        "questlog-frame should set atlas_tex_coords"
    );
    assert!(
        widget.nine_slice_atlas.is_none(),
        "questlog-frame should stay a direct atlas slice, not a nine-slice kit"
    );
}

#[test]
fn test_set_atlas_applies_atlas_tiling_flags() {
    let env = env();
    let (_, tex) = setup_texture(&env, "AtlasTileFlags");
    env.exec(&format!(
        r#"{tex}:SetAtlas("!minimal-scrollbar-track-middle")"#
    ))
    .unwrap();

    let (horiz_tile, vert_tile): (bool, bool) = env
        .eval(&format!("return {tex}:GetHorizTile(), {tex}:GetVertTile()"))
        .unwrap();
    assert!(
        !horiz_tile,
        "Vertical atlas should not enable horizontal tiling"
    );
    assert!(vert_tile, "Vertical atlas should enable vertical tiling");
}

#[test]
fn test_set_atlas_clears_previous_atlas_tiling_flags() {
    let env = env();
    let (_, tex) = setup_texture(&env, "AtlasTileClear");
    env.exec(&format!(
        r#"
        {tex}:SetAtlas("!minimal-scrollbar-track-middle")
        {tex}:SetAtlas("checkbox-minimal")
        "#
    ))
    .unwrap();

    let (horiz_tile, vert_tile): (bool, bool) = env
        .eval(&format!("return {tex}:GetHorizTile(), {tex}:GetVertTile()"))
        .unwrap();
    assert!(
        !horiz_tile,
        "Non-tiled atlas should clear horizontal tiling"
    );
    assert!(!vert_tile, "Non-tiled atlas should clear vertical tiling");
}

#[test]
fn test_set_atlas_uses_render_preferred_2x_texture_path() {
    let env = env();
    let (_, tex) = setup_texture(&env, "QuestLogTickSquareAtlas");
    env.exec(&format!(r#"{tex}:SetAtlas("questlog-icon-ticksquare")"#))
        .unwrap();

    let state = env.state().borrow();
    let id = state.widgets.get_id_by_name(&tex).unwrap();
    let widget = state.widgets.get(id).unwrap();

    assert_eq!(widget.atlas.as_deref(), Some("questlog-icon-ticksquare"));
    assert!(
        widget
            .texture
            .as_ref()
            .is_some_and(|path| path.contains("questlogframe2x")),
        "questlog-icon-ticksquare should prefer questlogframe2x for rendering, got: {:?}",
        widget.texture
    );
    assert_eq!(
        widget.tex_coords, widget.atlas_tex_coords,
        "render atlas fallback should preserve atlas UVs"
    );
}

#[test]
fn test_set_atlas_unknown() {
    let env = env();
    let (_, tex) = setup_texture(&env, "AtlasUnk");
    env.exec(&format!(
        r#"{tex}:SetAtlas("nonexistent-atlas-name-12345")"#
    ))
    .unwrap();

    let state = env.state().borrow();
    let id = state.widgets.get_id_by_name(&tex).unwrap();
    let widget = state.widgets.get(id).unwrap();
    assert_eq!(
        widget.atlas.as_deref(),
        Some("nonexistent-atlas-name-12345")
    );
    assert!(
        widget.texture.is_none(),
        "Unknown atlas should not set texture path"
    );
}

#[test]
fn test_get_atlas_default_nil() {
    let env = env();
    let (_, tex) = setup_texture(&env, "AtlasNil");
    let is_nil: bool = env
        .eval(&format!("return {tex}:GetAtlas() == nil"))
        .unwrap();
    assert!(is_nil, "GetAtlas should return nil by default");
}

#[test]
fn test_get_atlas_returns_name() {
    let env = env();
    let (_, tex) = setup_texture(&env, "AtlasGet");
    env.exec(&format!(r#"{tex}:SetAtlas("checkbox-minimal")"#))
        .unwrap();
    let name: String = env.eval(&format!("return {tex}:GetAtlas()")).unwrap();
    assert_eq!(name, "checkbox-minimal");
}

// ============================================================================
// SetAtlas - button parent propagation
// ============================================================================

#[test]
fn test_set_atlas_propagates_to_button_normal_texture() {
    let env = env();
    env.exec(
        r#"
        local btn = CreateFrame("Button", "AtlasBtnFrame", UIParent)
        btn:SetSize(30, 30)
        local normalTex = btn:CreateTexture()
        btn:SetNormalTexture(normalTex)
        normalTex:SetAtlas("checkbox-minimal")
    "#,
    )
    .unwrap();

    let state = env.state().borrow();
    let btn_id = state.widgets.get_id_by_name("AtlasBtnFrame").unwrap();
    let btn = state.widgets.get(btn_id).unwrap();
    assert!(
        btn.normal_texture.is_some(),
        "SetAtlas on NormalTexture child should propagate to parent button"
    );
    assert!(
        btn.normal_tex_coords.is_some(),
        "SetAtlas on NormalTexture child should set parent's normal_tex_coords"
    );
}

// ============================================================================
// Pixel grid and texel snapping stubs
// ============================================================================

#[test]
fn test_snap_to_pixel_grid() {
    let env = env();
    let (_, tex) = setup_texture(&env, "Snap");
    env.exec(&format!("{tex}:SetSnapToPixelGrid(true)"))
        .unwrap();
    let snap: bool = env
        .eval(&format!("return {tex}:IsSnappingToPixelGrid()"))
        .unwrap();
    assert!(snap);
}

#[test]
fn test_texel_snapping_bias() {
    let env = env();
    let (_, tex) = setup_texture(&env, "Bias");
    env.exec(&format!("{tex}:SetTexelSnappingBias(0.5)"))
        .unwrap();
    let bias: f64 = env
        .eval(&format!("return {tex}:GetTexelSnappingBias()"))
        .unwrap();
    assert_eq!(bias, 0.5);
}
