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

fn debug_tools_toc() -> PathBuf {
    blizzard_ui_dir().join("Blizzard_DebugTools/Blizzard_DebugTools.toc")
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
fn blizzard_debug_tools_toc_is_load_on_demand_with_allow_load_both() {
    let toc = TocFile::from_file(&debug_tools_toc()).expect("Blizzard_DebugTools TOC should parse");
    assert!(
        toc.is_load_on_demand(),
        "Blizzard_DebugTools declares `## LoadOnDemand: 1` (the framestack tooltip + table \
         inspector + texel-snapping visualizer + texture-info generator are developer-only \
         tools brought in by /framestack / /tinspect via UIParentLoadAddOn — must NOT \
         auto-load on Game-screen bring-up)"
    );
    assert!(
        !toc.is_secure_env(),
        "Blizzard_DebugTools does not declare UseSecureEnvironment"
    );
    assert!(
        toc.dependencies().is_empty(),
        "Blizzard_DebugTools has no `## Dependencies` line — it is self-contained and consumes \
         the standard SharedTooltipTemplate / TooltipBackdropTemplate / UIPanelCloseButton / \
         CallbackRegistryMixin / Mixin / SharedTooltip_OnLoad surface from FrameXML"
    );

    let toc_text =
        std::fs::read_to_string(debug_tools_toc()).expect("Blizzard_DebugTools TOC should read");
    assert!(
        toc_text.contains("## AllowLoad: Both"),
        "Blizzard_DebugTools declares `## AllowLoad: Both` (the developer tools are usable \
         from both the Glue and Game environments — /framestack works on the character-select \
         screen as well as in-game)"
    );
}

#[test]
fn blizzard_debug_tools_is_absent_from_game_auto_discovery() {
    let addons = discover_blizzard_addons_for_screen(&blizzard_ui_dir(), ScreenKind::Game);
    let in_game = addons.iter().any(|(name, _)| name == "Blizzard_DebugTools");
    assert!(
        !in_game,
        "Blizzard_DebugTools is `## LoadOnDemand: 1`, so it must NOT appear in Game-screen \
         auto-discovery — it is loaded explicitly by /framestack / /tinspect / DevTools_Dump \
         via UIParentLoadAddOn"
    );
}

prefork_full_ui_case! {
fn blizzard_debug_tools_loads_via_load_addon_without_errors(env: &WowLuaEnv) {

    {
        let mut state = env.state().borrow_mut();
        state.lua_errors.clear();
        state.lua_error_records.clear();
        state.lua_error_counts.clear();
    }

    load_addon(&env.loader_env(), &debug_tools_toc())
        .expect("Blizzard_DebugTools should load via Rust loader");

    let load_errors: Vec<String> = env
        .state()
        .borrow()
        .lua_errors
        .iter()
        .filter(|message| {
            message.contains("Blizzard_DebugTools")
                || message.contains("FrameStackTooltip")
                || message.contains("TableAttributeDisplay")
                || message.contains("TableInspector")
                || message.contains("TexelSnappingVisualizer")
                || message.contains("TextureInfoGenerator")
        })
        .cloned()
        .collect();
    assert!(
        load_errors.is_empty(),
        "Blizzard_DebugTools emitted Lua errors during explicit load:\n  {}",
        load_errors.join("\n  ")
    );
}
}

prefork_full_ui_case! {
fn devtools_dump_captures_frame_array_metadata(env: &WowLuaEnv) {
    let (
        dump_type,
        handler_type,
        frame_type,
        insert_ok,
        insert_return_nil,
        slot_one,
        dump_ok,
        dump_return_nil,
        messages,
    ): (String, String, String, bool, bool, String, bool, bool, Vec<String>) = env
        .eval(
            r#"
            local messages = {}
            DevTools_AddMessageHandler(function(message)
                messages[#messages + 1] = tostring(message)
            end)

            local frame = CreateFrame("Frame")
            local insertOk, insertResult = pcall(function()
                return tinsert(frame, "foo")
            end)
            local dumpOk, dumpResult = pcall(function()
                return DevTools_Dump(frame)
            end)

            return type(DevTools_Dump), type(DevTools_AddMessageHandler), type(frame),
                insertOk, insertResult == nil, frame[1], dumpOk, dumpResult == nil, messages
            "#,
        )
        .expect("DevTools frame dump probe should succeed");

    assert_eq!(dump_type, "function");
    assert_eq!(handler_type, "function");
    assert_eq!(frame_type, "table");
    assert!(insert_ok);
    assert!(insert_return_nil);
    assert_eq!(slot_one, "foo");
    assert!(dump_ok);
    assert!(dump_return_nil);
    assert!(!messages.is_empty());
    assert!(messages.join("\n").contains("foo"));
}
}

prefork_full_ui_case! {
fn blizzard_debug_tools_frame_stack_tooltip_is_created(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &debug_tools_toc())
        .expect("Blizzard_DebugTools should load via Rust loader");

    let tooltip_present: bool = env
        .eval(
            "return type(FrameStackTooltip) == 'table' \
                and FrameStackTooltip:IsShown() == false \
                and type(FrameStackTooltip.GenerateCallbackEvents) == 'function' \
                and type(FrameStackTooltip.commandKeys) == 'table' \
                and #FrameStackTooltip.commandKeys == 5",
        )
        .expect("FrameStackTooltip query should succeed");
    assert!(
        tooltip_present,
        "Blizzard_DebugTools.xml line 4 should create the toplevel `FrameStackTooltip` \
         GameTooltip (frameStrata=TOOLTIP, hidden=true, inherits=SharedTooltipTemplate, \
         enableKeyboard=true) which OnLoad mixes in CallbackRegistryMixin + \
         TextureInfoGeneratorMixin and creates 5 KeyCommand entries (LALT/RALT to walk the \
         framestack, CTRL to inspect a table, SHIFT to toggle texture info, CTRL+C to copy a \
         frame command)"
    );
}
}

prefork_full_ui_case! {
fn blizzard_debug_tools_frame_stack_helper_globals_are_defined(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &debug_tools_toc())
        .expect("Blizzard_DebugTools should load via Rust loader");

    let helpers_present: bool = env
        .eval(
            "return type(FrameStackTooltip_Toggle) == 'function' \
                and type(FrameStackTooltip_ToggleDefaults) == 'function' \
                and type(FrameStackTooltip_Show) == 'function' \
                and type(FrameStackTooltip_Hide) == 'function' \
                and type(FrameStackTooltip_IsFramestackEnabled) == 'function' \
                and type(FrameStackTooltip_IsShowHiddenEnabled) == 'function' \
                and type(FrameStackTooltip_IsHighlightEnabled) == 'function' \
                and type(FrameStackTooltip_IsShowRegionsEnabled) == 'function' \
                and type(FrameStackTooltip_IsShowAnchorsEnabled) == 'function' \
                and type(FrameStackTooltip_OnFramestackVisibilityUpdated) == 'function' \
                and type(FrameStackTooltip_InspectTable) == 'function' \
                and type(FrameStackTooltip_HandleFrameCommand) == 'function' \
                and type(FrameStackTooltip_ChangeHighlight) == 'function' \
                and type(FrameStackTooltip_OnUpdate) == 'function' \
                and type(DebugTooltip_OnLoad) == 'function' \
                and type(DebugIdentifierFrame_OnLoad) == 'function' \
                and type(CompareFunctionReturns) == 'function'",
        )
        .expect("FrameStackTooltip helper-global query should succeed");
    assert!(
        helpers_present,
        "Blizzard_DebugTools.lua publishes the `FrameStackTooltip_*` global surface that drives \
         the framestack tooltip: 5 cvar-readers (fstack_enabled / fstack_showhidden / \
         fstack_showhighlight / fstack_showregions / fstack_showanchors), Show/Hide/Toggle, \
         OnFramestackVisibilityUpdated, InspectTable / HandleFrameCommand / ChangeHighlight, \
         and the OnUpdate ticker. Blizzard_DebugTools.lua line 317 publishes \
         CompareFunctionReturns for `func1, func2, ...` ordered comparison"
    );
}
}

prefork_full_ui_case! {
fn blizzard_debug_tools_anchor_highlight_mixin_is_published(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &debug_tools_toc())
        .expect("Blizzard_DebugTools should load via Rust loader");

    let mixin_present: bool = env
        .eval(
            "return type(AnchorHighlightMixin) == 'table' \
                and type(AnchorHighlightMixin.RetrieveAnchorHighlight) == 'function' \
                and type(AnchorHighlightMixin.HighlightFrame) == 'function'",
        )
        .expect("AnchorHighlightMixin query should succeed");
    assert!(
        mixin_present,
        "Blizzard_DebugTools.lua line 194 should publish AnchorHighlightMixin with \
         RetrieveAnchorHighlight(pointIndex) — pulls a pooled FrameStackAnchorHighlightTemplate \
         from self.AnchorHighlights and HighlightFrame(baseFrame, showAnchors) — iterates the \
         baseFrame's GetNumPoints() anchors and positions an AnchorHighlight at each (used by \
         FrameStackHighlight to draw the green/yellow overlays)"
    );
}
}

prefork_full_ui_case! {
fn blizzard_debug_tools_table_inspector_mixin_is_published(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &debug_tools_toc())
        .expect("Blizzard_DebugTools should load via Rust loader");

    let mixin_present: bool = env
        .eval(
            "return type(TableInspectorMixin) == 'table' \
                and type(TableInspectorMixin.OnLoad) == 'function' \
                and type(TableInspectorMixin.OnHide) == 'function' \
                and type(TableInspectorMixin.Reset) == 'function' \
                and type(TableInspectorMixin.AddDataProvider) == 'function' \
                and type(TableInspectorMixin.RemoveAllDataProviders) == 'function' \
                and type(TableInspectorMixin.RefreshAllData) == 'function' \
                and type(TableInspectorMixin.UpdateLines) == 'function' \
                and type(TableInspectorMixin.OpenParentDisplay) == 'function' \
                and type(TableInspectorMixin.NavigateBackward) == 'function' \
                and type(TableInspectorMixin.NavigateForward) == 'function' \
                and type(TableInspectorMixin.DuplicateAttributeDisplay) == 'function' \
                and type(TableInspectorMixin.SetFocusedFrameShown) == 'function' \
                and type(TableInspectorMixin.SetDynamicUpdates) == 'function' \
                and type(TableInspectorMixin.UpdateFocusedHighlight) == 'function' \
                and type(TableInspectorMixin.SelectTable) == 'function' \
                and type(TableInspectorMixin.UpdateTableNavigation) == 'function' \
                and type(TableInspectorMixin.InspectTable) == 'function'",
        )
        .expect("TableInspectorMixin query should succeed");
    assert!(
        mixin_present,
        "Blizzard_TableInspector.lua line 9 should publish \
         `TableInspectorMixin = CreateFromMixins(ToolWindowOwnerMixin)` with the navigation + \
         data-provider stack: OnLoad/OnHide lifecycle, AddDataProvider / ClearData / \
         RemoveAllDataProviders for the attribute + anchor providers, RefreshAllData / \
         UpdateLines for the scroll grid, OpenParentDisplay + Navigate Backward/Forward + \
         DuplicateAttributeDisplay for the breadcrumb stack, SetFocusedFrameShown / \
         UpdateFocusedHighlight / SetDynamicUpdates for the FrameStackHighlight integration, \
         and SelectTable / UpdateTableNavigation / InspectTable as the public entry-point used \
         by FrameStackTooltip_InspectTable"
    );
}
}

prefork_full_ui_case! {
fn blizzard_debug_tools_table_inspector_data_provider_mixins_are_published(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &debug_tools_toc())
        .expect("Blizzard_DebugTools should load via Rust loader");

    let mixins_present: bool = env
        .eval(
            "return type(TableInspectorDataProviderMixin) == 'table' \
                and type(TableInspectorDataProviderMixin.Initialize) == 'function' \
                and type(TableInspectorDataProviderMixin.RefreshData) == 'function' \
                and type(TableInspectorDataProviderMixin.GetLines) == 'function' \
                and type(TableInspectorAnchorDataProviderMixin) == 'table' \
                and type(TableInspectorAnchorDataProviderMixin.Initialize) == 'function' \
                and type(TableInspectorAnchorDataProviderMixin.RefreshData) == 'function' \
                and type(TableInspectorAnchorDataProviderMixin.GetLines) == 'function' \
                and type(TableInspectorAttributeDataProviderMixin) == 'table' \
                and type(TableInspectorAttributeDataProviderMixin.Initialize) == 'function' \
                and type(TableInspectorAttributeDataProviderMixin.RefreshData) == 'function' \
                and type(TableInspectorAttributeDataProviderMixin.SortAttributes) == 'function' \
                and type(TableInspectorAttributeDataProviderMixin.GetLines) == 'function' \
                and type(TableInspectAnchorLineMixin) == 'table' \
                and type(TableAttributeLineMixin) == 'table' \
                and type(TableAttributeLineEditableMixin) == 'table' \
                and type(TableAttributeLineReferenceMixin) == 'table' \
                and type(TableAttributeLineFixedValueMixin) == 'table' \
                and type(TableAttributeLineTitleMixin) == 'table'",
        )
        .expect("TableInspectorDataProvider mixin query should succeed");
    assert!(
        mixins_present,
        "Blizzard_TableInspectorDataProvider.lua publishes the base \
         TableInspectorDataProviderMixin (Initialize/RefreshData/GetFocusedTable/\
         GetTableInspector/HideAllLines/Clear/GetLines), \
         Blizzard_TableInspectorAnchorDataProvider.lua extends it via \
         `CreateFromMixins(TableInspectorDataProviderMixin)` for anchor inspection plus the \
         per-line TableInspectAnchorLineMixin, and \
         Blizzard_TableInspectorAttributeDataProvider.lua extends it for attribute inspection \
         plus the 4 per-line variants TableAttributeLineMixin / TableAttributeLineEditableMixin \
         / TableAttributeLineReferenceMixin / TableAttributeLineFixedValueMixin / \
         TableAttributeLineTitleMixin"
    );
}
}

prefork_full_ui_case! {
fn blizzard_debug_tools_texture_info_generator_mixin_is_published(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &debug_tools_toc())
        .expect("Blizzard_DebugTools should load via Rust loader");

    let mixin_present: bool = env
        .eval(
            "return type(TextureInfoGeneratorMixin) == 'table' \
                and type(TextureInfoGeneratorMixin.CheckGetRegionsTextureInfo) == 'function' \
                and type(TextureInfoGeneratorMixin.CheckFormatTextureInfo) == 'function' \
                and type(TextureInfoGeneratorMixin.HandleTextureCommand) == 'function' \
                and type(TextureInfoGeneratorMixin.SetCurrentTextureAssets) == 'function' \
                and type(TextureInfoGeneratorMixin.GetCurrentTextureAssets) == 'function' \
                and type(TextureInfoGeneratorMixin.SetCheckIsMouseOverRegion) == 'function' \
                and type(TextureInfoGeneratorMixin.ShouldCheckIsMouseOverRegion) == 'function' \
                and type(TextureInfoGeneratorMixin.ShouldGenerateRegionInfo) == 'function'",
        )
        .expect("TextureInfoGeneratorMixin query should succeed");
    assert!(
        mixin_present,
        "Blizzard_TextureInfoGenerator.lua line 51 publishes TextureInfoGeneratorMixin with 9 \
         methods (CheckGetRegionsTextureInfo / CheckFormatTextureInfo / HandleTextureCommand / \
         SetCurrentTextureAssets / GetCurrentTextureAssets / SetCheckIsMouseOverRegion / \
         ShouldCheckIsMouseOverRegion / ShouldGenerateRegionInfo) — mixed into FrameStackTooltip \
         via `Mixin(self, TextureInfoGeneratorMixin)` in FrameStackTooltip_OnLoad"
    );
}
}

prefork_full_ui_case! {
fn blizzard_debug_tools_texel_snapping_visualizer_is_gated_on_gm_client(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &debug_tools_toc())
        .expect("Blizzard_DebugTools should load via Rust loader");

    let gm_gated: bool = env
        .eval(
            "return IsGMClient() == false \
                and _G.TexelSnappingVisualizerMixin == nil \
                and _G.TexelSnappingVisualizer == nil",
        )
        .expect("TexelSnappingVisualizer GM-gate query should succeed");
    assert!(
        gm_gated,
        "Blizzard_TexelSnappingVisualizer.lua lines 1-3 early-return with `if not IsGMClient() \
         then return end` — the simulator's `IsGMClient` returns false (see \
         src/lua_api/globals/utility_system_spell/mod.rs line 364), so neither the \
         TexelSnappingVisualizerMixin global nor the toplevel TexelSnappingVisualizer frame \
         (line 114: \
         `Mixin(CreateFrame('FRAME', 'TexelSnappingVisualizer', UIParent, \
         'TooltipBackdropTemplate'), TexelSnappingVisualizerMixin):OnCreated()`) should be \
         created — the file should silently exit during load and not emit Lua errors"
    );
}
}

prefork_full_ui_case! {
fn blizzard_debug_tools_xml_templates_are_registered(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &debug_tools_toc())
        .expect("Blizzard_DebugTools should load via Rust loader");

    let expected_templates = [
        "FrameHighlightTemplate",
        "FrameStackAnchorHighlightTemplate",
        "DebugIdentifierFrameNoNameTemplate",
        "DebugIdentifierFrameTemplate",
        "TableAttributeDisplayTemplate",
        "TableAttributeLineBaseTemplate",
        "TableAttributeLineTitleTemplate",
        "TableAttributeLineFixedValueTemplate",
        "TableAttributeLineEditableTemplate",
        "TableAttributeLineReferenceTemplate",
        "TableInspectAnchorDataProviderTitleTemplate",
        "TableInspectAnchorLineTemplate",
    ];
    for template_name in expected_templates {
        assert!(
            wow_ui_sim::xml::get_template(template_name).is_some(),
            "Blizzard_DebugTools should register `{template_name}` in the Frame template \
             registry — used by TableInspectorMixin / FrameStackHighlight / DebugIdentifierFrame \
             / per-line views to spawn child frames"
        );
    }
}
}

prefork_full_ui_case! {
fn blizzard_debug_tools_table_attribute_display_is_created_after_load(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &debug_tools_toc())
        .expect("Blizzard_DebugTools should load via Rust loader");

    let display_present: bool = env
        .eval(
            "return type(TableAttributeDisplay) == 'table' \
                and TableAttributeDisplay:IsShown() == false \
                and type(TableAttributeDisplay.InspectTable) == 'function' \
                and type(TableAttributeDisplay.SelectTable) == 'function' \
                and type(TableAttributeDisplay.NavigateBackward) == 'function' \
                and type(TableAttributeDisplay.NavigateForward) == 'function' \
                and type(DisplayTableInspectorWindow) == 'function'",
        )
        .expect("TableAttributeDisplay query should succeed");
    assert!(
        display_present,
        "Blizzard_TableInspector.xml line 284 should create the toplevel `TableAttributeDisplay` \
         frame from TableAttributeDisplayTemplate (parent=UIParent, frameStrata=DIALOG, \
         toplevel=true, movable=true, clampedToScreen=true, hidden=true, mixin=\
         TableInspectorMixin) — `FrameStackTooltip_InspectTable` calls \
         `TableAttributeDisplay:InspectTable(highlightFrame)` + `:Show()` to bring up the \
         per-frame attribute inspector. `DisplayTableInspectorWindow(focusedTable, \
         customTitle, tableFocusedCallback)` is the public entry point that spawns a fresh \
         inspector frame from the template via `CreateFrame`"
    );
}
}
