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

fn matchmaking_queue_dir() -> PathBuf {
    blizzard_ui_dir().join("Blizzard_MatchmakingQueueDisplay")
}

fn matchmaking_queue_toc() -> PathBuf {
    matchmaking_queue_dir().join("Blizzard_MatchmakingQueueDisplay.toc")
}

const MATCHMAKING_QUEUE_TOC_FILES: &[&str] = &[
    "Blizzard_MatchmakingQueueDisplay.lua",
    "Blizzard_MatchmakingQueueDisplay.xml",
];

const PUBLISHED_MIXINS: &[&str] = &[
    "QueueTypeSelectionButtonMixin",
    "QueueTypeSettingsFrameMixin",
    "QueueReadyButtonMixin",
    "MatchmakingQueueFrameMixin",
    "LeaveQueueButtonMixin",
];

const VIRTUAL_TEMPLATES: &[&str] = &[
    "QueueTypeSelectionButtonTemplate",
    "QueueReadyButtonTemplate",
    "LeaveQueueButtonTemplate",
    "QueueTypeSettingsFrameTemplate",
    "QueueSpinnerTemplate",
    "MatchmakingQueueFrameTemplate",
];

const QUEUE_TYPE_BUTTON_METHODS: &[&str] = &[
    "OnLoad",
    "OnClick",
    "OnEnter",
    "OnLeave",
    "SetSelected",
    "SetEnabled",
];

const QUEUE_TYPE_SETTINGS_METHODS: &[&str] = &[
    "OnLoad",
    "OnShow",
    "OnHide",
    "OnEvent",
    "OnQueueTypeSelected",
    "GetQueueType",
    "OnLeaveQueue",
    "UpdateButtons",
    "IsSelectionActive",
    "UpdateQueueTypeSelection",
    "SetPlayerReady",
];

const QUEUE_READY_BUTTON_METHODS: &[&str] = &[
    "OnShow",
    "OnHide",
    "OnEvent",
    "OnClick",
    "HasValidQueue",
    "Update",
];

const MATCHMAKING_QUEUE_FRAME_METHODS: &[&str] = &[
    "OnLoad",
    "ResetTimer",
    "OnTick",
    "UpdateTimerText",
    "StartTimer",
    "SetWaiting",
    "SetSquadSize",
];

const FILE_LOCAL_HELPERS: &[&str] = &[
    "GetQueueTypeButton",
    "ShowReadyGlow",
    "QueueTypeSettingsFrameEvents",
    "QueueReadyButtonEvents",
    "QueueTimeFormatter",
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

fn assert_mixin_methods_present(env: &WowLuaEnv, mixin_name: &str, methods: &[&str]) {
    for method in methods {
        let kind: String = env
            .eval(&format!("return type({mixin_name}['{method}'])"))
            .unwrap_or_else(|err| panic!("{mixin_name}.{method} probe failed: {err}"));
        assert_eq!(
            kind, "function",
            "{mixin_name}.{method} must publish as a function — defined directly on the mixin \
             via `function {mixin_name}:{method}()` in Blizzard_MatchmakingQueueDisplay.lua"
        );
    }
}

#[test]
fn blizzard_matchmaking_queue_find_toc_resolves_bare_variant() {
    let resolved = find_toc_file(&matchmaking_queue_dir())
        .expect("Blizzard_MatchmakingQueueDisplay TOC should resolve");
    assert_eq!(
        resolved,
        matchmaking_queue_toc(),
        "Blizzard_MatchmakingQueueDisplay ships exactly one bare TOC. The Plunderstorm queue \
         display module is mainline-restricted but ships a single bare TOC (no \
         `_Mainline.toc` variant) — the bare TOC resolves via `find_toc_file` after the \
         `_Mainline.toc` lookup misses"
    );
}

#[test]
fn blizzard_matchmaking_queue_toc_declares_eager_load_on_both_screens_with_sharedxml_dep() {
    let toc = TocFile::from_file(&matchmaking_queue_toc())
        .expect("Blizzard_MatchmakingQueueDisplay TOC should parse");
    assert!(
        !toc.is_load_on_demand(),
        "Blizzard_MatchmakingQueueDisplay omits `## LoadOnDemand:` — `## DefaultState: enabled` \
         makes it an eager-load addon. The Plunderstorm queue mixins / templates must be live \
         before any consumer addon (Blizzard_GlueXML PlunderstormLobby, Blizzard_PVPUI, \
         Blizzard_PlunderstormPrematchUI) instantiates a frame inheriting any of the 6 virtual \
         templates"
    );
    assert!(!toc.is_load_first());
    assert!(!toc.is_secure_env());
    assert_eq!(
        toc.dependencies(),
        vec!["Blizzard_SharedXML".to_string()],
        "Blizzard_MatchmakingQueueDisplay declares `## Dependencies: Blizzard_SharedXML` — \
         provides the SelectableButtonMixin (called via SelectableButtonMixin.OnLoad / \
         SetSelectedState delegation in QueueTypeSelectionButtonMixin), the SelectableButtonTemplate \
         the Button virtual template inherits, the SharedButtonTemplate the Ready / LeaveQueue \
         buttons inherit, the CallbackRegistrantTemplate / CallbackRegistrantMixin the Settings \
         frame uses for OnShow / OnHide registration delegation, the SecondsFormatterMixin the \
         QueueTimeFormatter local CreatesFromMixins, and the GridLayoutFrame template the \
         QueueContainer child consumes"
    );
    assert!(toc.optional_deps().is_empty());
    assert!(
        toc.saved_variables().is_empty(),
        "Blizzard_MatchmakingQueueDisplay declares zero saved variables — every queue selection / \
         ready state is server-driven via C_WoWLabsMatchmaking; no per-character persistence"
    );
}

#[test]
fn blizzard_matchmaking_queue_toc_pins_to_mainline_with_allow_load_both() {
    let toc = TocFile::from_file(&matchmaking_queue_toc())
        .expect("Blizzard_MatchmakingQueueDisplay TOC should parse");

    assert!(
        !toc.is_game_type_restricted(),
        "Blizzard_MatchmakingQueueDisplay declares `## AllowLoadGameType: mainline` — \
         `is_game_type_restricted()` (src/toc.rs:294-302) treats both `mainline` and `standard` \
         as the retail-unrestricted flavor and returns false. The Plunderstorm event-realm \
         matchmaking surface ships only on retail (mainline is the legacy retail flavor name; \
         classic flavors get the addon stripped because the simulator's discover sweep only \
         runs on retail), but the `is_game_type_restricted` accessor specifically distinguishes \
         classic-only / plunderstorm-only / etc restrictions, NOT the retail mainline label"
    );

    let raw = std::fs::read_to_string(matchmaking_queue_toc())
        .expect("Blizzard_MatchmakingQueueDisplay TOC should read");
    assert!(
        raw.contains("## AllowLoadGameType: mainline"),
        "TOC must declare `## AllowLoadGameType: mainline` — pins the addon to retail mainline. \
         The classic flavors don't ship the C_WoWLabsMatchmaking / C_GameRules / \
         Enum.PartyPlaylistEntry surface this addon consumes"
    );

    for screen in [
        ScreenKind::Game,
        ScreenKind::Login,
        ScreenKind::CharacterSelect,
        ScreenKind::CharacterCreate,
    ] {
        assert!(
            toc.allows_screen(screen),
            "Blizzard_MatchmakingQueueDisplay declares `## AllowLoad: Both` — `allows_screen` \
             (src/toc.rs:307) returns true for every ScreenKind when the value matches `both` \
             case-insensitively. The queue display surfaces on glue screens (PlunderstormLobby \
             at the character-select / login flow consumes the templates) AND on the game \
             screen (Blizzard_PVPUI consumes QueueTypeSettingsFrameTemplate as PVE/PVP queue \
             selector). The capitalized `Both` literal in the TOC normalizes through \
             `eq_ignore_ascii_case`. (Screen tested: {screen:?})"
        );
    }
}

#[test]
fn blizzard_matchmaking_queue_toc_lists_lua_then_xml_in_order() {
    let toc = TocFile::from_file(&matchmaking_queue_toc())
        .expect("Blizzard_MatchmakingQueueDisplay TOC should parse");
    assert_eq!(
        toc.files
            .iter()
            .map(|p| p.to_string_lossy().into_owned())
            .collect::<Vec<_>>(),
        MATCHMAKING_QUEUE_TOC_FILES,
        "TOC body must list lua FIRST then xml — even though the XML cross-loads the lua via \
         `<Script file=\"Blizzard_MatchmakingQueueDisplay.lua\"/>` (xml:3), the TOC-level lua \
         entry runs first to publish all 5 mixin globals at file scope before any \
         XML-instantiated frame's `mixin=\"QueueTypeSelectionButtonMixin\"` etc tries to \
         resolve via `_G`. The XML-side cross-load is idempotent (Blizzard's pattern: redefining \
         `MixinName = {{}}` would clobber, but `{{}}` is identity-fresh per evaluation, so the \
         second pass effectively re-declares the same mixin tables)"
    );
}

#[test]
fn blizzard_matchmaking_queue_directory_holds_three_entries() {
    let entries = std::fs::read_dir(matchmaking_queue_dir())
        .expect("Blizzard_MatchmakingQueueDisplay directory should read")
        .count();
    assert_eq!(
        entries, 3,
        "Directory must hold exactly 3 entries (1 TOC + 1 lua + 1 xml; no flavor subdirectory, \
         no Localization.lua — every text string the addon references comes from global locale \
         tables: WOWLABS_JOIN_GAME / WOWLABS_READY_GAME / WOWLABS_FIND_GAME_SOLO / DUO / TRIO / \
         WOWLABS_WAITING_ON_OTHER_PLAYERS / FRONT_END_LOBBY_PRACTICE / SOLOS / DUOS / TRIOS / \
         CANCEL / ERR_NOT_LEADER)"
    );
}

#[test]
fn blizzard_matchmaking_queue_auto_discovered_on_every_screen() {
    for screen in [
        ScreenKind::Game,
        ScreenKind::Login,
        ScreenKind::CharacterSelect,
        ScreenKind::CharacterCreate,
    ] {
        let addons = discover_blizzard_addons_for_screen(&blizzard_ui_dir(), screen);
        let found = addons
            .iter()
            .any(|(name, _)| name == "Blizzard_MatchmakingQueueDisplay");
        assert!(
            found,
            "Blizzard_MatchmakingQueueDisplay must be auto-discovered on every ScreenKind. The \
             `## DefaultState: enabled` + `## AllowLoad: Both` combo + retail mainline-only \
             game-type-restriction means every retail screen's discovery sweep picks it up into \
             the eager `addons` set. The queue display has to be live on glue screens \
             (PlunderstormLobby is a glue-only consumer) AND on the game screen (PVPUI's \
             QueueSelect consumes the same template). (Screen tested: {screen:?})"
        );
    }
}

prefork_full_ui_case! {
fn blizzard_matchmaking_queue_loads_without_addon_specific_lua_errors(env: &WowLuaEnv) {

    let load_errors: Vec<String> = env
        .state()
        .borrow()
        .lua_errors
        .iter()
        .filter(|message| {
            message.contains("Blizzard_MatchmakingQueueDisplay")
                || message.contains("QueueTypeSelectionButtonMixin")
                || message.contains("QueueTypeSettingsFrameMixin")
                || message.contains("QueueReadyButtonMixin")
                || message.contains("MatchmakingQueueFrameMixin")
                || message.contains("LeaveQueueButtonMixin")
        })
        .cloned()
        .collect();
    assert!(
        load_errors.is_empty(),
        "Blizzard_MatchmakingQueueDisplay emitted addon-specific Lua errors during load:\n  {}",
        load_errors.join("\n  ")
    );
}
}

prefork_full_ui_case! {
fn blizzard_matchmaking_queue_is_addon_loaded_after_auto_discovery(env: &WowLuaEnv) {

    let loaded: bool = env
        .eval("return C_AddOns.IsAddOnLoaded('Blizzard_MatchmakingQueueDisplay')")
        .expect("IsAddOnLoaded probe should succeed");
    assert!(
        loaded,
        "C_AddOns.IsAddOnLoaded('Blizzard_MatchmakingQueueDisplay') must return true after the \
         eager auto-discovery sweep — proves the queue-display addon registers with the \
         loaded-set during the standard Game-screen boot pipeline, no explicit load_addon call \
         required"
    );
}
}

prefork_full_ui_case! {
fn blizzard_matchmaking_queue_publishes_five_mixin_globals_as_tables(env: &WowLuaEnv) {

    for mixin in PUBLISHED_MIXINS {
        let kind: String = env
            .eval(&format!("return type({mixin})"))
            .unwrap_or_else(|err| panic!("{mixin} probe failed: {err}"));
        assert_eq!(
            kind, "table",
            "{mixin} must publish at `_G` as a table — declared via `{mixin} = {{}}` at file \
             scope in Blizzard_MatchmakingQueueDisplay.lua. None of the 5 mixins use \
             CreateFromMixins (they're standalone tables); QueueTypeSelectionButtonMixin instead \
             delegates to SelectableButtonMixin via direct `SelectableButtonMixin.OnLoad(self)` \
             / `SelectableButtonMixin.SetSelectedState(self, selected)` calls without inheriting \
             the method table"
        );
    }
}
}

prefork_full_ui_case! {
fn blizzard_matchmaking_queue_type_selection_button_mixin_carries_six_methods(env: &WowLuaEnv) {
    assert_mixin_methods_present(
        &env,
        "QueueTypeSelectionButtonMixin",
        QUEUE_TYPE_BUTTON_METHODS,
    );
}
}

prefork_full_ui_case! {
fn blizzard_matchmaking_queue_type_settings_frame_mixin_carries_eleven_methods(env: &WowLuaEnv) {
    assert_mixin_methods_present(
        &env,
        "QueueTypeSettingsFrameMixin",
        QUEUE_TYPE_SETTINGS_METHODS,
    );
}
}

prefork_full_ui_case! {
fn blizzard_matchmaking_queue_ready_button_mixin_carries_six_methods(env: &WowLuaEnv) {
    assert_mixin_methods_present(&env, "QueueReadyButtonMixin", QUEUE_READY_BUTTON_METHODS);
}
}

prefork_full_ui_case! {
fn blizzard_matchmaking_queue_frame_mixin_carries_seven_methods(env: &WowLuaEnv) {
    assert_mixin_methods_present(
        &env,
        "MatchmakingQueueFrameMixin",
        MATCHMAKING_QUEUE_FRAME_METHODS,
    );
}
}

prefork_full_ui_case! {
fn blizzard_matchmaking_queue_leave_queue_button_mixin_carries_single_onclick_method(env: &WowLuaEnv) {

    let onclick_kind: String = env
        .eval("return type(LeaveQueueButtonMixin.OnClick)")
        .expect("LeaveQueueButtonMixin.OnClick probe should succeed");
    assert_eq!(
        onclick_kind, "function",
        "LeaveQueueButtonMixin.OnClick must publish as a function — the entire mixin is just \
         the OnClick handler at lua:426 firing `EventRegistry:TriggerEvent(\
         'MatchmakingQueue.LeaveQueue')`. The QueueTypeSettingsFrameMixin's RegisterCallback \
         hook on that event triggers OnLeaveQueue → SetPlayerReady(false) + \
         GameReadyButton:Update(), so the mixin is the public bridge between an XML-bound Cancel \
         button and the queue-cancellation flow"
    );
}
}

prefork_full_ui_case! {
fn blizzard_matchmaking_queue_file_local_helpers_not_exposed_globally(env: &WowLuaEnv) {

    for helper in FILE_LOCAL_HELPERS {
        let kind: String = env
            .eval(&format!("return type({helper})"))
            .unwrap_or_else(|err| panic!("{helper} probe failed: {err}"));
        assert_eq!(
            kind, "nil",
            "{helper} must stay nil at `_G` — declared as `local` (lua:62 / 234 / 249 / 287 / \
             357) so it's scoped to the addon's Lua chunk only and never exposed as a global. \
             QueueTypeSettingsFrameEvents and QueueReadyButtonEvents are private \
             FrameUtil.RegisterFrameForEvents arrays; GetQueueTypeButton dispatches \
             Enum.PartyPlaylistEntry → child-button lookup; ShowReadyGlow wraps GlowEmitterFactory \
             show/hide for the GreenGlow anim; QueueTimeFormatter is a CreateFromMixins-based \
             SecondsFormatterMixin instance pre-initialized for the in-queue timer text"
        );
    }
}
}

prefork_full_ui_case! {
fn blizzard_matchmaking_queue_six_virtual_templates_register_with_xml_template_registry(env: &WowLuaEnv) {
    let _env = env;

    for template in VIRTUAL_TEMPLATES {
        let entry = wow_ui_sim::xml::get_template(template);
        assert!(
            entry.is_some(),
            "Virtual template `{template}` must register with the XML template registry — \
             declared as `<{{Button|Frame}} name=\"{template}\" virtual=\"true\" ...>` in \
             Blizzard_MatchmakingQueueDisplay.xml. The 6 templates are: \
             QueueTypeSelectionButtonTemplate (76 by 76 Button, \
             mixin=QueueTypeSelectionButtonMixin, inherits=SelectableButtonTemplate, \
             motionScriptsWhileDisabled=true, enableMouse=true); QueueReadyButtonTemplate \
             (255 by 50 Button, mixin=QueueReadyButtonMixin, inherits=SharedButtonTemplate, \
             text=WOWLABS_JOIN_GAME); LeaveQueueButtonTemplate (255 by 50 Button, \
             mixin=LeaveQueueButtonMixin, inherits=SharedButtonTemplate, text=CANCEL, \
             frameStrata=DIALOG); QueueTypeSettingsFrameTemplate (270 by 155 Frame, \
             mixin=QueueTypeSettingsFrameMixin, inherits=CallbackRegistrantTemplate, \
             frameStrata=HIGH); QueueSpinnerTemplate (86 by 86 Frame, mixin=SpinnerMixin); \
             MatchmakingQueueFrameTemplate (270 by 155 Frame, \
             mixin=MatchmakingQueueFrameMixin, frameStrata=DIALOG, hidden=true)"
        );
    }
}
}

prefork_full_ui_case! {
fn blizzard_matchmaking_queue_settings_frame_template_resolves_via_create_frame(env: &WowLuaEnv) {

    {
        let mut state = env.state().borrow_mut();
        state.lua_errors.clear();
        state.lua_error_records.clear();
        state.lua_error_counts.clear();
    }

    let frame_created: bool = env
        .eval(
            "local f = CreateFrame('Frame', nil, UIParent, 'QueueTypeSettingsFrameTemplate'); \
             return f ~= nil",
        )
        .expect("CreateFrame query should succeed");
    assert!(
        frame_created,
        "QueueTypeSettingsFrameTemplate must instantiate via CreateFrame. The template's \
         OnLoad handler (QueueTypeSettingsFrameMixin:OnLoad at lua:67) calls \
         AddDynamicEventMethod on EventRegistry and RegisterCallback for \
         MatchmakingQueue.LeaveQueue — both of which the simulator's stub EventRegistry \
         supports. The QueueContainer child grid pre-populates 4 child buttons (Training / \
         Solo / Duo / Trio) inheriting QueueTypeSelectionButtonTemplate with their \
         queueTypeString / queueTypeIcon / queueTypeIconSelected KeyValues set"
    );
}
}

prefork_full_ui_case! {
fn blizzard_matchmaking_queue_settings_frame_template_carries_four_queue_type_children(env: &WowLuaEnv) {

    let queue_container_resolves: bool = env
        .eval(
            "local f = CreateFrame('Frame', nil, UIParent, 'QueueTypeSettingsFrameTemplate'); \
             return f.QueueContainer ~= nil \
                and f.QueueContainer.Training ~= nil \
                and f.QueueContainer.Solo ~= nil \
                and f.QueueContainer.Duo ~= nil \
                and f.QueueContainer.Trio ~= nil",
        )
        .expect("QueueContainer probe should succeed");
    assert!(
        queue_container_resolves,
        "QueueTypeSettingsFrameTemplate must instantiate a QueueContainer GridLayoutFrame child \
         with 4 sub-buttons Training / Solo / Duo / Trio (xml:72-127). Each sub-button inherits \
         QueueTypeSelectionButtonTemplate and carries 4 KeyValues: layoutIndex, \
         partyPlaylistEntry (Enum.PartyPlaylistEntry.* type=global), queueTypeString \
         (FRONT_END_LOBBY_* type=global), queueTypeIcon / queueTypeIconSelected \
         (plunderstorm-glues-queueselector-* atlas paths)"
    );
}
}

prefork_full_ui_case! {
fn blizzard_matchmaking_queue_frame_template_carries_spinner_child(env: &WowLuaEnv) {

    let spinner_resolves: bool = env
        .eval(
            "local f = CreateFrame('Frame', nil, UIParent, 'MatchmakingQueueFrameTemplate'); \
             return f.QueueTimerSpinner ~= nil",
        )
        .expect("QueueTimerSpinner probe should succeed");
    assert!(
        spinner_resolves,
        "MatchmakingQueueFrameTemplate must instantiate a QueueTimerSpinner child inheriting \
         QueueSpinnerTemplate (xml:163). The spinner is the rotating ring + queue-size icon at \
         the top of the queue-pending dialog; its OnShow / OnHide handlers (SpinnerMixin) \
         drive the Anim AnimationGroup's Rotation childKey=Ring duration=2 degrees=-360"
    );
}
}
