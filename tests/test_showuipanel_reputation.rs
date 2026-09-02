//! Integration tests for ReputationFrame ShowUIPanel layout details.

use crate::common;

use std::path::PathBuf;
use wow_ui_sim::loader::load_addon;
use wow_ui_sim::lua_api::WowLuaEnv;

fn blizzard_ui_dir() -> PathBuf {
    wow_ui_sim::paths::default_blizzard_ui_addons_path().expect("Blizzard UI cache should be available")
}

/// Blizzard addons needed for the panel system (dependency order).
const PANEL_ADDONS: &[(&str, &str)] = &[
    ("Blizzard_SharedXMLBase", "Blizzard_SharedXMLBase.toc"),
    ("Blizzard_Menu", "Blizzard_Menu.toc"),
    ("Blizzard_Colors", "Blizzard_Colors_Mainline.toc"),
    ("Blizzard_SharedXML", "Blizzard_SharedXML_Mainline.toc"),
    (
        "Blizzard_SharedXMLGame",
        "Blizzard_SharedXMLGame_Mainline.toc",
    ),
    (
        "Blizzard_UIPanelTemplates",
        "Blizzard_UIPanelTemplates_Mainline.toc",
    ),
    (
        "Blizzard_FrameXMLBase",
        "Blizzard_FrameXMLBase_Mainline.toc",
    ),
    ("Blizzard_FrameEffects", "Blizzard_FrameEffects.toc"),
    ("Blizzard_LoadLocale", "Blizzard_LoadLocale.toc"),
    ("Blizzard_Fonts_Shared", "Blizzard_Fonts_Shared.toc"),
    ("Blizzard_HelpPlate", "Blizzard_HelpPlate.toc"),
    (
        "Blizzard_AccessibilityTemplates",
        "Blizzard_AccessibilityTemplates.toc",
    ),
    ("Blizzard_ObjectAPI", "Blizzard_ObjectAPI_Mainline.toc"),
    ("Blizzard_UIParent", "Blizzard_UIParent_Mainline.toc"),
    ("Blizzard_TextStatusBar", "Blizzard_TextStatusBar.toc"),
    ("Blizzard_MoneyFrame", "Blizzard_MoneyFrame_Mainline.toc"),
    ("Blizzard_POIButton", "Blizzard_POIButton.toc"),
    ("Blizzard_Flyout", "Blizzard_Flyout.toc"),
    ("Blizzard_StoreUI", "Blizzard_StoreUI_Mainline.toc"),
    ("Blizzard_MicroMenu", "Blizzard_MicroMenu_Mainline.toc"),
    ("Blizzard_EditMode", "Blizzard_EditMode.toc"),
    ("Blizzard_GarrisonBase", "Blizzard_GarrisonBase.toc"),
    ("Blizzard_GameTooltip", "Blizzard_GameTooltip_Mainline.toc"),
    (
        "Blizzard_UIParentPanelManager",
        "Blizzard_UIParentPanelManager_Mainline.toc",
    ),
    (
        "Blizzard_Settings_Shared",
        "Blizzard_Settings_Shared_Mainline.toc",
    ),
    (
        "Blizzard_SettingsDefinitions_Shared",
        "Blizzard_SettingsDefinitions_Shared.toc",
    ),
    (
        "Blizzard_SettingsDefinitions_Frame",
        "Blizzard_SettingsDefinitions_Frame_Mainline.toc",
    ),
    (
        "Blizzard_FrameXMLUtil",
        "Blizzard_FrameXMLUtil_Mainline.toc",
    ),
    ("Blizzard_ItemButton", "Blizzard_ItemButton_Mainline.toc"),
    ("Blizzard_QuickKeybind", "Blizzard_QuickKeybind.toc"),
    ("Blizzard_FrameXML", "Blizzard_FrameXML_Mainline.toc"),
    (
        "Blizzard_UIPanels_Game",
        "Blizzard_UIPanels_Game_Mainline.toc",
    ),
    ("Blizzard_ActionBar", "Blizzard_ActionBar_Mainline.toc"),
    ("Blizzard_UnitFrame", "Blizzard_UnitFrame_Mainline.toc"),
    ("Blizzard_TokenUI", "Blizzard_TokenUI.toc"),
];

fn setup_env() -> WowLuaEnv {
    let env = WowLuaEnv::new().expect("Failed to create Lua environment");
    env.set_screen_size(1024.0, 768.0);

    {
        let mut state = env.state().borrow_mut();
        state.addon_base_paths = vec![blizzard_ui_dir()];
    }

    let ui = blizzard_ui_dir();
    for (name, toc) in PANEL_ADDONS {
        let toc_path = ui.join(name).join(toc);
        if !toc_path.exists() {
            continue;
        }
        if let Err(e) = load_addon(&env.loader_env(), &toc_path) {
            eprintln!("[load {name}] FAILED: {e}");
        }
    }

    env.apply_post_load_workarounds();
    fire_startup_events(&env);
    env
}

fn fire_startup_events(env: &WowLuaEnv) {
    common::fire_addon_loaded(env, "WoWUISim");
    for event in ["VARIABLES_LOADED", "PLAYER_LOGIN"] {
        let _ = env.fire_event(event);
    }
    common::fire_player_entering_world(env, true, false);
    for event in [
        "UPDATE_BINDINGS",
        "DISPLAY_SIZE_CHANGED",
        "UI_SCALE_CHANGED",
    ] {
        let _ = env.fire_event(event);
    }
}

#[test]
fn show_ui_panel_locks_reputation_frame_layout() {
    test_timeout! {
        let env = setup_env();
        let result: String = env
            .eval(
                r#"
                local EPS = 0.75

                local function approx(actual, expected, eps)
                    if type(actual) ~= "number" or type(expected) ~= "number" then
                        return false
                    end
                    return math.abs(actual - expected) <= (eps or EPS)
                end

                local function rect(path, frame)
                    if type(frame) ~= "table" then
                        return nil, path .. "_missing"
                    end
                    local l, b, w, h = frame:GetRect()
                    if not (l and b and w and h) then
                        return nil, path .. "_missing_rect"
                    end
                    return { l = l, b = b, w = w, h = h, r = l + w, t = b + h }, nil
                end

                local function expect_rect(path, frame, expected_left, expected_bottom, expected_width, expected_height)
                    local r, e = rect(path, frame)
                    if not r then
                        return nil, e
                    end
                    if not approx(r.l, expected_left) then
                        return nil, path .. "_left=" .. tostring(r.l)
                    end
                    if not approx(r.b, expected_bottom) then
                        return nil, path .. "_bottom=" .. tostring(r.b)
                    end
                    if not approx(r.w, expected_width, 0.1) then
                        return nil, path .. "_width=" .. tostring(r.w)
                    end
                    if not approx(r.h, expected_height, 0.1) then
                        return nil, path .. "_height=" .. tostring(r.h)
                    end
                    return r, nil
                end

                if not CharacterFrame then
                    return "missing_character_frame"
                end
                if not ShowUIPanel then
                    return "missing_show_uipanel"
                end
                if not ReputationFrame then
                    return "missing_reputation_frame"
                end
                if PanelTemplates_SetTab then
                    PanelTemplates_SetTab(CharacterFrame, ReputationFrame:GetID())
                end
                if PaperDollFrame then
                    PaperDollFrame:Hide()
                end
                if TokenFrame then
                    TokenFrame:Hide()
                end
                ReputationFrame:Show()
                CharacterFrame.activeSubframe = "ReputationFrame"
                ShowUIPanel(CharacterFrame)
                CharacterFrame:RefreshDisplay()

                if not CharacterFrame:IsShown() then
                    return "character_not_shown"
                end
                if not ReputationFrame:IsShown() then
                    return "reputation_not_shown"
                end
                if PaperDollFrame and PaperDollFrame:IsShown() then
                    return "paperdoll_should_be_hidden"
                end

                if PanelTemplates_GetSelectedTab and PanelTemplates_GetSelectedTab(CharacterFrame) ~= 2 then
                    return "selected_tab=" .. tostring(PanelTemplates_GetSelectedTab(CharacterFrame))
                end

                local frameRect, frameErr = expect_rect("CharacterFrame", CharacterFrame, 16, 228, 400, 424)
                if not frameRect then return frameErr end

                local repRect, repErr = expect_rect("ReputationFrame", ReputationFrame, 16, 228, 400, 424)
                if not repRect then return repErr end

                local filterRect, filterErr = expect_rect(
                    "ReputationFrame.filterDropdown",
                    ReputationFrame.filterDropdown,
                    278,
                    597,
                    130,
                    25
                )
                if not filterRect then return filterErr end
                if not ReputationFrame.filterDropdown:IsShown() then
                    return "filter_dropdown_hidden"
                end

                local scrollBoxRect, scrollBoxErr = expect_rect(
                    "ReputationFrame.ScrollBox",
                    ReputationFrame.ScrollBox,
                    24,
                    234,
                    364,
                    354
                )
                if not scrollBoxRect then return scrollBoxErr end
                if not ReputationFrame.ScrollBox:IsShown() then
                    return "scroll_box_hidden"
                end

                local scrollBarRect, scrollBarErr = expect_rect(
                    "ReputationFrame.ScrollBar",
                    ReputationFrame.ScrollBar,
                    393,
                    238,
                    8,
                    348
                )
                if not scrollBarRect then return scrollBarErr end
                if not ReputationFrame.ScrollBar:IsShown() then
                    return "scroll_bar_hidden"
                end

                if not approx(scrollBarRect.l, scrollBoxRect.r + 5) then
                    return "scroll_bar_left=" .. tostring(scrollBarRect.l)
                end

                local detail = ReputationFrame.ReputationDetailFrame
                if not detail then
                    return "detail_frame_missing"
                end
                if detail:IsShown() then
                    return "detail_frame_should_start_hidden"
                end

                local detailRect, detailErr = expect_rect(
                    "ReputationFrame.ReputationDetailFrame",
                    detail,
                    416,
                    421,
                    212,
                    203
                )
                if not detailRect then return detailErr end

                detail:Show()
                if not detail:IsShown() then
                    return "detail_frame_not_shown_after_show"
                end

                local titleRect, titleErr = rect("ReputationDetailFrame.Title", detail.Title)
                if not titleRect then return titleErr end
                if not approx(titleRect.l, detailRect.l + 20) then
                    return "detail_title_left=" .. tostring(titleRect.l)
                end
                if not approx(titleRect.t, detailRect.t - 21) then
                    return "detail_title_top=" .. tostring(titleRect.t)
                end

                local atWarRect, atWarErr = expect_rect(
                    "ReputationDetailFrame.AtWarCheckbox",
                    detail.AtWarCheckbox,
                    430,
                    455,
                    26,
                    26
                )
                if not atWarRect then return atWarErr end

                local inactiveRect, inactiveErr = rect(
                    "ReputationDetailFrame.MakeInactiveCheckbox",
                    detail.MakeInactiveCheckbox
                )
                if not inactiveRect then return inactiveErr end
                if not approx(inactiveRect.w, 26, 0.1) or not approx(inactiveRect.h, 26, 0.1) then
                    return "inactive_checkbox_size=" .. tostring(inactiveRect.w) .. "x" .. tostring(inactiveRect.h)
                end

                local watchRect, watchErr = expect_rect(
                    "ReputationDetailFrame.WatchFactionCheckbox",
                    detail.WatchFactionCheckbox,
                    430,
                    432,
                    26,
                    26
                )
                if not watchRect then return watchErr end

                if not approx(inactiveRect.b, atWarRect.b) then
                    return "inactive_checkbox_bottom=" .. tostring(inactiveRect.b)
                end
                if not (inactiveRect.l > atWarRect.r + 30) then
                    return "inactive_checkbox_left=" .. tostring(inactiveRect.l)
                end
                if not approx(watchRect.l, atWarRect.l) then
                    return "watch_checkbox_left=" .. tostring(watchRect.l)
                end
                if not approx(watchRect.t, atWarRect.b + 3) then
                    return "watch_checkbox_top=" .. tostring(watchRect.t)
                end

                return "ok"
            "#,
            )
            .unwrap();
        assert_eq!(
            result, "ok",
            "ReputationFrame layout should stay fully locked after CharacterFrame shows the reputation subframe: {result}"
        );
    }
}

#[test]
fn reputation_filter_dropdown_mouse_down_materializes_menu_rows() {
    test_timeout! {
        let env = setup_env();
        let setup_result: String = env
            .eval(
                r#"
                if not CharacterFrame or not ReputationFrame or not ReputationFrame.filterDropdown then
                    return "missing_frame"
                end
                if PanelTemplates_SetTab then
                    PanelTemplates_SetTab(CharacterFrame, ReputationFrame:GetID())
                end
                return "ready"
                "#,
            )
            .unwrap();
        assert_eq!(setup_result, "ready");

        let state = env.state();
        let (reputation_id, dropdown_id) = {
            let sim = state.borrow();
            let reputation_id = sim
                .widgets
                .get_id_by_name("ReputationFrame")
                .expect("ReputationFrame should exist");
            let dropdown_id = sim
                .widgets
                .get(reputation_id)
                .and_then(|frame| frame.children_keys.get("filterDropdown"))
                .copied()
                .expect("ReputationFrame filterDropdown should exist");
            (reputation_id, dropdown_id)
        };
        env.fire_script_handler(reputation_id, "OnShow", Vec::new())
            .expect("ReputationFrame OnShow should dispatch");
        let left_button = env.lua_string("LeftButton");
        env.fire_script_handler(dropdown_id, "OnMouseDown", vec![left_button])
            .expect("reputation filter OnMouseDown should dispatch");

        let result: String = env
            .eval(
                r#"
                local dropdown = ReputationFrame.filterDropdown
                local buttons = {}
                if dropdown.menu then
                    for _, child in ipairs({ dropdown.menu:GetChildren() }) do
                        if child:GetObjectType() == "Button" then
                            buttons[#buttons + 1] = child
                        end
                    end
                end
                if #buttons ~= 4 then
                    return "button_count=" .. tostring(#buttons)
                end
                local playerName = UnitName("player")
                local expected = {"All", "Warband", playerName, "Show Legacy Reputations"}
                for index, expectedText in ipairs(expected) do
                    local button = buttons[index]
                    if not button then
                        return "missing_button_" .. tostring(index)
                    end
                    if not button:IsShown() then
                        return "hidden_button_" .. tostring(index)
                    end
                    local text = button:GetText()
                    if (text == nil or text == "") and button.fontString then
                        text = button.fontString:GetText()
                    end
                    if text ~= expectedText then
                        return "button_" .. tostring(index) .. "=" .. tostring(text)
                    end
                    if button:GetFrameStrata() ~= "FULLSCREEN_DIALOG" then
                        return "button_" .. tostring(index) .. "_strata=" .. tostring(button:GetFrameStrata())
                    end
                end

                return "ok"
                "#,
            )
            .unwrap();
        assert_eq!(result, "ok");
    }
}
