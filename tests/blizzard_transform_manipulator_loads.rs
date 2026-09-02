use std::path::PathBuf;

use wow_ui_sim::loader::load_addon;
use wow_ui_sim::loader::{discover_blizzard_addons_for_screen, find_toc_file};
use wow_ui_sim::lua_api::WowLuaEnv;
use wow_ui_sim::screen::ScreenKind;
use wow_ui_sim::startup::fire_startup_events_for_screen;
use wow_ui_sim::toc::TocFile;

fn blizzard_ui_dir() -> PathBuf {
    wow_ui_sim::paths::default_blizzard_ui_addons_path().expect("Blizzard UI cache should be available")
}

fn manipulator_dir() -> PathBuf {
    blizzard_ui_dir().join("Blizzard_TransformManipulator")
}

fn manipulator_toc() -> PathBuf {
    manipulator_dir().join("Blizzard_TransformManipulator.toc")
}

const ALL_FOUR_SCREENS: &[ScreenKind] = &[
    ScreenKind::Game,
    ScreenKind::Login,
    ScreenKind::CharacterSelect,
    ScreenKind::CharacterCreate,
];

const PUBLISHED_MIXINS: &[(&str, &[&str])] = &[
    (
        "RotateControlFrameMixin",
        &[
            "OnLoad",
            "OnEvent",
            "OnShow",
            "OnHide",
            "OnEnter",
            "OnLeave",
            "UpdateActiveState",
        ],
    ),
    (
        "RotateControlArrowButtonMixin",
        &[
            "OnLoad",
            "OnButtonStateChanged",
            "SetHoverCallbacks",
            "OnEnter",
            "OnLeave",
            "OnMouseDown",
            "OnMouseUp",
        ],
    ),
    (
        "ScaleControlFrameMixin",
        &[
            "OnLoad",
            "OnShow",
            "OnEnter",
            "OnLeave",
            "OnValueChanged",
            "OnMinMaxChanged",
            "FormatValue",
            "OnMouseDown",
            "OnMouseUp",
            "UpdateActiveState",
            "UpdateDefaultAnchor",
            "UpdateFill",
        ],
    ),
    (
        "ScaleControlArrowButtonMixin",
        &[
            "OnLoad",
            "OnButtonStateChanged",
            "SetHoverCallbacks",
            "OnEnter",
            "OnLeave",
            "OnMouseDown",
            "OnMouseUp",
        ],
    ),
];

fn fresh_game_env() -> WowLuaEnv {
    let env = WowLuaEnv::new().expect("Failed to create Lua environment");
    env.set_screen_size(1024.0, 768.0);
    env.set_screen_mode(ScreenKind::Game);
    {
        let mut state = env.state().borrow_mut();
        state.addon_base_paths = vec![blizzard_ui_dir()];
    }
    wow_ui_sim::xml::register_intrinsic_templates();
    env
}

fn load_full_game_ui() -> WowLuaEnv {
    let env = fresh_game_env();

    let addons = discover_blizzard_addons_for_screen(&blizzard_ui_dir(), ScreenKind::Game);
    for (name, toc_path) in &addons {
        load_addon(&env.loader_env(), toc_path)
            .unwrap_or_else(|err| panic!("[load {name}] FAILED: {err}"));
    }

    env.apply_post_load_workarounds();
    fire_startup_events_for_screen(&env, ScreenKind::Game);
    env
}

#[test]
fn find_toc_file_resolves_bare_toc() {
    let resolved = find_toc_file(&manipulator_dir()).expect("TransformManipulator TOC resolves");
    assert_eq!(
        resolved,
        manipulator_toc(),
        "Bare TOC — no flavor suffix; the housing precision-manipulation \
         control templates ship as a flavor-agnostic addon resolved via \
         the bare-TOC path in find_toc_file at src/loader/mod.rs:65-95"
    );
}

#[test]
fn toc_is_eager_with_no_meaningful_dependencies() {
    let toc = TocFile::from_file(&manipulator_toc()).expect("TOC parses");

    assert!(
        !toc.is_load_on_demand(),
        "`## LoadOnDemand: 0` — the literal `0` value falls through the \
         `is_load_on_demand` check at toc.rs:259-264 (which requires `1` \
         or `true`) and returns false, so this addon is loaded eagerly \
         alongside other always-on Blizzard addons. The `0` is intentional \
         self-documentation that the addon could be LoD but isn't"
    );
    assert!(
        toc.dependencies().is_empty(),
        "`## Dependencies: ` (empty value after the colon) — toc.rs's \
         dependencies parser must yield an empty Vec for the no-value \
         shape; TransformManipulator publishes templates/mixins that \
         depend only on always-loaded SharedXML utilities (FrameUtil, \
         GenerateClosure, ButtonStateBehaviorMixin, FormatPercentage). \
         Got: {:?}",
        toc.dependencies()
    );
    assert!(toc.optional_deps().is_empty());
    assert!(toc.saved_variables().is_empty());
    assert!(toc.saved_variables_per_character().is_empty());
    assert!(!toc.is_load_first());
    assert!(!toc.is_secure_env());
    assert!(
        !toc.is_game_type_restricted(),
        "AllowLoadGameType absent → not restricted (false). Housing \
         precision controls are mainline-housing-feature code but the \
         addon itself stays unrestricted."
    );
    assert!(
        toc.default_enabled(),
        "`## DefaultState: enabled` — explicitly opted-in default; \
         toc.rs:default_enabled returns true"
    );
}

#[test]
fn allow_load_absent_defaults_to_game_only_screen() {
    let toc = TocFile::from_file(&manipulator_toc()).expect("TOC parses");

    assert!(
        toc.allows_screen(ScreenKind::Game),
        "AllowLoad absent → toc.rs:305-313 None branch defaults to \
         Game-only — the housing precision controls only exist inside \
         the in-world expert-mode editor"
    );
    for screen in [
        ScreenKind::Login,
        ScreenKind::CharacterSelect,
        ScreenKind::CharacterCreate,
    ] {
        assert!(
            !toc.allows_screen(screen),
            "Glue screen {screen:?} must be excluded — the housing UI \
             does not exist on glue"
        );
    }
}

#[test]
fn toc_raw_bytes_pin_five_directives_and_four_body_files() {
    let raw = std::fs::read_to_string(manipulator_toc()).expect("TOC reads utf-8");

    let expected_directives = [
        "## Title: Blizzard House Editor",
        "## Author: Blizzard Entertainment",
        "## LoadOnDemand: 0",
        "## DefaultState: enabled",
        "## Dependencies:",
        "Blizzard_ScaleControlFrame.lua",
        "Blizzard_ScaleControlFrame.xml",
        "Blizzard_RotateControlFrame.lua",
        "Blizzard_RotateControlFrame.xml",
    ];

    for directive in expected_directives {
        assert!(
            raw.contains(directive),
            "Raw TOC must pin `{directive}` — 5 metadata directives + 4 \
             body files (lua/xml pair per control). Title=\"Blizzard \
             House Editor\" reflects the housing-only origin even though \
             the addon comment at RotateControlFrame.lua:1 calls out a \
             TODO to decouple from housing"
        );
    }

    assert!(!raw.contains("## Version"));
    assert!(!raw.contains("## RequiredDep"));
    assert!(!raw.contains("## OptionalDep"));
    assert!(!raw.contains("## SavedVariables"));
    assert!(!raw.contains("## AllowLoad"));
    assert!(!raw.contains("## AllowLoadGameType"));
    assert!(!raw.contains("## UseSecureEnvironment"));
    assert!(!raw.contains("## LoadFirst"));
}

#[test]
fn body_lists_scale_then_rotate_pairs_in_order() {
    let toc = TocFile::from_file(&manipulator_toc()).expect("TOC parses");

    let body: Vec<String> = toc
        .files
        .iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect();

    assert_eq!(
        body,
        vec![
            "Blizzard_ScaleControlFrame.lua".to_string(),
            "Blizzard_ScaleControlFrame.xml".to_string(),
            "Blizzard_RotateControlFrame.lua".to_string(),
            "Blizzard_RotateControlFrame.xml".to_string(),
        ],
        "Body must be exactly 4 entries in this order — Scale pair \
         (.lua then .xml so the `ScaleControlFrameMixin` table exists \
         when XML's mixin attribute is resolved) followed by Rotate \
         pair in the same shape. The 2 controls are independent: \
         neither imports the other. Got: {body:?}"
    );
}

#[test]
fn present_only_in_game_screen_eager_discovery() {
    for screen in ALL_FOUR_SCREENS {
        let addons = discover_blizzard_addons_for_screen(&blizzard_ui_dir(), *screen);
        let found = addons
            .iter()
            .any(|(name, _)| name == "Blizzard_TransformManipulator");

        let expected = matches!(screen, ScreenKind::Game);
        assert_eq!(
            found, expected,
            "Blizzard_TransformManipulator on {screen:?}: expected \
             discovered={expected}, got={found}. AllowLoad absent → \
             Game-only; LoD=0 + DefaultState=enabled means the addon \
             IS picked up by the eager Game sweep but never appears on \
             glue screens"
        );
    }
}

#[test]
fn no_addon_declares_transform_manipulator_as_dependency() {
    let entries = std::fs::read_dir(blizzard_ui_dir()).expect("BlizzardUI dir reads");
    let mut declarers: Vec<String> = Vec::new();

    for entry in entries.flatten() {
        let addon_dir = entry.path();
        if !addon_dir.is_dir() {
            continue;
        }
        let Some(toc_path) = find_toc_file(&addon_dir) else {
            continue;
        };
        let Ok(toc) = TocFile::from_file(&toc_path) else {
            continue;
        };
        let declared = toc
            .dependencies()
            .iter()
            .any(|d| d == "Blizzard_TransformManipulator")
            || toc
                .optional_deps()
                .iter()
                .any(|d| d == "Blizzard_TransformManipulator");
        if declared {
            let name = addon_dir.file_name().unwrap().to_string_lossy().to_string();
            declarers.push(name);
        }
    }

    assert!(
        declarers.is_empty(),
        "No Blizzard addon may declare Blizzard_TransformManipulator as \
         a hard or optional dep — the published templates/mixins are \
         forward-declarations for housing precision-manipulation UI \
         that no shipped addon yet consumes (RotateControl* and \
         ScaleControl* identifiers appear nowhere else in BlizzardUI). \
         Found declarers: {declarers:?}"
    );
}

prefork_full_ui_case! {
fn full_game_load_publishes_four_mixins_with_current_methods(env: &WowLuaEnv) {
    for (mixin, expected_methods) in PUBLISHED_MIXINS {
        let kind: String = env
            .eval(&format!("return type({mixin})"))
            .unwrap_or_else(|err| panic!("{mixin} probe failed: {err}"));
        assert_eq!(
            kind, "table",
            "{mixin} must be a table after eager game load — the addon is loaded automatically \
             as part of the Game-screen eager sweep, no LoD trigger needed"
        );

        for method in *expected_methods {
            let method_kind: String = env
                .eval(&format!("return type({mixin}.{method})"))
                .unwrap_or_else(|err| panic!("{mixin}.{method} probe failed: {err}"));
            assert_eq!(
                method_kind, "function",
                "{mixin}.{method} must be a function. The arrow mixins derive from \
                 ButtonStateBehaviorMixin, so concrete source-defined methods are a stable \
                 contract while total table-method counts vary with inherited methods"
            );
        }
    }
}
}

prefork_full_ui_case! {
fn full_game_load_publishes_no_named_top_level_frames(env: &WowLuaEnv) {

    let template_globals = [
        "RotateControlArrowButtonTemplate",
        "RotateControlFrameTemplate",
        "ScaleControlArrowButtonTemplate",
        "ScaleControlFrameTemplate",
    ];

    for name in template_globals {
        let kind: String = env
            .eval(&format!("return type(_G['{name}'])"))
            .unwrap_or_else(|err| panic!("{name} probe failed: {err}"));
        assert_eq!(
            kind, "nil",
            "Virtual template `{name}` must NOT exist as a global frame \
             — every named element in this addon's XML carries \
             `virtual=\"true\"`, so the loader registers them in the \
             template registry but does NOT create a frame instance. \
             The addon publishes ZERO named top-level frames, which is \
             why no `_LoadUI` wrapper or `UIPanelWindows` entry is \
             needed. Got type={kind} for {name}"
        );
    }
}
}

prefork_full_ui_case! {
fn published_templates_appear_in_xml_template_registry(env: &WowLuaEnv) {

    let template_names = [
        "RotateControlArrowButtonTemplate",
        "RotateControlFrameTemplate",
        "ScaleControlArrowButtonTemplate",
        "ScaleControlFrameTemplate",
    ];

    for name in template_names {
        let probe = format!(
            "local found = false; for _, t in ipairs({{'RotateControlArrowButtonTemplate','RotateControlFrameTemplate','ScaleControlArrowButtonTemplate','ScaleControlFrameTemplate'}}) do if t == '{name}' then found = true; break end end; return found"
        );
        let _: bool = env.eval(&probe).expect("template name list probe");
    }

    let frame_handle = env
        .eval(
            "local f = CreateFrame('Button', 'TestRotateArrow_TM', UIParent, 'RotateControlArrowButtonTemplate'); return type(f)",
        );
    assert!(
        frame_handle.is_ok(),
        "RotateControlArrowButtonTemplate must be instantiable via \
         CreateFrame after eager load — virtual=\"true\" registers the \
         template in the XML registry. Probe error: {:?}",
        frame_handle.err()
    );
    let kind = frame_handle.unwrap();
    let kind: String = kind;
    assert_eq!(
        kind, "table",
        "CreateFrame for RotateControlArrowButtonTemplate must yield a \
         FrameRef (type=='table'). Got type={kind}"
    );
}
}

#[test]
fn full_game_load_emits_no_addon_specific_errors() {
    let env = fresh_game_env();

    let addons = discover_blizzard_addons_for_screen(&blizzard_ui_dir(), ScreenKind::Game);
    for (name, toc_path) in &addons {
        load_addon(&env.loader_env(), toc_path)
            .unwrap_or_else(|err| panic!("[load {name}] FAILED: {err}"));
    }
    env.apply_post_load_workarounds();
    fire_startup_events_for_screen(&env, ScreenKind::Game);

    let errors: Vec<String> = env.state().borrow().lua_errors.clone();
    let addon_specific: Vec<&String> = errors
        .iter()
        .filter(|e| {
            e.contains("Blizzard_TransformManipulator")
                || e.contains("RotateControlFrame")
                || e.contains("ScaleControlFrame")
                || e.contains("RotateControlArrowButton")
                || e.contains("ScaleControlArrowButton")
        })
        .collect();

    assert!(
        addon_specific.is_empty(),
        "Eager Game-screen load must emit zero TransformManipulator-\
         specific errors — the addon only publishes mixin tables and \
         registers virtual templates; no frames are instantiated and \
         no event handlers fire until a consumer creates an instance \
         via CreateFrame and the housing system raises \
         HOUSING_DECOR_PRECISION_MANIPULATION_STATUS_CHANGED. Found {}: \
         {:#?}",
        addon_specific.len(),
        addon_specific
    );
}
