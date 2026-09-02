#![cfg(feature = "client-retail")]
use std::path::PathBuf;

use wow_ui_sim::loader::{discover_blizzard_addons_for_screen, load_addon};
use wow_ui_sim::lua_api::WowLuaEnv;
use wow_ui_sim::screen::ScreenKind;
use wow_ui_sim::startup::fire_startup_events_for_screen;
use wow_ui_sim::toc::TocFile;

fn blizzard_ui_dir() -> PathBuf {
    wow_ui_sim::client_profile::blizzard_ui_addons_dir_under(std::path::Path::new(env!(
        "CARGO_MANIFEST_DIR"
    )))
}

fn deprecated_chat_info_toc() -> PathBuf {
    blizzard_ui_dir().join("Blizzard_DeprecatedChatInfo/Blizzard_DeprecatedChatInfo.toc")
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

#[test]
fn blizzard_deprecated_chat_info_toc_is_minimal_with_no_flags_or_deps() {
    let toc = TocFile::from_file(&deprecated_chat_info_toc())
        .expect("Blizzard_DeprecatedChatInfo TOC should parse");
    assert!(
        !toc.is_load_on_demand(),
        "Blizzard_DeprecatedChatInfo declares `## LoadOnDemand: 0` — the SendChatMessage / \
         DoEmote / CancelEmote globals and the ~100 ChatFrame_* / ChatEdit_* aliases must \
         install before any chat addon Lua executes"
    );
    assert!(
        !toc.is_secure_env(),
        "Blizzard_DeprecatedChatInfo does not declare UseSecureEnvironment"
    );
    assert!(
        toc.dependencies().is_empty(),
        "Blizzard_DeprecatedChatInfo declares NO explicit dependencies despite referencing \
         ChatFrameUtil / ChatFrameMixin / ChatFrameEditBoxMixin / ChatFrameConstants / \
         MessageFrameScrollButtonConstants — alphabetical addon load order has \
         Blizzard_ChatFrameBase load before Blizzard_DeprecatedChatInfo so those globals \
         exist at module-eval time"
    );

    let toc_text = std::fs::read_to_string(deprecated_chat_info_toc())
        .expect("Blizzard_DeprecatedChatInfo TOC should read");
    assert!(
        !toc_text.contains("## AllowLoad:"),
        "Blizzard_DeprecatedChatInfo omits `## AllowLoad:` — defaults to Game-screen-only \
         (src/toc.rs:311), matching the legacy chat API in-game-only surface"
    );
    assert!(
        !toc_text.contains("## AllowLoadGameType:"),
        "Blizzard_DeprecatedChatInfo omits `## AllowLoadGameType:` so the shims install on \
         every game type without restriction"
    );
}

#[test]
fn blizzard_deprecated_chat_info_appears_in_game_discovery_only() {
    let game_addons = discover_blizzard_addons_for_screen(&blizzard_ui_dir(), ScreenKind::Game);
    let in_game = game_addons
        .iter()
        .any(|(name, _)| name == "Blizzard_DeprecatedChatInfo");
    assert!(
        in_game,
        "Blizzard_DeprecatedChatInfo (no AllowLoad flag, defaults to Game-only) should appear \
         in Game-screen auto-discovery"
    );

    let login_addons = discover_blizzard_addons_for_screen(&blizzard_ui_dir(), ScreenKind::Login);
    let in_login = login_addons
        .iter()
        .any(|(name, _)| name == "Blizzard_DeprecatedChatInfo");
    assert!(
        !in_login,
        "Blizzard_DeprecatedChatInfo should NOT appear on the Login / glue screens — chat \
         is an in-game concept"
    );
}

prefork_full_ui_case! {
fn blizzard_deprecated_chat_info_loads_without_errors(env: &WowLuaEnv) {

    let addon_errors: Vec<String> = env
        .state()
        .borrow()
        .lua_errors
        .iter()
        .filter(|message| {
            message.contains("DeprecatedChatInfo")
                || message.contains("Deprecated_ChatInfo")
                || message.contains("Deprecated_ChatFrame")
        })
        .cloned()
        .collect();
    assert!(
        addon_errors.is_empty(),
        "Blizzard_DeprecatedChatInfo emitted Lua errors during Game-screen load:\n  {}",
        addon_errors.join("\n  ")
    );
}
}

prefork_full_ui_case! {
fn blizzard_deprecated_chat_info_installs_chat_info_function_shims(env: &WowLuaEnv) {

    let installed: bool = env
        .eval(
            "return type(SendChatMessage) == 'function' \
                and type(DoEmote) == 'function' \
                and type(CancelEmote) == 'function'",
        )
        .expect("ChatInfo function-shim installation query should succeed");
    assert!(
        installed,
        "Deprecated_ChatInfo.lua should publish 3 forwarding global functions: \
         SendChatMessage(message, chatType, languageID, target) → \
         C_ChatInfo.SendChatMessage; DoEmote(emoteName, targetName, suppressMoveError) — \
         when emoteName is non-nil delegates to C_ChatInfo.PerformEmote, otherwise calls \
         C_ChatInfo.CancelEmote and returns false; CancelEmote() → C_ChatInfo.CancelEmote"
    );
}
}

prefork_full_ui_case! {
fn blizzard_deprecated_chat_info_installs_chat_constants(env: &WowLuaEnv) {

    let installed: bool = env
        .eval(
            "return CHAT_BUTTON_FLASH_TIME == ChatFrameConstants.ScrollToBottomFlashInterval \
                and CHAT_TELL_ALERT_TIME == ChatFrameConstants.WhisperSoundAlertCooldown \
                and MAX_COMMUNITY_NAME_LENGTH == ChatFrameConstants.TruncatedCommunityNameLength \
                and MAX_COMMUNITY_NAME_LENGTH_NO_CHANNEL \
                    == ChatFrameConstants.TruncatedCommunityNameWithoutChannelLength \
                and MAX_REMEMBERED_TELLS == ChatFrameConstants.MaxRememberedWhisperTargets \
                and MESSAGE_SCROLLBUTTON_INITIAL_DELAY \
                    == MessageFrameScrollButtonConstants.InitialScrollDelay \
                and MESSAGE_SCROLLBUTTON_SCROLL_DELAY \
                    == MessageFrameScrollButtonConstants.HeldScrollDelay \
                and MAX_WOW_CHAT_CHANNELS == Constants.ChatFrameConstants.MaxChatChannels \
                and MAX_CHARACTER_NAME_BYTES == Constants.ChatFrameConstants.MaxCharacterNameBytes \
                and NUM_CHAT_WINDOWS == Constants.ChatFrameConstants.MaxChatWindows \
                and MAX_COUNTDOWN_SECONDS == Constants.PartyCountdownConstants.MaxCountdownSeconds",
        )
        .expect("ChatFrame constants query should succeed");
    assert!(
        installed,
        "Deprecated_ChatFrame.lua line 8-20 should mirror 11 legacy chat constants from \
         ChatFrameConstants (CHAT_BUTTON_FLASH_TIME / CHAT_TELL_ALERT_TIME / \
         MAX_COMMUNITY_NAME_LENGTH / MAX_COMMUNITY_NAME_LENGTH_NO_CHANNEL / \
         MAX_REMEMBERED_TELLS), MessageFrameScrollButtonConstants \
         (MESSAGE_SCROLLBUTTON_INITIAL_DELAY / MESSAGE_SCROLLBUTTON_SCROLL_DELAY), \
         Constants.ChatFrameConstants (MAX_WOW_CHAT_CHANNELS / MAX_CHARACTER_NAME_BYTES / \
         NUM_CHAT_WINDOWS), and Constants.PartyCountdownConstants (MAX_COUNTDOWN_SECONDS)"
    );
}
}

prefork_full_ui_case! {
fn blizzard_deprecated_chat_info_installs_chat_frame_util_aliases(env: &WowLuaEnv) {

    let installed: bool = env
        .eval(
            "return Chat_AddSystemMessage == ChatFrameUtil.AddSystemMessage \
                and Chat_GetChannelColor == ChatFrameUtil.GetChannelColor \
                and Chat_GetChatFrame == ChatFrameUtil.GetChatFrame \
                and ChatEdit_ActivateChat == ChatFrameUtil.ActivateChat \
                and ChatEdit_ChooseBoxForSend == ChatFrameUtil.ChooseBoxForSend \
                and ChatEdit_GetActiveWindow == ChatFrameUtil.GetActiveWindow \
                and ChatFrame_AddMessageEventFilter == ChatFrameUtil.AddMessageEventFilter \
                and ChatFrame_DisplayChatHelp == ChatFrameUtil.DisplayChatHelp \
                and ChatFrame_DisplaySystemMessage == ChatFrameUtil.DisplaySystemMessage \
                and ChatFrame_OpenChat == ChatFrameUtil.OpenChat \
                and ChatFrame_RemoveMessageEventFilter == ChatFrameUtil.RemoveMessageEventFilter \
                and ChatFrame_ScrollToBottom == ChatFrameUtil.ScrollToBottom \
                and ChatFrame_SendTell == ChatFrameUtil.SendTell \
                and GetChatTimestampFormat == ChatFrameUtil.GetTimestampFormat \
                and SubstituteChatMessageBeforeSend == ChatFrameUtil.SubstituteChatMessageBeforeSend",
        )
        .expect("ChatFrameUtil alias installation query should succeed");
    assert!(
        installed,
        "Deprecated_ChatFrame.lua line 22-91 should mirror ~70 ChatFrameUtil methods to \
         legacy Chat_*/ChatEdit_*/ChatFrame_* / GetChatTimestampFormat / \
         SubstituteChatMessageBeforeSend globals as direct table reference assignments. The \
         15-symbol sample (Chat_AddSystemMessage / Chat_GetChannelColor / Chat_GetChatFrame \
         / ChatEdit_ActivateChat / ChatEdit_ChooseBoxForSend / ChatEdit_GetActiveWindow / \
         ChatFrame_AddMessageEventFilter / ChatFrame_DisplayChatHelp / \
         ChatFrame_DisplaySystemMessage / ChatFrame_OpenChat / \
         ChatFrame_RemoveMessageEventFilter / ChatFrame_ScrollToBottom / \
         ChatFrame_SendTell / GetChatTimestampFormat / SubstituteChatMessageBeforeSend) \
         must each equal the matching ChatFrameUtil.* function reference"
    );
}
}

prefork_full_ui_case! {
fn blizzard_deprecated_chat_info_installs_chat_frame_mixin_aliases(env: &WowLuaEnv) {

    let installed: bool = env
        .eval(
            "return ChatFrame_AddMessage == ChatFrameMixin.AddMessage \
                and ChatFrame_AddMessageGroup == ChatFrameMixin.AddMessageGroup \
                and ChatFrame_AddPrivateMessageTarget == ChatFrameMixin.AddPrivateMessageTarget \
                and ChatFrame_AddSingleMessageType == ChatFrameMixin.AddSingleMessageType \
                and ChatFrame_ContainsChannel == ChatFrameMixin.ContainsChannel \
                and ChatFrame_ContainsMessageGroup == ChatFrameMixin.ContainsMessageGroup \
                and ChatFrame_GetDefaultChatTarget == ChatFrameMixin.GetDefaultChatTarget \
                and ChatFrame_RegisterForChannels == ChatFrameMixin.RegisterForChannels \
                and ChatFrame_RegisterForMessages == ChatFrameMixin.RegisterForMessages \
                and ChatFrame_RemoveAllChannels == ChatFrameMixin.RemoveAllChannels \
                and ChatFrame_RemoveChannel == ChatFrameMixin.RemoveChannel \
                and ChatFrame_RemoveMessageGroup == ChatFrameMixin.RemoveMessageGroup \
                and ChatFrame_UpdateColorByID == ChatFrameMixin.UpdateColorByID \
                and ChatFrame_UpdateDefaultChatTarget == ChatFrameMixin.UpdateDefaultChatTarget",
        )
        .expect("ChatFrameMixin alias installation query should succeed");
    assert!(
        installed,
        "Deprecated_ChatFrame.lua line 93-112 should mirror 20 ChatFrameMixin methods to \
         legacy ChatFrame_* globals (the per-frame methods that the legacy code called as \
         free functions on a frame, where the modern code calls them as `chatFrame:Method()`)"
    );
}
}

prefork_full_ui_case! {
fn blizzard_deprecated_chat_info_installs_chat_edit_box_mixin_aliases(env: &WowLuaEnv) {

    let installed: bool = env
        .eval(
            "return ChatEdit_AddHistory == ChatFrameEditBoxMixin.AddHistory \
                and ChatEdit_ClearChat == ChatFrameEditBoxMixin.ClearChat \
                and ChatEdit_DoesCurrentChannelTargetMatch \
                    == ChatFrameEditBoxMixin.DoesCurrentChannelTargetMatch \
                and ChatEdit_ExtractChannel == ChatFrameEditBoxMixin.ExtractChannel \
                and ChatEdit_ExtractTellTarget == ChatFrameEditBoxMixin.ExtractTellTarget \
                and ChatEdit_GetChannelTarget == ChatFrameEditBoxMixin.GetChannelTarget \
                and ChatEdit_HandleChatType == ChatFrameEditBoxMixin.HandleChatType \
                and ChatEdit_ParseText == ChatFrameEditBoxMixin.ParseText \
                and ChatEdit_ResetChatType == ChatFrameEditBoxMixin.ResetChatType \
                and ChatEdit_ResetChatTypeToSticky == ChatFrameEditBoxMixin.ResetChatTypeToSticky \
                and ChatEdit_SendText == ChatFrameEditBoxMixin.SendText \
                and ChatEdit_SetDeactivated == ChatFrameEditBoxMixin.Deactivate \
                and ChatEdit_UpdateHeader == ChatFrameEditBoxMixin.UpdateHeader",
        )
        .expect("ChatFrameEditBoxMixin alias installation query should succeed");
    assert!(
        installed,
        "Deprecated_ChatFrame.lua line 114-126 should mirror 13 ChatFrameEditBoxMixin methods \
         to legacy ChatEdit_* globals. Notable: ChatEdit_SetDeactivated maps to the renamed \
         ChatFrameEditBoxMixin.Deactivate (line 125) — the mixin method dropped the 'Set' \
         prefix"
    );
}
}

prefork_full_ui_case! {
fn blizzard_deprecated_chat_info_load_deprecation_fallbacks_cvar_is_default_on(env: &WowLuaEnv) {

    let cvar_on: bool = env
        .eval("return GetCVarBool('loadDeprecationFallbacks')")
        .expect("GetCVarBool query should succeed");
    assert!(
        cvar_on,
        "The `loadDeprecationFallbacks` CVar must default to true (src/cvars.yaml:899 sets \
         '1') so the early-return guards at Deprecated_ChatInfo.lua:4 and \
         Deprecated_ChatFrame.lua:4 don't bail before any shim is defined. If this CVar \
         flips to false, ALL ~100 deprecated chat globals are skipped"
    );
}
}

#[test]
fn blizzard_deprecated_chat_info_has_no_xml_only_two_lua_files() {
    let dir = blizzard_ui_dir().join("Blizzard_DeprecatedChatInfo");
    let entries: Vec<String> = std::fs::read_dir(&dir)
        .expect("Blizzard_DeprecatedChatInfo dir should read")
        .flatten()
        .filter_map(|e| e.file_name().into_string().ok())
        .collect();

    let has_xml = entries.iter().any(|n| n.ends_with(".xml"));
    assert!(
        !has_xml,
        "Blizzard_DeprecatedChatInfo has NO XML files — pure Lua function-shim definitions \
         only. Got entries: {entries:?}"
    );

    let has_chat_info_lua = entries.iter().any(|n| n == "Deprecated_ChatInfo.lua");
    let has_chat_frame_lua = entries.iter().any(|n| n == "Deprecated_ChatFrame.lua");
    assert!(
        has_chat_info_lua && has_chat_frame_lua,
        "Blizzard_DeprecatedChatInfo should ship both `Deprecated_ChatInfo.lua` (3 \
         C_ChatInfo function shims) and `Deprecated_ChatFrame.lua` (~100 ChatFrameUtil / \
         ChatFrameMixin / ChatFrameEditBoxMixin aliases + chat constants)"
    );
}
