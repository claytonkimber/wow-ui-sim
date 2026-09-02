//! Integration tests for Blizzard_ZoneAbility and the C_ZoneAbility stub.

use crate::common;

use std::path::{Path, PathBuf};

use wow_ui_sim::loader::{find_toc_file, load_addon};
use wow_ui_sim::lua_api::WowLuaEnv;

const ZONE_ABILITY_ADDONS: &[(&str, &str)] = &[
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
    (
        "Blizzard_ActionBarController",
        "Blizzard_ActionBarController.toc",
    ),
    ("Blizzard_ZoneAbility", "Blizzard_ZoneAbility_Mainline.toc"),
];

fn blizzard_ui_dir() -> PathBuf {
    wow_ui_sim::client_profile::blizzard_ui_addons_dir_under(std::path::Path::new(env!(
        "CARGO_MANIFEST_DIR"
    )))
}

fn setup_env() -> WowLuaEnv {
    let env = WowLuaEnv::new().expect("Failed to create Lua environment");
    env.set_screen_size(1024.0, 768.0);

    let ui = blizzard_ui_dir();
    env.state().borrow_mut().addon_base_paths = vec![ui.clone()];

    load_blizzard_addons(&env, &ui);
    env.apply_post_load_workarounds();
    fire_startup_events(&env);
    env
}

fn load_blizzard_addons(env: &WowLuaEnv, ui: &Path) {
    for (name, toc) in ZONE_ABILITY_ADDONS {
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
fn zone_ability_frame_displays_seeded_active_ability() {
    test_timeout! {
        let env = setup_env();
        let (frame_shown, container_shown, button_count, spell_id, icon_present): (
            bool,
            bool,
            i64,
            i64,
            bool,
        ) = env
            .eval(
                r#"
                C_ZoneAbility._state.activeAbilities = {
                    {
                        zoneAbilityID = 1,
                        uiPriority = 1,
                        spellID = 372610,
                        textureKit = nil,
                        tutorialText = "Skyward Ascent",
                    },
                }

                ZoneAbilityFrame:UpdateDisplayedZoneAbilities()

                local buttonCount = 0
                local spellID = 0
                local iconTexture
                for button in ZoneAbilityFrame.SpellButtonContainer:EnumerateActive() do
                    buttonCount = buttonCount + 1
                    spellID = button:GetSpellID() or 0
                    iconTexture = button.Icon:GetTexture()
                end

                return ZoneAbilityFrame:IsShown() == true,
                    ExtraAbilityContainer:IsShown() == true,
                    buttonCount,
                    spellID,
                    iconTexture ~= nil and iconTexture ~= ""
                "#,
            )
            .unwrap();

        assert!(frame_shown, "ZoneAbilityFrame should show for an active ability");
        assert!(
            container_shown,
            "ExtraAbilityContainer should show when a zone ability is active"
        );
        assert_eq!(button_count, 1, "exactly one zone ability button should be active");
        assert_eq!(spell_id, 372610, "zone ability button should keep the seeded spell ID");
        assert!(
            icon_present,
            "zone ability button should resolve a visible icon texture"
        );
    }
}
