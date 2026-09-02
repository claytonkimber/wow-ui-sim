#![cfg(feature = "client-retail")]
use std::path::PathBuf;

use wow_ui_sim::loader::discover_blizzard_addons_for_screen;
use wow_ui_sim::loader::load_addon;
use wow_ui_sim::lua_api::WowLuaEnv;
use wow_ui_sim::screen::ScreenKind;
use wow_ui_sim::startup::fire_startup_events_for_screen;

fn blizzard_ui_dir() -> PathBuf {
    wow_ui_sim::client_profile::blizzard_ui_addons_dir_under(std::path::Path::new(env!(
        "CARGO_MANIFEST_DIR"
    )))
}

fn load_full_game_ui() -> WowLuaEnv {
    let env = WowLuaEnv::new().expect("Failed to create Lua environment");
    env.set_screen_size(1024.0, 768.0);
    env.set_screen_mode(ScreenKind::Game);

    {
        let mut state = env.state().borrow_mut();
        state.addon_base_paths = vec![blizzard_ui_dir()];
    }

    wow_ui_sim::xml::register_intrinsic_templates();

    let ui = blizzard_ui_dir();
    let addons = discover_blizzard_addons_for_screen(&ui, ScreenKind::Game);
    for (name, toc_path) in &addons {
        load_addon(&env.loader_env(), toc_path)
            .unwrap_or_else(|err| panic!("[load {name}] FAILED: {err}"));
    }

    env.apply_post_load_workarounds();
    fire_startup_events_for_screen(&env, ScreenKind::Game);
    env
}

prefork_full_ui_case! {
fn blizzard_chat_frame_base_top_level_frames_are_defined(env: &WowLuaEnv) {

    let frames_present: bool = env
        .eval(
            "return _G.GeneralDockManager ~= nil \
                and _G.FloatingChatFrameManager ~= nil \
                and _G.ChatFrame1 ~= nil",
        )
        .expect("frame query should succeed");
    assert!(
        frames_present,
        "GeneralDockManager, FloatingChatFrameManager, ChatFrame1 should be defined after load"
    );

    let mixins_present: bool = env
        .eval(
            "return type(ChatFrameMixin) == 'table' \
                and type(ChatAlertFrameMixin) == 'table' \
                and type(ChannelFrameButtonMixin) == 'table' \
                and type(MessageFrameScrollButtonMixin) == 'table' \
                and type(ChatFrameEditBoxBaseMixin) == 'table' \
                and type(ChatFrameEditBoxMixin) == 'table' \
                and type(FloatingChatFrameMixin) == 'table' \
                and type(PrimaryChatFrameMixin) == 'table' \
                and type(ChatFrameMenuButtonMixin) == 'table'",
        )
        .expect("mixin query should succeed");
    assert!(
        mixins_present,
        "Blizzard_ChatFrameBase mixins should be defined after load"
    );

    let constants_present: bool = env
        .eval(
            "return type(SlashCmdList) == 'table' \
                and type(ChatTypeInfo) == 'table' \
                and type(DEFAULT_CHAT_FRAME) ~= 'nil'",
        )
        .expect("constants query should succeed");
    assert!(
        constants_present,
        "Chat globals (SlashCmdList, ChatTypeInfo, DEFAULT_CHAT_FRAME) should be populated"
    );
}
}

prefork_full_ui_case! {
fn chat_frame_add_message_runs_without_errors(env: &WowLuaEnv) {

    {
        let mut state = env.state().borrow_mut();
        state.lua_errors.clear();
        state.lua_error_records.clear();
        state.lua_error_counts.clear();
    }

    env.exec("ChatFrame1:AddMessage('hello from blizzard_chat_frame_base test', 1, 1, 1)")
        .expect("ChatFrame1:AddMessage should run");

    let unexpected_errors: Vec<String> = env.state().borrow().lua_errors.clone();
    assert!(
        unexpected_errors.is_empty(),
        "ChatFrame1:AddMessage emitted Lua errors:\n  {}",
        unexpected_errors.join("\n  ")
    );
}
}
