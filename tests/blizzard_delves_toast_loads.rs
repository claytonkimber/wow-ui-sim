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

fn delves_toast_toc() -> PathBuf {
    blizzard_ui_dir().join("Blizzard_DelvesToast/Blizzard_DelvesToast.toc")
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
fn blizzard_delves_toast_toc_is_non_lod_game_only_with_no_deps() {
    let toc =
        TocFile::from_file(&delves_toast_toc()).expect("Blizzard_DelvesToast TOC should parse");
    assert!(
        !toc.is_load_on_demand(),
        "Blizzard_DelvesToast is non-LOD — the toast frame must exist on Game-screen bring-up \
         so the DELVE_ASSIST_ACTION event handler can fire any time without a load round-trip"
    );
    assert!(
        !toc.is_secure_env(),
        "Blizzard_DelvesToast does not declare UseSecureEnvironment"
    );
    let deps = toc.dependencies();
    assert!(
        deps.is_empty(),
        "Blizzard_DelvesToast declares NO dependencies — its only external surfaces \
         (AlertFrame_*, ChatAlertFrame, UnitPopup_OpenMenu, LinkUtil, NORMAL_FONT_COLOR, \
         PlaySound, SOUNDKIT, C_Spell, Enum.AssistActionType, NOTIFY_ASSIST_ACTION_* strings) \
         all live in the FrameXML/SharedXML always-loaded base, not in any other Blizzard_* \
         addon. Got {deps:?}"
    );

    let toc_text =
        std::fs::read_to_string(delves_toast_toc()).expect("Blizzard_DelvesToast TOC should read");
    assert!(
        toc_text.contains("## AllowLoad: game"),
        "Blizzard_DelvesToast declares `## AllowLoad: game` (the toast is in-game-only — no \
         glue-screen counterpart since assist actions only fire mid-delve)"
    );
    assert!(
        toc_text.contains("## AllowLoadGameType: standard"),
        "Blizzard_DelvesToast declares `## AllowLoadGameType: standard` (delves are a \
         retail-only feature — Classic flavors don't ship this addon)"
    );
}

#[test]
fn blizzard_delves_toast_appears_in_game_discovery() {
    let addons = discover_blizzard_addons_for_screen(&blizzard_ui_dir(), ScreenKind::Game);
    let in_game = addons
        .iter()
        .any(|(name, _)| name == "Blizzard_DelvesToast");
    assert!(
        in_game,
        "Blizzard_DelvesToast (non-LOD with `## AllowLoad: game` + `## AllowLoadGameType: \
         standard` + no deps) should appear in Game-screen auto-discovery so the toast frame \
         is ready before the first DELVE_ASSIST_ACTION event"
    );
}

#[test]
fn blizzard_delves_toast_is_absent_from_login_discovery() {
    let addons = discover_blizzard_addons_for_screen(&blizzard_ui_dir(), ScreenKind::Login);
    let in_login = addons
        .iter()
        .any(|(name, _)| name == "Blizzard_DelvesToast");
    assert!(
        !in_login,
        "Blizzard_DelvesToast carries `## AllowLoad: game` so it must NOT appear on the \
         Login / glue screens"
    );
}

prefork_full_ui_case! {
fn blizzard_delves_toast_loads_without_errors(env: &WowLuaEnv) {

    let addon_errors: Vec<String> = env
        .state()
        .borrow()
        .lua_errors
        .iter()
        .filter(|message| message.contains("DelvesToast"))
        .cloned()
        .collect();
    assert!(
        addon_errors.is_empty(),
        "Blizzard_DelvesToast emitted Lua errors during Game-screen load:\n  {}",
        addon_errors.join("\n  ")
    );
}
}

prefork_full_ui_case! {
fn blizzard_delves_toast_frame_is_created_hidden_with_close_button(env: &WowLuaEnv) {

    let frame_present: bool = env
        .eval(
            "return DelvesToastFrame ~= nil \
                and DelvesToastFrame:IsObjectType('Frame') \
                and not DelvesToastFrame:IsShown() \
                and DelvesToastFrame.CloseButton ~= nil \
                and DelvesToastFrame.Text ~= nil",
        )
        .expect("DelvesToastFrame query should succeed");
    assert!(
        frame_present,
        "DelvesToastFrame (Blizzard_DelvesToast.xml:15 — ContainedAlertFrame, parent=UIParent, \
         hidden, frameStrata=LOW, toplevel, mixin=DelvesToastMixin, 360x80) should be created \
         and hidden by default with a CloseButton (UIPanelCloseButton at TOPRIGHT) and a Text \
         FontString (FriendsFont_Large, justified LEFT/MIDDLE)"
    );
}
}

prefork_full_ui_case! {
fn blizzard_delves_toast_mixin_is_published_with_lifecycle_methods(env: &WowLuaEnv) {

    let mixin_present: bool = env
        .eval(
            "return type(DelvesToastMixin) == 'table' \
                and type(DelvesToastMixin.OnLoad) == 'function' \
                and type(DelvesToastMixin.OnEvent) == 'function' \
                and type(DelvesToastMixin.OnEnter) == 'function' \
                and type(DelvesToastMixin.OnLeave) == 'function' \
                and type(DelvesToastMixin.OnClick) == 'function' \
                and type(DelvesToastMixin.OnHyperlinkClick) == 'function' \
                and type(DelvesToastMixin.SetToast) == 'function'",
        )
        .expect("DelvesToastMixin query should succeed");
    assert!(
        mixin_present,
        "Blizzard_DelvesToast.lua line 1 should publish DelvesToastMixin with 8 methods: \
         OnLoad (registers DELVE_ASSIST_ACTION + AlertFrame_SetDuration + ChatAlertFrame \
         subsystem registration + close-button scripts), OnEvent (handles DELVE_ASSIST_ACTION \
         by formatting the assist message and routing through SetToast), OnEnter / OnLeave \
         (pause / resume the alert-frame fade-out animation), OnClick (LeftButton hides), \
         OnHyperlinkClick (RightButton on a `playername` hyperlink opens UnitPopup FRIEND \
         menu, otherwise falls through to OnClick), and SetToast (sets the message text + \
         plays UI_BNET_TOAST + fires AlertFrame_ShowNewAlert)"
    );
}
}

prefork_full_ui_case! {
fn blizzard_delves_toast_xml_animation_templates_are_registered(env: &WowLuaEnv) {
    let _env = env;

    let anim_in = wow_ui_sim::xml::get_anim_group_template("DelvesToastAnimInTemplate");
    let anim_out = wow_ui_sim::xml::get_anim_group_template("DelvesToastAnimOutTemplate");

    assert!(
        anim_in.is_some(),
        "DelvesToastAnimInTemplate (Blizzard_DelvesToast.xml:3 — virtual=true AnimationGroup \
         with two Alpha animations: order 1 fades from 1->0 instantly, order 2 fades 0->1 \
         over 0.2s; parentKey=animIn) should be registered with the XML template registry"
    );
    assert!(
        anim_out.is_some(),
        "DelvesToastAnimOutTemplate (Blizzard_DelvesToast.xml:8 — virtual=true AnimationGroup \
         inheriting DefaultAnimOutMixin with a 1->0 Alpha animation over 1.5s after a 4s \
         start delay; OnFinished method='OnFinished'; parentKey=waitAndAnimOut) should be \
         registered with the XML template registry"
    );
}
}

#[test]
fn blizzard_delves_toast_required_event_and_enum_are_available() {
    assert!(
        wow_ui_sim::event::is_registerable_event("DELVE_ASSIST_ACTION"),
        "DELVE_ASSIST_ACTION should be a registerable event (src/event/valid_events_a.rs:561) \
         — fired by the client when a delve assist-action notification needs to surface"
    );

    let env = WowLuaEnv::new().expect("Failed to create Lua environment");
    let enum_present: bool = env
        .eval(
            "return type(Enum) == 'table' \
                and type(Enum.AssistActionType) == 'table' \
                and Enum.AssistActionType.PlacedVo ~= nil \
                and Enum.AssistActionType.PlayerSlayer ~= nil \
                and Enum.AssistActionType.CapturedBuff ~= nil",
        )
        .expect("Enum.AssistActionType query should succeed");
    assert!(
        enum_present,
        "Enum.AssistActionType should be defined (src/lua_api/globals/enum_data/\
         missing_enums.lua:662) so DelvesToastMixin.OnEvent's GetFormattedMessage can branch \
         on PlacedVo / PlayerSlayer / CapturedBuff variants without raising a nil-index error"
    );
}
