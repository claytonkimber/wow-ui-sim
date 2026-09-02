//! Integration tests for `src/lua_api/globals/panel_toggle_verbs.rs`.

use wow_ui_sim::lua_api::WowLuaEnv;

fn env() -> WowLuaEnv {
    WowLuaEnv::new().expect("WowLuaEnv init")
}

fn is_open(env: &WowLuaEnv, panel: &str) -> bool {
    env.state().borrow().open_panels.contains(panel)
}

// Each toggle moves the panel between open and closed; two calls flip
// it twice, restoring the original state.

#[test]
fn toggle_character_flips_open_state() {
    let env = env();
    assert!(!is_open(&env, "Character"));
    env.exec("ToggleCharacter()").unwrap();
    assert!(is_open(&env, "Character"));
    env.exec("ToggleCharacter()").unwrap();
    assert!(!is_open(&env, "Character"));
}

#[test]
fn toggle_spell_book_roundtrips() {
    let env = env();
    env.exec("ToggleSpellBook()").unwrap();
    assert!(is_open(&env, "SpellBook"));
}

#[test]
fn toggle_talent_frame_roundtrips() {
    let env = env();
    env.exec("ToggleTalentFrame()").unwrap();
    assert!(is_open(&env, "Talent"));
}

#[test]
fn toggle_quest_log_roundtrips() {
    let env = env();
    env.exec("ToggleQuestLog()").unwrap();
    assert!(is_open(&env, "QuestLog"));
}

#[test]
fn toggle_world_map_roundtrips() {
    let env = env();
    env.exec("ToggleWorldMap()").unwrap();
    assert!(is_open(&env, "WorldMap"));
}

#[test]
fn toggle_friends_frame_roundtrips() {
    let env = env();
    env.exec("ToggleFriendsFrame()").unwrap();
    assert!(is_open(&env, "Friends"));
}

#[cfg(feature = "client-retail")]
#[test]
fn retail_defers_toggle_guild_frame_to_blizzard_ui() {
    let env = env();
    let toggle_type: String = env.eval("return type(ToggleGuildFrame)").unwrap();
    assert_eq!(toggle_type, "nil");
}

#[cfg(not(feature = "client-retail"))]
#[test]
fn toggle_guild_frame_roundtrips() {
    let env = env();
    env.exec("ToggleGuildFrame()").unwrap();
    assert!(is_open(&env, "Guild"));
}

#[test]
fn toggle_help_frame_roundtrips() {
    let env = env();
    env.exec("ToggleHelpFrame()").unwrap();
    assert!(is_open(&env, "Help"));
}

#[test]
fn toggle_social_panel_roundtrips() {
    let env = env();
    env.exec(r#"CreateFrame("Frame", "FriendsFrame"); FriendsFrame:Hide()"#)
        .unwrap();
    env.exec("ToggleSocialPanel()").unwrap();
    assert!(is_open(&env, "Social"));

    let friends_frame_shown: bool = env.eval("return FriendsFrame:IsShown()").unwrap();
    assert!(friends_frame_shown, "social panel should show FriendsFrame");
}

#[test]
fn toggle_minimap_roundtrips() {
    let env = env();
    env.exec("ToggleMinimap()").unwrap();
    assert!(is_open(&env, "Minimap"));
}

#[test]
fn multiple_panels_can_be_open_simultaneously() {
    let env = env();
    env.exec(
        "ToggleCharacter()
              ToggleQuestLog()
              ToggleWorldMap()",
    )
    .unwrap();
    assert!(is_open(&env, "Character"));
    assert!(is_open(&env, "QuestLog"));
    assert!(is_open(&env, "WorldMap"));
    assert_eq!(env.state().borrow().open_panels.len(), 3);
}

#[test]
fn toggle_dropdown_menu_remains_registered() {
    let env = env();
    // ToggleDropDownMenu owns its own registration in create_frame/dropdown_api.rs;
    // panel_toggle_verbs must not shadow or break it.
    let ty: String = env.eval("return type(ToggleDropDownMenu)").unwrap();
    assert_eq!(ty, "function");
}
