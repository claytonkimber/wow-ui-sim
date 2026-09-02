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
fn blizzard_chat_frame_top_level_frames_are_defined(env: &WowLuaEnv) {

    let frames_present: bool = env
        .eval(
            "return _G.ChatConfigFrame ~= nil \
                and _G.ChatAlertFrame ~= nil \
                and _G.TextToSpeechFrame ~= nil \
                and _G.BattlegroundChatFilters ~= nil",
        )
        .expect("frame query should succeed");
    assert!(
        frames_present,
        "ChatConfigFrame, ChatAlertFrame, TextToSpeechFrame, BattlegroundChatFilters should be defined after load"
    );

    let mixins_present: bool = env
        .eval(
            "return type(VoiceChatHeadsetButtonMixin) == 'table' \
                and type(VoiceChatHeadsetMixin) == 'table' \
                and type(VoiceChatTranscriptionButtonMixin) == 'table' \
                and type(VoiceChatTranscriptionMixin) == 'table' \
                and type(VoiceChatDotsMixin) == 'table' \
                and type(SpeechToTextMixin) == 'table' \
                and type(TextToSpeechButtonMixin) == 'table' \
                and type(TTSSettingsSliderMixin) == 'table' \
                and type(BattlegroundChatFiltersMixin) == 'table' \
                and type(ChatWindowTabMixin) == 'table' \
                and type(ChatConfigFrameTabManagerMixin) == 'table' \
                and type(ChatConfigWideCheckboxManagerMixin) == 'table' \
                and type(ChatConfigWideCheckboxMixin) == 'table' \
                and type(TextToSpeechCharacterSpecificButtonMixin) == 'table'",
        )
        .expect("mixin query should succeed");
    assert!(
        mixins_present,
        "Blizzard_ChatFrame mixins should be defined after load"
    );
}
}

prefork_full_ui_case! {
fn chat_config_frame_show_and_hide_run_without_errors(env: &WowLuaEnv) {

    {
        let mut state = env.state().borrow_mut();
        state.lua_errors.clear();
        state.lua_error_records.clear();
        state.lua_error_counts.clear();
    }

    env.exec("ChatConfigFrame:Show(); ChatConfigFrame:Hide()")
        .expect("ChatConfigFrame Show/Hide should run");

    let unexpected_errors: Vec<String> = env.state().borrow().lua_errors.clone();
    assert!(
        unexpected_errors.is_empty(),
        "ChatConfigFrame Show/Hide emitted Lua errors:\n  {}",
        unexpected_errors.join("\n  ")
    );
}
}
