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

fn notification_dir() -> PathBuf {
    blizzard_ui_dir().join("Blizzard_Notification")
}

fn notification_toc() -> PathBuf {
    notification_dir().join("Blizzard_Notification.toc")
}

const NOTIFICATION_TOC_FILES: &[&str] =
    &["Blizzard_Notification.xml", "Blizzard_NotificationUtil.lua"];

const PUBLIC_NAMESPACE: &str = "NotificationUtil";

const PUBLIC_NAMESPACE_METHODS: &[&str] = &[
    "AcquireNotification",
    "AcquireLargeNotification",
    "ReleaseNotification",
];

const BACKCOMPAT_GLOBAL_FUNCTIONS: &[&str] = &[
    "NotificationUtil_AcquireLargeNotification",
    "NotificationUtil_ReleaseNotification",
];

const VIRTUAL_TEMPLATES_NOT_IN_GLOBALS: &[&str] = &[
    "NotificationIconFrameTemplate",
    "LargeNotificationIconFrameTemplate",
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
fn blizzard_notification_find_toc_resolves_bare_variant() {
    let resolved = find_toc_file(&notification_dir()).expect("Blizzard_Notification TOC resolves");
    assert_eq!(
        resolved,
        notification_toc(),
        "Blizzard_Notification ships exactly one bare TOC — no `_Mainline.toc` and no \
         `_Classic.toc`. The notification-icon utility is a flavor-agnostic helper (it just \
         positions a `plunderstorm-new-dot-lg` atlas at a SetPoint anchor relative to a \
         parent frame); it has no flavor-specific behavior, so a single bare TOC suffices"
    );
}

#[test]
fn blizzard_notification_toc_declares_eager_load_with_allow_load_both() {
    let toc = TocFile::from_file(&notification_toc()).expect("Blizzard_Notification TOC parses");
    assert!(
        !toc.is_load_on_demand(),
        "TOC declares `## LoadOnDemand: 0` — the notification utility is eager-loaded so its \
         pool collection is ready for any caller (action bar / chat frame / mail icon / \
         arbitrary addon) on the very first SetPoint-relative summon. A LoD route would \
         require every consumer to first call C_AddOns.LoadAddOn before acquiring a frame, \
         which would defeat the utility's purpose as a low-friction shared pool"
    );
    assert!(!toc.is_load_first());
    assert!(!toc.is_secure_env());

    assert!(
        toc.allows_screen(ScreenKind::Game),
        "`## AllowLoad: Both` must enable Game-screen discovery — `allows_screen` at \
         src/toc.rs:307 returns true for any screen when AllowLoad is Both"
    );
    assert!(
        toc.allows_screen(ScreenKind::Login),
        "`## AllowLoad: Both` must ALSO enable Login-screen discovery — the notification icon \
         utility loads on glue screens too (CharacterSelect uses NotificationUtil to flag the \
         Saved Sets / Boost / new-feature icons on character rows). Distinct from the default \
         (omitted AllowLoad) which falls back to Game-only at src/toc.rs:311"
    );
    assert!(
        toc.allows_screen(ScreenKind::CharacterSelect),
        "`## AllowLoad: Both` must ALSO enable CharacterSelect — that is exactly the screen \
         where notification icons render on character-row buttons (the addon's TOC title is \
         literally `Blizzard Saved Sets`, hinting at the CharacterServices saved-character \
         flow)"
    );
    assert!(
        toc.allows_screen(ScreenKind::CharacterCreate),
        "`## AllowLoad: Both` must ALSO enable CharacterCreate — `Both` is the strongest \
         AllowLoad value, covering every screen kind"
    );

    assert!(
        toc.dependencies().is_empty(),
        "Zero `## RequiredDep:` / `## Dependencies:` — the notification utility is \
         self-contained: it uses CreateFramePoolCollection (foundational FrameXML), the \
         `plunderstorm-new-dot-lg` atlas (ships with the engine), and exposes a public \
         NotificationUtil table that other addons read by name. Nothing addon-side is required"
    );
    assert!(
        toc.optional_deps().is_empty(),
        "Zero `## OptionalDeps:` — no soft sibling addons. NotificationUtil is a leaf utility, \
         consumed by other addons but consuming none itself"
    );
    assert!(
        toc.saved_variables().is_empty(),
        "Zero saved variables — the notification icon is purely visual state pulled out of a \
         frame pool on demand. Nothing about the icon needs to persist across sessions; the \
         calling addon decides whether to summon an icon based on its own SavedVariables (e.g. \
         `boostUnused`, `newSetAvailable`)"
    );
    assert!(
        !toc.is_game_type_restricted(),
        "TOC omits `## AllowLoadGameType:` — the notification utility loads on every game \
         type. `is_game_type_restricted()` at src/toc.rs:294 returns false when the metadata \
         key is absent"
    );
}

#[test]
fn blizzard_notification_toc_declares_eager_load_in_raw_bytes() {
    let raw =
        std::fs::read_to_string(notification_toc()).expect("Blizzard_Notification TOC reads utf-8");
    assert!(
        raw.contains("## LoadOnDemand: 0"),
        "TOC must declare `## LoadOnDemand: 0` exactly. The explicit `0` is unusual — most \
         eager addons OMIT the key entirely; here the explicit `0` documents intent (the \
         author considered LoD and rejected it because the pool must be ready before any \
         consumer's first Acquire call)"
    );
    assert!(
        raw.contains("## AllowLoad: Both"),
        "TOC must declare `## AllowLoad: Both` exactly — case-sensitive `Both` (the parser at \
         src/toc.rs:307 uses `eq_ignore_ascii_case` so other casings work too, but the \
         canonical retail spelling is `Both`)"
    );
    assert!(
        !raw.contains("## RequiredDep:") && !raw.contains("## Dependencies:"),
        "TOC must NOT declare RequiredDep / Dependencies — the notification utility is a leaf \
         with zero hard dependencies"
    );
    assert!(
        !raw.contains("## SavedVariables"),
        "TOC must NOT declare SavedVariables / SavedVariablesPerCharacter — notification icons \
         are ephemeral pool-acquired frames, no persisted state"
    );
}

#[test]
fn blizzard_notification_toc_lists_two_files_xml_first_lua_after() {
    let toc = TocFile::from_file(&notification_toc()).expect("Blizzard_Notification TOC parses");
    let listed: Vec<String> = toc
        .files
        .iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect();
    assert_eq!(
        listed, NOTIFICATION_TOC_FILES,
        "TOC body must list exactly 2 files in canonical order: Blizzard_Notification.xml \
         first (registers the two virtual templates NotificationIconFrameTemplate + \
         LargeNotificationIconFrameTemplate; the templates MUST exist in the XML registry \
         before the Lua file's CreateFramePool calls reference them by name), then \
         Blizzard_NotificationUtil.lua (creates the pool collection at module-top-level, \
         registers both templates with the pool collection, and publishes the NotificationUtil \
         table). Reversing this order would crash the Lua at module load: \
         `notificationPoolCollection:CreatePool(\"FRAME\", nil, \
         \"NotificationIconFrameTemplate\")` would fail to resolve the template"
    );
}

#[test]
fn blizzard_notification_appears_in_game_and_glue_screens_eager_discovery() {
    let ui = blizzard_ui_dir();

    for screen in [
        ScreenKind::Game,
        ScreenKind::Login,
        ScreenKind::CharacterSelect,
        ScreenKind::CharacterCreate,
    ] {
        let addons = discover_blizzard_addons_for_screen(&ui, screen);
        let found = addons
            .iter()
            .any(|(name, _)| name == "Blizzard_Notification");
        assert!(
            found,
            "Blizzard_Notification (`## AllowLoad: Both`) must auto-discover on screen \
             {screen:?} — `## LoadOnDemand: 0` keeps it in the eager-load set, and \
             `## AllowLoad: Both` makes `allows_screen()` at src/toc.rs:306-313 return true on \
             every screen kind. Both flags together are required: AllowLoad governs which \
             screens see the addon, LoadOnDemand governs whether discovery happens eagerly. \
             The Saved Sets / character-row notification icons render on CharacterSelect, so \
             glue-screen coverage is functional, not vestigial"
        );
    }
}

prefork_full_ui_case! {
fn blizzard_notification_loads_without_addon_specific_lua_errors(env: &WowLuaEnv) {

    let load_errors: Vec<String> = env
        .state()
        .borrow()
        .lua_errors
        .iter()
        .filter(|message| {
            message.contains("Blizzard_Notification")
                || message.contains("NotificationUtil")
                || message.contains("NotificationIconFrameTemplate")
                || message.contains("LargeNotificationIconFrameTemplate")
        })
        .cloned()
        .collect();
    assert!(
        load_errors.is_empty(),
        "Blizzard_Notification emitted addon-specific Lua errors during load:\n  {}",
        load_errors.join("\n  ")
    );
}
}

prefork_full_ui_case! {
fn blizzard_notification_is_addon_loaded_after_eager_sweep(env: &WowLuaEnv) {

    let loaded: bool = env
        .eval("return C_AddOns.IsAddOnLoaded('Blizzard_Notification')")
        .expect("IsAddOnLoaded probe succeeds");
    assert!(
        loaded,
        "C_AddOns.IsAddOnLoaded('Blizzard_Notification') must return true after the eager \
         Game-screen sweep — `## LoadOnDemand: 0` puts the addon in the eager set, no \
         explicit load_addon call needed"
    );
}
}

prefork_full_ui_case! {
fn blizzard_notification_publishes_notification_util_table_with_three_methods(env: &WowLuaEnv) {

    let kind: String = env
        .eval(&format!("return type(_G.{PUBLIC_NAMESPACE})"))
        .unwrap_or_else(|err| panic!("type(_G.{PUBLIC_NAMESPACE}) probe failed: {err}"));
    assert_eq!(
        kind, "table",
        "_G.{PUBLIC_NAMESPACE} must publish as a table — Blizzard_NotificationUtil.lua line \
         14 declares `NotificationUtil = {{}};` then attaches three module-level functions \
         (AcquireNotification, AcquireLargeNotification, ReleaseNotification) on lines 16-27. \
         This is the public consumer-facing surface; addons that want a notification icon \
         call NotificationUtil.AcquireNotification(...) and stash the returned frame"
    );

    for method in PUBLIC_NAMESPACE_METHODS {
        let method_kind: String = env
            .eval(&format!("return type(_G.{PUBLIC_NAMESPACE}.{method})"))
            .unwrap_or_else(|err| panic!("type(NotificationUtil.{method}) probe failed: {err}"));
        assert_eq!(
            method_kind, "function",
            "NotificationUtil.{method} must publish as a function — these are the 3 public \
             methods. AcquireNotification (line 16) wraps the small 25x25 \
             NotificationIconFrameTemplate; AcquireLargeNotification (line 20) wraps the \
             30x30 LargeNotificationIconFrameTemplate; ReleaseNotification (line 24) reparents \
             the frame to nil and returns it to the pool. All three delegate to the \
             file-private AcquireNotificationFrame helper which routes through the pool \
             collection"
        );
    }
}
}

prefork_full_ui_case! {
fn blizzard_notification_publishes_two_backcompat_global_functions(env: &WowLuaEnv) {

    for name in BACKCOMPAT_GLOBAL_FUNCTIONS {
        let kind: String = env
            .eval(&format!("return type(_G.{name})"))
            .unwrap_or_else(|err| panic!("type(_G.{name}) probe failed: {err}"));
        assert_eq!(
            kind, "function",
            "_G.{name} must publish as a function — Blizzard_NotificationUtil.lua lines 29-35 \
             declare two top-level wrapper functions that delegate to NotificationUtil.* — \
             the legacy underscore-naming pattern (`Foo_Bar`) used by addons before the \
             namespace-table pattern took hold. The `Acquire` wrapper covers ONLY the Large \
             variant (no backcompat alias for the small AcquireNotification, suggesting the \
             small variant is newer or has no legacy callers); the `Release` wrapper covers \
             the single shared release path. Both are kept for any older Blizzard / addon \
             code that captured the global name before NotificationUtil existed as a table"
        );
    }

    let small_alias_absent: String = env
        .eval("return type(_G.NotificationUtil_AcquireNotification)")
        .expect("NotificationUtil_AcquireNotification absence probe succeeds");
    assert_eq!(
        small_alias_absent, "nil",
        "_G.NotificationUtil_AcquireNotification must NOT exist — the underscore-aliased \
         backcompat surface deliberately covers ONLY the Large + Release pair. Asserting the \
         absence pins the asymmetry: introducing a small-variant alias would silently \
         broaden the legacy API surface beyond what retail ships"
    );
}
}

prefork_full_ui_case! {
fn blizzard_notification_does_not_leak_virtual_templates_as_globals(env: &WowLuaEnv) {

    for template in VIRTUAL_TEMPLATES_NOT_IN_GLOBALS {
        let kind: String = env
            .eval(&format!("return type(_G.{template})"))
            .unwrap_or_else(|err| panic!("type(_G.{template}) probe failed: {err}"));
        assert_eq!(
            kind, "nil",
            "_G.{template} must be nil — `<Frame name=\"{template}\" ... virtual=\"true\">` at \
             top level of Blizzard_Notification.xml registers the frame as a TEMPLATE in the \
             XML template registry, NOT as a `_G` global. Templates are looked up by name \
             from CreateFramePool's third argument (\
             `notificationPoolCollection:CreatePool(\"FRAME\", nil, \
             \"NotificationIconFrameTemplate\")` at Blizzard_NotificationUtil.lua line 3), \
             never via `_G`"
        );
    }
}
}

prefork_full_ui_case! {
fn blizzard_notification_does_not_leak_file_private_helpers_as_globals(env: &WowLuaEnv) {

    for private in &["notificationPoolCollection", "AcquireNotificationFrame"] {
        let kind: String = env
            .eval(&format!("return type(_G.{private})"))
            .unwrap_or_else(|err| panic!("type(_G.{private}) probe failed: {err}"));
        assert_eq!(
            kind, "nil",
            "_G.{private} must be nil — Blizzard_NotificationUtil.lua line 2 declares \
             `local notificationPoolCollection = CreateFramePoolCollection();` and line 6 \
             declares `local function AcquireNotificationFrame(...)`. Both are file-scoped \
             `local` declarations that must NOT escape to `_G`. Leaking either would expose \
             the pool's mutable backing state to arbitrary callers (allowing them to \
             ReleaseAll the entire pool, breaking every consumer's stashed frame reference)"
        );
    }
}
}
