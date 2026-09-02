#![cfg(feature = "client-retail")]
use std::path::PathBuf;

use wow_ui_sim::loader::{discover_blizzard_addons_for_screen, find_toc_file, load_addon};
use wow_ui_sim::lua_api::WowLuaEnv;
use wow_ui_sim::screen::ScreenKind;
use wow_ui_sim::startup::fire_startup_events_for_screen;
use wow_ui_sim::toc::TocFile;

fn blizzard_ui_dir() -> PathBuf {
    wow_ui_sim::client_profile::blizzard_ui_addons_dir_under(std::path::Path::new(env!(
        "CARGO_MANIFEST_DIR"
    )))
}

fn maw_buffs_dir() -> PathBuf {
    blizzard_ui_dir().join("Blizzard_MawBuffs")
}

fn maw_buffs_toc() -> PathBuf {
    maw_buffs_dir().join("Blizzard_MawBuffs.toc")
}

const MAW_BUFFS_TOC_FILES: &[&str] = &["Blizzard_MawBuffs.xml"];

const PUBLISHED_MIXINS: &[&str] = &[
    "MawBuffsContainerMixin",
    "MawBuffsListMixin",
    "MawBuffMixin",
];

const CONTAINER_METHODS: &[&str] = &[
    "OnLoad",
    "OnEvent",
    "OnShow",
    "Update",
    "UpdateAlignment",
    "UpdateHelptip",
    "UpdateListState",
    "OnClick",
    "HighlightBuffAndShow",
    "HideBuffHighlight",
];

const LIST_METHODS: &[&str] = &[
    "OnLoad",
    "OnShow",
    "OnHide",
    "HighlightBuffAndShow",
    "HideBuffHighlight",
    "Update",
];

const BUFF_METHODS: &[&str] = &[
    "SetBuffInfo",
    "OnEnter",
    "RefreshTooltip",
    "OnClick",
    "OnLeave",
];

const VIRTUAL_TEMPLATES: &[&str] = &["MawBuffTemplate", "MawBuffsList", "MawBuffsContainer"];

const FILE_LOCAL_CONSTANTS: &[&str] = &[
    "MAW_BUFF_MAX_DISPLAY",
    "BUFF_HEIGHT",
    "BUFF_LIST_MIN_HEIGHT",
    "BUFF_LIST_PADDING_HEIGHT",
    "BUFF_LIST_NUM_COLUMNS",
];

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
fn blizzard_maw_buffs_find_toc_resolves_bare_variant() {
    let resolved = find_toc_file(&maw_buffs_dir()).expect("Blizzard_MawBuffs TOC should resolve");
    assert_eq!(
        resolved,
        maw_buffs_toc(),
        "Blizzard_MawBuffs ships exactly one bare TOC. The Maw / Torghast Anima Powers display \
         is a Shadowlands feature shared across every retail flavor, so no flavor-suffixed \
         variants exist — the bare TOC resolves via `find_toc_file` after the `_Mainline.toc` \
         lookup misses"
    );
}

#[test]
fn blizzard_maw_buffs_toc_declares_eager_load_with_two_required_deps() {
    let toc = TocFile::from_file(&maw_buffs_toc()).expect("Blizzard_MawBuffs TOC parses");
    assert!(
        !toc.is_load_on_demand(),
        "Blizzard_MawBuffs declares `## LoadOnDemand: 0` — the maw-buffs container is referenced \
         from Blizzard_ObjectiveTracker / Blizzard_ScenarioObjectiveTracker, so the addon must \
         be eagerly loaded for the ObjectiveTracker frame template chain to inherit it via \
         `inherits=\"MawBuffsContainer\"`"
    );
    assert!(
        !toc.is_load_first(),
        "Blizzard_MawBuffs does NOT declare `## LoadFirst: 1` — it consumes templates / atlases \
         from Blizzard_UIFrameManager (its required dep) and runs in the ordinary load pass"
    );
    assert!(
        !toc.is_secure_env(),
        "Blizzard_MawBuffs does NOT declare `## UseSecureEnvironment` — runs in the standard \
         (non-secure) Lua environment"
    );
    assert_eq!(
        toc.dependencies(),
        vec![
            "Blizzard_UIFrameManager".to_string(),
            "Blizzard_Colors".to_string(),
        ],
        "Blizzard_MawBuffs declares `## RequiredDep: Blizzard_UIFrameManager, \
         Blizzard_Colors` in that order. UIFrameManager publishes frame-style \
         infrastructure, while Blizzard_Colors supplies shared color data used \
         by the Maw buff presentation"
    );
    assert!(toc.optional_deps().is_empty());
    assert!(
        toc.saved_variables().is_empty(),
        "Blizzard_MawBuffs declares NO `## SavedVariables*` — the buff list is fully \
         server-driven (sourced from C_UnitAuras.GetAuraDataByIndex with the MAW filter); \
         only the helptip closed-state is persisted, and that goes through CVar bitfield \
         `closedInfoFrames` (LE_FRAME_TUTORIAL_9_0_JAILERS_TOWER_BUFFS), not the addon's own \
         saved-variables file"
    );
    assert!(
        !toc.is_game_type_restricted(),
        "Blizzard_MawBuffs omits `## AllowLoadGameType:` — Anima Powers / Torghast loaders are \
         universal across every retail flavor, so the addon is not game-type restricted"
    );
}

#[test]
fn blizzard_maw_buffs_toc_omits_allow_load_directive_routes_game_screen_only() {
    let toc = TocFile::from_file(&maw_buffs_toc()).expect("Blizzard_MawBuffs TOC parses");
    let raw = std::fs::read_to_string(maw_buffs_toc()).expect("Blizzard_MawBuffs TOC reads");
    assert!(
        !raw.contains("## AllowLoad"),
        "Blizzard_MawBuffs does NOT declare `## AllowLoad:` — `allows_screen` (src/toc.rs:311) \
         routes the missing-directive default to Game-screen only. The Jailers Tower / Maw \
         buff-stack display is unreachable from any glue screen"
    );
    assert!(
        toc.allows_screen(ScreenKind::Game),
        "Blizzard_MawBuffs must allow the Game screen — the buff-stack display is part of the \
         in-game ObjectiveTracker UI"
    );
    for screen in [
        ScreenKind::Login,
        ScreenKind::CharacterSelect,
        ScreenKind::CharacterCreate,
    ] {
        assert!(
            !toc.allows_screen(screen),
            "Blizzard_MawBuffs must NOT allow {screen:?} — missing `## AllowLoad:` \
             defaults to Game-screen only, and Maw buffs are unreachable on glue screens"
        );
    }
}

#[test]
fn blizzard_maw_buffs_toc_lists_single_xml_file() {
    let toc = TocFile::from_file(&maw_buffs_toc()).expect("Blizzard_MawBuffs TOC parses");
    let files: Vec<String> = toc
        .files
        .iter()
        .map(|p| p.to_string_lossy().replace('\\', "/"))
        .collect();
    assert_eq!(
        files, MAW_BUFFS_TOC_FILES,
        "Blizzard_MawBuffs TOC body lists exactly 1 file: `Blizzard_MawBuffs.xml`. The Lua \
         source is loaded via the XML's `<Script file=\"Blizzard_MawBuffs.lua\"/>` directive on \
         line 3 of the XML, NOT a separate TOC entry. This is the standard XML-first loading \
         pattern for addons whose Lua is purely mixin-attached (no global event / startup \
         registration outside the mixin OnLoad scripts)"
    );
}

#[test]
fn blizzard_maw_buffs_directory_holds_three_entries_toc_lua_xml() {
    let dir = maw_buffs_dir();
    let mut entries: Vec<String> = std::fs::read_dir(&dir)
        .expect("Blizzard_MawBuffs directory reads")
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    entries.sort();
    assert_eq!(
        entries,
        vec![
            "Blizzard_MawBuffs.lua".to_string(),
            "Blizzard_MawBuffs.toc".to_string(),
            "Blizzard_MawBuffs.xml".to_string(),
        ],
        "Blizzard_MawBuffs directory ships exactly 3 entries (TOC + Lua + XML), no flavor \
         subdirectories and no Localization.lua — strings are pulled from the global locale \
         table (JAILERS_TOWER_BUFFS_BUTTON_TEXT, JAILERS_TOWER_BUFFS_TUTORIAL)"
    );
}

#[test]
fn blizzard_maw_buffs_appears_exactly_once_in_game_screen_auto_discovery() {
    let addons = discover_blizzard_addons_for_screen(&blizzard_ui_dir(), ScreenKind::Game);
    let count = addons
        .iter()
        .filter(|(name, _)| name == "Blizzard_MawBuffs")
        .count();
    assert_eq!(
        count, 1,
        "Blizzard_MawBuffs must auto-discover EXACTLY ONCE on the Game screen — non-LoD with \
         missing `## AllowLoad:` defaults to Game-only, and the single bare TOC means no \
         flavor-variant duplication"
    );
}

#[test]
fn blizzard_maw_buffs_excluded_from_all_glue_screen_auto_discovery_passes() {
    for screen in [
        ScreenKind::Login,
        ScreenKind::CharacterSelect,
        ScreenKind::CharacterCreate,
    ] {
        let addons = discover_blizzard_addons_for_screen(&blizzard_ui_dir(), screen);
        let discovered = addons.iter().any(|(name, _)| name == "Blizzard_MawBuffs");
        assert!(
            !discovered,
            "Blizzard_MawBuffs MUST NOT appear in {screen:?} auto-discovery — the missing \
             `## AllowLoad:` defaults to Game-only and Maw / Jailers Tower content is \
             unreachable from any glue screen"
        );
    }
}

prefork_full_ui_case! {
fn blizzard_maw_buffs_loads_without_addon_specific_lua_errors(env: &WowLuaEnv) {

    let load_errors: Vec<String> = env
        .state()
        .borrow()
        .lua_errors
        .iter()
        .filter(|message| {
            message.contains("Blizzard_MawBuffs/")
                || message.contains("Blizzard_MawBuffs\\")
                || message.contains("MawBuffsContainerMixin")
                || message.contains("MawBuffsListMixin")
                || message.contains("MawBuffMixin")
                || message.contains("ShouldShowMawBuffs")
        })
        .cloned()
        .collect();
    assert!(
        load_errors.is_empty(),
        "Blizzard_MawBuffs emitted addon-specific Lua errors during Game-screen auto-load:\n  {}",
        load_errors.join("\n  ")
    );
}
}

prefork_full_ui_case! {
fn blizzard_maw_buffs_is_addon_loaded_returns_true_after_game_screen_load(env: &WowLuaEnv) {

    let loaded: bool = env
        .eval("return C_AddOns.IsAddOnLoaded('Blizzard_MawBuffs')")
        .expect("IsAddOnLoaded probe succeeds");
    assert!(
        loaded,
        "C_AddOns.IsAddOnLoaded('Blizzard_MawBuffs') must return true after Game-screen \
         auto-discovery — proves the eager-load TOC reaches the loaded-set even though the \
         body lists only an XML file (the XML's `<Script file=>` directive must transitively \
         publish the Lua mixins / globals before the loaded-flag flips)"
    );
}
}

prefork_full_ui_case! {
fn blizzard_maw_buffs_publishes_three_mixin_globals(env: &WowLuaEnv) {

    for mixin in PUBLISHED_MIXINS {
        let exists: bool = env
            .eval(&format!("return type(_G['{mixin}']) == 'table'"))
            .expect("mixin existence probe succeeds");
        assert!(
            exists,
            "After Game-screen auto-load, `{mixin}` must publish as a `_G` mixin table — \
             Blizzard_MawBuffs.lua declares 3 mixins: MawBuffsContainerMixin (line 9, 10 \
             methods covering the parent button container), MawBuffsListMixin (line 141, 6 \
             methods covering the dropdown buff-list panel including Update which positions \
             children in 4-column grid layout), and MawBuffMixin (line 229, 5 methods for the \
             individual buff icon button — SetBuffInfo / RefreshTooltip / OnEnter / OnLeave / \
             OnClick)"
        );
    }
}
}

prefork_full_ui_case! {
fn blizzard_maw_buffs_container_mixin_exposes_lifecycle_and_state_methods(env: &WowLuaEnv) {

    for method in CONTAINER_METHODS {
        let exists: bool = env
            .eval(&format!(
                "return type(MawBuffsContainerMixin['{method}']) == 'function'"
            ))
            .expect("MawBuffsContainerMixin method probe succeeds");
        assert!(
            exists,
            "MawBuffsContainerMixin must expose `:{method}()` — the container declares 10 \
             methods spanning lifecycle (OnLoad registers UNIT_AURA + GLOBAL_MOUSE_DOWN, OnEvent \
             dispatches both, OnShow runs UpdateAlignment), state management (Update walks 1..44 \
             aura slots via C_UnitAuras.GetAuraDataByIndex with MAW filter and rebuilds the buff \
             table, UpdateAlignment swaps anchor + texcoord based on \
             ObjectiveTrackerFrame.isOnLeftSideOfScreen, UpdateHelptip drives HelpTip:Show vs \
             :Hide for tutorial state, UpdateListState toggles enabled + list visibility), \
             dispatch (OnClick toggles list shown), and forwarders (HighlightBuffAndShow / \
             HideBuffHighlight delegate to self.List)"
        );
    }
}
}

prefork_full_ui_case! {
fn blizzard_maw_buffs_list_mixin_exposes_pool_and_layout_methods(env: &WowLuaEnv) {

    for method in LIST_METHODS {
        let exists: bool = env
            .eval(&format!(
                "return type(MawBuffsListMixin['{method}']) == 'function'"
            ))
            .expect("MawBuffsListMixin method probe succeeds");
        assert!(
            exists,
            "MawBuffsListMixin must expose `:{method}()` — the list panel declares 6 methods: \
             OnLoad caches self.button = parent and creates a CreateFramePool('BUTTON', self, \
             'MawBuffTemplate') under self.buffPool; OnShow / OnHide swap the parent button's \
             pushed / highlight atlases between `-pressed` and `-normalpressed` variants and \
             flip the parent button's pushed-text-offset based on \
             ObjectiveTrackerFrame.isOnLeftSideOfScreen; HighlightBuffAndShow / \
             HideBuffHighlight walk EnumerateActive on the buffPool to flip per-buff \
             HighlightBorder visibility; Update is the per-frame layout pass — \
             buffPool:ReleaseAll then a 4-column grid using BUFF_LIST_NUM_COLUMNS, with \
             max(BUFF_LIST_PADDING_HEIGHT + content, BUFF_LIST_MIN_HEIGHT) min-height clamp"
        );
    }
}
}

prefork_full_ui_case! {
fn blizzard_maw_buffs_buff_mixin_exposes_buff_button_methods(env: &WowLuaEnv) {

    for method in BUFF_METHODS {
        let exists: bool = env
            .eval(&format!(
                "return type(MawBuffMixin['{method}']) == 'function'"
            ))
            .expect("MawBuffMixin method probe succeeds");
        assert!(
            exists,
            "MawBuffMixin must expose `:{method}()` — the per-buff button declares 5 methods: \
             SetBuffInfo wires Icon / Border (rarity atlas via \
             C_Spell.GetMawPowerBorderAtlasBySpellID) / Count + CountRing (only shown when \
             count > 1) and refreshes the tooltip if the buff is currently hovered; \
             RefreshTooltip pins GameTooltip via SetUnitAura('player', slot, 'MAW'); OnEnter \
             delegates to RefreshTooltip; OnLeave hides GameTooltip and the HighlightBorder; \
             OnClick chat-links the maw-power via GetMawPowerLinkBySpellID when CHATLINK \
             modifier is held"
        );
    }
}
}

prefork_full_ui_case! {
fn blizzard_maw_buffs_publishes_should_show_maw_buffs_global_function(env: &WowLuaEnv) {

    let kind: String = env
        .eval("return type(ShouldShowMawBuffs)")
        .expect("ShouldShowMawBuffs type probe succeeds");
    assert_eq!(
        kind, "function",
        "Blizzard_MawBuffs.lua line 3 publishes `ShouldShowMawBuffs` as a global function — the \
         contract is a 1-line predicate returning `IsInJailersTower() or hasMawBuff or false`. \
         Used externally by Blizzard_ScenarioObjectiveTracker.lua:108 + 187 to gate the maw \
         buffs subtree visibility inside the scenario tracker. Must be reachable from outside \
         this addon (cross-addon export pattern)"
    );
}
}

prefork_full_ui_case! {
fn blizzard_maw_buffs_file_local_constants_stay_nil_at_global_scope(env: &WowLuaEnv) {

    for name in FILE_LOCAL_CONSTANTS {
        let kind: String = env
            .eval(&format!("return type(_G['{name}'])"))
            .expect("file-local constant probe succeeds");
        assert_eq!(
            kind, "nil",
            "`{name}` must NOT publish at `_G` — Blizzard_MawBuffs.lua declares 5 file-local \
             constants via `local NAME = value;` at module scope: MAW_BUFF_MAX_DISPLAY = 44 \
             (line 1, scan ceiling for C_UnitAuras slots), BUFF_HEIGHT = 45 (line 143), \
             BUFF_LIST_MIN_HEIGHT = 159 (line 144), BUFF_LIST_PADDING_HEIGHT = 36 (line 145), \
             BUFF_LIST_NUM_COLUMNS = 4 (line 146). All 5 are `local` so they stay confined to \
             the addon's compiled chunk environment and do not leak into the shared `_G` table"
        );
    }
}
}

prefork_full_ui_case! {
fn blizzard_maw_buffs_registers_three_virtual_xml_templates(env: &WowLuaEnv) {
    let _env = env;

    for template in VIRTUAL_TEMPLATES {
        let registered = wow_ui_sim::xml::get_template(template).is_some();
        assert!(
            registered,
            "Virtual template `{template}` must register with the XML template registry. \
             Blizzard_MawBuffs.xml declares 3 virtual templates: \
             MawBuffTemplate (Button, mixin=MawBuffMixin, motionScriptsWhileDisabled=true, \
             45 by 45) — the per-buff icon button with Icon (35x35 ARTWORK), CircleMask \
             (TempPortraitAlphaMask), Border + HighlightBorder (textureSubLevel=2), CountRing + \
             Count FontString (OVERLAY); MawBuffsList (Frame, mixin=MawBuffsListMixin, \
             210 by 159) — the dropdown panel with TopBG / BottomBG / MiddleBG (tiled \
             jailerstower-animapowerlist-tile) backgrounds; MawBuffsContainer (Button, \
             mixin=MawBuffsContainerMixin, 253 by 50) — the parent button with NormalTexture / \
             PushedTexture / HighlightTexture / DisabledTexture (jailerstower-animapowerbutton \
             atlases) and a parentKey=List child Frame inheriting MawBuffsList anchored \
             TOPRIGHT-to-TOPLEFT"
        );
    }
}
}

prefork_full_ui_case! {
fn blizzard_maw_buffs_required_dep_blizzard_uiframemanager_loads_first(env: &WowLuaEnv) {

    let dep_loaded: bool = env
        .eval("return C_AddOns.IsAddOnLoaded('Blizzard_UIFrameManager')")
        .expect("Blizzard_UIFrameManager IsAddOnLoaded probe succeeds");
    assert!(
        dep_loaded,
        "Blizzard_UIFrameManager must be loaded after the Game-screen auto-discovery sweep — \
         it is Blizzard_MawBuffs's `## RequiredDep:`, so the loader's dependency-graph walk \
         must drag it in before MawBuffs's load. Without UIFrameManager loaded, the \
         jailerstower-animapowerlist atlas chrome / texture-kit infrastructure that the \
         MawBuffsContainer button styles consume would resolve as nil, and any frame \
         instantiation deriving from MawBuffsContainer would short-circuit"
    );
}
}
