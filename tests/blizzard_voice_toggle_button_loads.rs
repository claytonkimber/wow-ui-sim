use std::path::PathBuf;

use wow_ui_sim::loader::load_addon;
use wow_ui_sim::loader::{discover_blizzard_addons_for_screen, find_toc_file};
use wow_ui_sim::lua_api::WowLuaEnv;
use wow_ui_sim::screen::ScreenKind;
use wow_ui_sim::startup::fire_startup_events_for_screen;
use wow_ui_sim::toc::TocFile;

fn blizzard_ui_dir() -> PathBuf {
    wow_ui_sim::paths::default_blizzard_ui_addons_path().expect("Blizzard UI cache should be available")
}

fn voice_toggle_button_dir() -> PathBuf {
    blizzard_ui_dir().join("Blizzard_VoiceToggleButton")
}

fn voice_toggle_button_toc() -> PathBuf {
    voice_toggle_button_dir().join("Blizzard_VoiceToggleButton.toc")
}

const GLUE_SCREENS: &[ScreenKind] = &[
    ScreenKind::Login,
    ScreenKind::CharacterSelect,
    ScreenKind::CharacterCreate,
];

const TOC_DEPENDENCIES: &[&str] = &["Blizzard_SharedXML"];

const BODY_FILES: &[&str] = &["VoiceToggleButton.lua", "VoiceToggleButton.xml"];

const MIXIN_GLOBALS: &[&str] = &[
    "VoiceToggleButtonMixin",
    "VoiceToggleButtonAlwaysVisibileMixin",
    "VoiceToggleButtonOnlyVisibleWhenLoggedInMixin",
    "VoiceToggleMuteMixin",
    "VoiceToggleDeafenMixin",
    "RosterToggleButtonMixin",
    "RosterSelfDeafenButtonMixin",
    "RosterSelfMuteButtonMixin",
    "RosterMemberMuteButtonMixin",
];

const FREE_FUNCTION_GLOBALS: &[&str] = &[
    "VoiceChat_ToggleMutedFromUserAction",
    "VoiceChat_ToggleDeafenedFromUserAction",
];

const STATE_CONSTANT_VALUES: &[(&str, i32)] = &[
    ("MUTE_SILENCE_STATE_NONE", 0),
    ("MUTE_SILENCE_STATE_MUTE", 1),
    ("MUTE_SILENCE_STATE_SILENCE", 2),
    ("MUTE_SILENCE_STATE_MUTE_AND_SILENCE", 3),
    ("MUTE_SILENCE_STATE_PARENTAL_MUTE", 4),
    ("MUTE_SILENCE_STATE_MUTE_AND_PARENTAL_MUTE", 5),
];

const VIRTUAL_TEMPLATES: &[&str] = &[
    "VoiceToggleButtonTemplate",
    "ToggleVoiceDeafenButtonTemplate",
    "ToggleVoiceMuteButtonTemplate",
    "RosterVoiceToggleButtonTemplate",
    "RosterSelfDeafenButtonTemplate",
    "RosterSelfMuteButtonTemplate",
    "RosterMemberMuteButtonTemplate",
];

fn fresh_game_env() -> WowLuaEnv {
    let env = WowLuaEnv::new().expect("Failed to create Lua environment");
    env.set_screen_size(1024.0, 768.0);
    env.set_screen_mode(ScreenKind::Game);
    {
        let mut state = env.state().borrow_mut();
        state.addon_base_paths = vec![blizzard_ui_dir()];
    }
    wow_ui_sim::xml::register_intrinsic_templates();
    env
}

fn load_full_game_ui() -> WowLuaEnv {
    let env = fresh_game_env();

    let addons = discover_blizzard_addons_for_screen(&blizzard_ui_dir(), ScreenKind::Game);
    for (name, toc_path) in &addons {
        load_addon(&env.loader_env(), toc_path)
            .unwrap_or_else(|err| panic!("[load {name}] FAILED: {err}"));
    }

    env.apply_post_load_workarounds();
    fire_startup_events_for_screen(&env, ScreenKind::Game);
    env
}

#[test]
fn find_toc_file_resolves_bare_variant() {
    let resolved =
        find_toc_file(&voice_toggle_button_dir()).expect("VoiceToggleButton TOC resolves");
    assert_eq!(
        resolved,
        voice_toggle_button_toc(),
        "find_toc_file at src/loader/mod.rs:65-95 falls back to the \
         bare `<addon>.toc` when no `_Mainline.toc` exists. \
         Blizzard_VoiceToggleButton ships exactly ONE TOC \
         (`Blizzard_VoiceToggleButton.toc`, no flavor suffix) — the \
         button templates are flavor-agnostic (PropertyButtonTemplate \
         + state-atlas + visibility-query plumbing works identically \
         on mainline and classic), so a single TOC is sufficient. \
         Note: per session memory, classic-flavor Blizzard_UnitPopup_Mists \
         declares this as a hard dep while mainline \
         Blizzard_UnitPopup folds the voice toggle into \
         Blizzard_UnitPopupShared's voice plumbing instead — but \
         this addon itself is single-flavor"
    );
}

#[test]
fn toc_is_eager_with_one_dependency() {
    let toc = TocFile::from_file(&voice_toggle_button_toc()).expect("TOC parses");

    assert!(
        !toc.is_load_on_demand(),
        "No `## LoadOnDemand` directive → eagerly loaded. The button \
         templates must be alive before any addon that references \
         them via XML inherits=\"VoiceToggleButtonTemplate\" can \
         instantiate frames; eager loading guarantees the template \
         registry is populated before consumer XML parses"
    );

    let deps = toc.dependencies();
    assert_eq!(
        deps, TOC_DEPENDENCIES,
        "TOC must declare exactly Blizzard_SharedXML as a hard dep. \
         Blizzard_SharedXML provides PropertyButtonTemplate (the XML \
         base for VoiceToggleButtonTemplate via `inherits=\"...\"`), \
         PropertyButtonMixin (the Lua base whose OnLoad is called \
         from VoiceToggleButtonMixin:OnLoad — line 4 of \
         VoiceToggleButton.lua), CreateFromMixins (used 8 times to \
         build the inheritance graph), and FlagsMixin (used by \
         VoiceToggleMuteMixin:SetupMuteButton at line 82 to track \
         the MUTE/SILENCE/PARENTAL_MUTE state combinations). Got: {deps:?}"
    );

    assert!(toc.optional_deps().is_empty());
    assert!(toc.load_with().is_empty());
    assert!(toc.saved_variables().is_empty());
    assert!(toc.saved_variables_per_character().is_empty());
    assert!(!toc.is_load_first());
    assert!(toc.default_enabled());
}

#[test]
fn allow_load_game_restricts_to_in_world() {
    let toc = TocFile::from_file(&voice_toggle_button_toc()).expect("TOC parses");

    assert!(
        toc.allows_screen(ScreenKind::Game),
        "`## AllowLoad: game` (lowercase, matched case-insensitively \
         at toc.rs:308 via eq_ignore_ascii_case) → Game-only. The \
         voice toggle templates are only consumed by in-world UI \
         (chat frame voice button, party/raid roster voice \
         indicators); glue screens have no voice-chat surface that \
         needs them"
    );
    for screen in GLUE_SCREENS {
        assert!(
            !toc.allows_screen(*screen),
            "Glue screen {screen:?} must be excluded — `AllowLoad: \
             game` matches only the Game variant via toc.rs:308"
        );
    }
}

#[test]
fn no_allow_load_game_type_means_unrestricted() {
    let toc = TocFile::from_file(&voice_toggle_button_toc()).expect("TOC parses");

    assert!(
        !toc.is_game_type_restricted(),
        "TOC has no `## AllowLoadGameType` directive — \
         is_game_type_restricted() at toc.rs:294-302 returns false \
         when the metadata key is absent (unwrap_or(false) branch). \
         The addon loads on every flavor without filtering, which is \
         what enables Classic Mists's Blizzard_UnitPopup_Mists to \
         take it as a hard dep without itself needing flavor-specific \
         routing"
    );
}

#[test]
fn toc_raw_bytes_pin_directives_and_body_files() {
    let raw = std::fs::read_to_string(voice_toggle_button_toc()).expect("TOC reads utf-8");

    let expected_directives = [
        "## Title: Blizzard_VoiceToggleButton",
        "## Author: Blizzard Entertainment",
        "## DefaultState: enabled",
        "## Dependencies: Blizzard_SharedXML",
        "## AllowLoad: game",
    ];

    for line in expected_directives {
        assert!(raw.contains(line), "Raw TOC must pin directive `{line}`");
    }

    for path in BODY_FILES {
        assert!(
            raw.contains(path),
            "Raw TOC must pin body path `{path}`. Body order is \
             load-critical: VoiceToggleButton.lua MUST run before \
             VoiceToggleButton.xml because the XML's mixin=\"...\" \
             attributes resolve mixin tables by name at template \
             registration time — if the XML loaded first, every \
             template registration would bind a nil mixin. The same \
             addon-internal load-order constraint applies to all \
             mixin-bearing addons, but it's especially tight here \
             since the .xml file references 6 different mixin \
             globals (VoiceToggleButtonMixin, VoiceToggleDeafenMixin, \
             VoiceToggleMuteMixin, RosterSelfDeafenButtonMixin, \
             RosterSelfMuteButtonMixin, RosterMemberMuteButtonMixin)"
        );
    }

    assert!(!raw.contains("## LoadOnDemand"));
    assert!(!raw.contains("## LoadFirst"));
    assert!(!raw.contains("## LoadWith"));
    assert!(!raw.contains("## OptionalDeps"));
    assert!(!raw.contains("## SavedVariables"));
    assert!(
        !raw.contains("## AllowLoadGameType"),
        "TOC must not carry an AllowLoadGameType directive — the \
         absence is what makes is_game_type_restricted() return \
         false; this is also what lets the addon load unmodified on \
         classic Mists builds"
    );
}

#[test]
fn body_files_exist_on_disk() {
    for path in BODY_FILES {
        let resolved = voice_toggle_button_dir().join(path);
        assert!(
            resolved.is_file(),
            "Body file `{path}` must exist at {resolved:?}. The \
             addon ships exactly two body files plus the TOC — \
             nothing else (no Localization.lua, no per-flavor \
             subdirs, no xsd schema)"
        );
    }
}

#[test]
fn appears_in_game_eager_discovery() {
    let addons = discover_blizzard_addons_for_screen(&blizzard_ui_dir(), ScreenKind::Game);
    let found = addons
        .iter()
        .any(|(name, _)| name == "Blizzard_VoiceToggleButton");
    assert!(
        found,
        "Blizzard_VoiceToggleButton must appear in Game eager \
         discovery — chat frame's mute/deafen buttons + party/raid \
         roster voice indicators consume the templates, so the addon \
         must be alive at PLAYER_ENTERING_WORLD"
    );
}

#[test]
fn absent_from_glue_screens_eager_discovery() {
    for screen in GLUE_SCREENS {
        let addons = discover_blizzard_addons_for_screen(&blizzard_ui_dir(), *screen);
        let found = addons
            .iter()
            .any(|(name, _)| name == "Blizzard_VoiceToggleButton");
        assert!(
            !found,
            "Blizzard_VoiceToggleButton must NOT appear on \
             {screen:?} — AllowLoad:game restricts to in-world via \
             toc.rs:308, checked at loader/mod.rs:527 BEFORE pool \
             partitioning"
        );
    }
}

#[test]
fn dep_directories_exist_on_disk() {
    for dep in TOC_DEPENDENCIES {
        let dir = blizzard_ui_dir().join(dep);
        assert!(
            dir.is_dir(),
            "Hard-dep directory `{dep}` must exist on disk"
        );
        assert!(
            find_toc_file(&dir).is_some(),
            "{dep} must have a discoverable TOC"
        );
    }
}

prefork_full_ui_case! {
fn full_game_load_publishes_mixin_globals(env: &WowLuaEnv) {

    for mixin in MIXIN_GLOBALS {
        let kind: String = env
            .eval(&format!("return type({mixin})"))
            .unwrap_or_else(|err| panic!("{mixin} probe failed: {err}"));
        assert_eq!(
            kind, "table",
            "{mixin} must be a global table after load. The addon \
             publishes 9 mixins forming a 3-level inheritance graph: \
             VoiceToggleButtonMixin is the root chassis (composes \
             with PropertyButtonMixin via OnLoad delegation rather \
             than CreateFromMixins to keep the property-button \
             plumbing isolated); \
             VoiceToggleButtonAlwaysVisibileMixin (sic — Blizzard \
             ships the typo `Visibile`, not `Visible`) and \
             VoiceToggleButtonOnlyVisibleWhenLoggedInMixin extend the \
             root with always-show vs C_VoiceChat.IsLoggedIn() \
             visibility predicates; VoiceToggleMuteMixin and \
             VoiceToggleDeafenMixin further extend OnlyVisibleWhenLoggedIn \
             to bind self-mute / self-deafen state via \
             AddStateAtlas+SetAccessorFunction+SetMutatorFunction; \
             RosterToggleButtonMixin extends the root with \
             ShouldShowLocalPlayerOnly / ShouldShowRemotePlayerOnly \
             predicates that consult GetParent()'s voice channel ID \
             + member ID + IsLocalPlayer; \
             RosterSelfDeafenButtonMixin / RosterSelfMuteButtonMixin / \
             RosterMemberMuteButtonMixin extend RosterToggleButton \
             with the per-row variants (self-only deafen, self-only \
             mute, remote-only mute with VOICE_CHAT_CHANNEL_MEMBER_*\
             event registration)"
        );
    }
}
}

prefork_full_ui_case! {
fn full_game_load_publishes_free_functions(env: &WowLuaEnv) {

    for func in FREE_FUNCTION_GLOBALS {
        let kind: String = env
            .eval(&format!("return type({func})"))
            .unwrap_or_else(|err| panic!("{func} probe failed: {err}"));
        assert_eq!(
            kind, "function",
            "{func} must be a global function after load. \
             VoiceChat_ToggleMutedFromUserAction is bound as the \
             mutator for VoiceToggleMuteMixin via \
             SetMutatorFunction(VoiceChat_ToggleMutedFromUserAction) \
             at line 84 of VoiceToggleButton.lua — it plays \
             SOUNDKIT.UI_VOICECHAT_MUTEON or MUTEOFF based on the \
             pre-toggle C_VoiceChat.IsMuted() state, then calls \
             C_VoiceChat.ToggleMuted(). \
             VoiceChat_ToggleDeafenedFromUserAction is the symmetric \
             function for the deafen toggle. Both are exported as \
             globals (rather than locals) because external callers \
             — chat frame's right-click context menu, accessibility \
             keybinding handlers — invoke them directly outside of \
             the mixin's OnClick path"
        );
    }
}
}

prefork_full_ui_case! {
fn full_game_load_publishes_state_constants(env: &WowLuaEnv) {

    for (name, expected) in STATE_CONSTANT_VALUES {
        let value: i32 = env
            .eval(&format!("return {name}"))
            .unwrap_or_else(|err| panic!("{name} probe failed: {err}"));
        assert_eq!(
            value, *expected,
            "{name} must equal {expected}. The 6 \
             MUTE_SILENCE_STATE_* constants form a bitfield: NONE=0, \
             MUTE=1<<0, SILENCE=1<<1, PARENTAL_MUTE=1<<2 — combined \
             values MUTE_AND_SILENCE=3 (MUTE|SILENCE) and \
             MUTE_AND_PARENTAL_MUTE=5 (MUTE|PARENTAL_MUTE). \
             VoiceToggleMuteMixin:SetupMuteButton (line 73) builds a \
             FlagsMixin instance and stores per-flag state via \
             SetOrClear, then maps each combined value to a \
             distinct atlas in OnLoad's AddStateAtlas calls \
             (chatframe-button-icon-mic-on/off/silenced/silenced-off \
             plus voicechat-icon-mic-silenced/mutesilenced for the \
             parental variants). The values must match exactly so \
             FlagsMixin's bit arithmetic produces the expected \
             AddStateAtlas keys"
        );
    }
}
}

prefork_full_ui_case! {
fn full_game_load_inheritance_chain_links_propertybuttonmixin(env: &WowLuaEnv) {

    let on_load_kind: String = env
        .eval("return type(VoiceToggleButtonMixin.OnLoad)")
        .unwrap_or_else(|err| panic!("VoiceToggleButtonMixin.OnLoad probe failed: {err}"));
    assert_eq!(
        on_load_kind, "function",
        "VoiceToggleButtonMixin.OnLoad must exist as a function. \
         Line 3-5 of VoiceToggleButton.lua: \
         `function VoiceToggleButtonMixin:OnLoad() \
         PropertyButtonMixin.OnLoad(self); end` — the root chassis \
         ONLY delegates to PropertyButtonMixin.OnLoad, no further \
         setup. This composition pattern (call the parent's OnLoad \
         explicitly with `self`) is used instead of \
         CreateFromMixins(PropertyButtonMixin) because the chassis \
         must inherit BOTH from PropertyButtonMixin (for the \
         accessor/mutator/state-atlas plumbing) AND set up via \
         middleclass-style mixin composition for the visibility \
         query function — CreateFromMixins would shallow-copy \
         PropertyButtonMixin's table at module-load time and miss \
         later monkey-patches"
    );

    let logged_in_on_load: String = env
        .eval("return type(VoiceToggleButtonOnlyVisibleWhenLoggedInMixin.OnLoad)")
        .unwrap_or_else(|err| {
            panic!("VoiceToggleButtonOnlyVisibleWhenLoggedInMixin.OnLoad probe failed: {err}")
        });
    assert_eq!(
        logged_in_on_load, "function",
        "VoiceToggleButtonOnlyVisibleWhenLoggedInMixin.OnLoad must \
         exist after CreateFromMixins shallow copy + override (line \
         16-21). It registers VOICE_CHAT_LOGIN/LOGOUT state-update \
         events and binds C_VoiceChat.IsLoggedIn() as the visibility \
         query. The mute and deafen mixins inherit this OnLoad chain \
         and add their own state atlas + accessor/mutator setup on \
         top"
    );
}
}

prefork_full_ui_case! {
fn full_game_load_registers_virtual_templates(env: &WowLuaEnv) {
    let _env = env;

    for template in VIRTUAL_TEMPLATES {
        let resolved = wow_ui_sim::xml::get_template(template);
        assert!(
            resolved.is_some(),
            "Virtual template `{template}` must be registered. \
             VoiceToggleButton.xml ships exactly 7 virtual templates \
             — zero non-virtual frames, just like Blizzard_UnitPopup. \
             VoiceToggleButtonTemplate is the root \
             (inherits=PropertyButtonTemplate, mixin=VoiceToggleButtonMixin, \
             27x26 with chatframe-button-up/down/highlight atlases); \
             ToggleVoiceDeafenButtonTemplate and \
             ToggleVoiceMuteButtonTemplate add the deafen/mute mixin \
             on top of the root and inherit all the KeyValues; \
             RosterVoiceToggleButtonTemplate is a parallel root for \
             the smaller (14x14, no atlas) per-row variants used in \
             party/raid voice-active overlays; \
             RosterSelfDeafenButtonTemplate, \
             RosterSelfMuteButtonTemplate, \
             RosterMemberMuteButtonTemplate add the three roster \
             mixin variants atop RosterVoiceToggleButtonTemplate. \
             The two-root structure (chatframe vs roster) maps to \
             the two visual contexts: chatframe buttons are \
             standalone with full button chrome; roster buttons \
             overlay raid/party member rows and use only their icon \
             with no border"
        );
    }
}
}

prefork_full_ui_case! {
fn full_game_load_emits_no_addon_specific_errors(env: &WowLuaEnv) {

    let errors: Vec<String> = env.state().borrow().lua_errors.clone();
    let addon_specific: Vec<&String> = errors
        .iter()
        .filter(|e| e.contains("Blizzard_VoiceToggleButton/"))
        .collect();

    assert!(
        addon_specific.is_empty(),
        "Full game-screen load must emit zero \
         Blizzard_VoiceToggleButton body errors. The 2 active body \
         files (~270 lua + ~50 xml lines, the smallest addon \
         analyzed so far in this campaign) span the 9-mixin \
         inheritance graph + 2 free toggle functions + 6 state \
         constants + 7 virtual templates. Any failure breaks the \
         chat frame voice buttons and the raid/party roster voice \
         indicators. Found: {addon_specific:?}"
    );
}
}
