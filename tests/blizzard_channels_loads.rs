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

fn channels_toc() -> PathBuf {
    blizzard_ui_dir().join("Blizzard_Channels/Blizzard_Channels.toc")
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
fn blizzard_channels_loads_without_errors(env: &WowLuaEnv) {

    {
        let mut state = env.state().borrow_mut();
        state.lua_errors.clear();
        state.lua_error_records.clear();
        state.lua_error_counts.clear();
    }

    load_addon(&env.loader_env(), &channels_toc())
        .expect("Blizzard_Channels should load via Rust loader");

    let load_errors: Vec<String> = env.state().borrow().lua_errors.clone();
    assert!(
        load_errors.is_empty(),
        "Blizzard_Channels emitted Lua errors during load:\n  {}",
        load_errors.join("\n  ")
    );

    let frames_present: bool = env
        .eval(
            "return _G.ChannelFrame ~= nil \
                and _G.CreateChannelPopup ~= nil \
                and _G.VoiceActivityManager ~= nil",
        )
        .expect("frame query should succeed");
    assert!(
        frames_present,
        "ChannelFrame, CreateChannelPopup, VoiceActivityManager should be defined after load"
    );

    let mixins_present: bool = env
        .eval(
            "return type(ChannelFrameMixin) == 'table' \
                and type(ChannelListMixin) == 'table' \
                and type(ChannelRosterMixin) == 'table' \
                and type(ChannelRosterButtonMixin) == 'table' \
                and type(ChannelButtonBaseMixin) == 'table' \
                and type(ChannelButtonMixin) == 'table' \
                and type(ChannelButtonTextMixin) == 'table' \
                and type(ChannelButtonVoiceMixin) == 'table' \
                and type(ChannelButtonCommunityMixin) == 'table' \
                and type(ChannelButtonHeaderMixin) == 'table' \
                and type(CreateChannelPopupMixin) == 'table' \
                and type(VoiceActivityManagerMixin) == 'table' \
                and type(VoiceActivityNotificationBaseMixin) == 'table' \
                and type(VoiceActivityNotificationMixin) == 'table' \
                and type(VoiceChatActivateChannelPromptMixin) == 'table'",
        )
        .expect("mixin query should succeed");
    assert!(
        mixins_present,
        "Blizzard_Channels mixins should be defined after load"
    );
}
}

prefork_full_ui_case! {
fn channel_frame_show_and_hide_run_without_errors(env: &WowLuaEnv) {

    {
        let mut state = env.state().borrow_mut();
        state.lua_errors.clear();
        state.lua_error_records.clear();
        state.lua_error_counts.clear();
    }

    env.exec("ChannelFrame:Show(); ChannelFrame:Hide()")
        .expect("ChannelFrame Show/Hide should run");

    let unexpected_errors: Vec<String> = env.state().borrow().lua_errors.clone();
    assert!(
        unexpected_errors.is_empty(),
        "ChannelFrame Show/Hide emitted Lua errors:\n  {}",
        unexpected_errors.join("\n  ")
    );
}
}
