use std::path::PathBuf;

use wow_ui_sim::loader::{discover_blizzard_addons_for_screen, find_toc_file};
use wow_ui_sim::lua_api::WowLuaEnv;
use wow_ui_sim::screen::ScreenKind;
use wow_ui_sim::toc::TocFile;

fn blizzard_ui_dir() -> PathBuf {
    wow_ui_sim::paths::default_blizzard_ui_addons_path().expect("Blizzard UI cache should be available")
}

fn shared_widget_frames_dir() -> PathBuf {
    blizzard_ui_dir().join("Blizzard_SharedWidgetFrames")
}

fn shared_widget_frames_toc() -> PathBuf {
    shared_widget_frames_dir().join("Blizzard_SharedWidgetFrames.toc")
}

const PUBLIC_MIXINS: &[&str] = &[
    "WidgetCenterDisplayFrameMixin",
    "UIWidgetCenterDisplayFrameButtonMixin",
    "UIWidgetCenterDisplayFrameExtraButtonMixin",
];

#[test]
fn find_toc_file_resolves_bare_toc() {
    let resolved =
        find_toc_file(&shared_widget_frames_dir()).expect("SharedWidgetFrames TOC resolves");
    assert_eq!(
        resolved,
        shared_widget_frames_toc(),
        "Blizzard_SharedWidgetFrames ships exactly one bare \
         `Blizzard_SharedWidgetFrames.toc` — no flavor variants. The \
         centered-widget-display dialog uses GENERIC_WIDGET_DISPLAY_SHOW + \
         C_GenericWidgetDisplay, both cross-flavor APIs. The single dialog \
         frame this addon adds (UIWidgetCenterDisplayFrame) reuses retail and \
         classic widget primitives, so one TOC suffices"
    );
}

#[test]
fn toc_declares_explicit_load_on_demand_zero_with_required_dep_only() {
    let toc = TocFile::from_file(&shared_widget_frames_toc()).expect("TOC parses");

    assert!(
        !toc.is_load_on_demand(),
        "TOC must declare `## LoadOnDemand: 0` (explicit zero, semantically \
         equivalent to absent). Eager-load is intentional: \
         UIWidgetCenterDisplayFrame.lua's OnLoad calls \
         `self:RegisterEvent(\"GENERIC_WIDGET_DISPLAY_SHOW\")`, and the event \
         can fire from the server at any moment after login. Lazy-loading \
         this addon would mean the first GENERIC_WIDGET_DISPLAY_SHOW event \
         arrives before any frame is registered to receive it"
    );
    assert!(!toc.is_load_first());
    assert!(!toc.is_secure_env());
    assert!(!toc.is_glue_only());

    let deps = toc.dependencies();
    assert_eq!(
        deps,
        ["Blizzard_UIWidgets"],
        "TOC must declare exactly one hard dep via `## RequiredDep: \
         Blizzard_UIWidgets`. The XML inherits `UIWidgetContainerTemplate` \
         (lives in Blizzard_UIWidgets), and the .lua references \
         `DefaultWidgetLayout`, `GetFinalNameFromTextureKit`, \
         `TextureKitConstants`, and `C_GenericWidgetDisplay` — all published \
         by Blizzard_UIWidgets. Without the dep, NineSlicePanelTemplate / \
         UIWidgetContainerTemplate inheritance would fail at XML parse time. \
         Got: {deps:?}"
    );
    assert!(toc.optional_deps().is_empty());
    assert!(toc.saved_variables().is_empty());
    assert!(toc.saved_variables_per_character().is_empty());
}

#[test]
fn toc_lacks_allow_load_so_falls_through_to_game_only() {
    let toc = TocFile::from_file(&shared_widget_frames_toc()).expect("TOC parses");

    assert!(
        toc.allows_screen(ScreenKind::Game),
        "Without `## AllowLoad`, src/toc.rs None arm restricts the addon to \
         the Game screen. Generic widget displays are server-driven popups \
         (campaign-completion, generic notifications) that have no glue \
         representation"
    );

    for screen in [
        ScreenKind::Login,
        ScreenKind::CharacterSelect,
        ScreenKind::CharacterCreate,
    ] {
        assert!(
            !toc.allows_screen(screen),
            "Glue screen {screen:?} must NOT be allowed — \
             GENERIC_WIDGET_DISPLAY_SHOW only fires in-game; no glue server \
             flow ever sends widget-display payloads"
        );
    }
}

#[test]
fn toc_raw_bytes_pin_three_metadata_lines_and_no_extra_directives() {
    let raw = std::fs::read_to_string(shared_widget_frames_toc()).expect("TOC reads utf-8");

    assert!(raw.contains("## Title: Blizzard_SharedWidgetFrames"));
    assert!(raw.contains("## LoadOnDemand: 0"));
    assert!(raw.contains("## RequiredDep: Blizzard_UIWidgets"));

    assert!(
        !raw.contains("## Dependencies"),
        "TOC must use `## RequiredDep` (singular) NOT `## Dependencies` — \
         RequiredDep is the older form used by some legacy addons. The \
         simulator's `dependencies()` accessor honors both forms (see \
         src/toc.rs `dependencies()` fallthrough); this addon uses the \
         RequiredDep spelling exclusively"
    );
    assert!(!raw.contains("## OptionalDep"));
    assert!(!raw.contains("## AllowLoad"));
    assert!(!raw.contains("## AllowLoadGameType"));
    assert!(!raw.contains("## SavedVariables"));
    assert!(!raw.contains("## SavedVariablesPerCharacter"));
    assert!(!raw.contains("## DefaultState"));
    assert!(!raw.contains("## Author"));
    assert!(!raw.contains("## Version"));
}

#[test]
fn toc_body_lists_lua_then_xml_pair_for_single_dialog_frame() {
    let raw = std::fs::read_to_string(shared_widget_frames_toc()).expect("TOC reads utf-8");

    let body_lines: Vec<&str> = raw
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            !trimmed.is_empty() && !trimmed.starts_with('#')
        })
        .collect();

    assert_eq!(
        body_lines,
        vec![
            "Blizzard_UIWidgetCenterDisplayFrame.lua",
            "Blizzard_UIWidgetCenterDisplayFrame.xml",
        ],
        "TOC body MUST list exactly 2 entries, .lua before .xml. Lua-first \
         loads WidgetCenterDisplayFrameMixin / \
         UIWidgetCenterDisplayFrameButtonMixin / \
         UIWidgetCenterDisplayFrameExtraButtonMixin into `_G` so the XML \
         `mixin=\"...\"` attributes resolve at parse time. Reversing the \
         order would leave the Frame's mixin reference unresolved (XML \
         parser sees a nil mixin and the OnLoad / OnEvent / OnHide handlers \
         never bind). Got: {body_lines:?}"
    );
}

#[test]
fn root_directory_holds_only_toc_lua_and_xml() {
    let dir = shared_widget_frames_dir();
    assert!(dir.join("Blizzard_SharedWidgetFrames.toc").is_file());
    assert!(
        dir.join("Blizzard_UIWidgetCenterDisplayFrame.lua")
            .is_file()
    );
    assert!(
        dir.join("Blizzard_UIWidgetCenterDisplayFrame.xml")
            .is_file()
    );

    let entries: Vec<String> = std::fs::read_dir(&dir)
        .expect("read addon dir")
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(
        entries.len(),
        3,
        "Blizzard_SharedWidgetFrames directory must contain exactly 3 \
         entries (1 toc + 1 lua + 1 xml) — no helpers, no nested \
         directories, no extra mixin files. The addon's only purpose is to \
         host the single UIWidgetCenterDisplayFrame dialog. Got: {entries:?}"
    );
}

#[test]
fn eager_discovery_includes_addon_on_game_screen_only() {
    let ui = blizzard_ui_dir();

    let game_addons = discover_blizzard_addons_for_screen(&ui, ScreenKind::Game);
    let game_found = game_addons
        .iter()
        .any(|(name, _)| name == "Blizzard_SharedWidgetFrames");
    assert!(
        game_found,
        "Blizzard_SharedWidgetFrames MUST appear in Game-screen eager \
         discovery — `## LoadOnDemand: 0` keeps it eager so the dialog frame \
         exists before the first GENERIC_WIDGET_DISPLAY_SHOW event arrives"
    );

    for screen in [
        ScreenKind::Login,
        ScreenKind::CharacterSelect,
        ScreenKind::CharacterCreate,
    ] {
        let addons = discover_blizzard_addons_for_screen(&ui, screen);
        let found = addons
            .iter()
            .any(|(name, _)| name == "Blizzard_SharedWidgetFrames");
        assert!(
            !found,
            "Blizzard_SharedWidgetFrames must be excluded from eager \
             discovery on {screen:?} — Game-only fallthrough applies, and no \
             glue-screen addon hard-deps on it"
        );
    }
}

prefork_full_ui_case! {
    fn full_game_load_emits_no_addon_specific_lua_errors(env: &WowLuaEnv) {

        let errors: Vec<String> = env.state().borrow().lua_errors.clone();
        let relevant: Vec<&String> = errors
            .iter()
            .filter(|e| {
                e.contains("Blizzard_SharedWidgetFrames")
                    || e.contains("UIWidgetCenterDisplayFrame")
                    || e.contains("WidgetCenterDisplayFrameMixin")
            })
            .collect();
        assert!(
            relevant.is_empty(),
            "Eager load via full Game UI discovery must emit zero addon-specific \
             Lua errors. Got:\n  {}",
            relevant
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join("\n  ")
        );
    }
}

prefork_full_ui_case! {
    fn is_addon_loaded_returns_true_after_full_game_discovery(env: &WowLuaEnv) {

        let loaded: bool = env
            .eval("return C_AddOns.IsAddOnLoaded('Blizzard_SharedWidgetFrames')")
            .expect("IsAddOnLoaded probe");
        assert!(
            loaded,
            "C_AddOns.IsAddOnLoaded('Blizzard_SharedWidgetFrames') must be true \
             after eager discovery — the LoadOnDemand=0 directive guarantees \
             immediate inclusion when the addon's RequiredDep (Blizzard_UIWidgets) \
             is itself eager"
        );

        let dep_loaded: bool = env
            .eval("return C_AddOns.IsAddOnLoaded('Blizzard_UIWidgets')")
            .expect("Blizzard_UIWidgets IsAddOnLoaded probe");
        assert!(
            dep_loaded,
            "Blizzard_UIWidgets must be loaded — it's the single RequiredDep, \
             and UIWidgetContainerTemplate / DefaultWidgetLayout would fail to \
             resolve without it"
        );
    }
}

prefork_full_ui_case! {
    fn publishes_three_mixin_tables_for_dialog_and_buttons(env: &WowLuaEnv) {

        for mixin in PUBLIC_MIXINS {
            let kind: String = env
                .eval(&format!("return type({mixin})"))
                .unwrap_or_else(|_| panic!("{mixin} probe failed"));
            assert_eq!(
                kind, "table",
                "_G.{mixin} must be a table — XML `mixin=\"{mixin}\"` references \
                 must resolve at parse time. WidgetCenterDisplayFrameMixin lives \
                 on the dialog itself, the two button mixins live on the close / \
                 extra-action buttons inside the dialog"
            );
        }
    }
}

prefork_full_ui_case! {
    fn widget_center_display_frame_mixin_publishes_lifecycle_methods(env: &WowLuaEnv) {

        for method in ["OnLoad", "OnEvent", "OnHide", "Setup", "SetupButtons"] {
            let kind: String = env
                .eval(&format!(
                    "return type(WidgetCenterDisplayFrameMixin.{method})"
                ))
                .unwrap_or_else(|_| panic!("{method} probe failed"));
            assert_eq!(
                kind, "function",
                "WidgetCenterDisplayFrameMixin.{method} must be a function. \
                 OnLoad registers GENERIC_WIDGET_DISPLAY_SHOW; OnEvent dispatches \
                 to Setup; OnHide unregisters the widget set; Setup binds \
                 displayInfo to the dialog (title, atlas background, widget \
                 container, buttons); SetupButtons positions the close / extra \
                 action buttons based on which texts are non-empty"
            );
        }
    }
}

prefork_full_ui_case! {
    fn ui_widget_center_display_frame_button_mixin_on_click_calls_close(env: &WowLuaEnv) {

        let kind: String = env
            .eval("return type(UIWidgetCenterDisplayFrameButtonMixin.OnClick)")
            .expect("OnClick probe");
        assert_eq!(
            kind, "function",
            "UIWidgetCenterDisplayFrameButtonMixin.OnClick must be a function — \
             hides the parent dialog and calls C_GenericWidgetDisplay.Close(). \
             Without OnClick the close button would visually depress but not \
             dismiss the dialog (and the server would never get the close \
             signal)"
        );
    }
}

prefork_full_ui_case! {
    fn ui_widget_center_display_frame_extra_button_mixin_on_click_acknowledges(env: &WowLuaEnv) {

        let kind: String = env
            .eval("return type(UIWidgetCenterDisplayFrameExtraButtonMixin.OnClick)")
            .expect("ExtraButton OnClick probe");
        assert_eq!(
            kind, "function",
            "UIWidgetCenterDisplayFrameExtraButtonMixin.OnClick must be a \
             function — calls C_GenericWidgetDisplay.Acknowledge() (NOT Close). \
             The extra button is the affirmative action when the server attaches \
             a non-empty extraButtonText, and the server distinguishes \
             Acknowledge (took the extra action) from Close (dismissed) for \
             analytics + state tracking"
        );
    }
}

prefork_full_ui_case! {
    fn ui_widget_center_display_frame_published_as_dialog_strata_hidden_default(env: &WowLuaEnv) {

        let kind: String = env
            .eval("return type(UIWidgetCenterDisplayFrame)")
            .expect("UIWidgetCenterDisplayFrame frame probe");
        assert!(
            kind == "table" || kind == "userdata",
            "_G.UIWidgetCenterDisplayFrame must be a Frame (table-or-userdata) — \
             the XML at line 3 declares `<Frame name=\"UIWidgetCenterDisplayFrame\" \
             parent=\"UIParent\" frameStrata=\"DIALOG\" hidden=\"true\">`. Without \
             a global frame, the GENERIC_WIDGET_DISPLAY_SHOW event would have no \
             registered receiver. Got: {kind}"
        );

        let visible: bool = env
            .eval("return UIWidgetCenterDisplayFrame:IsShown()")
            .expect("IsShown probe");
        assert!(
            !visible,
            "UIWidgetCenterDisplayFrame must be hidden at load — `hidden=\"true\"` \
             in the XML. The dialog only shows in response to a server-driven \
             GENERIC_WIDGET_DISPLAY_SHOW event with valid displayInfo. Auto-show \
             on login would be a focus-stealing UX bug"
        );

        let strata: String = env
            .eval("return UIWidgetCenterDisplayFrame:GetFrameStrata()")
            .expect("GetFrameStrata probe");
        assert_eq!(
            strata, "DIALOG",
            "UIWidgetCenterDisplayFrame must be on DIALOG strata — XML attr \
             `frameStrata=\"DIALOG\"`. DIALOG is below FULLSCREEN_DIALOG and \
             TOOLTIP, so completion-dialog popups can layer over MEDIUM-strata \
             gameplay UI but tooltips on top stay readable"
        );
    }
}

prefork_full_ui_case! {
    fn ui_widget_center_display_frame_has_expected_subframe_children(env: &WowLuaEnv) {

        for child_key in [
            "Background",
            "NineSlice",
            "TitleContainer",
            "WidgetContainer",
        ] {
            let kind: String = env
                .eval(&format!(
                    "return type(UIWidgetCenterDisplayFrame.{child_key})"
                ))
                .unwrap_or_else(|_| panic!("{child_key} probe failed"));
            assert!(
                kind == "table" || kind == "userdata",
                "UIWidgetCenterDisplayFrame.{child_key} must exist (parentKey \
                 child). Background is the BACKGROUND-layer Texture filling the \
                 frame at 80% black; NineSlice is the bordered panel (5px \
                 outset); TitleContainer holds the Game36Font_Shadow2 title; \
                 WidgetContainer hosts the inner widget-set frames registered \
                 via UIWidgetContainerTemplate"
            );
        }

        for button_key in ["ExtraButton", "CloseButton"] {
            let kind: String = env
                .eval(&format!(
                    "return type(UIWidgetCenterDisplayFrame.{button_key})"
                ))
                .unwrap_or_else(|_| panic!("{button_key} probe failed"));
            assert!(
                kind == "table" || kind == "userdata",
                "UIWidgetCenterDisplayFrame.{button_key} must exist as a \
                 100x25 UIPanelButtonTemplate child. CloseButton is always shown \
                 (defaults to CLOSE), ExtraButton is hidden by default and \
                 shown only when displayInfo carries non-empty extraButtonText"
            );
        }
    }
}

prefork_full_ui_case! {
    fn extra_button_starts_hidden_close_button_starts_visible(env: &WowLuaEnv) {

        let extra_visible: bool = env
            .eval("return UIWidgetCenterDisplayFrame.ExtraButton:IsShown()")
            .expect("ExtraButton IsShown probe");
        assert!(
            !extra_visible,
            "ExtraButton must be hidden at load — XML attr `hidden=\"true\"`. \
             Most generic widget displays are pure-info popups with only a \
             close button; the extra button only surfaces for displays that \
             offer an affirmative action"
        );

        let close_visible: bool = env
            .eval("return UIWidgetCenterDisplayFrame.CloseButton:IsShown()")
            .expect("CloseButton IsShown probe");
        assert!(
            close_visible,
            "CloseButton must be visible at load (no hidden attr in XML). The \
             close button is the universal escape hatch — every generic widget \
             display has one"
        );
    }
}
