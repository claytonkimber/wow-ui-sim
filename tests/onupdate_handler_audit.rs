use crate::common;

use std::path::PathBuf;

use wow_ui_sim::loader::{discover_blizzard_addons_for_screen, find_toc_file, load_addon};
use wow_ui_sim::lua_api::WowLuaEnv;
use wow_ui_sim::screen::ScreenKind;
use wow_ui_sim::startup::settle_headless_startup;

const BUFF_AUDIT_ADDONS: &[(&str, &str)] = &[
    ("Blizzard_SharedXMLBase", "Blizzard_SharedXMLBase.toc"),
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
    ("Blizzard_Minimap", "Blizzard_Minimap_Mainline.toc"),
    ("Blizzard_BuffFrame", "Blizzard_BuffFrame.toc"),
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
    ("Blizzard_TokenUI", "Blizzard_TokenUI.toc"),
    ("Blizzard_ActionBar", "Blizzard_ActionBar_Mainline.toc"),
];

fn blizzard_ui_dir() -> PathBuf {
    wow_ui_sim::client_profile::blizzard_ui_addons_dir_under(std::path::Path::new(env!(
        "CARGO_MANIFEST_DIR"
    )))
}

fn load_settled_game_ui() -> WowLuaEnv {
    let env = WowLuaEnv::new().expect("Failed to create Lua environment");
    env.set_screen_size(1024.0, 768.0);
    env.set_screen_mode(ScreenKind::Game);
    env.state().borrow_mut().addon_base_paths = vec![blizzard_ui_dir()];

    let addons = discover_blizzard_addons_for_screen(&blizzard_ui_dir(), ScreenKind::Game);
    for (name, toc_path) in &addons {
        load_addon(&env.loader_env(), toc_path)
            .unwrap_or_else(|err| panic!("Failed to load Blizzard addon {name}: {err}"));
    }

    env.apply_post_load_workarounds();
    settle_headless_startup(&env);
    env
}

fn load_buff_audit_env() -> WowLuaEnv {
    let env = WowLuaEnv::new().expect("Failed to create Lua environment");
    env.set_screen_size(1024.0, 768.0);

    let ui = blizzard_ui_dir();
    env.state().borrow_mut().addon_base_paths = vec![ui.clone()];

    for (name, toc) in BUFF_AUDIT_ADDONS {
        let addon_dir = ui.join(name);
        let requested_toc = addon_dir.join(toc);
        let toc_path = if requested_toc.exists() {
            requested_toc
        } else if let Some(discovered_toc) = find_toc_file(&addon_dir) {
            discovered_toc
        } else {
            continue;
        };
        if let Err(error) = load_addon(&env.loader_env(), &toc_path) {
            eprintln!("[load {name}] FAILED: {error}");
        }
    }

    env.apply_post_load_workarounds();
    env.exec(
        r#"
        if not PlayerFrame then
            PlayerFrame = CreateFrame("Frame", "PlayerFrame", UIParent)
        end
        PlayerFrame.unit = "player"
        "#,
    )
    .expect("Failed to create PlayerFrame stub for aura audit");
    env
}

#[test]
fn leave_instance_group_button_queries_group_state_even_when_mutators_noop() {
    test_timeout! {
        let env = load_settled_game_ui();

        env.exec(
            r#"
            local manager = assert(CompactRaidFrameManager, "missing CompactRaidFrameManager")
            local bottom = assert(manager.BottomButtons, "missing bottom buttons")
            for _, child in ipairs({ bottom:GetChildren() }) do
                local name = child and child.GetName and child:GetName()
                if name and name:find("LeaveInstanceGroupButton", 1, true) then
                    AuditLeaveButton = child
                    break
                end
            end
            assert(AuditLeaveButton, "missing leave-instance button")

            local script = assert(AuditLeaveButton:GetScript("OnUpdate"), "missing OnUpdate script")
            script(AuditLeaveButton, 0.016)
            "#,
        )
        .unwrap();

        let _ = env.state().borrow().widgets.take_render_dirty_with_ids();

        let (walk_in_calls, can_leave_calls, in_group_calls, in_instance_calls, lfg_id_calls, set_text_calls, set_enabled_calls): (
            i32,
            i32,
            i32,
            i32,
            i32,
            i32,
            i32,
        ) = env
            .eval(
                r#"
                local button = assert(AuditLeaveButton, "missing leave-instance button")
                local counts = {
                    walkIn = 0,
                    canLeave = 0,
                    inGroup = 0,
                    inInstance = 0,
                    lfgId = 0,
                    setText = 0,
                    setEnabled = 0,
                }

                local originalIsPartyWalkIn = C_PartyInfo.IsPartyWalkIn
                C_PartyInfo.IsPartyWalkIn = function(...)
                    counts.walkIn = counts.walkIn + 1
                    return originalIsPartyWalkIn(...)
                end

                local originalCanLeave = PartyUtil.CanLeaveInstance
                PartyUtil.CanLeaveInstance = function(...)
                    counts.canLeave = counts.canLeave + 1
                    return originalCanLeave(...)
                end

                local originalIsInGroup = IsInGroup
                IsInGroup = function(...)
                    counts.inGroup = counts.inGroup + 1
                    return originalIsInGroup(...)
                end

                local originalIsInInstance = IsInInstance
                IsInInstance = function(...)
                    counts.inInstance = counts.inInstance + 1
                    return originalIsInInstance(...)
                end

                local originalGetPartyLFGID = GetPartyLFGID
                GetPartyLFGID = function(...)
                    counts.lfgId = counts.lfgId + 1
                    return originalGetPartyLFGID(...)
                end

                local originalSetText = button.SetText
                button.SetText = function(self, ...)
                    counts.setText = counts.setText + 1
                    return originalSetText(self, ...)
                end

                local originalSetEnabled = button.SetEnabled
                button.SetEnabled = function(self, ...)
                    counts.setEnabled = counts.setEnabled + 1
                    return originalSetEnabled(self, ...)
                end

                local script = assert(button:GetScript("OnUpdate"), "missing OnUpdate script")
                script(button, 0.016)

                return counts.walkIn, counts.canLeave, counts.inGroup, counts.inInstance,
                    counts.lfgId, counts.setText, counts.setEnabled
                "#,
            )
            .unwrap();

        let (dirty_mask, dirty_ids) = env.state().borrow().widgets.take_render_dirty_with_ids();
        let dirty_ids = dirty_ids.unwrap_or_default();
        assert_eq!(walk_in_calls, 1, "walk-in query should still run each tick");
        assert_eq!(can_leave_calls, 1, "leave-instance eligibility should still run each tick");
        assert_eq!(in_group_calls, 2, "PartyUtil.CanLeaveInstance should still re-check group state twice");
        assert_eq!(
            in_instance_calls, 0,
            "solo state should return before querying instance state"
        );
        assert_eq!(
            lfg_id_calls, 0,
            "solo state should return before querying LFG state"
        );
        assert_eq!(set_text_calls, 1, "button text setter is still invoked from the handler");
        assert_eq!(set_enabled_calls, 1, "button enabled setter is still invoked from the handler");
        assert_eq!(
            dirty_mask, 0,
            "leave-instance OnUpdate should stay visually clean in the settled case"
        );
        assert_eq!(
            dirty_ids.len(),
            0,
            "leave-instance OnUpdate should not enqueue dirty frame ids in settled case"
        );
    }
}

#[test]
fn buff_button_onupdate_still_formats_duration_and_reapplies_font_decisions_after_noop_text() {
    test_timeout! {
        let env = load_buff_audit_env();
        env.exec(
            r#"
            AuditAuraContainer = CreateFrame("Frame", nil, UIParent)
            assert(AuditAuraContainer, "missing audited aura container")
            AuditAuraContainer.GetAuraWarningAlphaForDuration = function(self, duration)
                return 1
            end

            AuditBuffButton = CreateFrame("BUTTON", nil, AuditAuraContainer, "AuraButtonTemplate")
            assert(AuditBuffButton, "missing audited aura button")
            Mixin(AuditBuffButton, AuraButtonMixin)
            AuditBuffButton:OnLoad()
            assert(AuditBuffButton.Update, "missing audited aura button Update")
            DEFAULT_AURA_DURATION_FONT = DEFAULT_AURA_DURATION_FONT or GameFontNormalSmall
            SMALLER_AURA_DURATION_FONT = SMALLER_AURA_DURATION_FONT or GameFontNormalSmall
            SMALLER_AURA_DURATION_OFFSET_Y = SMALLER_AURA_DURATION_OFFSET_Y or 0
            SMALLER_AURA_DURATION_FONT_MIN_THRESHOLD = 1
            SMALLER_AURA_DURATION_FONT_MAX_THRESHOLD = 100000

            AuditBuffButton:Update({
                auraType = "Buff",
                index = 1,
                texture = 136116,
                count = 0,
                duration = 300,
                expirationTime = GetTime() + 300,
                timeMod = 1,
                auraInstanceID = 1,
            })

            local script = assert(AuditBuffButton:GetScript("OnUpdate"), "missing OnUpdate script")
            script(AuditBuffButton, 0.016)
            "#,
        )
        .unwrap();

        let _ = env.state().borrow().widgets.take_render_dirty_with_ids();

        let (seconds_calls, formatted_text_calls, font_object_calls, point_calls, shown_calls, vertex_color_calls, alpha_calls): (
            i32,
            i32,
            i32,
            i32,
            i32,
            i32,
            i32,
        ) = env
            .eval(
                r#"
                local button = assert(AuditBuffButton, "missing audited buff button")
                local duration = assert(button.Duration, "missing duration label")
                local counts = {
                    seconds = 0,
                    formattedText = 0,
                    fontObject = 0,
                    point = 0,
                    shown = 0,
                    vertexColor = 0,
                    alpha = 0,
                }

                local originalSecondsToTimeAbbrev = SecondsToTimeAbbrev
                SecondsToTimeAbbrev = function(...)
                    counts.seconds = counts.seconds + 1
                    return originalSecondsToTimeAbbrev(...)
                end

                local originalSetFormattedText = duration.SetFormattedText
                duration.SetFormattedText = function(self, ...)
                    counts.formattedText = counts.formattedText + 1
                    return originalSetFormattedText(self, ...)
                end

                local originalSetFontObject = duration.SetFontObject
                duration.SetFontObject = function(self, ...)
                    counts.fontObject = counts.fontObject + 1
                    return originalSetFontObject(self, ...)
                end

                local originalSetPoint = duration.SetPoint
                duration.SetPoint = function(self, ...)
                    counts.point = counts.point + 1
                    return originalSetPoint(self, ...)
                end

                local originalSetShown = duration.SetShown
                duration.SetShown = function(self, ...)
                    counts.shown = counts.shown + 1
                    return originalSetShown(self, ...)
                end

                local originalSetVertexColor = duration.SetVertexColor
                duration.SetVertexColor = function(self, ...)
                    counts.vertexColor = counts.vertexColor + 1
                    return originalSetVertexColor(self, ...)
                end

                local originalSetAlpha = button.SetAlpha
                button.SetAlpha = function(self, ...)
                    counts.alpha = counts.alpha + 1
                    return originalSetAlpha(self, ...)
                end

                local script = assert(button:GetScript("OnUpdate"), "missing OnUpdate script")
                script(button, 0.016)

                return counts.seconds, counts.formattedText, counts.fontObject, counts.point,
                    counts.shown, counts.vertexColor, counts.alpha
                "#,
            )
            .unwrap();

        let (dirty_mask, dirty_ids) = env.state().borrow().widgets.take_render_dirty_with_ids();
        let dirty_ids = dirty_ids.unwrap_or_default();
        assert_eq!(seconds_calls, 1, "duration formatting helper should still run each tick");
        assert_eq!(formatted_text_calls, 1, "Duration:SetFormattedText should still run each tick");
        assert_eq!(font_object_calls, 1, "font threshold logic should still reapply the font object");
        assert_eq!(point_calls, 1, "font threshold logic should still reapply the duration anchor");
        assert_eq!(shown_calls, 1, "UpdateDuration should still re-evaluate duration visibility");
        assert_eq!(vertex_color_calls, 1, "UpdateDuration should still reapply duration color");
        assert_eq!(alpha_calls, 1, "warning-alpha path should still re-run each tick");
        assert_eq!(
            dirty_mask, 0,
            "second buff-button tick should be visually clean once no-op setters fast-path"
        );
        assert_eq!(
            dirty_ids.len(),
            0,
            "buff button OnUpdate should not enqueue dirty frame ids in settled state"
        );
    }
}
