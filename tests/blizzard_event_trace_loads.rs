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

fn event_trace_toc() -> PathBuf {
    blizzard_ui_dir().join("Blizzard_EventTrace/Blizzard_EventTrace.toc")
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
fn blizzard_event_trace_toc_is_load_on_demand_with_allow_load_both_and_no_deps() {
    let toc = TocFile::from_file(&event_trace_toc()).expect("Blizzard_EventTrace TOC should parse");

    assert!(
        toc.is_load_on_demand(),
        "Blizzard_EventTrace declares `## LoadOnDemand: 1` — the EventTrace debug panel \
         is brought in on-demand by the developer's `/eventtrace` chat command (or by \
         `Blizzard_DebugTools`-style dev workflows), so it must NOT auto-load on screen \
         bring-up"
    );
    assert!(
        !toc.is_secure_env(),
        "Blizzard_EventTrace does not declare `## UseSecureEnvironment` — it runs in the \
         standard taint environment (it observes events via `RegisterAllEvents` and \
         `hooksecurefunc(EventRegistry, 'TriggerEvent', ...)`, both of which work from \
         insecure code)"
    );
    assert!(
        toc.dependencies().is_empty(),
        "Blizzard_EventTrace has no `## Dependencies` line — it is self-contained and uses \
         the standard `ButtonFrameTemplate` / `WowScrollBoxList` / `MinimalScrollBar` / \
         `WowStyle1FilterDropdownTemplate` / `SearchBoxTemplate` / `SharedTooltipTemplate` / \
         `ToolWindowOwnerMixin` / `EventRegistry` / `CreateDataProvider` / `CreateCounter` \
         surface from FrameXML"
    );
    assert!(
        !toc.is_game_type_restricted(),
        "Blizzard_EventTrace declares no `## AllowLoadGameType:` line, so \
         `is_game_type_restricted()` returns false and the addon is reachable from \
         standard-retail discovery (just gated behind `LoadOnDemand`)"
    );

    let saved_vars = toc.saved_variables();
    assert_eq!(
        saved_vars,
        vec!["EventTraceSavedVars".to_string()],
        "Blizzard_EventTrace declares exactly one `## SavedVariables: EventTraceSavedVars` \
         entry — the SavedVariables blob holds the user's filter list, panel size, and \
         show-arguments / show-timestamp / show-secret-values / log-CR-events toggles \
         (defaults at Blizzard_EventTrace.lua:33-49)"
    );

    let toc_text =
        std::fs::read_to_string(event_trace_toc()).expect("Blizzard_EventTrace TOC should read");
    assert!(
        toc_text.contains("## AllowLoad: Both"),
        "Blizzard_EventTrace declares `## AllowLoad: Both` — the panel works in both the \
         glue (login/character-select) and game environments because it observes generic \
         events that fire on both screens"
    );
    assert!(
        toc_text.contains("## ShowInDebugList: 1"),
        "Blizzard_EventTrace declares `## ShowInDebugList: 1` — surfaces the addon in the \
         developer-only Debug Tools addon list (alongside Blizzard_DebugTools / \
         Blizzard_Console)"
    );
    assert!(
        toc_text.contains("## IconTexture: Interface\\ICONS\\inv_misc_note_06"),
        "Blizzard_EventTrace declares `## IconTexture: Interface\\ICONS\\inv_misc_note_06` \
         — the addon-list icon. Recording the literal path here so a future texture rename \
         doesn't silently drop the icon"
    );
}

#[test]
fn blizzard_event_trace_allows_both_game_and_login_screens() {
    let toc = TocFile::from_file(&event_trace_toc()).expect("Blizzard_EventTrace TOC should parse");

    assert!(
        toc.allows_screen(ScreenKind::Game),
        "`## AllowLoad: Both` must allow the Game screen (src/toc.rs:307)"
    );
    assert!(
        toc.allows_screen(ScreenKind::Login),
        "`## AllowLoad: Both` must allow the Login screen (src/toc.rs:307) — distinguishes \
         this addon from the Game-only default at src/toc.rs:311"
    );
    assert!(
        toc.allows_screen(ScreenKind::CharacterSelect),
        "`## AllowLoad: Both` must allow CharacterSelect — `is_glue()` covers all glue \
         screens"
    );
}

#[test]
fn blizzard_event_trace_is_absent_from_auto_discovery_on_game_and_login() {
    let game_addons = discover_blizzard_addons_for_screen(&blizzard_ui_dir(), ScreenKind::Game);
    let in_game = game_addons
        .iter()
        .any(|(name, _)| name == "Blizzard_EventTrace");
    assert!(
        !in_game,
        "Blizzard_EventTrace is `## LoadOnDemand: 1`, so it must NOT appear in Game-screen \
         auto-discovery — it is loaded explicitly by the `/eventtrace` chat command"
    );

    let login_addons = discover_blizzard_addons_for_screen(&blizzard_ui_dir(), ScreenKind::Login);
    let in_login = login_addons
        .iter()
        .any(|(name, _)| name == "Blizzard_EventTrace");
    assert!(
        !in_login,
        "Blizzard_EventTrace is `## LoadOnDemand: 1`, so it must NOT appear in Login-screen \
         auto-discovery either — even though `## AllowLoad: Both` permits the screen, the \
         LOD gate keeps it out of the auto-load set"
    );
}

prefork_full_ui_case! {
fn blizzard_event_trace_loads_via_load_addon_without_addon_specific_errors(env: &WowLuaEnv) {

    {
        let mut state = env.state().borrow_mut();
        state.lua_errors.clear();
        state.lua_error_records.clear();
        state.lua_error_counts.clear();
    }

    load_addon(&env.loader_env(), &event_trace_toc())
        .expect("Blizzard_EventTrace should load via Rust loader");

    let load_errors: Vec<String> = env
        .state()
        .borrow()
        .lua_errors
        .iter()
        .filter(|message| {
            message.contains("EventTrace")
                || message.contains("Blizzard_EventTrace")
                || message.contains("EventTracePanel")
                || message.contains("ToolWindowOwnerMixin")
        })
        .cloned()
        .collect();

    assert!(
        load_errors.is_empty(),
        "Blizzard_EventTrace emitted addon-specific Lua errors during explicit load:\n  {}",
        load_errors.join("\n  ")
    );
}
}

prefork_full_ui_case! {
fn create_window_supports_tool_window_owner_contract(env: &WowLuaEnv) {

    let result: (bool, bool, f64, f64, bool, bool, bool) = env
        .eval(
            r#"
            local window = CreateWindow(false, true)
            local owner = CreateFrame("Frame")

            window:SetTitle("Event Trace")
            window:SetWindowSize(640, 480)
            window:SetMinSize(700, 500)
            window:SetFocus()
            window:StartMoving()
            window:StopMovingOrSizing()
            window:StartSizing()
            window:StopMovingOrSizing()

            owner:SetWindow(window)
            owner:SetAllPoints(window)

            local _, relativeTo = owner:GetPoint(1)
            local width, height = window:GetSize()
            local startsTopmost = window:IsTopmost()
            window:SetTopmost(false)
            local endsTopmost = window:IsTopmost()
            window:Close()

            return owner:GetWindow() == window,
                relativeTo == window,
                width,
                height,
                startsTopmost,
                endsTopmost,
                window:IsShown()
            "#,
        )
        .expect("CreateWindow behavior probe should evaluate");

    assert!(
        result.0,
        "SetWindow/GetWindow must preserve SimpleWindow identity"
    );
    assert!(
        result.1,
        "SetAllPoints must anchor the owner to the SimpleWindow"
    );
    assert_eq!((result.2, result.3), (700.0, 500.0));
    assert!(
        result.4,
        "topMost creation argument must initialize topmost state"
    );
    assert!(!result.5, "SetTopmost(false) must update topmost state");
    assert!(!result.6, "Close must hide the modeled external window");
}
}

prefork_full_ui_case! {
fn blizzard_event_trace_is_addon_loaded_returns_true_after_explicit_load(env: &WowLuaEnv) {

    let pre_load: bool = env
        .eval("return C_AddOns.IsAddOnLoaded('Blizzard_EventTrace') and true or false")
        .expect("IsAddOnLoaded probe should succeed");
    assert!(
        !pre_load,
        "Before the explicit load, IsAddOnLoaded('Blizzard_EventTrace') must return false \
         — confirms Game-screen auto-discovery did not load this LoadOnDemand addon"
    );

    load_addon(&env.loader_env(), &event_trace_toc())
        .expect("Blizzard_EventTrace should load via Rust loader");

    let post_load: bool = env
        .eval("return C_AddOns.IsAddOnLoaded('Blizzard_EventTrace') and true or false")
        .expect("IsAddOnLoaded probe should succeed");
    assert!(
        post_load,
        "After explicit `load_addon`, IsAddOnLoaded('Blizzard_EventTrace') must return true \
         — `mark_addon_loaded` (src/loader/addon.rs:131) registers the folder name in the \
         loaded-set"
    );
}
}

prefork_full_ui_case! {
fn blizzard_event_trace_singleton_publishes_with_correct_parent_after_load(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &event_trace_toc())
        .expect("Blizzard_EventTrace should load via Rust loader");

    let probe: (String, String) = env
        .eval("return EventTrace:GetName(), EventTrace:GetParent():GetName()")
        .expect("EventTrace name+parent probe should succeed");
    assert_eq!(
        probe,
        ("EventTrace".to_string(), "UIParent".to_string()),
        "The non-virtual `<Frame name=\"EventTrace\" parent=\"UIParent\" \
         mixin=\"EventTracePanelMixin\" inherits=\"ButtonFrameTemplate\">` (xml:168) must \
         publish as a global table whose `:GetName()` is 'EventTrace' and `:GetParent()` is \
         UIParent — proves the XML-to-widget pipeline parented the singleton correctly \
         under the in-game UIParent and not the glue-screen GlueParent"
    );
}
}

prefork_full_ui_case! {
fn blizzard_event_trace_publishes_six_mixin_globals(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &event_trace_toc())
        .expect("Blizzard_EventTrace should load via Rust loader");

    let mixins_present: (bool, bool, bool, bool, bool, bool) = env
        .eval(
            "return type(EventTraceButtonBehaviorMixin) == 'table', \
                    type(EventTraceScrollBoxButtonMixin) == 'table', \
                    type(EventTracePanelMixin) == 'table', \
                    type(EventTraceLogEventButtonMixin) == 'table', \
                    type(EventTraceLogMessageButtonMixin) == 'table', \
                    type(EventTraceFilterButtonMixin) == 'table'",
        )
        .expect("mixin-global probe should succeed");
    assert_eq!(
        mixins_present,
        (true, true, true, true, true, true),
        "Blizzard_EventTrace.lua publishes six mixin globals: \
         `EventTraceButtonBehaviorMixin` (lua:51 — OnEnter/OnLeave/SetAlternateOverlayShown), \
         `EventTraceScrollBoxButtonMixin` (lua:65 — Flash), \
         `EventTracePanelMixin = CreateFromMixins(ToolWindowOwnerMixin)` (lua:71 — the panel \
         driver: OnLoad/OnShow/OnHide/OnEvent + 50+ methods), \
         `EventTraceLogEventButtonMixin` (lua:811 — log-row renderer for events), \
         `EventTraceLogMessageButtonMixin` (lua:928 — log-row renderer for free-form \
         messages), `EventTraceFilterButtonMixin` (lua:955 — filter-list row toggle)"
    );
}
}

prefork_full_ui_case! {
fn blizzard_event_trace_panel_mixin_inherits_tool_window_owner_methods(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &event_trace_toc())
        .expect("Blizzard_EventTrace should load via Rust loader");

    let methods_present: (bool, bool) = env
        .eval(
            "return type(EventTracePanelMixin.OnSetDebugToolVisible) == 'function', \
                    type(EventTracePanelMixin.MoveToNewWindow) == 'function'",
        )
        .expect("ToolWindowOwnerMixin inheritance probe should succeed");
    assert_eq!(
        methods_present,
        (true, true),
        "`EventTracePanelMixin = CreateFromMixins(ToolWindowOwnerMixin)` (lua:71) must pull \
         in `MoveToNewWindow` from ToolWindowOwnerMixin (the panel calls \
         `self:MoveToNewWindow(EVENTTRACE_HEADER, 1000, 600, 930, 300)` from OnShow at \
         lua:124), and the addon's own `OnSetDebugToolVisible` (lua:73) must publish as a \
         function field on the mixin (it is registered as an EventRegistry callback for \
         `SET_DEBUG_TOOL_VISIBLE` from OnLoad at lua:120)"
    );
}
}

prefork_full_ui_case! {
fn blizzard_event_trace_panel_has_six_named_subframes_after_load(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &event_trace_toc())
        .expect("Blizzard_EventTrace should load via Rust loader");

    let children_present: (bool, bool, bool, bool, bool, bool) = env
        .eval(
            "return type(EventTrace.TitleBar) == 'table', \
                    type(EventTrace.ResizeButton) == 'table', \
                    type(EventTrace.SubtitleBar) == 'table', \
                    type(EventTrace.Log) == 'table', \
                    type(EventTrace.Filter) == 'table', \
                    type(EventTrace.Log.Events) == 'table'",
        )
        .expect("subframe probe should succeed");
    assert_eq!(
        children_present,
        (true, true, true, true, true, true),
        "EventTrace XML (xml:168-376) declares six parentKey-named subframes: TitleBar \
         (PanelDragBarTemplate, xml:174), ResizeButton (PanelResizeButtonTemplate, xml:183), \
         SubtitleBar (xml:188 — holds ViewLog/ViewFilter/OptionsDropdown), Log (xml:212 — \
         the events-list view), Filter (xml:314 — the filter-edit view, hidden=true), and \
         Log.Events (xml:258 — the WowScrollBoxList that renders log entries). All six must \
         publish as `:GetParent()`-reachable tables via the parentKey pipeline"
    );
}
}

prefork_full_ui_case! {
fn blizzard_event_trace_panel_has_subtitle_bar_with_three_navigation_buttons(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &event_trace_toc())
        .expect("Blizzard_EventTrace should load via Rust loader");

    let buttons_present: (bool, bool, bool) = env
        .eval(
            "return type(EventTrace.SubtitleBar.ViewLog) == 'table', \
                    type(EventTrace.SubtitleBar.ViewFilter) == 'table', \
                    type(EventTrace.SubtitleBar.OptionsDropdown) == 'table'",
        )
        .expect("subtitle-bar button probe should succeed");
    assert_eq!(
        buttons_present,
        (true, true, true),
        "EventTrace.SubtitleBar (xml:188) holds three navigation widgets: ViewLog button \
         (xml:195, EventTraceMenuButtonTemplate), ViewFilter button (xml:200, \
         EventTraceMenuButtonTemplate), OptionsDropdown (xml:205, \
         WowStyle1FilterDropdownTemplate). All three must publish as parentKey-resolved \
         children — confirms WowStyle1FilterDropdownTemplate is registered (the dropdown \
         intrinsic) and EventTraceMenuButtonTemplate template chain (Button → \
         EventTraceButtonBehaviorTemplate) resolves at XML-load time"
    );
}
}

prefork_full_ui_case! {
fn blizzard_event_trace_log_view_has_search_bar_and_scrollbox_pair(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &event_trace_toc())
        .expect("Blizzard_EventTrace should load via Rust loader");

    let log_widgets_present: (bool, bool, bool, bool, bool) = env
        .eval(
            "return type(EventTrace.Log.Bar) == 'table', \
                    type(EventTrace.Log.Bar.SearchBox) == 'table', \
                    type(EventTrace.Log.Events.ScrollBox) == 'table', \
                    type(EventTrace.Log.Events.ScrollBar) == 'table', \
                    type(EventTrace.Log.Search) == 'table'",
        )
        .expect("log-view probe should succeed");
    assert_eq!(
        log_widgets_present,
        (true, true, true, true, true),
        "EventTrace.Log (xml:212) hosts the events-list view: Bar (xml:218 — holds the \
         SearchBox / DiscardAllButton / PlaybackButton / MarkButton), Bar.SearchBox \
         (xml:250, SearchBoxTemplate), Events.ScrollBox + ScrollBar (xml:264/277, \
         WowScrollBoxList + MinimalScrollBar), and Search (xml:285 — secondary scroll-list \
         showing search results). All five must publish — confirms SearchBoxTemplate / \
         WowScrollBoxList / MinimalScrollBar intrinsics resolve"
    );
}
}

prefork_full_ui_case! {
fn blizzard_event_trace_filter_view_has_three_action_buttons_and_scrollbox(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &event_trace_toc())
        .expect("Blizzard_EventTrace should load via Rust loader");

    let filter_widgets_present: (bool, bool, bool, bool, bool) = env
        .eval(
            "return type(EventTrace.Filter.Bar.CheckAllButton) == 'table', \
                    type(EventTrace.Filter.Bar.UncheckAllButton) == 'table', \
                    type(EventTrace.Filter.Bar.DiscardAllButton) == 'table', \
                    type(EventTrace.Filter.ScrollBox) == 'table', \
                    type(EventTrace.Filter.ScrollBar) == 'table'",
        )
        .expect("filter-view probe should succeed");
    assert_eq!(
        filter_widgets_present,
        (true, true, true, true, true),
        "EventTrace.Filter (xml:314, hidden=true) hosts the filter-edit view: Bar with \
         three action buttons CheckAllButton / UncheckAllButton / DiscardAllButton (xml:337/\
         342/347, all EventTraceMenuButtonTemplate), plus ScrollBox + ScrollBar (xml:355/\
         368, WowScrollBoxList + MinimalScrollBar). All five must publish"
    );
}
}

prefork_full_ui_case! {
fn blizzard_event_trace_tooltip_publishes_as_hidden_game_tooltip_child(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &event_trace_toc())
        .expect("Blizzard_EventTrace should load via Rust loader");

    let tooltip_probe: (String, String, bool) = env
        .eval(
            "return EventTraceTooltip:GetName(), \
                    EventTraceTooltip:GetParent():GetName(), \
                    EventTraceTooltip:IsShown()",
        )
        .expect("EventTraceTooltip probe should succeed");
    assert_eq!(
        tooltip_probe,
        (
            "EventTraceTooltip".to_string(),
            "EventTrace".to_string(),
            false,
        ),
        "`<GameTooltip name=\"EventTraceTooltip\" frameStrata=\"TOOLTIP\" hidden=\"true\" \
         parent=\"EventTrace\" inherits=\"SharedTooltipTemplate\"/>` (xml:385) must publish \
         as a hidden GameTooltip child of the EventTrace panel — proves \
         SharedTooltipTemplate resolves and that GameTooltip-typed widgets accept a \
         non-UIParent parent"
    );
}
}

prefork_full_ui_case! {
fn blizzard_event_trace_saved_vars_seed_default_panel_size_and_show_toggles(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &event_trace_toc())
        .expect("Blizzard_EventTrace should load via Rust loader");

    let defaults: (bool, bool, bool, bool, bool, f64, f64) = env
        .eval(
            "return EventTraceSavedVars.LogEventsWhenHidden, \
                    EventTraceSavedVars.ShowArguments, \
                    EventTraceSavedVars.ShowTimestamp, \
                    EventTraceSavedVars.ShowSecretValues, \
                    EventTraceSavedVars.LogCREvents, \
                    EventTraceSavedVars.Size.Width, \
                    EventTraceSavedVars.Size.Height",
        )
        .expect("EventTraceSavedVars probe should succeed");
    assert_eq!(
        defaults,
        (false, true, true, true, true, 715.0, 400.0),
        "EventTraceSavedVars defaults (Blizzard_EventTrace.lua:33-49): \
         LogEventsWhenHidden=false (the panel pauses logging when hidden), \
         ShowArguments=true / ShowTimestamp=true / ShowSecretValues=true (event-row display \
         toggles default to verbose), LogCREvents=true (CallbackRegistry events get logged), \
         Size.Width=715 (MinPanelWidth from lua:1), Size.Height=400 (DefaultPanelHeight \
         from lua:4)"
    );

    let user_filter_count: f64 = env
        .eval("return #EventTraceSavedVars.Filters.User")
        .expect("EventTraceSavedVars.Filters.User length probe should succeed");
    assert_eq!(
        user_filter_count, 0.0,
        "EventTraceSavedVars.Filters.User defaults to an empty array (lua:42) — the \
         user-edited filter list starts empty and is populated when the user adds entries \
         via the filter view"
    );
}
}
