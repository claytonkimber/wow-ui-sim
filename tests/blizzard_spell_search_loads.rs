use std::path::PathBuf;

use wow_ui_sim::loader::{discover_blizzard_addons_for_screen, load_addon};
use wow_ui_sim::lua_api::WowLuaEnv;
use wow_ui_sim::screen::ScreenKind;
use wow_ui_sim::toc::TocFile;

fn blizzard_ui_dir() -> PathBuf {
    wow_ui_sim::paths::default_blizzard_ui_addons_path().expect("Blizzard UI cache should be available")
}

fn spell_search_dir() -> PathBuf {
    blizzard_ui_dir().join("Blizzard_SpellSearch")
}

fn spell_search_toc() -> PathBuf {
    spell_search_dir().join("Blizzard_SpellSearch.toc")
}

const PUBLISHED_MIXINS: &[&str] = &[
    "SpellSearchSourceMixin",
    "TraitSearchSourceMixin",
    "PvPTalentsSearchSourceMixin",
    "SpellBookItemSearchSourceMixin",
    "BaseSpellSearchFilterMixin",
    "SpellSearchTextFilterMixin",
    "SpellSearchNameFilterMixin",
    "SpellSearchActionBarFilterMixin",
    "SpellSearchAssistedCombatFilterMixin",
    "SpellSearchControllerMixin",
    "SpellSearchPreviewResultMixin",
    "SpellSearchPreviewContainerMixin",
    "SpellSearchBoxMixin",
];

const VIRTUAL_TEMPLATES: &[(&str, &str)] = &[
    ("SpellSearchPreviewResultTemplate", "Button"),
    ("SpellSearchSuggestedResultButtonTemplate", "Button"),
    ("SpellSearchPreviewContainerTemplate", "Frame"),
    ("SpellSearchBoxTemplate", "EditBox"),
];

#[test]
fn toc_declares_load_on_demand_with_no_dependencies() {
    let toc = TocFile::from_file(&spell_search_toc()).expect("SpellSearch TOC parses");

    assert!(
        toc.is_load_on_demand(),
        "`## LoadOnDemand: 1` MUST resolve to is_load_on_demand() == true. \
         Blizzard_SpellSearch is summoned by SharedTalentUI / SpellBook / \
         PvPTalents callers via `C_AddOns.LoadAddOn('Blizzard_SpellSearch')` \
         when those panels first show their search box, so the addon stays \
         out of the eager Game-screen sweep until needed"
    );

    assert!(
        toc.dependencies().is_empty(),
        "Blizzard_SpellSearch declares NO `## Dependencies:` directive — \
         the addon is self-contained: SpellSearchUtil only references the \
         eagerly-available ActionButtonUtil global (published by \
         Blizzard_FrameXML) and standard C_Spell / C_SpellBook / \
         C_SpecializationInfo namespaces. Got {:?}",
        toc.dependencies()
    );
}

#[test]
fn toc_omits_optional_metadata_directives() {
    let toc = TocFile::from_file(&spell_search_toc()).expect("SpellSearch TOC parses");

    assert!(!toc.is_load_first());
    assert!(!toc.is_secure_env());
    assert!(toc.saved_variables().is_empty());
    assert!(toc.optional_deps().is_empty());
    assert!(
        toc.default_enabled(),
        "SpellSearch defaults to enabled (no `## DefaultState:` directive)"
    );
    assert!(
        !toc.is_game_type_restricted(),
        "Absent `## AllowLoadGameType:` means is_game_type_restricted() == \
         false — search UI is reused across mainline talent/spellbook/PvP \
         panels with no flavor lockout"
    );
}

#[test]
fn toc_raw_bytes_pin_two_metadata_directives_only() {
    let raw = std::fs::read_to_string(spell_search_toc()).expect("TOC reads utf-8");

    let expected_directives = ["## Title: Blizzard Spell Search", "## LoadOnDemand: 1"];

    for directive in expected_directives {
        assert!(
            raw.contains(directive),
            "Raw bytes MUST pin `{directive}` — SpellSearch TOC is the \
             smallest LoD addon in this batch (only 2 metadata lines + 10 \
             body entries). Note `## Title: Blizzard Spell Search` uses \
             SPACE separators (not the underscore-form `Blizzard_SpellSearch` \
             — display title differs from addon directory name)"
        );
    }

    assert!(!raw.contains("## Author"));
    assert!(!raw.contains("## Dependencies"));
    assert!(!raw.contains("## RequiredDep"));
    assert!(!raw.contains("## OptionalDep"));
    assert!(!raw.contains("## SavedVariables"));
    assert!(!raw.contains("## AllowLoad:"));
    assert!(!raw.contains("## AllowLoadGameType"));
    assert!(!raw.contains("## DefaultState"));
    assert!(!raw.contains("## LoadFirst"));
    assert!(!raw.contains("## UseSecureEnvironment"));
    assert!(!raw.contains("## Version"));
}

#[test]
fn body_resolves_to_ten_entries_in_canonical_order() {
    let toc = TocFile::from_file(&spell_search_toc()).expect("TOC parses");

    let body: Vec<String> = toc
        .files
        .iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect();

    let expected = [
        "Blizzard_SpellSearchUtil.lua",
        "Blizzard_SpellSearchSource.lua",
        "Blizzard_SpellSearchFilter.lua",
        "Blizzard_SpellSearchTextFilter.lua",
        "Blizzard_SpellSearchNameFilter.lua",
        "Blizzard_SpellSearchActionBarFilter.lua",
        "Blizzard_SpellSearchAssistedCombatFilter.lua",
        "Blizzard_SpellSearchController.lua",
        "Blizzard_SpellSearchTemplates.lua",
        "Blizzard_SpellSearchTemplates.xml",
    ];

    assert_eq!(
        body.len(),
        expected.len(),
        "Body must contain exactly 10 entries — 9 Lua files (Util, Source, \
         base Filter, 4 derived filters Text/Name/ActionBar/AssistedCombat, \
         Controller, Templates) + 1 XML (Templates.xml). Got: {body:?}"
    );

    for (i, want) in expected.iter().enumerate() {
        assert_eq!(
            &body[i], want,
            "Body entry {i}: expected {want}, got {}",
            body[i]
        );
    }
}

#[test]
fn util_lua_loads_first_to_publish_match_type_enums_before_consumers() {
    let toc = TocFile::from_file(&spell_search_toc()).expect("TOC parses");

    let first = toc
        .files
        .first()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();

    assert_eq!(
        first, "Blizzard_SpellSearchUtil.lua",
        "Blizzard_SpellSearchUtil.lua MUST be FIRST in the body — it \
         publishes the `SpellSearchUtil` namespace table at line 1 \
         (`SpellSearchUtil = {{}}`) along with its three enum subtables \
         (MatchType, SourceType, FilterType) and the \
         ActionBarStatusTooltips / ActionBarStatusMatchTypes lookup \
         tables. Every later Filter/Source/Controller file dereferences \
         SpellSearchUtil.SourceType.Trait / SpellSearchUtil.MatchType.* / \
         SpellSearchUtil.FilterType.* — so Util must run before any of them"
    );
}

#[test]
fn source_lua_loads_before_filter_lua_for_source_type_enum_use() {
    let toc = TocFile::from_file(&spell_search_toc()).expect("TOC parses");

    let body: Vec<String> = toc
        .files
        .iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect();

    let source_idx = body
        .iter()
        .position(|f| f == "Blizzard_SpellSearchSource.lua")
        .expect("Source.lua present");
    let filter_idx = body
        .iter()
        .position(|f| f == "Blizzard_SpellSearchFilter.lua")
        .expect("Filter.lua present");
    let controller_idx = body
        .iter()
        .position(|f| f == "Blizzard_SpellSearchController.lua")
        .expect("Controller.lua present");

    assert!(
        source_idx < filter_idx,
        "Source.lua must precede Filter.lua — Filter.lua's \
         BaseSpellSearchFilterMixin:GetAggregateMatchResults dereferences \
         SpellSearchUtil.SourceType.Trait/PvPTalent/SpellBookItem to key \
         resultsBySourceType, and InternalGetMatchType* helpers route by \
         the same enum. Source.lua publishes the parent SpellSearchSourceMixin \
         table that these depend on for source-type-keyed dispatch"
    );

    assert!(
        filter_idx < controller_idx,
        "Filter.lua + 4 derived filter files must precede Controller.lua \
         — SpellSearchControllerMixin:Init at line 19-22 calls \
         CreateAndInitFromMixin(SpellSearchTextFilterMixin, ...) etc., so \
         the 4 derived filter mixins must be published before the \
         controller's Init runs"
    );
}

#[test]
fn templates_xml_is_last_for_mixin_resolution() {
    let toc = TocFile::from_file(&spell_search_toc()).expect("TOC parses");

    let body: Vec<String> = toc
        .files
        .iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect();
    let last = body.last().expect("body non-empty").clone();

    assert_eq!(
        last, "Blizzard_SpellSearchTemplates.xml",
        "Templates.xml MUST be LAST — its <Frame mixin=\"...\"> attributes \
         (SpellSearchPreviewResultMixin, SpellSearchPreviewContainerMixin, \
         SpellSearchBoxMixin) are resolved at template-registration time \
         against globals that Templates.lua publishes one entry earlier. \
         Templates.lua MUST come immediately before so its three mixin \
         tables exist when XML scanning binds them"
    );
}

#[test]
fn spell_search_pulls_into_game_screen_via_shared_talent_ui_dep() {
    let addons = discover_blizzard_addons_for_screen(&blizzard_ui_dir(), ScreenKind::Game);
    let found = addons
        .iter()
        .any(|(name, _)| name == "Blizzard_SpellSearch");

    assert!(
        found,
        "Blizzard_SpellSearch (LoadOnDemand=1) MUST appear in Game-screen \
         auto-discovery despite the LoD flag. \
         Blizzard_SharedTalentUI/Blizzard_SharedTalentUI.toc declares \
         `## LoadOnDemand: 0` AND `## Dependencies: Blizzard_SpellSearch`, \
         so pull_required_lod_addons (src/loader/mod.rs:553) drains \
         SpellSearch out of the LoD pool and into the eager-load set. \
         This is the documented behavior for LoD addons that are listed \
         as deps of non-LoD addons — auto-discovery must not silently \
         drop a declared dependency just because the dep is itself LoD"
    );
}

#[test]
fn spell_search_does_not_auto_discover_on_glue_screens() {
    let glue_screens = [
        ScreenKind::Login,
        ScreenKind::CharacterSelect,
        ScreenKind::CharacterCreate,
    ];

    for screen in glue_screens {
        let addons = discover_blizzard_addons_for_screen(&blizzard_ui_dir(), screen);
        let found = addons
            .iter()
            .any(|(name, _)| name == "Blizzard_SpellSearch");
        assert!(
            !found,
            "Blizzard_SpellSearch must NOT appear in auto-discovery for \
             glue screen {screen:?}. SpellSearch's TOC has no \
             `## AllowLoad: glue/both` directive, so the None-branch at \
             src/toc.rs:307 restricts it to ScreenKind::Game. \
             SharedTalentUI is also Game-only, so the dep-pull that \
             promotes SpellSearch on Game has no analogue on the glue \
             screens"
        );
    }
}

prefork_full_ui_case! {
fn explicit_load_addon_succeeds_with_no_addon_specific_lua_errors(env: &WowLuaEnv) {

    {
        let mut state = env.state().borrow_mut();
        state.lua_errors.clear();
        state.lua_error_records.clear();
        state.lua_error_counts.clear();
    }

    load_addon(&env.loader_env(), &spell_search_toc())
        .expect("Blizzard_SpellSearch should load via Rust loader");

    let load_errors: Vec<String> = env.state().borrow().lua_errors.clone();
    let needles = ["SpellSearch", "BaseSpellSearchFilter", "SpellSearchUtil"];
    let matched: Vec<&String> = load_errors
        .iter()
        .filter(|e| needles.iter().any(|n| e.contains(n)))
        .collect();

    assert!(
        matched.is_empty(),
        "Explicit load_addon for Blizzard_SpellSearch must emit zero \
         addon-specific Lua errors. Found {} matching errors:\n{:#?}",
        matched.len(),
        matched
    );
}
}

prefork_full_ui_case! {
fn is_addon_loaded_reports_true_after_explicit_load(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &spell_search_toc()).expect("Blizzard_SpellSearch should load");

    let loaded: bool = env
        .eval("return C_AddOns.IsAddOnLoaded('Blizzard_SpellSearch')")
        .expect("IsAddOnLoaded query");
    assert!(
        loaded,
        "After explicit load_addon, \
         C_AddOns.IsAddOnLoaded('Blizzard_SpellSearch') must return true \
         — confirms the loader registered the LoD addon name even though \
         it didn't surface in the eager Game-screen sweep"
    );
}
}

prefork_full_ui_case! {
fn util_publishes_namespace_with_three_enum_subtables(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &spell_search_toc()).expect("Blizzard_SpellSearch should load");

    let probe = "return type(SpellSearchUtil) == 'table' and \
                 type(SpellSearchUtil.MatchType) == 'table' and \
                 type(SpellSearchUtil.SourceType) == 'table' and \
                 type(SpellSearchUtil.FilterType) == 'table' and \
                 type(SpellSearchUtil.ActionBarStatusTooltips) == 'table' and \
                 type(SpellSearchUtil.ActionBarStatusMatchTypes) == 'table'";
    let ok: bool = env.eval(probe).expect("SpellSearchUtil namespace probe");
    assert!(
        ok,
        "Util.lua MUST publish the SpellSearchUtil namespace table with \
         3 enum subtables (MatchType, SourceType, FilterType) plus 2 lookup \
         tables (ActionBarStatusTooltips, ActionBarStatusMatchTypes). The \
         enum tables are the source-of-truth for how every Filter/Source/ \
         Controller method routes its work"
    );
}
}

prefork_full_ui_case! {
fn match_type_enum_values_pin_eight_canonical_entries(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &spell_search_toc()).expect("Blizzard_SpellSearch should load");

    let probe = "return SpellSearchUtil.MatchType.DescriptionMatch == 1 and \
                 SpellSearchUtil.MatchType.NameMatch == 2 and \
                 SpellSearchUtil.MatchType.RelatedMatch == 3 and \
                 SpellSearchUtil.MatchType.ExactMatch == 4 and \
                 SpellSearchUtil.MatchType.NotOnActionBar == 5 and \
                 SpellSearchUtil.MatchType.OnInactiveBonusBar == 6 and \
                 SpellSearchUtil.MatchType.OnDisabledActionBar == 7 and \
                 SpellSearchUtil.MatchType.AssistedCombat == 8";
    let ok: bool = env.eval(probe).expect("MatchType enum probe");
    assert!(
        ok,
        "MatchType MUST pin 8 canonical numeric values 1..8 — these are \
         compared in DefaultResultSort (Filter.lua:12-18) where larger \
         value = higher priority result. ExactMatch=4 outranks NameMatch=2 \
         which outranks DescriptionMatch=1; ActionBar group 5/6/7 reflects \
         escalating action-bar absence severity; AssistedCombat=8 is the \
         strongest match for assisted-combat-rotation auto-pick"
    );
}
}

prefork_full_ui_case! {
fn source_and_filter_type_enums_pin_canonical_values(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &spell_search_toc()).expect("Blizzard_SpellSearch should load");

    let probe = "return SpellSearchUtil.SourceType.Trait == 1 and \
                 SpellSearchUtil.SourceType.PvPTalent == 2 and \
                 SpellSearchUtil.SourceType.SpellBookItem == 3 and \
                 SpellSearchUtil.FilterType.Text == 1 and \
                 SpellSearchUtil.FilterType.ActionBar == 2 and \
                 SpellSearchUtil.FilterType.Name == 3 and \
                 SpellSearchUtil.FilterType.AssistedCombat == 4";
    let ok: bool = env.eval(probe).expect("SourceType/FilterType probe");
    assert!(
        ok,
        "SourceType MUST pin 3 entries (Trait=1, PvPTalent=2, \
         SpellBookItem=3) and FilterType MUST pin 4 entries (Text=1, \
         ActionBar=2, Name=3, AssistedCombat=4). These index the \
         Controller's searchSources / searchFilters tables and the \
         Filter mixin's resultsBySourceType cache"
    );
}
}

prefork_full_ui_case! {
fn util_publishes_canonical_helper_functions(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &spell_search_toc()).expect("Blizzard_SpellSearch should load");

    let probe = "return type(SpellSearchUtil.DoStringsMatch) == 'function' and \
                 type(SpellSearchUtil.DoesStringContain) == 'function' and \
                 type(SpellSearchUtil.GetTooltipForActionBarStatus) == 'function' and \
                 type(SpellSearchUtil.GetActionbarStatusForSpell) == 'function' and \
                 type(SpellSearchUtil.GetActionbarStatusForSpellBookItem) == 'function' and \
                 type(SpellSearchUtil.GetActionbarStatusForSpellBookItemInfo) == 'function' and \
                 type(SpellSearchUtil.GetActionBarStatusForTraitNode) == 'function' and \
                 type(SpellSearchUtil.GetActionBarStatusForTraitNodeEntry) == 'function' and \
                 type(SpellSearchUtil.IsActionBarMatchType) == 'function'";
    let ok: bool = env.eval(probe).expect("Util helper probe");
    assert!(
        ok,
        "Util.lua MUST publish 9 module functions: DoStringsMatch \
         (case-insensitive utf8 compare via strcmputf8i), DoesStringContain \
         (lowercase substring find), GetTooltipForActionBarStatus + \
         GetActionbarStatusForSpell + GetActionbarStatusForSpellBookItem + \
         GetActionbarStatusForSpellBookItemInfo + \
         GetActionBarStatusForTraitNode + GetActionBarStatusForTraitNodeEntry \
         (action-bar-presence helpers that route to ActionButtonUtil), \
         IsActionBarMatchType (3-way OR over the bar-absence MatchTypes)"
    );
}
}

prefork_full_ui_case! {
fn publishes_thirteen_mixin_tables(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &spell_search_toc()).expect("Blizzard_SpellSearch should load");

    for mixin in PUBLISHED_MIXINS {
        let probe = format!("return type({mixin}) == 'table'");
        let ok: bool = env
            .eval(&probe)
            .unwrap_or_else(|err| panic!("mixin probe ({mixin}): {err}"));
        assert!(
            ok,
            "Mixin {mixin} MUST publish as a global table after SpellSearch \
             load. The 13 mixins span the 4-layer architecture: \
             SpellSearchSourceMixin + 3 derived (Trait/PvPTalents/ \
             SpellBookItem) for the data-getter layer; \
             BaseSpellSearchFilterMixin + 4 derived (Text/Name/ActionBar/ \
             AssistedCombat) for the per-strategy match-type computation; \
             SpellSearchControllerMixin for the orchestration layer; and \
             3 template-binding mixins (PreviewResult, PreviewContainer, \
             SearchBox) consumed by the XML mixin=\"...\" attributes"
        );
    }
}
}

prefork_full_ui_case! {
fn derived_filter_mixins_inherit_base_filter_methods(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &spell_search_toc()).expect("Blizzard_SpellSearch should load");

    let probe = "return type(SpellSearchTextFilterMixin.GetIsEnabled) == 'function' and \
                 type(SpellSearchTextFilterMixin.GetIsActive) == 'function' and \
                 type(SpellSearchTextFilterMixin.SetEnabled) == 'function' and \
                 type(SpellSearchTextFilterMixin.GetAggregateMatchResults) == 'function' and \
                 type(SpellSearchNameFilterMixin.GetIsEnabled) == 'function' and \
                 type(SpellSearchNameFilterMixin.GetAggregateMatchResults) == 'function' and \
                 type(SpellSearchActionBarFilterMixin.GetIsEnabled) == 'function' and \
                 type(SpellSearchActionBarFilterMixin.GetAggregateMatchResults) == 'function' and \
                 type(SpellSearchAssistedCombatFilterMixin.GetIsEnabled) == 'function' and \
                 type(SpellSearchAssistedCombatFilterMixin.GetAggregateMatchResults) == 'function'";
    let ok: bool = env.eval(probe).expect("derived filter inheritance probe");
    assert!(
        ok,
        "All 4 derived filter mixins (Text/Name/ActionBar/AssistedCombat) \
         are declared via `CreateFromMixins(BaseSpellSearchFilterMixin)` so \
         each one MUST inherit the base method surface — GetIsEnabled / \
         GetIsActive / SetEnabled / GetAggregateMatchResults at minimum. \
         Without the inheritance, Controller's IsFilterEnabled / \
         GetActiveSearchFilter dispatch would throw on every filter type"
    );
}
}

prefork_full_ui_case! {
fn derived_source_mixins_inherit_get_source_type(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &spell_search_toc()).expect("Blizzard_SpellSearch should load");

    let probe = "return type(TraitSearchSourceMixin.GetSourceType) == 'function' and \
                 type(PvPTalentsSearchSourceMixin.GetSourceType) == 'function' and \
                 type(SpellBookItemSearchSourceMixin.GetSourceType) == 'function'";
    let ok: bool = env.eval(probe).expect("derived source inheritance probe");
    assert!(
        ok,
        "All 3 derived source mixins (Trait/PvPTalents/SpellBookItem) are \
         declared via `CreateFromMixins(SpellSearchSourceMixin)` so each \
         MUST inherit the base method GetSourceType. Source.lua uses \
         self.sourceType assignment in each derived Init to set the enum \
         tag that GetSourceType returns — this is how filters route \
         resultsBySourceType lookups"
    );
}
}

prefork_full_ui_case! {
fn controller_publishes_fifteen_orchestration_methods(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &spell_search_toc()).expect("Blizzard_SpellSearch should load");

    let probe = "return type(SpellSearchControllerMixin.Init) == 'function' and \
                 type(SpellSearchControllerMixin.IsInitialized) == 'function' and \
                 type(SpellSearchControllerMixin.IsFilterEnabled) == 'function' and \
                 type(SpellSearchControllerMixin.SetFilterDisabled) == 'function' and \
                 type(SpellSearchControllerMixin.RunFilterOnce) == 'function' and \
                 type(SpellSearchControllerMixin.ActivateSearchFilter) == 'function' and \
                 type(SpellSearchControllerMixin.GetMatchTypeForSourceTypeEntry) == 'function' and \
                 type(SpellSearchControllerMixin.UpdateEnabledSearchTypes) == 'function' and \
                 type(SpellSearchControllerMixin.GetActiveSearchFilter) == 'function' and \
                 type(SpellSearchControllerMixin.GetActiveSearchFilterType) == 'function' and \
                 type(SpellSearchControllerMixin.UpdateActiveSearchResults) == 'function' and \
                 type(SpellSearchControllerMixin.GetActiveSearchResults) == 'function' and \
                 type(SpellSearchControllerMixin.ClearActiveSearchResults) == 'function' and \
                 type(SpellSearchControllerMixin.GetActiveSearchResultsSorter) == 'function' and \
                 type(SpellSearchControllerMixin.GetSearchSourceByType) == 'function'";
    let ok: bool = env.eval(probe).expect("controller method probe");
    assert!(
        ok,
        "SpellSearchControllerMixin MUST publish all 15 orchestration \
         methods. Init seeds searchSources/searchFilters tables (Text+Name \
         enabled by default, ActionBar+AssistedCombat opt-in). \
         RunFilterOnce / ActivateSearchFilter drive the active-filter \
         state machine. GetMatchTypeForSourceTypeEntry is the per-spell \
         hot-path that talent/spellbook UIs call to colorize matched rows"
    );
}
}

prefork_full_ui_case! {
fn preview_result_mixin_publishes_nine_methods(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &spell_search_toc()).expect("Blizzard_SpellSearch should load");

    let probe = "return type(SpellSearchPreviewResultMixin.Init) == 'function' and \
                 type(SpellSearchPreviewResultMixin.SetHighlighted) == 'function' and \
                 type(SpellSearchPreviewResultMixin.OnClick) == 'function' and \
                 type(SpellSearchPreviewResultMixin.OnEnter) == 'function' and \
                 type(SpellSearchPreviewResultMixin.OnShow) == 'function' and \
                 type(SpellSearchPreviewResultMixin.GetIndex) == 'function' and \
                 type(SpellSearchPreviewResultMixin.GetResultID) == 'function' and \
                 type(SpellSearchPreviewResultMixin.GetResultType) == 'function' and \
                 type(SpellSearchPreviewResultMixin.GetResultInfo) == 'function'";
    let ok: bool = env.eval(probe).expect("preview result method probe");
    assert!(
        ok,
        "SpellSearchPreviewResultMixin MUST publish 9 methods bound to \
         SpellSearchPreviewResultTemplate's Scripts block (OnClick/OnEnter/ \
         OnShow) plus the Init data-binding entry point (sets Name/Icon/ \
         HighlightTexture from elementData) and 4 result accessors that \
         the parent ScrollBox dispatches through"
    );
}
}

prefork_full_ui_case! {
fn search_box_mixin_publishes_eight_input_methods(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &spell_search_toc()).expect("Blizzard_SpellSearch should load");

    let probe = "return type(SpellSearchBoxMixin.OnLoad) == 'function' and \
                 type(SpellSearchBoxMixin.OnTextChanged) == 'function' and \
                 type(SpellSearchBoxMixin.OnKeyDown) == 'function' and \
                 type(SpellSearchBoxMixin.OnEnterPressed) == 'function' and \
                 type(SpellSearchBoxMixin.OnFocusLost) == 'function' and \
                 type(SpellSearchBoxMixin.OnFocusGained) == 'function' and \
                 type(SpellSearchBoxMixin.SetSearchText) == 'function' and \
                 type(SpellSearchBoxMixin.EvaluateSearchText) == 'function'";
    let ok: bool = env.eval(probe).expect("search box method probe");
    assert!(
        ok,
        "SpellSearchBoxMixin MUST publish the 8 input-handler methods bound \
         to SpellSearchBoxTemplate's Scripts block. OnKeyDown routes UP/ \
         DOWN to the preview container's CycleHighlightedResult{{Up,Down}}. \
         OnEnterPressed first tries SelectHighlightedResult, falls back \
         to UpdateFullResults. EvaluateSearchText returns nil for input \
         shorter than MIN_CHARACTER_SEARCH"
    );
}
}

prefork_full_ui_case! {
fn xml_registers_all_four_virtual_templates(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &spell_search_toc()).expect("Blizzard_SpellSearch should load");

    for (template, widget_type) in VIRTUAL_TEMPLATES {
        let probe = format!(
            "local ok, frame = pcall(function() \
                return CreateFrame({widget_type:?}, nil, UIParent, {template:?}) \
             end) \
             return ok and frame ~= nil"
        );
        let ok: bool = env
            .eval(&probe)
            .unwrap_or_else(|err| panic!("template probe ({template}): {err}"));
        assert!(
            ok,
            "Virtual template {template} (registered by Templates.xml) must \
             materialize via CreateFrame as widget_type {widget_type:?}. \
             The 4 templates form the search panel composition: \
             SpellSearchPreviewResultTemplate (Button 176x27 row inside \
             ScrollBox, atlas search-rowbg with HighlightTexture / \
             IconFrame / Icon / Name children); \
             SpellSearchSuggestedResultButtonTemplate (Button 176x27, \
             hidden by default, used as fallback when no real results exist); \
             SpellSearchPreviewContainerTemplate (Frame holding ScrollBox + \
             OverflowCount, maximumEntries KeyValue=3); \
             SpellSearchBoxTemplate (EditBox inheriting SearchBoxTemplate, \
             letters=40, parentKey SearchBox)"
        );
    }
}
}

prefork_full_ui_case! {
fn preview_result_template_exposes_parent_key_children(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &spell_search_toc()).expect("Blizzard_SpellSearch should load");

    let probe = "local row = CreateFrame('Button', nil, UIParent, \
                                          'SpellSearchPreviewResultTemplate') \
                 if not row then return 'row nil' end \
                 if not row.HighlightTexture then return 'no HighlightTexture' end \
                 if not row.IconFrame then return 'no IconFrame' end \
                 if not row.Icon then return 'no Icon' end \
                 if not row.Name then return 'no Name' end \
                 return 'OK'";
    let report: String = env
        .eval(probe)
        .expect("preview result parentKey children probe");
    assert_eq!(
        report, "OK",
        "SpellSearchPreviewResultTemplate's <Layers> declare 4 parentKey \
         children that the mixin's Init/SetHighlighted methods reference: \
         HighlightTexture (atlas search-highlight, hidden=true at start), \
         IconFrame (atlas talents-search-suggestion-itemborder), Icon \
         (the spell icon, anchored inside IconFrame with 1px inset), \
         Name (GameFontHighlightSmall FontString anchored to the Icon's \
         right edge). Without these four, Init's self.Name:SetText / \
         self.Icon:SetTexture / self.HighlightTexture:SetShown would \
         throw on every row binding"
    );
}
}

prefork_full_ui_case! {
fn preview_container_template_keyvalue_pins_maximum_entries(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &spell_search_toc()).expect("Blizzard_SpellSearch should load");

    let probe = "local container = CreateFrame('Frame', nil, UIParent, \
                                                'SpellSearchPreviewContainerTemplate') \
                 if not container then return 'nil' end \
                 return tostring(container.maximumEntries)";
    let report: String = env.eval(probe).expect("preview container KeyValue probe");
    assert_eq!(
        report, "3",
        "SpellSearchPreviewContainerTemplate's <KeyValues> block declares \
         `maximumEntries=3 type=number` — this caps the dropdown preview \
         to 3 rows before the OverflowCount text bar takes over with \
         TALENT_FRAME_SEARCH_PREVIEW_OVERFLOW_FORMAT for the remaining \
         count. SetPreviewResults (Templates.lua:82-112) reads \
         self.maximumEntries to gate displayedCount before inserting \
         additional results into the DataProvider. Got: {report}"
    );
}
}
