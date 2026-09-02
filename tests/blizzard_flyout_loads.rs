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

fn flyout_toc() -> PathBuf {
    blizzard_ui_dir().join("Blizzard_Flyout/Blizzard_Flyout.toc")
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
fn blizzard_flyout_toc_is_load_first_with_shared_xml_base_dep_and_allow_load_both() {
    let toc = TocFile::from_file(&flyout_toc()).expect("Blizzard_Flyout TOC should parse");

    assert!(
        !toc.is_load_on_demand(),
        "Blizzard_Flyout has no `## LoadOnDemand` line — the flyout templates \
         (FlyoutButtonTemplate / FlyoutPopupTemplate / FlyoutPopupButtonTemplate) are \
         consumed by many other addons (action bars, character panel spec selector, \
         class trainer abilities, etc.) so they must be available at startup before \
         dependent addons load"
    );
    assert!(
        !toc.is_secure_env(),
        "Blizzard_Flyout does not declare `## UseSecureEnvironment` — the flyout system \
         is a generic UI primitive that runs in the standard taint environment"
    );
    assert_eq!(
        toc.dependencies(),
        vec!["Blizzard_SharedXMLBase".to_string()],
        "Blizzard_Flyout declares `## Dependencies: Blizzard_SharedXMLBase` — the single \
         dependency provides `ButtonStateBehaviorMixin` \
         (Blizzard_SharedXMLBase/ButtonStateBehavior.lua:2) which FlyoutButtonMixin \
         extends via `CreateFromMixins(ButtonStateBehaviorMixin)` (Flyout.lua:7)"
    );
    assert!(
        !toc.is_game_type_restricted(),
        "Blizzard_Flyout declares no `## AllowLoadGameType:` line, so \
         `is_game_type_restricted()` returns false and the addon is reachable from \
         standard-retail discovery"
    );

    assert!(
        toc.saved_variables().is_empty(),
        "Blizzard_Flyout declares no `## SavedVariables` — the flyout templates are \
         pure UI primitives with no persistent state (open/closed state lives on the \
         FlyoutButton instance via `self.popup` and is rebuilt on every login)"
    );

    let toc_text = std::fs::read_to_string(flyout_toc()).expect("Blizzard_Flyout TOC should read");
    assert!(
        toc_text.contains("## LoadFirst: 1"),
        "Blizzard_Flyout declares `## LoadFirst: 1` — the loader gives this addon \
         priority within its dependency tier so the templates are registered before \
         other addons in the same tier try to consume them"
    );
    assert!(
        toc_text.contains("## DefaultState: enabled"),
        "Blizzard_Flyout declares `## DefaultState: enabled` — the addon is enabled by \
         default in fresh user profiles (the user CAN disable it via the Addons UI but \
         doing so would break every consumer of FlyoutButtonTemplate)"
    );
    assert!(
        toc_text.contains("## AllowLoad: Both"),
        "Blizzard_Flyout declares `## AllowLoad: Both` — flyout templates are also used \
         on glue screens (e.g. character-create class flyout selectors), so the addon \
         must auto-load on Login and CharacterSelect in addition to Game"
    );
}

#[test]
fn blizzard_flyout_allows_all_screens_including_glue() {
    let toc = TocFile::from_file(&flyout_toc()).expect("Blizzard_Flyout TOC should parse");

    assert!(
        toc.allows_screen(ScreenKind::Game),
        "`## AllowLoad: Both` must allow the Game screen (src/toc.rs:307)"
    );
    assert!(
        toc.allows_screen(ScreenKind::Login),
        "`## AllowLoad: Both` must allow the Login screen — distinguishes this addon \
         from the Game-only default at src/toc.rs:311"
    );
    assert!(
        toc.allows_screen(ScreenKind::CharacterSelect),
        "`## AllowLoad: Both` must allow CharacterSelect — `is_glue()` covers all glue \
         screens so the character-create class flyouts get the templates"
    );
}

#[test]
fn blizzard_flyout_auto_loads_on_game_and_login_screens() {
    let game_addons = discover_blizzard_addons_for_screen(&blizzard_ui_dir(), ScreenKind::Game);
    let in_game = game_addons
        .iter()
        .any(|(name, _)| name == "Blizzard_Flyout");
    assert!(
        in_game,
        "Blizzard_Flyout has no `## LoadOnDemand` line and `## AllowLoad: Both`, so it \
         MUST appear in Game-screen auto-discovery — its templates are foundational and \
         must be available before consumers load"
    );

    let login_addons = discover_blizzard_addons_for_screen(&blizzard_ui_dir(), ScreenKind::Login);
    let in_login = login_addons
        .iter()
        .any(|(name, _)| name == "Blizzard_Flyout");
    assert!(
        in_login,
        "`## AllowLoad: Both` plus no LoadOnDemand means Blizzard_Flyout MUST appear in \
         Login-screen auto-discovery as well — character-create flyout selectors need it"
    );
}

prefork_full_ui_case! {
fn blizzard_flyout_loads_via_full_game_ui_without_errors(env: &WowLuaEnv) {

    let load_errors: Vec<String> = env
        .state()
        .borrow()
        .lua_errors
        .iter()
        .filter(|message| message.contains("Flyout") || message.contains("Blizzard_Flyout"))
        .cloned()
        .collect();

    assert!(
        load_errors.is_empty(),
        "Blizzard_Flyout emitted Lua errors during the full Game-screen load:\n  {}",
        load_errors.join("\n  ")
    );
}
}

prefork_full_ui_case! {
fn blizzard_flyout_is_addon_loaded_returns_true_after_full_game_ui_load(env: &WowLuaEnv) {

    let post_load: bool = env
        .eval("return C_AddOns.IsAddOnLoaded('Blizzard_Flyout') and true or false")
        .expect("IsAddOnLoaded probe should succeed");
    assert!(
        post_load,
        "After full Game-screen load, IsAddOnLoaded('Blizzard_Flyout') must return true \
         — auto-discovery picks up the addon (no LoadOnDemand) and `mark_addon_loaded` \
         (src/loader/addon.rs:131) registers it"
    );
}
}

prefork_full_ui_case! {
fn blizzard_flyout_publishes_three_template_mixin_globals(env: &WowLuaEnv) {

    let mixins_present: (bool, bool, bool) = env
        .eval(
            "return type(FlyoutButtonMixin) == 'table', \
                    type(FlyoutPopupMixin) == 'table', \
                    type(FlyoutPopupButtonMixin) == 'table'",
        )
        .expect("flyout mixin probe should succeed");
    assert_eq!(
        mixins_present,
        (true, true, true),
        "Blizzard_Flyout/Flyout.lua publishes three mixin globals: \
         `FlyoutButtonMixin = CreateFromMixins(ButtonStateBehaviorMixin)` (lua:7 — the \
         toggle button that opens/closes the popup, with arrow-direction state \
         management), `FlyoutPopupMixin = {{}}` (lua:207 — the popup container that \
         attaches to a flyout button, hosts FlyoutPopupButton children, and renders the \
         9-slice background nine textures), `FlyoutPopupButtonMixin = {{}}` (lua:352 — \
         the per-row click target inside a popup that auto-closes the popup on click). \
         The popup direction state machine (UP/DOWN/LEFT/RIGHT) drives both arrow \
         rotation and texture anchor swaps in UpdateBackground"
    );
}
}

prefork_full_ui_case! {
fn blizzard_flyout_button_mixin_inherits_button_state_behavior_methods(env: &WowLuaEnv) {

    let inherited_methods: (bool, bool, bool, bool) = env
        .eval(
            "return type(FlyoutButtonMixin.OnEnter) == 'function', \
                    type(FlyoutButtonMixin.OnLeave) == 'function', \
                    type(FlyoutButtonMixin.OnMouseDown) == 'function', \
                    type(FlyoutButtonMixin.OnMouseUp) == 'function'",
        )
        .expect("ButtonStateBehaviorMixin inheritance probe should succeed");
    assert_eq!(
        inherited_methods,
        (true, true, true, true),
        "FlyoutButtonMixin is `CreateFromMixins(ButtonStateBehaviorMixin)` and \
         additionally defines its own OnEnter/OnLeave/OnMouseDown/OnMouseUp wrappers \
         (Flyout.lua:27-41) that explicitly delegate to \
         `ButtonStateBehaviorMixin.OnEnter(self)` etc. The wrappers exist so the XML \
         script bindings (`<OnEnter method=\"OnEnter\"/>` etc.) point at FlyoutButton's \
         own table instead of the parent — confirms the parent mixin's methods are \
         present on FlyoutButtonMixin via the merge"
    );
}
}

prefork_full_ui_case! {
fn blizzard_flyout_button_mixin_publishes_popup_state_machine_methods(env: &WowLuaEnv) {

    let methods_present: (bool, bool, bool, bool, bool, bool, bool) = env
        .eval(
            "return type(FlyoutButtonMixin.SetPopup) == 'function', \
                    type(FlyoutButtonMixin.ClearPopup) == 'function', \
                    type(FlyoutButtonMixin.GetPopup) == 'function', \
                    type(FlyoutButtonMixin.HasPopup) == 'function', \
                    type(FlyoutButtonMixin.IsPopupOpen) == 'function', \
                    type(FlyoutButtonMixin.TogglePopup) == 'function', \
                    type(FlyoutButtonMixin.ClosePopup) == 'function'",
        )
        .expect("FlyoutButtonMixin popup-state probe should succeed");
    assert_eq!(
        methods_present,
        (true, true, true, true, true, true, true),
        "FlyoutButtonMixin publishes the popup-state-machine API: SetPopup (lua:48 — \
         attach a FlyoutPopupTemplate-derived popup), ClearPopup (lua:60 — close + \
         detach), GetPopup / HasPopup (lua:72/76 — accessors), IsPopupOpen (lua:94 — \
         checks `popup:IsAttachedToButton(self)` round-trip), TogglePopup (lua:99 — \
         the central toggle: registers/unregisters EventRegistry callback for \
         FlyoutPopupEvent.Hidden, calls `popup:AttachToButton(self)` or \
         `popup:DetatchFromButton()`, then OnPopupToggled), ClosePopup (lua:112 — \
         no-op if not open, otherwise toggles)"
    );
}
}

prefork_full_ui_case! {
fn blizzard_flyout_button_mixin_publishes_arrow_state_machine_methods(env: &WowLuaEnv) {

    let methods_present: (bool, bool, bool, bool, bool) = env
        .eval(
            "return type(FlyoutButtonMixin.UpdateArrowShown) == 'function', \
                    type(FlyoutButtonMixin.UpdateArrowPosition) == 'function', \
                    type(FlyoutButtonMixin.UpdateArrowRotation) == 'function', \
                    type(FlyoutButtonMixin.UpdateArrowTexture) == 'function', \
                    type(FlyoutButtonMixin.GetArrowRotation) == 'function'",
        )
        .expect("FlyoutButtonMixin arrow-state probe should succeed");
    assert_eq!(
        methods_present,
        (true, true, true, true, true),
        "FlyoutButtonMixin publishes the arrow-rendering API that responds to popup \
         direction + open/closed state: UpdateArrowShown (lua:134 — gates on \
         `HasPopup()`), UpdateArrowPosition (lua:139 — anchors Arrow to TOP/BOTTOM/\
         LEFT/RIGHT depending on popupDirection, with offset switching between \
         openArrowOffset and closedArrowOffset), UpdateArrowRotation (lua:178 — calls \
         SetClampedTextureRotation), UpdateArrowTexture (lua:183 — switches between \
         arrowDownTexture/arrowOverTexture/arrowNormalTexture KeyValues based on \
         IsDown/IsOver), GetArrowRotation (lua:156 — direction → degrees + 180° flip \
         when popup is open)"
    );
}
}

prefork_full_ui_case! {
fn blizzard_flyout_popup_mixin_publishes_attach_detach_and_layout_methods(env: &WowLuaEnv) {

    let methods_present: (bool, bool, bool, bool, bool, bool, bool, bool) = env
        .eval(
            "return type(FlyoutPopupMixin.IsAttachedToButton) == 'function', \
                    type(FlyoutPopupMixin.AttachToButton) == 'function', \
                    type(FlyoutPopupMixin.DetatchFromButton) == 'function', \
                    type(FlyoutPopupMixin.Close) == 'function', \
                    type(FlyoutPopupMixin.GetDirection) == 'function', \
                    type(FlyoutPopupMixin.IsHorizontal) == 'function', \
                    type(FlyoutPopupMixin.UpdatePosition) == 'function', \
                    type(FlyoutPopupMixin.UpdateBackground) == 'function'",
        )
        .expect("FlyoutPopupMixin probe should succeed");
    assert_eq!(
        methods_present,
        (true, true, true, true, true, true, true, true),
        "FlyoutPopupMixin (lua:207) publishes the popup lifecycle + layout API: \
         IsAttachedToButton (lua:209), AttachToButton (lua:213 — SetParent + \
         UpdatePosition + UpdateBackground + Show + propagate SetPopup to children that \
         expose the method, i.e. FlyoutPopupButton-derived rows), DetatchFromButton \
         (lua:230 — Hide + clear flyoutButton; note Blizzard's typo `Detatch` not \
         `Detach`), Close (lua:238), GetDirection (lua:244 — proxies to button's \
         popupDirection), IsHorizontal (lua:248 — LEFT/RIGHT), UpdatePosition (lua:257 \
         — anchors popup relative to button via direction), UpdateBackground (lua:274 \
         — the big direction state machine that swaps Start/HorizontalMiddle/\
         VerticalMiddle/End anchors AND rotations to render the four-piece tiled \
         background in the correct orientation)"
    );
}
}

prefork_full_ui_case! {
fn blizzard_flyout_popup_button_mixin_publishes_set_popup_and_close_methods(env: &WowLuaEnv) {

    let methods_present: (bool, bool, bool, bool) = env
        .eval(
            "return type(FlyoutPopupButtonMixin.SetPopup) == 'function', \
                    type(FlyoutPopupButtonMixin.GetPopup) == 'function', \
                    type(FlyoutPopupButtonMixin.ClosePopup) == 'function', \
                    type(FlyoutPopupButtonMixin.OnClick) == 'function'",
        )
        .expect("FlyoutPopupButtonMixin probe should succeed");
    assert_eq!(
        methods_present,
        (true, true, true, true),
        "FlyoutPopupButtonMixin (lua:352) is the per-row mixin used for buttons \
         INSIDE a popup. Publishes four methods: SetPopup (lua:354 — assigns the \
         parent popup, with assertsafe guard against re-assignment), GetPopup \
         (lua:359), ClosePopup (lua:363 — proxies to `popup:Close()`), OnClick \
         (lua:369 — auto-closes the popup when the row is clicked, wired by \
         `<OnClick method=\"OnClick\"/>` at Flyout.xml:64). Derived mixins must call \
         `FlyoutPopupButton_OnClick` (per the lua:350 comment) to keep the \
         click-to-close contract"
    );
}
}
