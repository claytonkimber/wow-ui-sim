use std::io::Write;

use wow_ui_sim::loader::{
    LoadResult, MissingRequirement, MissingRequirementKind, NilSymbolEnvironment,
    NilSymbolObservation, NilSymbolObservationKind, load_addon,
};
use wow_ui_sim::lua_api::WowLuaEnv;

fn find_global_observation<'a>(
    result: &'a LoadResult,
    name: &str,
) -> Option<&'a NilSymbolObservation> {
    result.nil_symbol_observations.iter().find(|observation| {
        matches!(
            &observation.kind,
            NilSymbolObservationKind::Global { name: observed } if observed == name
        )
    })
}

fn find_namespace_requirement<'a>(
    result: &'a LoadResult,
    namespace: &str,
) -> Option<&'a MissingRequirement> {
    result.missing_requirements.iter().find(|requirement| {
        matches!(
            &requirement.kind,
            MissingRequirementKind::CNamespace { namespace: required }
                if required == namespace
        )
    })
}

fn find_method_requirement<'a>(
    result: &'a LoadResult,
    namespace: &str,
    method: &str,
) -> Option<&'a MissingRequirement> {
    result.missing_requirements.iter().find(|requirement| {
        matches!(
            &requirement.kind,
            MissingRequirementKind::CMethod {
                namespace: required_namespace,
                method: required_method,
            } if required_namespace == namespace && required_method == method
        )
    })
}

fn create_test_addon_with_missing_symbol_accesses() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let addon_dir = dir.path();

    let toc_path = addon_dir.join("TestNilSymbols.toc");
    let mut toc = std::fs::File::create(&toc_path).unwrap();
    writeln!(toc, "## Title: TestNilSymbols").unwrap();
    writeln!(toc, "TestNilSymbols.lua").unwrap();

    let lua_path = addon_dir.join("TestNilSymbols.lua");
    let mut lua = std::fs::File::create(&lua_path).unwrap();
    writeln!(
        lua,
        r#"local _ = MissingGlobalSymbol
local _ = C_MissingNamespace
local _ = C_Container.MissingMethod
local _ = C_Container.MissingMethod
local _ = _G.OptionalMissingGlobal
local _ = _G["OptionalMissingGlobal"]
local _ = _G.DynamicThenDirectMissingGlobal
local _ = DynamicThenDirectMissingGlobal
local _ = _G.C_ExplicitMissingNamespace
local _ = _G.C_Container.ExplicitMissingMethod
"#
    )
    .unwrap();

    dir
}

fn create_test_addon_with_late_symbol_publication() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let addon_dir = dir.path();

    let toc_path = addon_dir.join("TestLatePublication.toc");
    let mut toc = std::fs::File::create(&toc_path).unwrap();
    writeln!(toc, "## Title: TestLatePublication").unwrap();
    writeln!(toc, "Early.lua").unwrap();
    writeln!(toc, "Published.xml").unwrap();
    writeln!(toc, "Late.lua").unwrap();

    let mut early = std::fs::File::create(addon_dir.join("Early.lua")).unwrap();
    writeln!(
        early,
        r#"local _ = PublishedByLua
local _ = PublishedByXml
local _ = StillMissingGlobal
local _ = C_Container.StillMissingMethod
"#
    )
    .unwrap();

    let mut published = std::fs::File::create(addon_dir.join("Published.xml")).unwrap();
    writeln!(
        published,
        r#"<Ui xmlns="http://www.blizzard.com/wow/ui/">
    <Frame name="PublishedByXml"/>
</Ui>"#
    )
    .unwrap();

    let mut late = std::fs::File::create(addon_dir.join("Late.lua")).unwrap();
    writeln!(late, "PublishedByLua = true").unwrap();

    dir
}

fn create_test_secure_addon_with_late_symbol_publication() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let addon_dir = dir.path();

    let mut toc = std::fs::File::create(addon_dir.join("TestSecureLatePublication.toc")).unwrap();
    writeln!(toc, "## Title: TestSecureLatePublication").unwrap();
    writeln!(toc, "## UseSecureEnvironment: 1").unwrap();
    writeln!(toc, "PublicMiss.lua [LoadIntoEnvironment global]").unwrap();
    writeln!(toc, "Frames.xml").unwrap();
    writeln!(toc, "Late.lua").unwrap();

    std::fs::write(
        addon_dir.join("PublicMiss.lua"),
        "local _ = SecurePublicationMustNotResolvePublic\n",
    )
    .unwrap();
    std::fs::write(
        addon_dir.join("Frames.xml"),
        r#"<Ui xmlns="http://www.blizzard.com/wow/ui/">
    <Frame name="LateSecureFunctionTemplate" virtual="true">
        <Scripts>
            <OnShow function="LaterSecureFunction"/>
        </Scripts>
    </Frame>
    <Frame name="EarlySecureFunctionFrame" inherits="LateSecureFunctionTemplate"/>
    <Frame name="MissingSecureFunctionFrame">
        <Scripts>
            <OnShow function="StillMissingSecureFunction"/>
        </Scripts>
    </Frame>
</Ui>"#,
    )
    .unwrap();
    std::fs::write(
        addon_dir.join("Late.lua"),
        r#"function LaterSecureFunction(self)
    self.lateSecureHandlerRan = true
end
function SecurePublicationMustNotResolvePublic()
end
local frame = CreateFrame("Frame", "LateSecureFunctionFrame", nil, "LateSecureFunctionTemplate")
frame:Hide()
frame:Show()
SecureLateHandlerExecuted = frame.lateSecureHandlerRan == true
"#,
    )
    .unwrap();

    dir
}

fn create_test_addon_with_cleared_publication() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let addon_dir = dir.path();

    let mut toc = std::fs::File::create(addon_dir.join("TestClearedPublication.toc")).unwrap();
    writeln!(toc, "## Title: TestClearedPublication").unwrap();
    writeln!(toc, "TestClearedPublication.lua").unwrap();

    let mut lua = std::fs::File::create(addon_dir.join("TestClearedPublication.lua")).unwrap();
    writeln!(lua, "local _ = PublishedThenCleared").unwrap();
    writeln!(lua, "PublishedThenCleared = true").unwrap();
    writeln!(lua, "PublishedThenCleared = nil").unwrap();

    dir
}

fn create_test_addon_with_lua_failure() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let addon_dir = dir.path();

    std::fs::write(
        addon_dir.join("TestLuaFailure.toc"),
        "## Title: TestLuaFailure\nTestLuaFailure.lua\n",
    )
    .unwrap();
    std::fs::write(
        addon_dir.join("TestLuaFailure.lua"),
        "error('typed diagnostic failure probe')\n",
    )
    .unwrap();

    dir
}

fn write_runtime_event_warning_addon(
    root: &std::path::Path,
    addon_name: &str,
    lua_source: &str,
) {
    let addon_dir = root.join(addon_name);
    std::fs::create_dir_all(&addon_dir).unwrap();
    std::fs::write(
        addon_dir.join(format!("{addon_name}.toc")),
        format!("## Title: {addon_name}\n{addon_name}.lua\n"),
    )
    .unwrap();
    std::fs::write(addon_dir.join(format!("{addon_name}.lua")), lua_source).unwrap();
}

fn write_runtime_event_warning_addon_fixture() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    write_runtime_event_warning_addon(
        dir.path(),
        "OuterEventLoader",
        r#"local frame = CreateFrame("Frame")
frame:RegisterEvent("ADDON_LOADED")
frame:SetScript("OnEvent", function(_, _, addonName)
    if addonName == "OuterEventLoader" then
        local loaded, reason = C_AddOns.LoadAddOn("RuntimeParent")
        assert(loaded, tostring(reason))
    end
end)
"#,
    );
    write_runtime_event_warning_addon(
        dir.path(),
        "RuntimeParent",
        r#"local _ = RuntimeParentMissingGlobal
local _ = C_Container.RuntimeParentMissingMethod
local frame = CreateFrame("Frame")
frame:RegisterEvent("ADDON_LOADED")
frame:SetScript("OnEvent", function(_, _, addonName)
    if addonName == "RuntimeParent" then
        local loaded, reason = C_AddOns.LoadAddOn("RuntimeChild")
        assert(loaded, tostring(reason))
    end
end)
"#,
    );
    write_runtime_event_warning_addon(
        dir.path(),
        "RuntimeChild",
        r#"local _ = RuntimeChildMissingGlobal
local _ = C_Container.RuntimeChildMissingMethod
"#,
    );
    dir
}

fn create_nested_publication_addons() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let outer_dir = dir.path().join("OuterConsumer");
    let nested_dir = dir.path().join("NestedPublisher");
    std::fs::create_dir_all(&outer_dir).unwrap();
    std::fs::create_dir_all(&nested_dir).unwrap();

    let mut outer_toc = std::fs::File::create(outer_dir.join("OuterConsumer.toc")).unwrap();
    writeln!(outer_toc, "## Title: OuterConsumer").unwrap();
    writeln!(outer_toc, "OuterConsumer.lua").unwrap();
    let mut outer_lua = std::fs::File::create(outer_dir.join("OuterConsumer.lua")).unwrap();
    writeln!(
        outer_lua,
        r#"local _ = NestedPublishedGlobal
local loaded, reason = C_AddOns.LoadAddOn("NestedPublisher")
assert(loaded, tostring(reason))
local recordPublication = rawget(_G, "__wow_record_public_global_publication")
if type(recordPublication) == "function" then
    recordPublication("NestedPublishedGlobal")
end
"#
    )
    .unwrap();

    let mut nested_toc = std::fs::File::create(nested_dir.join("NestedPublisher.toc")).unwrap();
    writeln!(nested_toc, "## Title: NestedPublisher").unwrap();
    writeln!(nested_toc, "NestedPublisher.lua").unwrap();
    let mut nested_lua = std::fs::File::create(nested_dir.join("NestedPublisher.lua")).unwrap();
    writeln!(
        nested_lua,
        r#"local _ = C_Container.NestedMissingMethod
local _ = NestedResolvedGlobal
NestedResolvedGlobal = true
NestedPublishedGlobal = true
"#
    )
    .unwrap();

    dir
}

#[test]
fn load_addon_separates_nil_observations_requirements_and_failures() {
    let env = WowLuaEnv::new().unwrap();
    let dir = create_test_addon_with_missing_symbol_accesses();
    let toc_path = dir.path().join("TestNilSymbols.toc");

    let result = load_addon(&env.loader_env(), &toc_path).expect("addon load should succeed");

    assert!(
        result.warnings.is_empty(),
        "nil-symbol diagnostics must not become loader failures: {:?}",
        result.warnings
    );

    let missing_global = find_global_observation(&result, "MissingGlobalSymbol")
        .expect("direct missing global must remain observable");
    assert_eq!(missing_global.attribution.addon_name, "TestNilSymbols");
    assert_eq!(
        missing_global.attribution.source.as_deref(),
        Some("TestNilSymbols.lua")
    );
    assert_eq!(missing_global.attribution.line, Some(1));
    assert_eq!(
        missing_global.attribution.environment,
        NilSymbolEnvironment::Public
    );

    let namespace = find_namespace_requirement(&result, "C_MissingNamespace")
        .expect("missing C namespace must remain a strict requirement");
    assert_eq!(namespace.attribution.addon_name, "TestNilSymbols");
    assert_eq!(namespace.attribution.line, Some(2));

    let method = find_method_requirement(&result, "C_Container", "MissingMethod")
        .expect("missing C method must remain a strict requirement");
    assert_eq!(method.attribution.line, Some(3));
    assert_eq!(
        result
            .missing_requirements
            .iter()
            .filter(|requirement| {
                matches!(
                    &requirement.kind,
                    MissingRequirementKind::CMethod { namespace, method }
                        if namespace == "C_Container" && method == "MissingMethod"
                )
            })
            .count(),
        1,
        "repeated C method accesses must deduplicate"
    );

    assert!(
        find_namespace_requirement(&result, "C_ExplicitMissingNamespace").is_some(),
        "explicit _G C namespace access remains a strict requirement"
    );
    assert!(
        find_method_requirement(&result, "C_Container", "ExplicitMissingMethod").is_some(),
        "explicit _G C member access remains a strict requirement"
    );
    assert!(
        find_global_observation(&result, "OptionalMissingGlobal").is_none(),
        "explicit optional regular _G probes must remain non-observations"
    );
    assert!(
        find_global_observation(&result, "DynamicThenDirectMissingGlobal").is_some(),
        "a prior optional _G probe must not hide a later direct-global observation"
    );
}

#[test]
fn lua_load_failure_remains_fatal_warning() {
    let env = WowLuaEnv::new().unwrap();
    let dir = create_test_addon_with_lua_failure();
    let toc_path = dir.path().join("TestLuaFailure.toc");

    let result = load_addon(&env.loader_env(), &toc_path).expect("loader should report file failure");

    assert!(
        result
            .warnings
            .iter()
            .any(|warning| warning.contains("typed diagnostic failure probe")),
        "genuine Lua load failure must remain in warnings: {:?}",
        result.warnings
    );
    assert!(result.nil_symbol_observations.is_empty());
    assert!(result.missing_requirements.is_empty());
}

#[test]
fn load_addon_reconciles_public_globals_published_before_completion() {
    let env = WowLuaEnv::new().unwrap();
    let dir = create_test_addon_with_late_symbol_publication();
    let toc_path = dir.path().join("TestLatePublication.toc");

    let result = load_addon(&env.loader_env(), &toc_path).expect("addon load should succeed");

    assert!(result.warnings.is_empty());
    assert!(
        find_global_observation(&result, "PublishedByLua").is_none(),
        "late Lua publication should resolve its early nil access: {:?}",
        result.nil_symbol_observations
    );
    assert!(
        find_global_observation(&result, "PublishedByXml").is_none(),
        "named XML frame publication should resolve its early nil access: {:?}",
        result.nil_symbol_observations
    );
    assert!(
        find_global_observation(&result, "StillMissingGlobal").is_some(),
        "regular global still nil at addon completion must remain observable: {:?}",
        result.nil_symbol_observations
    );
    assert!(
        find_method_requirement(&result, "C_Container", "StillMissingMethod").is_some(),
        "C method gap must remain a requirement after regular publication: {:?}",
        result.missing_requirements
    );
}

#[test]
fn secure_publication_resolves_only_secure_same_addon_accesses() {
    let env = WowLuaEnv::new().unwrap();
    let dir = create_test_secure_addon_with_late_symbol_publication();
    let toc_path = dir.path().join("TestSecureLatePublication.toc");

    let result = load_addon(&env.loader_env(), &toc_path).expect("secure addon load should succeed");

    let (public_type, secure_type, handler_executed): (String, String, bool) = env
        .eval(
            r#"
            return type(rawget(_G, "LaterSecureFunction")),
                   type(rawget(__secureenv, "LaterSecureFunction")),
                   rawget(__secureenv, "SecureLateHandlerExecuted") == true
            "#,
        )
        .expect("secure publication state should be queryable");
    assert_eq!(public_type, "nil", "secure publication must not leak into _G");
    assert_eq!(secure_type, "function");
    assert!(handler_executed, "late secure XML function handler should execute");

    assert!(result.warnings.is_empty());
    assert!(
        find_global_observation(&result, "LaterSecureFunction").is_none(),
        "late secure publication should resolve its secure-origin nil access: {:?}",
        result.nil_symbol_observations
    );
    let missing_secure = find_global_observation(&result, "StillMissingSecureFunction")
        .expect("a genuinely missing secure function must remain observable");
    assert_eq!(
        missing_secure.attribution.environment,
        NilSymbolEnvironment::Secure
    );
    let unresolved_public =
        find_global_observation(&result, "SecurePublicationMustNotResolvePublic")
            .expect("secure publication must not resolve a public-origin miss");
    assert_eq!(
        unresolved_public.attribution.environment,
        NilSymbolEnvironment::Public
    );
    let state = env.state().borrow();
    assert!(
        state.global_publications.is_empty(),
        "completed addon loads must clear public publication records"
    );
    assert!(
        state.secure_global_publications.is_empty(),
        "completed addon loads must clear secure publication records"
    );
}

#[test]
fn publication_guard_cleared_global_remains_warned() {
    let env = WowLuaEnv::new().unwrap();
    let dir = create_test_addon_with_cleared_publication();
    let toc_path = dir.path().join("TestClearedPublication.toc");

    let result = load_addon(&env.loader_env(), &toc_path).expect("addon load should succeed");

    assert!(result.warnings.is_empty());
    assert!(
        find_global_observation(&result, "PublishedThenCleared").is_some(),
        "global cleared before addon completion must remain observable: {:?}",
        result.nil_symbol_observations
    );
}

#[test]
fn runtime_addon_event_warnings_are_finalized_once_with_their_owners() {
    let env = WowLuaEnv::new().unwrap();
    let dir = write_runtime_event_warning_addon_fixture();
    env.state().borrow_mut().addon_base_paths = vec![dir.path().to_path_buf()];
    let toc_path = dir.path().join("OuterEventLoader/OuterEventLoader.toc");

    let result = load_addon(&env.loader_env(), &toc_path).expect("outer addon load should succeed");
    assert!(
        result.warnings.is_empty(),
        "runtime warnings must not appear before the outer ADDON_LOADED event: {:?}",
        result.warnings
    );

    env.fire_event_with_args("ADDON_LOADED", &[env.lua_string("OuterEventLoader")])
        .expect("outer ADDON_LOADED event should load the runtime addon chain");

    let diagnostics = env.drain_runtime_addon_diagnostics();
    assert!(
        diagnostics.warnings.is_empty(),
        "nil-symbol diagnostics must not become runtime failures: {:?}",
        diagnostics.warnings
    );
    for (addon_name, symbol) in [
        ("RuntimeParent", "RuntimeParentMissingGlobal"),
        ("RuntimeChild", "RuntimeChildMissingGlobal"),
    ] {
        assert_eq!(
            diagnostics
                .nil_symbol_observations
                .iter()
                .filter(|observation| {
                    observation.attribution.addon_name == addon_name
                        && matches!(
                            &observation.kind,
                            NilSymbolObservationKind::Global { name } if name == symbol
                        )
                })
                .count(),
            1,
            "runtime observation must retain owner and deduplicate: {diagnostics:?}"
        );
    }
    for (addon_name, method) in [
        ("RuntimeParent", "RuntimeParentMissingMethod"),
        ("RuntimeChild", "RuntimeChildMissingMethod"),
    ] {
        assert_eq!(
            diagnostics
                .missing_requirements
                .iter()
                .filter(|requirement| {
                    requirement.attribution.addon_name == addon_name
                        && matches!(
                            &requirement.kind,
                            MissingRequirementKind::CMethod { namespace, method: required }
                                if namespace == "C_Container" && required == method
                        )
                })
                .count(),
            1,
            "runtime requirement must retain owner and deduplicate: {diagnostics:?}"
        );
    }
    assert_eq!(diagnostics.nil_symbol_observations.len(), 2);
    assert_eq!(diagnostics.missing_requirements.len(), 2);
    assert!(
        env.drain_runtime_addon_diagnostics().is_empty(),
        "runtime diagnostic drain should consume all channels"
    );
}

#[test]
fn publication_guard_nested_addon_does_not_resolve_outer_warning() {
    let env = WowLuaEnv::new().unwrap();
    let dir = create_nested_publication_addons();
    env.state().borrow_mut().addon_base_paths = vec![dir.path().to_path_buf()];
    let toc_path = dir.path().join("OuterConsumer/OuterConsumer.toc");

    let result = load_addon(&env.loader_env(), &toc_path).expect("outer addon load should succeed");
    let nested_global: bool = env
        .eval("return NestedPublishedGlobal == true")
        .expect("nested publication should be readable");

    assert!(nested_global, "nested addon should publish its global");
    assert!(result.warnings.is_empty());
    assert_eq!(
        result
            .missing_requirements
            .iter()
            .filter(|requirement| {
                requirement.attribution.addon_name == "NestedPublisher"
                    && matches!(
                        &requirement.kind,
                        MissingRequirementKind::CMethod { namespace, method }
                            if namespace == "C_Container" && method == "NestedMissingMethod"
                    )
            })
            .count(),
        1,
        "nested addon requirement should propagate exactly once: {:?}",
        result.missing_requirements
    );
    assert!(
        find_global_observation(&result, "NestedResolvedGlobal").is_none(),
        "nested addon's resolved global should stay reconciled: {:?}",
        result.nil_symbol_observations
    );
    let outer_observation = find_global_observation(&result, "NestedPublishedGlobal")
        .expect("nested publication must not resolve the outer addon's observation");
    assert_eq!(outer_observation.attribution.addon_name, "OuterConsumer");
}
