#![cfg(feature = "client-retail")]
use std::path::PathBuf;

use wow_ui_sim::loader::{discover_blizzard_addons_for_screen, find_toc_file, load_addon};
use wow_ui_sim::lua_api::WowLuaEnv;
use wow_ui_sim::paths::default_blizzard_ui_addons_path;
use wow_ui_sim::screen::ScreenKind;
use wow_ui_sim::startup::fire_startup_events_for_screen;
use wow_ui_sim::toc::TocFile;

fn blizzard_ui_dir() -> PathBuf {
    default_blizzard_ui_addons_path().expect("Blizzard UI cache should be available")
}

fn game_menu_dir() -> PathBuf {
    blizzard_ui_dir().join("Blizzard_GameMenu")
}

fn game_menu_mainline_toc() -> PathBuf {
    game_menu_dir().join("Blizzard_GameMenu_Mainline.toc")
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
fn blizzard_game_menu_picks_mainline_toc_variant() {
    let resolved = find_toc_file(&game_menu_dir())
        .expect("Blizzard_GameMenu directory must contain a discoverable TOC");
    let resolved_name = resolved
        .file_name()
        .expect("resolved TOC must have a filename")
        .to_str()
        .expect("resolved TOC filename must be utf-8");

    assert_eq!(
        resolved_name, "Blizzard_GameMenu_Mainline.toc",
        "Blizzard_GameMenu ships TWO TOC files (Blizzard_GameMenu_Mainline.toc and \
         Blizzard_GameMenu_WoWLabs.toc) and `find_toc_file` (src/loader/mod.rs:65) \
         prefers the `_Mainline.toc` variant — the WoWLabs variant is the \
         plunderstorm-only build and is reachable only by direct path"
    );
}

#[test]
fn blizzard_game_menu_mainline_toc_declares_current_dependencies() {
    let toc = TocFile::from_file(&game_menu_mainline_toc())
        .expect("Blizzard_GameMenu Mainline TOC parse");

    assert!(
        !toc.is_load_on_demand(),
        "Blizzard_GameMenu has no `## LoadOnDemand` line — the Esc-key game menu must \
         be eagerly loaded so its UIPanel.FrameHidden / Store.FrameHidden EventRegistry \
         hooks are wired up before the user presses Escape"
    );
    assert!(
        !toc.is_secure_env(),
        "Blizzard_GameMenu does not declare `## UseSecureEnvironment` — the menu runs \
         in the standard taint environment (Logout / Quit are protected by separate \
         StaticPopup confirmations, not by secure-env)"
    );
    assert!(
        !toc.is_game_type_restricted(),
        "Blizzard_GameMenu_Mainline.toc declares no top-level `## AllowLoadGameType` \
         line — per-line bracketed `[AllowLoadGameType standard, wowhack]` annotations \
         gate the Shared/* files individually instead, so `is_game_type_restricted()` \
         returns false at the addon level"
    );
    assert!(
        toc.saved_variables().is_empty(),
        "Blizzard_GameMenu declares no `## SavedVariables` — the menu has no \
         persistable state (the Ratings button visibility is recomputed on every show, \
         not stored)"
    );

    let deps = toc.dependencies();
    assert_eq!(
        deps,
        vec![
            "Blizzard_GameTooltip".to_string(),
            "Blizzard_GameMenuEsc".to_string(),
            "Blizzard_StaticPopup".to_string(),
        ],
        "Blizzard_GameMenu declares its tooltip, Escape-handler, and static-popup \
         dependencies in current retail order. Got: {:?}",
        deps
    );

    let toc_text = std::fs::read_to_string(game_menu_mainline_toc())
        .expect("Blizzard_GameMenu Mainline TOC should read");
    assert!(
        toc_text.contains("## DefaultState: enabled"),
        "Blizzard_GameMenu_Mainline.toc declares `## DefaultState: enabled` — the \
         Esc-key menu is core UI and on by default"
    );
    assert!(
        toc_text.contains("Shared\\GameMenuFrame.lua [AllowLoadGameType standard, wowhack]"),
        "Per-line bracketed `[AllowLoadGameType standard, wowhack]` gates the Shared \
         GameMenuFrame.lua to retail (`standard`) and the WoW Hack/Esports (`wowhack`) \
         game types — Plunderstorm and Classic flavors get a different file"
    );
    assert!(
        toc_text.contains("WoWLabs\\GameMenuFrame.lua [AllowLoadGameType plunderstorm]"),
        "Per-line bracketed `[AllowLoadGameType plunderstorm]` gates the WoWLabs \
         GameMenuFrame.lua to the Plunderstorm seasonal mode only — on standard retail \
         this file is skipped by the per-line filter and the Shared variant loads instead"
    );
    assert!(
        !toc_text.contains("## AllowLoad:"),
        "Blizzard_GameMenu_Mainline.toc declares NO top-level `## AllowLoad:` line — \
         this means `allows_screen` falls through to the default which (per src/toc.rs:311) \
         is Game-only. The Esc-key menu has no glue-screen analogue (login/character-select \
         use their own GlueParent menu surface)"
    );
}

#[test]
fn blizzard_game_menu_defaults_to_game_screen_only() {
    let toc = TocFile::from_file(&game_menu_mainline_toc())
        .expect("Blizzard_GameMenu Mainline TOC parse");

    assert!(
        toc.allows_screen(ScreenKind::Game),
        "Missing `## AllowLoad:` defaults to Game-only (src/toc.rs:311) — Game must be \
         allowed"
    );
    assert!(
        !toc.allows_screen(ScreenKind::Login),
        "Missing `## AllowLoad:` excludes Login — the login glue screen has its own \
         GlueParent menu, not the in-world Esc-key menu"
    );
    assert!(
        !toc.allows_screen(ScreenKind::CharacterSelect),
        "Missing `## AllowLoad:` excludes CharacterSelect for the same reason — the \
         realm-list/character-select glue screen has a separate menu surface"
    );
}

#[test]
fn blizzard_game_menu_auto_loads_on_game_and_skips_login() {
    let game_addons = discover_blizzard_addons_for_screen(&blizzard_ui_dir(), ScreenKind::Game);
    let in_game = game_addons
        .iter()
        .any(|(name, _)| name == "Blizzard_GameMenu");
    assert!(
        in_game,
        "Blizzard_GameMenu has no `## LoadOnDemand` line and defaults to Game-only \
         (no `## AllowLoad:` declaration), so it MUST appear in Game-screen \
         auto-discovery"
    );

    let login_addons = discover_blizzard_addons_for_screen(&blizzard_ui_dir(), ScreenKind::Login);
    let in_login = login_addons
        .iter()
        .any(|(name, _)| name == "Blizzard_GameMenu");
    assert!(
        !in_login,
        "Game-only default excludes Blizzard_GameMenu from Login auto-discovery"
    );
}

prefork_full_ui_case! {
fn blizzard_game_menu_loads_via_full_game_ui_without_errors(env: &WowLuaEnv) {

    let load_errors: Vec<String> = env
        .state()
        .borrow()
        .lua_errors
        .iter()
        .filter(|message| {
            message.contains("GameMenu")
                || message.contains("GameMenuFrame")
                || message.contains("GameMenuFrameMixin")
        })
        .cloned()
        .collect();

    assert!(
        load_errors.is_empty(),
        "Blizzard_GameMenu emitted Lua errors during the full Game-screen load:\n  {}",
        load_errors.join("\n  ")
    );
}
}

prefork_full_ui_case! {
fn blizzard_game_menu_exit_button_requests_quit_after_full_game_load(env: &WowLuaEnv) {

    let result: String = env
        .eval(
            r#"
            GameMenuFrame:Show()

            local quitButton
            for button in GameMenuFrame.buttonPool:EnumerateActive() do
                if button:GetText() == EXIT_GAME then
                    quitButton = button
                    break
                end
            end
            if not quitButton then
                return "missing-quit-button"
            end

            local onclick = quitButton:GetScript("OnClick")
            if type(onclick) ~= "function" then
                return "missing-onclick"
            end

            local ok, err = pcall(function() quitButton:Click() end)
            if not ok then
                return "onclick-error:" .. tostring(err)
            end

            return A_Admin.IsSimulatorExitRequested() and "quit-requested" or "quit-not-requested"
        "#,
        )
        .expect("full-load GameMenu exit button probe should run");

    assert_eq!(result, "quit-requested");
}
}

prefork_full_ui_case! {
fn blizzard_game_menu_is_addon_loaded_returns_true_after_full_game_ui_load(env: &WowLuaEnv) {

    let post_load: bool = env
        .eval("return C_AddOns.IsAddOnLoaded('Blizzard_GameMenu') and true or false")
        .expect("IsAddOnLoaded probe should succeed");
    assert!(
        post_load,
        "After full Game-screen load, IsAddOnLoaded('Blizzard_GameMenu') must return \
         true — auto-discovery picks up the addon (no LoadOnDemand) and \
         `mark_addon_loaded` registers it"
    );
}
}

prefork_full_ui_case! {
fn blizzard_game_menu_publishes_top_level_frame_and_mixin(env: &WowLuaEnv) {

    let surface: (bool, bool, String) = env
        .eval(
            "return type(GameMenuFrame) == 'table', \
                    type(GameMenuFrameMixin) == 'table', \
                    G_GameMenuFrameContextKey",
        )
        .expect("GameMenu surface probe should succeed");
    assert_eq!(
        surface,
        (true, true, "GameMenuFrame".to_string()),
        "GameMenuFrame.xml line 4 declares the named global GameMenuFrame (toplevel \
         DIALOG strata frame parented to UIParent, hidden, inheriting \
         MainMenuFrameTemplate + CallbackRegistrantTemplate, mixed with \
         GameMenuFrameMixin); GameMenuFrame.lua line 4 declares the GameMenuFrameMixin \
         table; line 2 publishes G_GameMenuFrameContextKey = \"GameMenuFrame\" — the \
         context-key string used by Store.FrameHidden / UIPanel.FrameHidden callbacks \
         to recognize that the closing UI panel was the game menu and re-show it"
    );
}
}

prefork_full_ui_case! {
fn blizzard_game_menu_publishes_named_subframes(env: &WowLuaEnv) {

    let subframes: (bool, bool, bool) = env
        .eval(
            "return type(GameMenuFrame.NewOptionsFrame) == 'table', \
                    type(GameMenuFrame.NewExternalEventFrame) == 'table', \
                    type(GameMenuFrame.EditModeNotification) == 'table'",
        )
        .expect("GameMenuFrame subframe probe should succeed");
    assert_eq!(
        subframes,
        (true, true, true),
        "GameMenuFrame.xml lines 18-38 declare three parentKey'd children: \
         `NewOptionsFrame` (NewFeatureLabelTemplate, scale 0.8 — the \"NEW\" badge \
         next to the Options button when CurrentVersionHasNewUnseenSettings()), \
         `NewExternalEventFrame` (same template — the \"NEW\" badge for the \
         GAMEMENU_EXTERNALEVENT button when C_ExternalEventURL.IsNew()), \
         `EditModeNotification` (frameLevel 100, hidden, 21x21 — the orange \
         notification dot drawn over the Edit Mode button when \
         EditModeManagerFrame.Tutorial:HasHelptipsToShow())"
    );
}
}

prefork_full_ui_case! {
fn blizzard_game_menu_uses_dialog_strata_and_main_menu_template(env: &WowLuaEnv) {

    let frame_props: (String, bool, bool) = env
        .eval(
            "return GameMenuFrame:GetFrameStrata(), \
                    GameMenuFrame:GetParent():GetName() == 'UIParent', \
                    GameMenuFrame:IsToplevel() and true or false",
        )
        .expect("GameMenuFrame property probe should succeed");
    assert_eq!(
        frame_props,
        ("DIALOG".to_string(), true, true),
        "GameMenuFrame.xml line 4 declares `frameStrata=\"DIALOG\"` (places the menu \
         above normal UI), `parent=\"UIParent\"` (canonical top-level parenting), and \
         `toplevel=\"true\"` (UIPanelManager-managed; takes input focus when shown). \
         These three properties together make the Esc-key menu a modal-style overlay \
         that blocks click-through to underlying UI"
    );
}
}

prefork_full_ui_case! {
fn blizzard_game_menu_starts_hidden_at_load_time(env: &WowLuaEnv) {

    let visibility: (bool, bool) = env
        .eval(
            "return GameMenuFrame:IsShown() and true or false, \
                    GameMenuFrame:IsVisible() and true or false",
        )
        .expect("GameMenuFrame visibility probe should succeed");
    assert_eq!(
        visibility,
        (false, false),
        "GameMenuFrame.xml line 4 declares `hidden=\"true\"` — the menu must start \
         hidden so it does not block the world view at login. The user opens it via \
         the Esc keybind which calls ShowUIPanel(GameMenuFrame). IsShown() and \
         IsVisible() both report false at load time"
    );
}
}

prefork_full_ui_case! {
fn blizzard_game_menu_publishes_lifecycle_methods_via_mixin(env: &WowLuaEnv) {

    let methods: (bool, bool, bool, bool) = env
        .eval(
            "return type(GameMenuFrameMixin.OnLoad) == 'function', \
                    type(GameMenuFrameMixin.OnShow) == 'function', \
                    type(GameMenuFrameMixin.OnHide) == 'function', \
                    type(GameMenuFrameMixin.OnEvent) == 'function'",
        )
        .expect("GameMenuFrameMixin lifecycle probe should succeed");
    assert_eq!(
        methods,
        (true, true, true, true),
        "GameMenuFrame.lua publishes the four standard XML <Scripts> methods on \
         GameMenuFrameMixin: OnLoad (line 11 — chains to MainMenuFrameMixin.OnLoad and \
         registers the Store.FrameHidden / UIPanel.FrameHidden static event methods), \
         OnShow (line 18 — chains to CallbackRegistrantMixin.OnShow, registers \
         STORE_STATUS_CHANGED / TRIAL_STATUS_UPDATE, calls UpdateMicroButtons + \
         InitButtons + EventRegistry:TriggerEvent('GameMenuFrame.Shown')), OnHide \
         (line 34 — symmetric unregister + UpdateMicroButtons), OnEvent (line 46 — \
         simply re-runs InitButtons since both events just need to refresh Trial / \
         Store button states)"
    );
}
}

prefork_full_ui_case! {
fn blizzard_game_menu_publishes_button_init_and_callback_methods(env: &WowLuaEnv) {

    let menu_methods: (bool, bool, bool) = env
        .eval(
            "return type(GameMenuFrameMixin.InitButtons) == 'function', \
                    type(GameMenuFrameMixin.OnStoreFrameClosed) == 'function', \
                    type(GameMenuFrameMixin.OnUIPanelHidden) == 'function'",
        )
        .expect("GameMenuFrameMixin button method probe should succeed");
    assert_eq!(
        menu_methods,
        (true, true, true),
        "GameMenuFrame.lua publishes the menu-build helpers: InitButtons (line 62 — \
         the central button-list rebuild that calls AddButton for each row, conditionally \
         adds the External Event / Store / Account Rewards / AddOns / Splash / Edit Mode \
         / Ratings / Support / Macros buttons based on Kiosk / GameRulesUtil / \
         CanEnterEditMode flags, then adds Logout + Exit + AddCloseButton at the bottom), \
         OnStoreFrameClosed (line 50 — Store.FrameHidden callback that re-shows the \
         menu when contextKey matches G_GameMenuFrameContextKey), OnUIPanelHidden \
         (line 56 — UIPanel.FrameHidden equivalent for non-Store panels like Settings \
         / AddonList / EditMode / RatingMenu)"
    );
}
}

prefork_full_ui_case! {
fn blizzard_game_menu_publishes_ratings_and_logout_helpers(env: &WowLuaEnv) {

    let ratings_helpers: (bool, bool, bool) = env
        .eval(
            "return type(GameMenuFrameMixin.SetRatingsButtonShown) == 'function', \
                    type(GameMenuFrameMixin.GetRatingsButtonShown) == 'function', \
                    type(GameMenuFrameMixin.GetLogoutText) == 'function'",
        )
        .expect("Ratings/logout helper probe should succeed");
    assert_eq!(
        ratings_helpers,
        (true, true, true),
        "GameMenuFrame.lua publishes the per-locale-overridable helpers: \
         SetRatingsButtonShown (line 164 — sets self.ratingsButtonShown; called from \
         koKR Localization.lua line 11 to enable the Ratings button on Korean clients \
         only), GetRatingsButtonShown (line 168 — read accessor used in InitButtons \
         line 144), GetLogoutText (line 172 — returns LOG_OUT by default; explicitly \
         documented `-- Can be overridden.` because seasonal modes like Plunderstorm \
         override it to return PLUNDERSTORM_LEAVE_MATCH)"
    );

    let close_override: bool = env
        .eval("return type(MainMenuFrameMixin.CloseMenu) == 'function'")
        .expect("MainMenuFrameMixin.CloseMenu probe should succeed");
    assert!(
        close_override,
        "GameMenuFrame.lua line 177 installs `function MainMenuFrameMixin:CloseMenu()` \
         which simply calls HideUIPanel(self) — this is a global override that adds \
         a CloseMenu method to the SHARED MainMenuFrameMixin (not GameMenuFrameMixin), \
         consumed by the Account Rewards / Plunderstorm sub-paths that need to dismiss \
         the menu without exiting the game"
    );
}
}
