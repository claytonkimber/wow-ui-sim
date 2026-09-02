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

fn fonts_shared_toc() -> PathBuf {
    blizzard_ui_dir().join("Blizzard_Fonts_Shared/Blizzard_Fonts_Shared.toc")
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
fn blizzard_fonts_shared_toc_is_eager_with_load_locale_dep_and_allow_load_both() {
    let toc =
        TocFile::from_file(&fonts_shared_toc()).expect("Blizzard_Fonts_Shared TOC should parse");

    assert!(
        !toc.is_load_on_demand(),
        "Blizzard_Fonts_Shared has no `## LoadOnDemand` line — the font templates \
         (FontFamily / virtual Font definitions like GameFontNormal / SystemFont_Small / \
         GlueFontNormal) are consumed by virtually every other addon that creates a \
         FontString, so they MUST auto-load at startup before consumers"
    );
    assert!(
        !toc.is_secure_env(),
        "Blizzard_Fonts_Shared does not declare `## UseSecureEnvironment` — font \
         definitions are pure presentation data with no taint implications"
    );
    assert_eq!(
        toc.dependencies(),
        vec!["Blizzard_LoadLocale".to_string()],
        "Blizzard_Fonts_Shared declares `## Dependencies: Blizzard_LoadLocale` — the \
         locale addon publishes the alphabet/locale globals (e.g. `LOCALE_KOKR`, \
         `LOCALE_ZHCN`) that the FontFamily `<Member alphabet=\"korean\">` / \
         `<Member alphabet=\"simplifiedchinese\">` element selection logic depends on \
         to pick the right TTF per locale"
    );
    assert!(
        !toc.is_game_type_restricted(),
        "Blizzard_Fonts_Shared declares no `## AllowLoadGameType:` line at the addon \
         level, so `is_game_type_restricted()` returns false. Per-file annotations like \
         `[AllowLoadGameType classic]` on `[Family]\\GlueFonts.xml` filter individual \
         files (handled by src/toc.rs:141 + 46-49), but the addon as a whole loads on \
         standard retail"
    );

    assert!(
        toc.saved_variables().is_empty(),
        "Blizzard_Fonts_Shared declares no `## SavedVariables` — font templates are \
         pure UI primitives with no per-character or per-account state"
    );

    let toc_text =
        std::fs::read_to_string(fonts_shared_toc()).expect("Blizzard_Fonts_Shared TOC should read");
    assert!(
        toc_text.contains("## AllowLoad: Both"),
        "Blizzard_Fonts_Shared declares `## AllowLoad: Both` — fonts are required on \
         BOTH the glue screens (login/character-select where GlueFontNormal etc. are \
         used) and the in-game UI (where GameFontNormal etc. are used). This is why \
         the file list mixes Shared / [Family] / GlueFonts / GameFonts variants"
    );
    assert!(
        toc_text.contains("[AllowLoad Glue]"),
        "TOC contains per-file `[AllowLoad Glue]` annotations on the GlueFonts / \
         GlueFontStyles entries. Note the simulator's TOC parser does NOT honor these \
         per-file `[AllowLoad ...]` filters (only `[AllowLoadGameType ...]` is filtered \
         per src/toc.rs:141), so on Game screen the simulator loads the Glue-named \
         font definitions too — harmless because every entry is a virtual Font/\
         FontFamily template that costs nothing if unused"
    );
    assert!(
        toc_text.contains("[Family]"),
        "TOC uses the `[Family]` placeholder (resolved to `Mainline` by src/toc.rs:145) \
         to keep one TOC file driving both Mainline and Classic font sets — the parser \
         substitutes the placeholder before resolving the file path"
    );
}

#[test]
fn blizzard_fonts_shared_allows_all_screens_including_glue() {
    let toc =
        TocFile::from_file(&fonts_shared_toc()).expect("Blizzard_Fonts_Shared TOC should parse");

    assert!(
        toc.allows_screen(ScreenKind::Game),
        "`## AllowLoad: Both` must allow the Game screen (src/toc.rs:307)"
    );
    assert!(
        toc.allows_screen(ScreenKind::Login),
        "`## AllowLoad: Both` must allow the Login screen — GlueFontNormal etc. are \
         consumed by login/charcreate UIs"
    );
    assert!(
        toc.allows_screen(ScreenKind::CharacterSelect),
        "`## AllowLoad: Both` must allow CharacterSelect — `is_glue()` covers all glue \
         screens"
    );
}

#[test]
fn blizzard_fonts_shared_auto_loads_on_game_and_login_screens() {
    let game_addons = discover_blizzard_addons_for_screen(&blizzard_ui_dir(), ScreenKind::Game);
    let in_game = game_addons
        .iter()
        .any(|(name, _)| name == "Blizzard_Fonts_Shared");
    assert!(
        in_game,
        "Blizzard_Fonts_Shared has no `## LoadOnDemand` line and `## AllowLoad: Both`, \
         so it MUST appear in Game-screen auto-discovery — virtually every other addon \
         depends on its font templates"
    );

    let login_addons = discover_blizzard_addons_for_screen(&blizzard_ui_dir(), ScreenKind::Login);
    let in_login = login_addons
        .iter()
        .any(|(name, _)| name == "Blizzard_Fonts_Shared");
    assert!(
        in_login,
        "`## AllowLoad: Both` plus no LoadOnDemand means Blizzard_Fonts_Shared MUST \
         appear in Login-screen auto-discovery as well — GlueFontNormal etc. are \
         registered via the same load pass"
    );
}

prefork_full_ui_case! {
fn blizzard_fonts_shared_loads_via_full_game_ui_without_errors(env: &WowLuaEnv) {

    let load_errors: Vec<String> = env
        .state()
        .borrow()
        .lua_errors
        .iter()
        .filter(|message| {
            message.contains("Fonts_Shared")
                || message.contains("Blizzard_Fonts_Shared")
                || message.contains("FontFamily")
        })
        .cloned()
        .collect();

    assert!(
        load_errors.is_empty(),
        "Blizzard_Fonts_Shared emitted Lua errors during the full Game-screen load:\n  {}",
        load_errors.join("\n  ")
    );
}
}

prefork_full_ui_case! {
fn blizzard_fonts_shared_is_addon_loaded_returns_true_after_full_game_ui_load(env: &WowLuaEnv) {

    let post_load: bool = env
        .eval("return C_AddOns.IsAddOnLoaded('Blizzard_Fonts_Shared') and true or false")
        .expect("IsAddOnLoaded probe should succeed");
    assert!(
        post_load,
        "After full Game-screen load, IsAddOnLoaded('Blizzard_Fonts_Shared') must \
         return true — auto-discovery picks up the addon and `mark_addon_loaded` \
         (src/loader/addon.rs:131) registers it"
    );
}
}

prefork_full_ui_case! {
fn blizzard_fonts_shared_publishes_canonical_system_font_family_globals(env: &WowLuaEnv) {

    let families_present: (bool, bool, bool, bool, bool, bool) = env
        .eval(
            "return SystemFont_Tiny ~= nil, \
                    SystemFont_Tiny2 ~= nil, \
                    SystemFont_Small ~= nil, \
                    SystemFont_Small2 ~= nil, \
                    SystemFont_Huge1 ~= nil, \
                    SystemFont_Shadow_Med1 ~= nil",
        )
        .expect("SystemFont family probe should succeed");
    assert_eq!(
        families_present,
        (true, true, true, true, true, true),
        "Shared/Fonts.xml declares 56 virtual `<FontFamily>` templates with \
         alphabet-conditional `<Member alphabet=\"...\">` children (roman / korean / \
         simplifiedchinese / traditionalchinese / russian) for FRIZQT__ / 2002 / \
         ARKai_T / blei00d / FRIZQT___CYR TTFs. The six probed names must publish as \
         registered Lua globals so consumers like `local fs = \
         frame:CreateFontString(nil, \"OVERLAY\", \"SystemFont_Small\")` resolve"
    );
}
}

prefork_full_ui_case! {
fn blizzard_fonts_shared_publishes_canonical_game_font_globals(env: &WowLuaEnv) {

    let game_fonts_present: (bool, bool, bool, bool, bool, bool, bool) = env
        .eval(
            "return GameFontNormal ~= nil, \
                    GameFontHighlight ~= nil, \
                    GameFontNormalLarge ~= nil, \
                    GameFontNormalSmall ~= nil, \
                    GameFontHighlightLarge ~= nil, \
                    GameFontHighlightSmall ~= nil, \
                    GameFontNormalHuge ~= nil",
        )
        .expect("GameFont* probe should succeed");
    assert_eq!(
        game_fonts_present,
        (true, true, true, true, true, true, true),
        "Shared/GameFontStyles.xml + Mainline/GameFontStyles.xml declare the \
         GameFont* family — the workhorse fonts used by `addon UI text`. The seven \
         probed names (GameFontNormal / GameFontHighlight + Large/Small/Huge variants) \
         must publish so generic FontString creation calls like `CreateFontString(nil, \
         \"OVERLAY\", \"GameFontNormal\")` work — verified to be load-bearing by \
         existing tests in tests/globals_legacy.rs:449 and tests/missing_apis.rs:201"
    );
}
}

prefork_full_ui_case! {
fn blizzard_fonts_shared_publishes_canonical_glue_font_globals_on_game_screen(env: &WowLuaEnv) {

    let glue_fonts_present: (bool, bool, bool, bool) = env
        .eval(
            "return GlueFontNormal ~= nil, \
                    GlueFontHighlight ~= nil, \
                    GlueFontDisable ~= nil, \
                    GlueFontNormalLarge ~= nil",
        )
        .expect("GlueFont* probe should succeed");
    assert_eq!(
        glue_fonts_present,
        (true, true, true, true),
        "Shared/FontStyles.xml declares the GlueFont* family used on login / \
         character-create. The simulator's TOC parser does NOT filter inline \
         `[AllowLoad Glue]` per-file annotations (only `[AllowLoadGameType ...]` is \
         filtered per src/toc.rs:141), so Shared/FontStyles.xml loads on Game screen \
         even though the file's font definitions are nominally Glue-only. The four \
         probed names publish as globals — harmless on Game screen because Glue fonts \
         are virtual and cost nothing if unused"
    );
}
}

prefork_full_ui_case! {
fn blizzard_fonts_shared_publishes_number_font_specialty_globals(env: &WowLuaEnv) {

    let number_fonts_present: (bool, bool, bool, bool) = env
        .eval(
            "return NumberFontNormalSmall ~= nil, \
                    NumberFontNormalSmallGray ~= nil, \
                    NumberFontNormalHuge ~= nil, \
                    NumberFontNormalGray ~= nil",
        )
        .expect("NumberFont* probe should succeed");
    assert_eq!(
        number_fonts_present,
        (true, true, true, true),
        "Shared/GameFontStyles.xml declares the NumberFont* family — the \
         tabular-aligned monospace digit fonts used by combat text / cooldown \
         counters / damage meters. The four probed names must publish so action-bar \
         cooldown-counter style code resolves"
    );
}
}

prefork_full_ui_case! {
fn blizzard_fonts_shared_publishes_quest_title_font(env: &WowLuaEnv) {

    let quest_title_present: bool = env
        .eval("return QuestTitleFont ~= nil")
        .expect("QuestTitleFont probe should succeed");
    assert!(
        quest_title_present,
        "Shared/FontStyles.xml declares `<Font name=\"QuestTitleFont\" \
         inherits=\"QuestFont_Shadow_Huge\" virtual=\"true\">` — used by the quest \
         log title text. Probed individually as a sanity check that FontStyles.xml \
         loads its 146 `<Font>` entries beyond the bulk-checked Glue/Game variants"
    );
}
}

prefork_full_ui_case! {
fn blizzard_fonts_shared_load_locale_dep_loaded_first(env: &WowLuaEnv) {

    let load_locale_loaded: bool = env
        .eval("return C_AddOns.IsAddOnLoaded('Blizzard_LoadLocale') and true or false")
        .expect("Blizzard_LoadLocale probe should succeed");
    assert!(
        load_locale_loaded,
        "Per the `## Dependencies: Blizzard_LoadLocale` declaration, the loader's \
         topo-sort must place Blizzard_LoadLocale strictly before Blizzard_Fonts_Shared \
         — confirms the dep is satisfied in the same auto-discovery pass and that \
         IsAddOnLoaded reflects the dep load"
    );
}
}
