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

fn ime_dir() -> PathBuf {
    blizzard_ui_dir().join("Blizzard_IME")
}

fn ime_toc() -> PathBuf {
    ime_dir().join("Blizzard_IME.toc")
}

const CANDIDATE_PARENT_KEYS: &[&str] =
    &["c1", "c2", "c3", "c4", "c5", "c6", "c7", "c8", "c9", "c10"];

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
fn blizzard_ime_find_toc_resolves_bare_variant() {
    let resolved = find_toc_file(&ime_dir()).expect("Blizzard_IME TOC should resolve");
    assert_eq!(
        resolved,
        ime_toc(),
        "Blizzard_IME ships exactly one bare TOC — pure-XML candidate display module resolves \
         via `find_toc_file` fallthrough"
    );
}

#[test]
fn blizzard_ime_toc_declares_no_required_deps_with_two_optional_deps() {
    let toc = TocFile::from_file(&ime_toc()).expect("Blizzard_IME TOC should parse");

    assert!(
        !toc.is_load_on_demand(),
        "Blizzard_IME omits `## LoadOnDemand:` — auto-loaded so the IMECandidatesFrame is ready \
         the moment the platform-side IME activates a composition session (Asian / accented \
         input). Cannot be LoD because the engine's IME bridge expects the named frame to exist \
         unconditionally"
    );
    assert!(!toc.is_load_first());
    assert!(!toc.is_secure_env());
    assert!(
        toc.dependencies().is_empty(),
        "Blizzard_IME declares zero `## Dependencies:` / `## RequiredDep` — the only template \
         it consumes externally is TooltipBorderBackdropTemplate from Blizzard_SharedXML, which \
         is implicitly loaded by every screen's foundational shared XML pass before any addon \
         runs"
    );
    assert_eq!(
        toc.optional_deps(),
        vec![
            "Blizzard_FrameXML".to_string(),
            "Blizzard_GlueXML".to_string(),
        ],
        "Blizzard_IME's singular `## OptionalDep:` directive must expose both screen roots"
    );

    let raw = std::fs::read_to_string(ime_toc()).expect("Blizzard_IME TOC should read");
    assert!(
        raw.contains("## OptionalDep: Blizzard_FrameXML, Blizzard_GlueXML"),
        "Blizzard_IME's raw TOC declares `## OptionalDep: Blizzard_FrameXML, Blizzard_GlueXML` \
         (singular form) — both optional because the addon must work on either side of the \
         Frame/Glue split (Game screen consumes Blizzard_FrameXML; Login / CharacterSelect / \
         CharacterCreate consume Blizzard_GlueXML). Verified by raw-text inspection because \
         the simulator parser does not recognize the singular spelling — see assertion above"
    );
    assert!(
        toc.saved_variables().is_empty(),
        "Blizzard_IME declares zero saved variables — the candidate frame has no persistent \
         state; every IME composition session re-anchors the candidates afresh"
    );
}

#[test]
fn blizzard_ime_toc_declares_default_state_enabled_and_allow_load_both() {
    let toc = TocFile::from_file(&ime_toc()).expect("Blizzard_IME TOC should parse");

    assert!(
        !toc.is_game_type_restricted(),
        "Blizzard_IME omits `## AllowLoadGameType` — `is_game_type_restricted()` returns false \
         (src/toc.rs:301 default branch). Available across every game type"
    );

    let raw = std::fs::read_to_string(ime_toc()).expect("Blizzard_IME TOC should read");
    assert!(
        raw.contains("## DefaultState: enabled"),
        "TOC must declare `## DefaultState: enabled` — auto-enabled on first install, no user \
         opt-in required for IME candidate display"
    );
    assert!(
        raw.contains("## AllowLoad: Both"),
        "TOC must declare `## AllowLoad: Both` — allows_screen returns true for every \
         ScreenKind because IME composition can begin on the Login / CharacterSelect / \
         CharacterCreate name fields just as easily as on the Game screen's chat edit boxes \
         (src/toc.rs:307 `eq_ignore_ascii_case(\"both\") => true`)"
    );
}

#[test]
fn blizzard_ime_toc_lists_only_the_xml_file() {
    let toc = TocFile::from_file(&ime_toc()).expect("Blizzard_IME TOC should parse");
    assert_eq!(
        toc.files
            .iter()
            .map(|p| p.to_string_lossy().into_owned())
            .collect::<Vec<_>>(),
        vec!["IME.xml"],
        "TOC body must list exactly `IME.xml`. Note: the file is bare-named `IME.xml`, NOT \
         `Blizzard_IME.xml` — Blizzard_IME predates the modern naming convention so it ships \
         the legacy un-prefixed XML filename. There is NO Lua file at all; the addon is pure \
         widget definition (no mixin, no script handlers)"
    );
}

#[test]
fn blizzard_ime_directory_holds_two_entries() {
    let entries = std::fs::read_dir(ime_dir())
        .expect("Blizzard_IME directory should read")
        .count();
    assert_eq!(
        entries, 2,
        "Directory must hold exactly 2 entries (1 TOC + 1 XML) — no Lua, no flavor \
         subdirectory, no Localization.lua. The strings shown in the candidate frame come from \
         the platform IME bridge's UTF-8 payload, not from a localization table"
    );
}

#[test]
fn blizzard_ime_appears_in_every_screen_auto_discovery() {
    for screen in [
        ScreenKind::Game,
        ScreenKind::Login,
        ScreenKind::CharacterSelect,
        ScreenKind::CharacterCreate,
    ] {
        let addons = discover_blizzard_addons_for_screen(&blizzard_ui_dir(), screen);
        let found = addons.iter().any(|(name, _)| name == "Blizzard_IME");
        assert!(
            found,
            "Blizzard_IME must appear in every ScreenKind auto-discovery sweep — \
             `## AllowLoad: Both` makes allows_screen return true unconditionally so IME \
             composition is available on every screen including the Login / glue flows. \
             (Screen tested: {screen:?})"
        );
    }
}

prefork_full_ui_case! {
fn blizzard_ime_loads_without_addon_specific_lua_errors(env: &WowLuaEnv) {

    let load_errors: Vec<String> = env
        .state()
        .borrow()
        .lua_errors
        .iter()
        .filter(|message| {
            message.contains("Blizzard_IME")
                || message.contains("IMECandidatesFrame")
                || message.contains("IMECandidate")
        })
        .cloned()
        .collect();
    assert!(
        load_errors.is_empty(),
        "Blizzard_IME emitted addon-specific Lua errors during load:\n  {}",
        load_errors.join("\n  ")
    );
}
}

prefork_full_ui_case! {
fn blizzard_ime_is_addon_loaded_via_game_screen_pass(env: &WowLuaEnv) {

    let loaded: bool = env
        .eval("return C_AddOns.IsAddOnLoaded('Blizzard_IME')")
        .expect("IsAddOnLoaded probe should succeed");
    assert!(
        loaded,
        "C_AddOns.IsAddOnLoaded('Blizzard_IME') must return true after the Game-screen pass — \
         no explicit LoD call needed because `## DefaultState: enabled` plus the absence of \
         `## LoadOnDemand:` makes the addon part of the eager auto-discovery sweep"
    );
}
}

prefork_full_ui_case! {
fn blizzard_ime_virtual_candidate_template_stays_nil_at_global_scope(env: &WowLuaEnv) {

    let kind: String = env
        .eval("return type(_G['IMECandidate'])")
        .expect("IMECandidate probe should succeed");
    assert_eq!(
        kind, "nil",
        "IMECandidate must NOT publish at `_G` — declared as `virtual=\"true\"` at IME.xml:5 \
         so the loader keeps it in the template registry only. Consumed via `inherits=` by \
         each of the c1..c10 child frames inside IMECandidatesFrame, never resolved through \
         the global scope"
    );
}
}

prefork_full_ui_case! {
fn blizzard_ime_named_frame_publishes_with_tooltip_strata_and_starts_hidden(env: &WowLuaEnv) {

    let kind: String = env
        .eval("return type(IMECandidatesFrame)")
        .expect("IMECandidatesFrame probe should succeed");
    assert_eq!(
        kind, "table",
        "IMECandidatesFrame must publish at `_G` as a table — the only named non-virtual frame \
         the addon ships, declared at IME.xml:26 with `name=\"IMECandidatesFrame\"` \
         `parent=\"UIParent\"` `inherits=\"TooltipBorderBackdropTemplate\"`"
    );

    let name: String = env
        .eval("return IMECandidatesFrame:GetName()")
        .expect("IMECandidatesFrame:GetName() probe should succeed");
    assert_eq!(
        name, "IMECandidatesFrame",
        "IMECandidatesFrame:GetName() must echo the XML `name` attribute"
    );

    let strata: String = env
        .eval("return IMECandidatesFrame:GetFrameStrata()")
        .expect("IMECandidatesFrame:GetFrameStrata() probe should succeed");
    assert_eq!(
        strata, "TOOLTIP",
        "IMECandidatesFrame must report TOOLTIP strata — declared `frameStrata=\"TOOLTIP\"` at \
         IME.xml:26 so the IME candidate list floats above every other frame including \
         tooltips would normally only reach DIALOG. TOOLTIP strata ensures the candidate \
         picker is never occluded by chat / mail / quest popups while the user is composing"
    );

    let hidden: bool = env
        .eval("return IMECandidatesFrame:IsShown() == false")
        .expect("IMECandidatesFrame:IsShown() probe should succeed");
    assert!(
        hidden,
        "IMECandidatesFrame declares `hidden=\"true\"` (IME.xml:26) — must start hidden until \
         the platform IME bridge fires a composition-start event and Show()s the frame"
    );
}
}

prefork_full_ui_case! {
fn blizzard_ime_named_frame_carries_ten_candidate_children(env: &WowLuaEnv) {

    for parent_key in CANDIDATE_PARENT_KEYS {
        let kind: String = env
            .eval(&format!("return type(IMECandidatesFrame['{parent_key}'])"))
            .unwrap_or_else(|err| panic!("IMECandidatesFrame.{parent_key} probe failed: {err}"));
        assert_eq!(
            kind, "table",
            "IMECandidatesFrame.{parent_key} must publish as a parentKey child Frame inheriting \
             IMECandidate (IME.xml:31-80). The 10 fixed candidate slots c1..c10 form a \
             vertical stack (each anchored TOPLEFT relativePoint=BOTTOMLEFT to the previous), \
             matching the maximum candidate window size the platform IME bridge emits per \
             composition step"
        );
    }
}
}

prefork_full_ui_case! {
fn blizzard_ime_candidate_children_carry_label_and_candidate_fontstrings(env: &WowLuaEnv) {

    let label_kind: String = env
        .eval("return type(IMECandidatesFrame.c1.label)")
        .expect("IMECandidatesFrame.c1.label probe should succeed");
    assert_eq!(
        label_kind, "table",
        "IMECandidatesFrame.c1.label must publish as a parentKey FontString inherited from \
         IMECandidate (IME.xml:9). The `label` text holds the index character (1..10) the \
         user types to select that candidate; uses System_IME font for proper CJK rendering"
    );

    let candidate_kind: String = env
        .eval("return type(IMECandidatesFrame.c1.candidate)")
        .expect("IMECandidatesFrame.c1.candidate probe should succeed");
    assert_eq!(
        candidate_kind, "table",
        "IMECandidatesFrame.c1.candidate must publish as a parentKey FontString inherited from \
         IMECandidate (IME.xml:15). The `candidate` text holds the actual UTF-8 candidate \
         word/character emitted by the platform IME for the current composition step"
    );
}
}

prefork_full_ui_case! {
fn blizzard_ime_named_frame_carries_background_selection_and_reading_layers(env: &WowLuaEnv) {

    for (parent_key, label, layer_rationale) in [
        (
            "background",
            "background Texture",
            "BACKGROUND layer — semi-transparent black (rgba=0,0,0,0.9) inset 3px on every side \
             to draw the candidate panel body behind the TooltipBorderBackdropTemplate edge \
             frame",
        ),
        (
            "selection",
            "selection Texture",
            "ARTWORK layer — gray highlight (rgba=0.6,0.6,0.6,0.2) the IME bridge SetPoints \
             over the currently-highlighted candidate row to indicate which slot the next Tab \
             / arrow press will commit",
        ),
        (
            "reading",
            "reading FontString",
            "OVERLAY layer — System_IME font, anchored LEFT 15,0; holds the in-progress \
             romaji / pinyin reading string above the candidate slots while the user is still \
             composing",
        ),
    ] {
        let kind: String = env
            .eval(&format!("return type(IMECandidatesFrame['{parent_key}'])"))
            .unwrap_or_else(|err| panic!("IMECandidatesFrame.{parent_key} probe failed: {err}"));
        assert_eq!(
            kind, "table",
            "IMECandidatesFrame.{parent_key} ({label}) must publish — {layer_rationale}"
        );
    }
}
}

#[test]
fn blizzard_ime_addon_ships_no_lua_files_at_all() {
    let lua_files: Vec<_> = std::fs::read_dir(ime_dir())
        .expect("Blizzard_IME directory should read")
        .flatten()
        .filter(|e| {
            e.path()
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|s| s.eq_ignore_ascii_case("lua"))
                .unwrap_or(false)
        })
        .collect();
    assert!(
        lua_files.is_empty(),
        "Blizzard_IME must ship zero `.lua` files — the addon is pure XML widget definition. \
         The platform IME bridge drives every visual update from C++ via the engine's IME \
         interop layer, so no Lua mixin is required. Files found: {lua_files:?}"
    );
}
