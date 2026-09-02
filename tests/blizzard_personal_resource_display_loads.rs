#![cfg(feature = "client-retail")]
use std::path::PathBuf;

use wow_ui_sim::loader::{discover_all_blizzard_addons, discover_blizzard_addons_for_screen};
use wow_ui_sim::loader::{find_toc_file, load_addon};
use wow_ui_sim::lua_api::WowLuaEnv;
use wow_ui_sim::screen::ScreenKind;
use wow_ui_sim::startup::fire_startup_events_for_screen;
use wow_ui_sim::toc::TocFile;

fn blizzard_ui_dir() -> PathBuf {
    wow_ui_sim::client_profile::blizzard_ui_addons_dir_under(std::path::Path::new(env!(
        "CARGO_MANIFEST_DIR"
    )))
}

fn prd_dir() -> PathBuf {
    blizzard_ui_dir().join("Blizzard_PersonalResourceDisplay")
}

fn prd_toc() -> PathBuf {
    prd_dir().join("Blizzard_PersonalResourceDisplay.toc")
}

const PRD_TOC_FILES: &[&str] = &[
    "AlternatePowerBars/DemonHunterAlternatePower.lua",
    "AlternatePowerBars/EvokerAlternatePower.lua",
    "AlternatePowerBars/MonkAlternatePower.lua",
    "AlternatePowerBars/ManaAlternatePower.lua",
    "Blizzard_PersonalResourceDisplay.lua",
    "Blizzard_PersonalResourceDisplay.xml",
];

const REQUIRED_DEPS: &[&str] = &["Blizzard_EditMode"];

const PUBLIC_MIXINS: &[&str] = &[
    "PersonalResourceDisplayMixin",
    "DemonHunterAlternatePowerBarMixin",
    "EvokerAlternatePowerBarMixin",
    "MonkAlternatePowerBarMixin",
    "ManaAlternatePowerMixin",
    "PriestAlternatePowerBarMixin",
    "DruidAlternatePowerBarMixin",
];

const PUBLIC_NAMED_FRAMES: &[&str] = &["PersonalResourceDisplayFrame"];

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
fn blizzard_personal_resource_display_find_toc_resolves_bare_variant() {
    let resolved =
        find_toc_file(&prd_dir()).expect("Blizzard_PersonalResourceDisplay TOC resolves");
    assert_eq!(
        resolved,
        prd_toc(),
        "Blizzard_PersonalResourceDisplay ships exactly one bare TOC — no \
         `_Mainline.toc` variant. The Personal Resource Display (PRD) is the \
         self-nameplate that appears under the player's character — health bar, \
         power bar, class resources (combo points / runes / arcane charges / \
         soul shards / chi / etc.), and a class-specific alternate power bar \
         (DH soul fragments / Evoker ebon might / Monk stagger). The flavor split \
         is handled by `## AllowLoadGameType: mainline` (per-TOC retail-only flag) \
         instead of a separate `_Mainline.toc` file"
    );

    let mainline = prd_dir().join("Blizzard_PersonalResourceDisplay_Mainline.toc");
    assert!(
        !mainline.exists(),
        "There must be NO `_Mainline.toc` at {} — the bare TOC with \
         `## AllowLoadGameType: mainline` is the canonical entry point",
        mainline.display()
    );
}

#[test]
fn blizzard_personal_resource_display_toc_declares_eager_mainline_with_edit_mode_dep() {
    let toc = TocFile::from_file(&prd_toc()).expect("Blizzard_PersonalResourceDisplay TOC parses");

    assert!(
        !toc.is_load_on_demand(),
        "TOC must NOT declare `## LoadOnDemand:` — PRD is foundational nameplate UI \
         that must be ready by the first PLAYER_ENTERING_WORLD event so the \
         self-nameplate renders the moment the character appears. Lazy-loading \
         would mean the player's first frame in-world has no resource display"
    );
    assert!(!toc.is_load_first());
    assert!(!toc.is_secure_env());

    assert!(
        toc.allows_screen(ScreenKind::Game),
        "Default-Game-only `allows_screen` at src/toc.rs:311 returns true for \
         ScreenKind::Game when AllowLoad is omitted — PRD is in-world UI"
    );
    for screen in [
        ScreenKind::Login,
        ScreenKind::CharacterSelect,
        ScreenKind::CharacterCreate,
    ] {
        assert!(
            !toc.allows_screen(screen),
            "Omitted `## AllowLoad:` must NOT enable {screen:?} — PRD is the \
             self-nameplate in 3D world space, glue screens have no nameplate \
             surface"
        );
    }

    assert_eq!(
        toc.dependencies(),
        REQUIRED_DEPS,
        "TOC must declare exactly 1 RequiredDep (`Blizzard_EditMode`) — the \
         PersonalResourceDisplayFrame inherits the \
         `EditModePersonalResourceDisplaySystemTemplate` virtual template defined \
         in Blizzard_EditMode, so EditMode's template registry must be populated \
         before PRD's XML parses. The dep is hard rather than optional because \
         the inherits resolves at parse time and a missing template raises a \
         loader error. `dependencies()` at src/toc.rs:210-217 reads `Dependencies` \
         here as the canonical retail spelling"
    );

    assert!(
        toc.optional_deps().is_empty(),
        "Zero `## OptionalDeps:` — no soft sibling addons"
    );
    assert!(
        toc.saved_variables().is_empty(),
        "Zero saved variables — PRD is a pure live-state mirror of player health / \
         power / class resources fetched from UnitHealth / UnitPower / etc. on every \
         OnUpdate tick; the only persistent state lives in the EditMode layout \
         system (position / scale), which Blizzard_EditMode owns"
    );
}

#[test]
fn blizzard_personal_resource_display_toc_declares_metadata_in_raw_bytes() {
    let raw = std::fs::read_to_string(prd_toc())
        .expect("Blizzard_PersonalResourceDisplay TOC reads utf-8");
    assert!(
        raw.contains("## Title: Blizzard_PersonalResourceDisplay"),
        "TOC must declare `## Title: Blizzard_PersonalResourceDisplay` exactly. \
         UNUSUAL: underscore-namespace title spelling rather than the space-and- \
         prose form (`Blizzard Personal Resource Display`); minority pattern, \
         suggests the addon was scaffolded from a code template rather than \
         hand-typed"
    );
    assert!(
        raw.contains("## AllowLoadGameType: mainline"),
        "TOC must declare `## AllowLoadGameType: mainline` exactly — UNUSUAL: \
         most Blizzard-shipped addons use a separate `_Mainline.toc` file for \
         retail-only behavior. PRD instead uses the per-TOC \
         `## AllowLoadGameType: mainline` flag, which routes through \
         `is_allowed_game_type` at src/toc.rs:45-57; the function ONLY accepts \
         `mainline` or `standard` so this TOC loads on Mainline and is filtered \
         out on Classic / WotLK / Cata flavors"
    );
    assert!(
        raw.contains("## DefaultState: enabled"),
        "TOC must declare `## DefaultState: enabled` exactly — the explicit \
         `enabled` documents that PRD is foundational UI infrastructure; \
         disabling would remove the self-nameplate from the world entirely, \
         which is reachable but not the default user experience"
    );
    assert!(
        raw.contains("## Dependencies: Blizzard_EditMode"),
        "TOC must declare `## Dependencies: Blizzard_EditMode` exactly (singular \
         dep, no comma)"
    );
    assert!(
        !raw.contains("## LoadOnDemand"),
        "TOC must NOT declare `## LoadOnDemand:` — PRD is eager so the OnLoad / \
         OnEvent chain wires up before PLAYER_ENTERING_WORLD fires"
    );
    assert!(
        !raw.contains("## AllowLoad:"),
        "TOC must NOT declare `## AllowLoad:` — Game-only is the default; PRD \
         uses `## AllowLoadGameType:` for the flavor filter, NOT `## AllowLoad:` \
         for the screen filter"
    );
    assert!(
        !raw.contains("## SavedVariables"),
        "TOC must NOT declare any `## SavedVariables*` keys — pure stateless \
         mirror of live player state"
    );
    assert!(
        !raw.contains("## OptionalDeps"),
        "TOC must NOT declare any `## OptionalDeps:` — zero soft siblings"
    );
    assert!(
        !raw.contains("## Author"),
        "TOC must NOT declare `## Author:` — the addon omits the author key, \
         like Blizzard_PerksProgram and a handful of other Blizzard-shipped \
         addons"
    );
    assert!(
        !raw.contains("## Version"),
        "TOC must NOT declare `## Version:` — UNUSUAL omission compared to most \
         Blizzard-shipped addons which ship `## Version: 1.0`; together with the \
         missing `## Author:` the metadata profile is minimal"
    );
}

#[test]
fn blizzard_personal_resource_display_toc_lists_six_files_alternate_power_bars_first() {
    let toc = TocFile::from_file(&prd_toc()).expect("Blizzard_PersonalResourceDisplay TOC parses");
    let listed: Vec<String> = toc
        .files
        .iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect();
    assert_eq!(
        listed, PRD_TOC_FILES,
        "TOC body must list exactly 6 files: 4 alternate-power-bar Lua files \
         first (DemonHunter, Evoker, Monk, Mana), then the base Lua and base XML \
         last. The TOC comment `# Alternate Power Bars (must be loaded before \
         Base)` documents the ordering: each alternate-bar file publishes its \
         mixins before the base Lua's SetupAlternatePowerBar selects one for \
         the current class and specialization"
    );
}

#[test]
fn blizzard_personal_resource_display_appears_in_game_screen_eager_discovery_only() {
    let ui = blizzard_ui_dir();

    let game_addons = discover_blizzard_addons_for_screen(&ui, ScreenKind::Game);
    let in_game = game_addons
        .iter()
        .any(|(name, _)| name == "Blizzard_PersonalResourceDisplay");
    assert!(
        in_game,
        "Blizzard_PersonalResourceDisplay must appear in Game-screen eager \
         discovery — eager (no LoadOnDemand) and Game-only by default, with \
         AllowLoadGameType: mainline accepted by `is_allowed_game_type` for \
         Mainline runs"
    );

    for screen in [
        ScreenKind::Login,
        ScreenKind::CharacterSelect,
        ScreenKind::CharacterCreate,
    ] {
        let glue_addons = discover_blizzard_addons_for_screen(&ui, screen);
        let in_glue = glue_addons
            .iter()
            .any(|(name, _)| name == "Blizzard_PersonalResourceDisplay");
        assert!(
            !in_glue,
            "Blizzard_PersonalResourceDisplay must NOT appear in {screen:?} \
             eager discovery — default Game-only `allows_screen` filters glue \
             screens"
        );
    }
}

#[test]
fn blizzard_personal_resource_display_appears_in_full_addon_inventory() {
    let ui = blizzard_ui_dir();
    let inventory = discover_all_blizzard_addons(&ui);
    let found = inventory
        .iter()
        .any(|(name, _)| name == "Blizzard_PersonalResourceDisplay");
    assert!(
        found,
        "Blizzard_PersonalResourceDisplay must appear in \
         `discover_all_blizzard_addons` — the full inventory walks every \
         parseable TOC under Interface/BlizzardUI regardless of LoadOnDemand or \
         AllowLoadGameType"
    );
}

prefork_full_ui_case! {
fn blizzard_personal_resource_display_loads_without_addon_specific_lua_errors(env: &WowLuaEnv) {

    let load_errors: Vec<String> = env
        .state()
        .borrow()
        .lua_errors
        .iter()
        .filter(|message| {
            message.contains("Blizzard_PersonalResourceDisplay")
                || message.contains("PersonalResourceDisplay")
                || message.contains("AlternatePowerBar")
                || message.contains("DemonHunterAlternatePower")
                || message.contains("EvokerAlternatePower")
                || message.contains("MonkAlternatePower")
        })
        .cloned()
        .collect();
    assert!(
        load_errors.is_empty(),
        "Blizzard_PersonalResourceDisplay emitted addon-specific Lua errors \
         during load:\n  {}",
        load_errors.join("\n  ")
    );
}
}

prefork_full_ui_case! {
fn blizzard_personal_resource_display_is_addon_loaded_after_eager_sweep(env: &WowLuaEnv) {

    let loaded: bool = env
        .eval("return C_AddOns.IsAddOnLoaded('Blizzard_PersonalResourceDisplay')")
        .expect("IsAddOnLoaded probe succeeds");
    assert!(
        loaded,
        "C_AddOns.IsAddOnLoaded('Blizzard_PersonalResourceDisplay') must return \
         true after the eager Game-screen sweep — the addon is eager so the \
         Game-screen sweep loads it directly"
    );
}
}

prefork_full_ui_case! {
fn blizzard_personal_resource_display_publishes_seven_mixin_tables(env: &WowLuaEnv) {

    for mixin in PUBLIC_MIXINS {
        let kind: String = env
            .eval(&format!("return type(_G.{mixin})"))
            .unwrap_or_else(|err| panic!("type(_G.{mixin}) probe failed: {err}"));
        assert_eq!(
            kind, "table",
            "_G.{mixin} must publish as a table — the base display and all \
             four alternate-power files load before the frame XML. The Mana \
             file publishes its base mixin plus Priest and Druid derivatives"
        );
    }
}
}

prefork_full_ui_case! {
fn blizzard_personal_resource_display_creates_personal_resource_display_frame_global(env: &WowLuaEnv) {

    for frame in PUBLIC_NAMED_FRAMES {
        let kind: String = env
            .eval(&format!("return type(_G.{frame})"))
            .unwrap_or_else(|err| panic!("type(_G.{frame}) probe failed: {err}"));
        assert_eq!(
            kind, "table",
            "_G.{frame} must publish as a frame userdata (FrameRef reports as \
             `'table'` via the custom __type metamethod). \
             Blizzard_PersonalResourceDisplay.xml ships exactly 1 named non-virtual \
             frame: PersonalResourceDisplayFrame (parent=UIParent, \
             inherits=EditModePersonalResourceDisplaySystemTemplate from \
             Blizzard_EditMode, mixin=PersonalResourceDisplayMixin, 200x75 default \
             size). Children include the HealthBarsContainer (with TempMaxHealthLoss \
             + healthBar status bars), PowerBar (mana/rage/etc. with FeedbackFrame / \
             FullPowerFrame), AlternatePowerBar (hidden-by-default for class-specific \
             alternate power), and ClassFrameContainer (hidden-by-default for combo \
             points / runes / arcane charges / soul shards / chi / etc. — populated \
             at runtime by SetupClassBar via the CLASS_FRAME_INFO_MAP table keyed \
             on Constants.UICharacterClasses with per-class template names like \
             PaladinPowerBarFrameTemplate / RogueComboPointBarTemplate / \
             RuneFrameTemplate / MageArcaneChargesFrameTemplate). ZERO virtual \
             templates — the entire layout is concrete; consumer addons cannot \
             extend it"
        );
    }
}
}
