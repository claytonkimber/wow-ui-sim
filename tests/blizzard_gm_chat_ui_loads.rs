#![cfg(feature = "client-retail")]
use std::path::PathBuf;

use wow_ui_sim::loader::{discover_blizzard_addons_for_screen, find_toc_file, load_addon};
use wow_ui_sim::lua_api::WowLuaEnv;
use wow_ui_sim::screen::ScreenKind;
use wow_ui_sim::toc::TocFile;

fn blizzard_ui_dir() -> PathBuf {
    wow_ui_sim::client_profile::blizzard_ui_addons_dir_under(std::path::Path::new(env!(
        "CARGO_MANIFEST_DIR"
    )))
}

fn gm_chat_ui_dir() -> PathBuf {
    blizzard_ui_dir().join("Blizzard_GMChatUI")
}

fn gm_chat_ui_toc() -> PathBuf {
    gm_chat_ui_dir().join("Blizzard_GMChatUI.toc")
}

fn status_ui_toc() -> PathBuf {
    blizzard_ui_dir()
        .join("Blizzard_StatusUI")
        .join("Blizzard_StatusUI.toc")
}

fn load_gm_chat_ui_with_dependencies(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &status_ui_toc())
        .expect("Blizzard_StatusUI (LoD dep of Blizzard_GMChatUI) should load explicitly");
    load_addon(&env.loader_env(), &gm_chat_ui_toc())
        .expect("Blizzard_GMChatUI should load via explicit Rust loader call");
}

#[test]
fn blizzard_gm_chat_ui_find_toc_resolves_bare_variant() {
    let resolved = find_toc_file(&gm_chat_ui_dir()).expect("Blizzard_GMChatUI TOC should resolve");
    assert_eq!(
        resolved,
        gm_chat_ui_toc(),
        "Blizzard_GMChatUI ships only the bare `Blizzard_GMChatUI.toc` (no flavor variants) — \
         `find_toc_file` (src/loader/mod.rs:65) falls through to the bare `.toc` suffix after \
         the flavor-specific lookups miss"
    );
}

#[test]
fn blizzard_gm_chat_ui_toc_declares_lod_with_single_status_ui_dep() {
    let toc = TocFile::from_file(&gm_chat_ui_toc()).expect("Blizzard_GMChatUI TOC should parse");
    assert!(
        toc.is_load_on_demand(),
        "Blizzard_GMChatUI declares `## LoadOnDemand: 1` — the GM chat frame is loaded lazily \
         the first time a Game Master whispers the player (the client receives a CHAT_MSG_WHISPER \
         with arg6 == \"GM\" and routes through LoadAddOn(\"Blizzard_GMChatUI\"))"
    );
    assert!(
        !toc.is_load_first(),
        "Blizzard_GMChatUI does not declare `## LoadFirst: 1` — LoadOnDemand precludes any \
         load-order priority since it only loads on explicit demand"
    );
    assert!(
        !toc.is_secure_env(),
        "Blizzard_GMChatUI does not declare `## UseSecureEnvironment` — the GM chat frame uses \
         the standard ScrollingMessageFrame surface and SetCVar(\"lastTalkedToGM\") which routes \
         its own protected actions"
    );
    assert_eq!(
        toc.dependencies(),
        vec!["Blizzard_StatusUI".to_string()],
        "Blizzard_GMChatUI declares exactly one dep: Blizzard_StatusUI — the StatusUIFrame / \
         StatusUIMixin chain that GMChatStatusFrame inherits from (XML line 129 \
         `<Button name=\"GMChatStatusFrame\" inherits=\"StatusUIFrame\" \
         mixin=\"GMChatStatusMixin\">`, and Blizzard_GMChatUI.lua line 188 calls \
         `StatusUIMixin.OnLoad(self)` from GMChatStatusMixin:OnLoad)"
    );
}

#[test]
fn blizzard_gm_chat_ui_toc_declares_no_allow_load_directive() {
    let toc_text =
        std::fs::read_to_string(gm_chat_ui_toc()).expect("Blizzard_GMChatUI TOC should read");
    assert!(
        !toc_text.contains("## AllowLoad:"),
        "Blizzard_GMChatUI omits `## AllowLoad:` entirely — LoadOnDemand keeps it out of any \
         auto-discovery regardless of screen, so no AllowLoad gating is needed"
    );
    assert!(
        !toc_text.contains("## AllowLoadGameType:"),
        "Blizzard_GMChatUI omits `## AllowLoadGameType:` — the GM chat surface is reused across \
         mainline and classic flavors"
    );
    assert!(
        toc_text.contains("## LoadOnDemand: 1"),
        "Blizzard_GMChatUI raw TOC must contain `## LoadOnDemand: 1`"
    );
}

#[test]
fn blizzard_gm_chat_ui_toc_lists_bootstrap_lua_xml_and_localization() {
    let toc = TocFile::from_file(&gm_chat_ui_toc()).expect("Blizzard_GMChatUI TOC should parse");
    let files: Vec<String> = toc
        .files
        .iter()
        .map(|p| p.to_string_lossy().replace('\\', "/"))
        .collect();
    assert_eq!(
        files,
        vec![
            "Blizzard_GMChatUI_Bootstrap.lua".to_string(),
            "Blizzard_GMChatUI.lua".to_string(),
            "Blizzard_GMChatUI.xml".to_string(),
            "Localization.lua".to_string(),
        ],
        "Blizzard_GMChatUI TOC lists exactly 4 files in source order: the interaction bootstrap, \
         main Lua helpers required by the XML scripts, XML roots for GMChatFrame and \
         GMChatStatusFrame, then the localization stub. The Bootstrap annotation does not reorder \
         normal explicit addon loading"
    );
}

#[test]
fn blizzard_gm_chat_ui_directory_ships_four_files_plus_toc() {
    let dir = gm_chat_ui_dir();
    let mut entries: Vec<String> = std::fs::read_dir(&dir)
        .expect("Blizzard_GMChatUI directory should exist")
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    entries.sort();
    assert_eq!(
        entries,
        vec![
            "Blizzard_GMChatUI.lua".to_string(),
            "Blizzard_GMChatUI.toc".to_string(),
            "Blizzard_GMChatUI.xml".to_string(),
            "Blizzard_GMChatUI_Bootstrap.lua".to_string(),
            "Localization.lua".to_string(),
        ],
        "Blizzard_GMChatUI directory ships exactly 5 entries (TOC + bootstrap + Lua + XML + \
         Localization) with no flavor subdirectory or media folder"
    );
}

#[test]
fn blizzard_gm_chat_ui_excluded_from_all_screen_auto_discovery_due_to_lod() {
    let ui = blizzard_ui_dir();
    for screen in [
        ScreenKind::Game,
        ScreenKind::Login,
        ScreenKind::CharacterSelect,
        ScreenKind::CharacterCreate,
    ] {
        let addons = discover_blizzard_addons_for_screen(&ui, screen);
        let discovered = addons.iter().any(|(name, _)| name == "Blizzard_GMChatUI");
        assert!(
            !discovered,
            "Blizzard_GMChatUI MUST NOT appear in {screen:?} auto-discovery — `## LoadOnDemand: \
             1` keeps it out of every auto-discovery pass. The addon loads only via an explicit \
             LoadAddOn call from the chat-handler that sees the first GM whisper"
        );
    }
}

prefork_full_ui_case! {
fn blizzard_gm_chat_ui_loads_explicitly_without_addon_specific_lua_errors(env: &WowLuaEnv) {
    load_gm_chat_ui_with_dependencies(env);

    let lua_errors: Vec<String> = env.state().borrow().lua_errors.clone();
    let related: Vec<&String> = lua_errors
        .iter()
        .filter(|e| {
            e.contains("Blizzard_GMChatUI/")
                || e.contains("Blizzard_GMChatUI\\")
                || e.contains("GMChatFrame_")
                || e.contains("GMChatStatusMixin")
        })
        .collect();
    assert!(
        related.is_empty(),
        "Blizzard_GMChatUI emitted addon-specific Lua errors during explicit LoadAddOn:\n  {}",
        related
            .iter()
            .map(|s| s.as_str())
            .collect::<Vec<_>>()
            .join("\n  ")
    );
}
}

prefork_full_ui_case! {
fn blizzard_gm_chat_ui_is_addon_loaded_returns_true_after_explicit_load(env: &WowLuaEnv) {
    load_gm_chat_ui_with_dependencies(env);

    let loaded: bool = env
        .eval("return C_AddOns.IsAddOnLoaded('Blizzard_GMChatUI')")
        .expect("IsAddOnLoaded query should succeed");
    assert!(
        loaded,
        "After explicit LoadAddOn, `C_AddOns.IsAddOnLoaded('Blizzard_GMChatUI')` should return \
         true — the addon registers itself as loaded once the Rust loader processes its TOC"
    );
}
}

prefork_full_ui_case! {
fn blizzard_gm_chat_ui_publishes_chat_frame_lifecycle_helpers(env: &WowLuaEnv) {
    load_gm_chat_ui_with_dependencies(env);

    for helper in [
        "GMChatFrame_OnLoad",
        "GMChatFrame_OnEvent",
        "GMChatFrame_OnShow",
        "GMChatFrame_OnHide",
        "GMChatFrame_OnUpdate",
        "GMChatFrame_Show",
        "GMChatFrame_Close",
        "GMChatFrame_IsGM",
    ] {
        let exists: bool = env
            .eval(&format!("return type(_G['{helper}']) == 'function'"))
            .expect("helper existence query should succeed");
        assert!(
            exists,
            "After LoadAddOn, `{helper}` should be published as a `_G` function — \
             Blizzard_GMChatUI.lua publishes 8 GMChatFrame_* lifecycle and helper globals \
             consumed by the GMChatFrame XML's `<Scripts><OnLoad function=\"...\"/>` bindings \
             plus the chat-handler's GMChatFrame_IsGM(playerName) lookup"
        );
    }
}
}

prefork_full_ui_case! {
fn blizzard_gm_chat_ui_publishes_gm_chat_status_mixin_with_two_methods(env: &WowLuaEnv) {
    load_gm_chat_ui_with_dependencies(env);

    let mixin_exists: bool = env
        .eval("return type(_G['GMChatStatusMixin']) == 'table'")
        .expect("mixin existence query should succeed");
    assert!(
        mixin_exists,
        "GMChatStatusMixin should publish as a `_G` table — Blizzard_GMChatUI.lua line 183 \
         declares the mixin with OnLoad / OnClick methods bound by the XML \
         `<Button name=\"GMChatStatusFrame\" mixin=\"GMChatStatusMixin\">` declaration"
    );

    for method in ["OnLoad", "OnClick"] {
        let method_exists: bool = env
            .eval(&format!(
                "return type(_G['GMChatStatusMixin']['{method}']) == 'function'"
            ))
            .expect("mixin method query should succeed");
        assert!(
            method_exists,
            "GMChatStatusMixin.{method} should be a function — owned by Blizzard_GMChatUI.lua"
        );
    }
}
}

prefork_full_ui_case! {
fn blizzard_gm_chat_ui_publishes_gm_chat_frame_global(env: &WowLuaEnv) {
    load_gm_chat_ui_with_dependencies(env);

    let frames_exist: bool = env
        .eval(
            "local chat = _G['GMChatFrame']; local status = _G['GMChatStatusFrame']; \
             return type(chat) == 'table' and type(chat.GetName) == 'function' \
                and type(status) == 'table' and type(status.GetName) == 'function'",
        )
        .expect("GM chat frame existence query should succeed");
    assert!(
        frames_exist,
        "After LoadAddOn, both named XML roots must publish globally. GMChatFrame is the \
         ScrollingMessageFrame root; GMChatStatusFrame is the Button root inheriting the virtual \
         StatusUIFrame template. A failure while finalizing GMChatFrame must not abort processing \
         before the status frame is created"
    );
}
}

prefork_full_ui_case! {
fn blizzard_gm_chat_ui_is_gm_returns_falsy_for_unknown_player(env: &WowLuaEnv) {
    load_gm_chat_ui_with_dependencies(env);

    let result: Option<bool> = env
        .eval("local v = GMChatFrame_IsGM('UnknownPlayer'); return v == nil or v == false")
        .expect("GMChatFrame_IsGM lookup should succeed");
    assert!(
        result.unwrap_or(false),
        "GMChatFrame_IsGM('UnknownPlayer') should return nil/false for any player not yet \
         observed as a GM — the `ListOfGMs` table starts empty and is only populated when a \
         CHAT_MSG_WHISPER with arg6 == 'GM' is received"
    );
}
}
