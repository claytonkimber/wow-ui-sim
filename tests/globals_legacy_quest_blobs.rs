#![cfg(feature = "gui")]

//! Tests for quest log, POI blobs, map ID, tooltip, and cursor position.

use wow_ui_sim::iced_app::{
    RegistryQuadBatchParams, build_quad_batch_for_registry_with_quest_blobs,
};
use wow_ui_sim::lua_api::WowLuaEnv;

fn env() -> WowLuaEnv {
    WowLuaEnv::new().expect("Failed to create Lua environment")
}

fn build_strata_buckets(env: &WowLuaEnv) -> Vec<Vec<u64>> {
    let mut state = env.state().borrow_mut();
    state.ensure_layout_rects();
    let _ = state.get_strata_buckets();
    state.strata_buckets.as_ref().unwrap().clone()
}

fn build_quads(env: &WowLuaEnv) -> wow_ui_sim::render::QuadBatch {
    let buckets = build_strata_buckets(env);
    let state = env.state().borrow();
    build_quad_batch_for_registry_with_quest_blobs(
        RegistryQuadBatchParams::new(&state.widgets, (1024.0, 768.0), &buckets)
            .quest_blobs(Some(&state.quest_blobs)),
    )
}

// ============================================================================
// Quest log selection and description text
// ============================================================================

#[test]
fn test_quest_log_set_get_selected() {
    let env = env();
    let (before, after): (i32, i32) = env
        .eval(
            r#"
            local before = C_QuestLog.GetSelectedQuest()
            C_QuestLog.SetSelectedQuest(80000)
            return before, C_QuestLog.GetSelectedQuest()
        "#,
        )
        .unwrap();
    assert_eq!(before, 0, "initially no quest selected");
    assert_eq!(after, 80000, "SetSelectedQuest stores the ID");
}

#[test]
fn test_get_quest_log_quest_text_returns_description() {
    let env = env();
    let (desc, obj): (String, String) = env
        .eval(
            r#"
            C_QuestLog.SetSelectedQuest(80000)
            return GetQuestLogQuestText()
        "#,
        )
        .unwrap();
    assert!(
        desc.contains("Ironforge expedition"),
        "description should contain quest text, got: {desc}"
    );
    assert!(
        obj.contains("Ironforge Relics"),
        "objectives should contain objective text, got: {obj}"
    );
}

#[test]
fn test_get_quest_log_quest_text_no_selection() {
    let env = env();
    let (desc, obj): (String, String) = env
        .eval(
            r#"
            C_QuestLog.SetSelectedQuest(0)
            return GetQuestLogQuestText()
        "#,
        )
        .unwrap();
    assert_eq!(desc, "", "no quest selected → empty description");
    assert_eq!(obj, "", "no quest selected → empty objectives");
}

#[test]
fn test_is_current_quest_failed_tracks_selected_quest() {
    let env = env();
    {
        let mut state = env.state().borrow_mut();
        state.quest_log_entries.entries[0].is_failed = true;
    }

    let is_failed: bool = env
        .eval(
            r#"
            C_QuestLog.SetSelectedQuest(80000)
            return IsCurrentQuestFailed()
        "#,
        )
        .unwrap();

    assert!(is_failed, "selected failed quest should report failed");
}

#[test]
fn test_is_current_quest_failed_false_without_selection() {
    let env = env();
    let is_failed: bool = env.eval("return IsCurrentQuestFailed()").unwrap();
    assert!(!is_failed, "no selected quest should not report failed");
}

#[test]
fn test_get_quest_poi_blob_count_known_quest() {
    let env = env();
    let count: i32 = env.eval("return GetQuestPOIBlobCount(80000)").unwrap();
    assert_eq!(count, 1, "Quest 80000 should have 1 blob");
}

#[test]
fn test_get_quest_poi_blob_count_unknown_quest() {
    let env = env();
    let count: i32 = env.eval("return GetQuestPOIBlobCount(99999)").unwrap();
    assert_eq!(count, 0, "Unknown quest should have 0 blobs");
}

#[test]
fn test_draw_blob_stores_quest_id() {
    let env = env();

    env.exec(
        r#"
        local poi = CreateFrame("Frame", "TestPOI", UIParent)
        poi:SetMapID(2248)
        poi:DrawBlob(80000, true)
    "#,
    )
    .unwrap();

    let state = env.state().borrow();
    let poi_id = state.widgets.get_id_by_name("TestPOI").unwrap();
    let blob = state.quest_blobs.get(&poi_id).unwrap();
    assert_eq!(blob.map_id, 2248);
    assert_eq!(blob.active_quests, vec![80000]);
}

#[test]
fn test_draw_blob_emits_fill_geometry_into_render_batch() {
    let env = env();

    env.exec(
        r#"
        local poi = CreateFrame("QuestPOIFrame", "RenderPOI", UIParent)
        poi:SetPoint("TOPLEFT", UIParent, "TOPLEFT", 100, -120)
        poi:SetSize(200, 180)
        poi:SetMapID(2248)
        poi:SetFillTexture("Interface/QuestBlobFill")
        poi:SetFillAlpha(0.75)
        poi:DrawBlob(80000, true)
    "#,
    )
    .unwrap();

    let batch = build_quads(&env);
    assert!(
        batch
            .texture_requests
            .iter()
            .any(|request| request.path == "Interface/QuestBlobFill"),
        "DrawBlob should enqueue the configured fill texture for rendering"
    );
    assert!(
        batch
            .texture_requests
            .iter()
            .filter(|request| request.path == "Interface/QuestBlobFill")
            .all(|request| request.vertex_count > 0),
        "DrawBlob should emit blob-specific geometry for the configured fill texture"
    );
}

#[test]
fn test_draw_blob_multiple_quests() {
    let env = env();

    env.exec(
        r#"
        local poi = CreateFrame("Frame", "TestPOI2", UIParent)
        poi:DrawBlob(80000, true)
        poi:DrawBlob(80001, true)
    "#,
    )
    .unwrap();

    let state = env.state().borrow();
    let poi_id = state.widgets.get_id_by_name("TestPOI2").unwrap();
    let blob = state.quest_blobs.get(&poi_id).unwrap();
    assert_eq!(blob.active_quests, vec![80000, 80001]);
}

#[test]
fn test_draw_blob_no_duplicates() {
    let env = env();

    env.exec(
        r#"
        local poi = CreateFrame("Frame", "TestPOI3", UIParent)
        poi:DrawBlob(80000, true)
        poi:DrawBlob(80000, true)
    "#,
    )
    .unwrap();

    let state = env.state().borrow();
    let poi_id = state.widgets.get_id_by_name("TestPOI3").unwrap();
    let blob = state.quest_blobs.get(&poi_id).unwrap();
    assert_eq!(
        blob.active_quests,
        vec![80000],
        "Should not duplicate quest IDs"
    );
}

#[test]
fn test_draw_none_clears_blobs() {
    let env = env();

    env.exec(
        r#"
        local poi = CreateFrame("Frame", "TestPOI4", UIParent)
        poi:DrawBlob(80000, true)
        poi:DrawBlob(80001, true)
        poi:DrawNone()
    "#,
    )
    .unwrap();

    let state = env.state().borrow();
    let poi_id = state.widgets.get_id_by_name("TestPOI4").unwrap();
    let blob = state.quest_blobs.get(&poi_id).unwrap();
    assert!(
        blob.active_quests.is_empty(),
        "DrawNone should clear all blobs"
    );
}

#[test]
fn test_draw_none_clears_rendered_blob_geometry() {
    let env = env();

    env.exec(
        r#"
        local poi = CreateFrame("QuestPOIFrame", "RenderPOIClear", UIParent)
        poi:SetPoint("TOPLEFT", UIParent, "TOPLEFT", 100, -120)
        poi:SetSize(200, 180)
        poi:SetMapID(2248)
        poi:SetFillTexture("Interface/QuestBlobFill")
        poi:SetFillAlpha(0.75)
        poi:DrawBlob(80000, true)
        poi:DrawNone()
    "#,
    )
    .unwrap();

    let batch = build_quads(&env);
    assert!(
        !batch
            .texture_requests
            .iter()
            .any(|request| request.path == "Interface/QuestBlobFill"),
        "DrawNone should remove blob-specific fill geometry from the render batch"
    );
}

#[test]
fn test_quest_blob_render_inputs_persist_on_first_setter_use() {
    let env = env();

    env.exec(
        r#"
        local poi = CreateFrame("Frame", "TestPOIStyled", UIParent)
        poi:SetFillTexture("Interface\\WorldMap\\UI-QuestBlob-Inside")
        poi:SetBorderTexture("Interface\\WorldMap\\UI-QuestBlob-Outside")
        poi:SetFillAlpha(128)
        poi:SetBorderAlpha(192)
        poi:SetBorderScalar(1.0)
    "#,
    )
    .unwrap();

    let state = env.state().borrow();
    let poi_id = state.widgets.get_id_by_name("TestPOIStyled").unwrap();
    let blob = state.quest_blobs.get(&poi_id).unwrap();
    assert!(blob.active_quests.is_empty());
    assert_eq!(
        blob.fill_texture.as_deref(),
        Some("Interface\\WorldMap\\UI-QuestBlob-Inside")
    );
    assert_eq!(
        blob.border_texture.as_deref(),
        Some("Interface\\WorldMap\\UI-QuestBlob-Outside")
    );
    assert_eq!(blob.fill_alpha, Some(128.0));
    assert_eq!(blob.border_alpha, Some(192.0));
    assert_eq!(blob.border_scalar, Some(1.0));
}

#[test]
fn test_set_map_id_and_get_map_id() {
    let env = env();

    env.exec(
        r#"
        local poi = CreateFrame("Frame", "TestPOI5", UIParent)
        poi:SetMapID(37)
    "#,
    )
    .unwrap();

    let map_id: i32 = env.eval("return TestPOI5:GetMapID()").unwrap();
    assert_eq!(map_id, 37);
}

#[test]
fn test_update_mouse_over_tooltip_hit() {
    let env = env();

    // Quest 80000 blob is on map 2248, centered around (0.45, 0.58)
    let quest_id: i32 = env
        .eval(
            r#"
        local poi = CreateFrame("Frame", "HitTestPOI", UIParent)
        poi:SetMapID(2248)
        poi:DrawBlob(80000, true)
        local qid, count = poi:UpdateMouseOverTooltip(0.45, 0.58)
        return qid or 0
    "#,
        )
        .unwrap();
    assert_eq!(quest_id, 80000, "Should hit quest 80000 blob");
}

#[test]
fn test_update_mouse_over_tooltip_returns_first_matching_quest_and_blob_count() {
    let env = env();

    let (quest_id, blob_count): (i32, i32) = env
        .eval(
            r#"
        local poi = CreateFrame("Frame", "OrderedHitPOI", UIParent)
        poi:SetMapID(37)
        poi:DrawBlob(80001, true)
        poi:DrawBlob(80002, true)
        local qid, count = poi:UpdateMouseOverTooltip(0.74, 0.77)
        return qid or 0, count or 0
    "#,
        )
        .unwrap();

    assert_eq!(
        quest_id, 80001,
        "Should return the first matching active quest"
    );
    assert_eq!(blob_count, 1, "Should report the number of matching blobs");
}

#[test]
fn test_update_mouse_over_tooltip_miss() {
    let env = env();

    let is_nil: bool = env
        .eval(
            r#"
        local poi = CreateFrame("Frame", "MissPOI", UIParent)
        poi:SetMapID(2248)
        poi:DrawBlob(80000, true)
        local qid = poi:UpdateMouseOverTooltip(0.1, 0.1)
        return qid == nil
    "#,
        )
        .unwrap();
    assert!(is_nil, "Point outside blob should return nil");
}

#[test]
fn test_update_mouse_over_tooltip_no_blobs() {
    let env = env();

    let is_nil: bool = env
        .eval(
            r#"
        local poi = CreateFrame("Frame", "EmptyPOI", UIParent)
        local qid = poi:UpdateMouseOverTooltip(0.5, 0.5)
        return qid == nil
    "#,
        )
        .unwrap();
    assert!(is_nil, "No active blobs should return nil");
}

#[test]
fn test_get_tooltip_index_identity() {
    let env = env();

    let result: i32 = env
        .eval(
            r#"
        local poi = CreateFrame("Frame", "TooltipIdxPOI", UIParent)
        return poi:GetTooltipIndex(3)
    "#,
        )
        .unwrap();
    assert_eq!(result, 3, "GetTooltipIndex should return the input index");
}

#[test]
fn test_get_tooltip_index_first() {
    let env = env();

    let result: i32 = env
        .eval(
            r#"
        local poi = CreateFrame("Frame", "TooltipIdxPOI2", UIParent)
        return poi:GetTooltipIndex(1)
    "#,
        )
        .unwrap();
    assert_eq!(result, 1);
}

#[test]
fn test_get_cursor_position_reads_mouse_state() {
    let env = env();
    env.set_screen_size(1600.0, 1200.0);

    env.state().borrow_mut().mouse_position = Some((200.0, 300.0));

    let (x, y): (f64, f64) = env.eval("return GetCursorPosition()").unwrap();
    assert!((x - 200.0).abs() < 0.1, "x should be 200, got {x}");
    assert!((y - 900.0).abs() < 0.1, "y should be 900, got {y}");
}

#[test]
fn test_get_cursor_position_default_when_no_mouse() {
    let env = env();

    // mouse_position is None by default
    let (x, y): (f64, f64) = env.eval("return GetCursorPosition()").unwrap();
    assert!((x - 0.0).abs() < 0.1, "default x should be 0, got {x}");
    assert!((y - 0.0).abs() < 0.1, "default y should be 0, got {y}");
}

#[test]
fn test_get_cursor_position_updates_dynamically() {
    let env = env();

    env.state().borrow_mut().mouse_position = Some((100.0, 50.0));
    let (x1, _): (f64, f64) = env.eval("return GetCursorPosition()").unwrap();

    env.state().borrow_mut().mouse_position = Some((700.0, 500.0));
    let (x2, _): (f64, f64) = env.eval("return GetCursorPosition()").unwrap();

    assert!((x1 - 100.0).abs() < 0.1);
    assert!((x2 - 700.0).abs() < 0.1, "Should reflect updated position");
}
