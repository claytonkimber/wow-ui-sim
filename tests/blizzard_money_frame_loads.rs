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

fn money_frame_dir() -> PathBuf {
    blizzard_ui_dir().join("Blizzard_MoneyFrame")
}

fn money_frame_mainline_toc() -> PathBuf {
    money_frame_dir().join("Blizzard_MoneyFrame_Mainline.toc")
}

const MONEY_FRAME_TOC_FILES: &[&str] = &[
    "Shared/MoneyFrame.lua",
    "Shared/MoneyFrame.xml",
    "Mainline/MoneyFrame.lua",
    "Mainline/MoneyFrame.xml",
    "Shared/MoneyInputFrame.lua",
    "Shared/MoneyInputFrame.xml",
    "Mainline/MoneyInputFrame.lua",
    "Mainline/MoneyInputFrame.xml",
    "Mainline/Localization.lua",
];

const MODULE_CONSTANTS: &[(&str, i64)] = &[
    ("MONEY_ICON_WIDTH", 19),
    ("MONEY_ICON_WIDTH_SMALL", 13),
    ("MONEY_BUTTON_SPACING", -4),
    ("MONEY_BUTTON_SPACING_SMALL", -4),
    ("MONEY_TEXT_VADJUST", 0),
    ("COIN_BUTTON_WIDTH", 32),
];

const PUBLIC_MIXINS: &[&str] = &[
    "MoneyDenominationDisplayMixin",
    "MoneyDisplayFrameMixin",
    "SmallMoneyFrameMixin",
    "LargeMoneyInputBoxMixin",
    "LargeMoneyInputFrameMixin",
    "MoneyFrameEditBoxMixin",
    "MoneyInputFrameMixin",
];

const MONEY_FRAME_PUBLIC_FUNCTIONS: &[&str] = &[
    "GetMoneyTypeInfoField",
    "AddMoneyTypeInfo",
    "MoneyFrame_UpdateTrialErrorButton",
    "SetMoneyFrameColorByFrame",
    "GetMoneyFrame",
    "SetMoneyFrameColor",
    "GetDenominationsFromCopper",
    "MoneyFrame_OnLoadMoneyType",
    "MoneyFrame_OnLoad",
    "SmallMoneyFrame_OnLoad",
    "MoneyFrame_OnEvent",
    "MoneyFrame_OnEnter",
    "MoneyFrame_OnLeave",
    "MoneyFrame_OnHide",
    "MoneyFrame_SetType",
    "MoneyFrame_SetMaxDisplayWidth",
    "MoneyFrame_UpdateMoney",
    "MoneyFrame_SetDisplayForced",
    "MoneyFrame_Update",
];

const MONEY_INPUT_PUBLIC_FUNCTIONS: &[&str] = &[
    "MoneyInputFrame_SetCopperShown",
    "MoneyInputFrame_SetEnabled",
    "MoneyInputFrame_ResetMoney",
    "MoneyInputFrame_ClearFocus",
    "MoneyInputFrame_SetGoldOnly",
    "MoneyInputFrame_GetCopper",
    "MoneyInputFrame_SetTextColor",
    "MoneyInputFrame_SetCopper",
    "MoneyInputFrame_OnTextChanged",
    "MoneyInputFrame_SetCompact",
    "MoneyInputFrame_SetPreviousFocus",
    "MoneyInputFrame_SetNextFocus",
    "MoneyInputFrame_SetOnValueChangedFunc",
    "MoneyInputFrame_OnShow",
    "MoneyInputFrame_OpenPopup",
    "MoneyInputFrame_ClosePopup",
    "MoneyInputFrame_PickupPlayerMoney",
    "MoneyInputFrameButton_OpenPopup",
];

const VIRTUAL_TEMPLATES_THAT_MUST_NOT_LEAK: &[&str] = &[
    "MoneyDenominationDisplayTemplate",
    "MoneyDisplayFrameTemplate",
    "MoneyFrameButtonTemplate",
    "MoneyFrameTemplate",
    "SmallMoneyFrameTemplate",
    "SmallDenominationTemplate",
    "SmallAlternateCurrencyFrameTemplate",
    "TooltipMoneyFrameTemplate",
    "LargeMoneyInputBoxTemplate",
    "LargeMoneyInputFrameTemplate",
    "MoneyFrameEditBoxTemplate",
    "MoneyInputFrameTemplate",
    "FixedCoinFrameTemplate",
];

const BUILTIN_MONEY_TYPES: &[&str] = &[
    "PLAYER",
    "ACCOUNT",
    "STATIC",
    "QUEST_REWARDS",
    "AUCTION",
    "AUCTION_TOOLTIP",
    "PLAYER_TRADE",
    "TARGET_TRADE",
    "SEND_MAIL",
    "SEND_MAIL_COD",
    "GUILDBANK",
    "GUILDBANKWITHDRAW",
    "GUILD_REPAIR",
    "TOOLTIP",
    "BLACKMARKET",
    "GUILDBANKCASHFLOW",
    "REFORGE",
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
fn blizzard_money_frame_find_toc_resolves_mainline_only_variant() {
    let resolved = find_toc_file(&money_frame_dir()).expect("Blizzard_MoneyFrame TOC resolves");
    assert_eq!(
        resolved,
        money_frame_mainline_toc(),
        "Blizzard_MoneyFrame ships exactly one TOC variant — `Blizzard_MoneyFrame_Mainline.toc` \
         — with no `_Classic.toc` counterpart and no bare TOC. `find_toc_file` must resolve \
         the `_Mainline` variant via its first lookup probe; classic flavors have an \
         entirely different copper / silver / gold currency presentation and do not consume \
         this addon's surface"
    );

    let bare = money_frame_dir().join("Blizzard_MoneyFrame.toc");
    assert!(
        !bare.exists(),
        "There must be NO bare TOC at {} — the addon ships only the `_Mainline` suffixed \
         variant. The absence is structurally important: if a bare TOC were added, \
         find_toc_file would still prefer the `_Mainline.toc` first probe, but auditors \
         would lose the visible Mainline-only signal carried by the directory layout",
        bare.display()
    );

    let classic = money_frame_dir().join("Blizzard_MoneyFrame_Classic.toc");
    assert!(
        !classic.exists(),
        "There must be NO `_Classic.toc` at {} — the addon is mainline-exclusive at the TOC \
         layer (Classic clients carry their own copper / silver / gold display surface that \
         pre-dates the mixin-driven retail design)",
        classic.display()
    );
}

#[test]
fn blizzard_money_frame_toc_declares_shared_xml_dep_with_no_default_state() {
    let toc =
        TocFile::from_file(&money_frame_mainline_toc()).expect("Blizzard_MoneyFrame TOC parses");
    assert!(
        !toc.is_load_on_demand(),
        "TOC omits `## LoadOnDemand:` — it auto-loads. Note: the TOC also omits \
         `## DefaultState:` (unusual for the Blizzard tree where most addons spell out \
         `## DefaultState: enabled`). The simulator's discovery layer doesn't inspect \
         DefaultState; an addon enters the eager `addons` set when it parses, has no \
         AllowLoadGameType restriction, allows the active screen, and is non-LOD"
    );
    assert!(!toc.is_load_first());
    assert!(!toc.is_secure_env());

    let deps = toc.dependencies();
    assert_eq!(
        deps,
        vec!["Blizzard_SharedXML".to_string()],
        "TOC must declare exactly one `## Dependencies:` entry — Blizzard_SharedXML. The \
         money-frame surface inherits foundational templates (LargeInputBoxTemplate at \
         Shared/MoneyInputFrame.xml line 3) and font / color globals from SharedXML. Got \
         {deps:?}"
    );

    assert!(toc.optional_deps().is_empty());
    assert!(
        toc.saved_variables().is_empty(),
        "Zero saved variables — money state is server-driven (player money is read via \
         GetMoneyAmount, account-bank money via C_Bank.FetchDepositedMoney); there's no \
         client-side persistence at this layer"
    );
}

#[test]
fn blizzard_money_frame_toc_declares_allow_load_both_capital_b_with_mainline_only() {
    let raw = std::fs::read_to_string(money_frame_mainline_toc()).expect("Mainline TOC reads");
    assert!(
        raw.contains("## AllowLoad: Both"),
        "TOC must declare `## AllowLoad: Both` exactly with capital `B`. The money-frame \
         templates are consumed by both in-world surfaces (mailbox / vendor / trade / guild \
         bank dialogs) AND glue surfaces (character-select boost-product confirmation \
         dialogs that quote a copper / silver / gold price). The case-insensitive matcher \
         at src/toc.rs:307 normalizes through `eq_ignore_ascii_case`"
    );
    assert!(
        raw.contains("## AllowLoadGameType: mainline"),
        "TOC must declare `## AllowLoadGameType: mainline` — Classic clients carry their \
         own legacy money-frame surface"
    );
    assert!(
        !raw.contains("## DefaultState"),
        "TOC must NOT declare `## DefaultState:` — the addon relies on the absence of \
         `## LoadOnDemand:` to enter the eager auto-discovery set"
    );
    assert!(
        !raw.contains("## LoadOnDemand"),
        "TOC must NOT declare `## LoadOnDemand:` — the money-frame templates must be \
         registered before any consumer XML loads (every consumer addon's TOC body \
         instantiates a MoneyFrameTemplate / SmallMoneyFrameTemplate / \
         MoneyInputFrameTemplate before its own scripts run)"
    );
}

#[test]
fn blizzard_money_frame_toc_is_unrestricted_on_every_screen() {
    let toc =
        TocFile::from_file(&money_frame_mainline_toc()).expect("Blizzard_MoneyFrame TOC parses");
    assert!(
        !toc.is_game_type_restricted(),
        "TOC declares `## AllowLoadGameType: mainline`; `is_game_type_restricted` \
         (src/toc.rs:294) treats `mainline` as the retail target and returns false"
    );

    for screen in [
        ScreenKind::Game,
        ScreenKind::Login,
        ScreenKind::CharacterSelect,
        ScreenKind::CharacterCreate,
    ] {
        assert!(
            toc.allows_screen(screen),
            "`## AllowLoad: Both` must allow every ScreenKind via the case-insensitive \
             matcher at src/toc.rs:307. The money-frame templates are referenced from glue \
             surfaces too (boost-product price quotes on character-select / character-create \
             confirmation dialogs). (Screen tested: {screen:?})"
        );
    }
}

#[test]
fn blizzard_money_frame_toc_lists_nine_files_in_shared_then_mainline_pairs() {
    let toc =
        TocFile::from_file(&money_frame_mainline_toc()).expect("Blizzard_MoneyFrame TOC parses");
    let listed: Vec<String> = toc
        .files
        .iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect();
    assert_eq!(
        listed, MONEY_FRAME_TOC_FILES,
        "TOC body must list exactly 9 files in declaration order — for each of MoneyFrame \
         and MoneyInputFrame, the Shared (lua, xml) pair loads BEFORE the Mainline (lua, \
         xml) pair so the Mainline files can override or extend Shared mixins / functions \
         (LargeMoneyInputBoxMixin is redefined in Mainline/MoneyInputFrame.lua line 252 \
         after Shared/MoneyInputFrame.lua line 7 declares it). The 9th file \
         Mainline/Localization.lua loads last to set up locale-specific MONEY_TEXT_VADJUST \
         overrides for zhCN / zhTW. The TOC parser normalizes raw `\\` separators to `/` \
         when constructing the relative paths"
    );
}

#[test]
fn blizzard_money_frame_appears_on_every_screen_discovery() {
    for screen in [
        ScreenKind::Game,
        ScreenKind::Login,
        ScreenKind::CharacterSelect,
        ScreenKind::CharacterCreate,
    ] {
        let addons = discover_blizzard_addons_for_screen(&blizzard_ui_dir(), screen);
        let found = addons.iter().any(|(name, _)| name == "Blizzard_MoneyFrame");
        assert!(
            found,
            "Blizzard_MoneyFrame (`## AllowLoad: Both`, no LoadOnDemand) must appear in \
             every screen's discovery sweep. (Screen tested: {screen:?})"
        );
    }
}

prefork_full_ui_case! {
fn blizzard_money_frame_loads_without_addon_specific_lua_errors(env: &WowLuaEnv) {

    let addon_errors: Vec<String> = env
        .state()
        .borrow()
        .lua_errors
        .iter()
        .filter(|message| {
            message.contains("Blizzard_MoneyFrame")
                || message.contains("MoneyFrame")
                || message.contains("MoneyInputFrame")
                || message.contains("MoneyDenomination")
                || message.contains("MoneyDisplayFrame")
                || message.contains("MoneyTypeInfo")
                || message.contains("LargeMoneyInput")
        })
        .cloned()
        .collect();
    assert!(
        addon_errors.is_empty(),
        "Blizzard_MoneyFrame emitted addon-specific Lua errors during load:\n  {}",
        addon_errors.join("\n  ")
    );
}
}

prefork_full_ui_case! {
fn blizzard_money_frame_is_addon_loaded_after_auto_discovery(env: &WowLuaEnv) {
    let loaded: bool = env
        .eval("return C_AddOns.IsAddOnLoaded('Blizzard_MoneyFrame')")
        .expect("IsAddOnLoaded probe succeeds");
    assert!(
        loaded,
        "C_AddOns.IsAddOnLoaded('Blizzard_MoneyFrame') must return true after the eager \
         Game-screen auto-discovery sweep"
    );
}
}

prefork_full_ui_case! {
fn blizzard_money_frame_publishes_module_constants(env: &WowLuaEnv) {
    for (name, expected) in MODULE_CONSTANTS {
        let value: i64 = env
            .eval(&format!("return _G.{name}"))
            .unwrap_or_else(|err| panic!("{name} probe failed: {err}"));
        assert_eq!(
            value, *expected,
            "Module-level constant `{name}` must publish at `_G` with value {expected}. \
             MoneyFrame.lua declares 6 numeric constants at file scope that consumers read \
             by name to size and space coin icons (icon widths, button gaps, copper/silver/ \
             gold button width). Values are pixel measurements at the canonical UI scale"
        );
    }
}
}

prefork_full_ui_case! {
fn blizzard_money_frame_publishes_seven_public_mixins_as_tables(env: &WowLuaEnv) {
    for mixin in PUBLIC_MIXINS {
        let kind: String = env
            .eval(&format!("return type(_G.{mixin})"))
            .unwrap_or_else(|err| panic!("type({mixin}) probe failed: {err}"));
        assert_eq!(
            kind, "table",
            "Mixin `{mixin}` must publish at `_G` as a table. The 7 mixins span the public \
             surface: MoneyDenominationDisplayMixin / MoneyDisplayFrameMixin (Shared, the \
             reusable per-coin display + multi-coin frame), SmallMoneyFrameMixin (Mainline, \
             the small variant used in inventory rows), LargeMoneyInputBoxMixin / \
             LargeMoneyInputFrameMixin (Shared, the gold-only large input editor — \
             LargeMoneyInputBoxMixin is redefined in Mainline so the post-load value comes \
             from the Mainline file), MoneyFrameEditBoxMixin / MoneyInputFrameMixin \
             (Mainline, the 3-field copper / silver / gold input frame)"
        );
    }
}
}

prefork_full_ui_case! {
fn blizzard_money_frame_publishes_money_frame_public_functions(env: &WowLuaEnv) {
    for func in MONEY_FRAME_PUBLIC_FUNCTIONS {
        let kind: String = env
            .eval(&format!("return type(_G.{func})"))
            .unwrap_or_else(|err| panic!("type({func}) probe failed: {err}"));
        assert_eq!(
            kind, "function",
            "Money-frame helper `{func}` must publish at `_G` as a function. The 19 \
             helpers cover the type-info table accessors (Get/AddMoneyTypeInfo*), color \
             setters (SetMoneyFrameColor*), denomination decomposition \
             (GetDenominationsFromCopper), per-frame lifecycle (MoneyFrame_OnLoad / \
             OnLoadMoneyType / OnEvent / OnEnter / OnLeave / OnHide), state setters \
             (SetType / SetMaxDisplayWidth / SetDisplayForced), and the Update entry \
             points (UpdateMoney / UpdateTrialErrorButton / Update) consumers call by name"
        );
    }
}
}

prefork_full_ui_case! {
fn blizzard_money_frame_publishes_money_input_public_functions(env: &WowLuaEnv) {
    for func in MONEY_INPUT_PUBLIC_FUNCTIONS {
        let kind: String = env
            .eval(&format!("return type(_G.{func})"))
            .unwrap_or_else(|err| panic!("type({func}) probe failed: {err}"));
        assert_eq!(
            kind, "function",
            "Money-input helper `{func}` must publish at `_G` as a function. The 18 \
             MoneyInputFrame_* / MoneyInputFrameButton_* helpers form the public driver \
             API that mailbox / send-mail / trade / guild-bank dialogs call to script \
             3-field copper / silver / gold input — covering enable / reset / focus, \
             gold-only mode, value get / set, color, compact mode, focus chaining \
             (Previous / Next), the OnValueChanged callback registration, OnShow / \
             OnTextChanged script handlers, and the popup pickup flow"
        );
    }
}
}

prefork_full_ui_case! {
fn blizzard_money_frame_get_money_type_info_field_returns_canonical_metadata(env: &WowLuaEnv) {

    for ty in BUILTIN_MONEY_TYPES {
        let probe = format!("return type(GetMoneyTypeInfoField('{ty}', 'UpdateFunc'))");
        let kind: String = env.eval(&probe).unwrap_or_else(|err| {
            panic!("GetMoneyTypeInfoField('{ty}', 'UpdateFunc') probe: {err}")
        });
        assert_eq!(
            kind, "function",
            "GetMoneyTypeInfoField('{ty}', 'UpdateFunc') must return a function — every \
             entry in the file-private MoneyTypeInfo table populated by Shared/MoneyFrame.lua \
             defines a callback that yields the current copper amount (e.g. \
             GetSendMailMoney for SEND_MAIL, GetGuildBankMoney for GUILDBANK, \
             self.staticMoney for the static-amount types). UpdateFunc is the only field \
             present on all 17 built-in types — collapse, canPickup, showSmallerCoins, \
             checkGoldThreshold are auxiliary and selectively populated"
        );
    }

    let player_pickup: bool = env
        .eval("return GetMoneyTypeInfoField('PLAYER', 'canPickup') == 1")
        .expect("PLAYER canPickup probe succeeds");
    assert!(
        player_pickup,
        "GetMoneyTypeInfoField('PLAYER', 'canPickup') must equal 1 — PLAYER, SEND_MAIL, \
         SEND_MAIL_COD, and PLAYER_TRADE carry `canPickup = 1`; this is the marker the \
         cursor / drag-drop pickup driver consults to decide whether a coin button can be \
         dragged off"
    );
}
}

prefork_full_ui_case! {
fn blizzard_money_frame_add_money_type_info_registers_then_no_overwrite(env: &WowLuaEnv) {

    let probe = "\
        AddMoneyTypeInfo('TEST_PROBE', { collapse = 7, marker = 'fresh' }); \
        local first = GetMoneyTypeInfoField('TEST_PROBE', 'collapse'); \
        AddMoneyTypeInfo('TEST_PROBE', { collapse = 99, marker = 'overwrite-attempt' }); \
        local second = GetMoneyTypeInfoField('TEST_PROBE', 'collapse'); \
        return first == 7 and second == 7";
    let preserved: bool = env
        .eval(probe)
        .expect("AddMoneyTypeInfo overwrite-protection probe succeeds");
    assert!(
        preserved,
        "AddMoneyTypeInfo must register a new key on first call and silently no-op on \
         every subsequent call with the same key (Shared/MoneyFrame.lua line 25 short-\
         circuits when the entry exists). This is the contract third-party addons rely on \
         to publish a custom money type without risking accidental overwrite"
    );
}
}

prefork_full_ui_case! {
fn blizzard_money_frame_get_denominations_from_copper_publishes_as_function(env: &WowLuaEnv) {
    let kind: String = env
        .eval("return type(GetDenominationsFromCopper)")
        .expect("GetDenominationsFromCopper type probe succeeds");
    assert_eq!(
        kind, "function",
        "GetDenominationsFromCopper must publish at `_G` as a function. Mainline's \
         Shared/MoneyFrame.lua line 338 defines it as a one-line wrapper that delegates to \
         `C_CurrencyInfo.GetCoinText(money, ' ')` (a separator-formatting helper that yields \
         the localized `12g 34s 56c` string). The wrapper itself must publish as a global \
         even on hosts where the underlying C_CurrencyInfo.GetCoinText is not implemented \
         (the simulator does not currently surface GetCoinText), because the function \
         binding is a load-time effect of the addon Lua running"
    );
}
}

prefork_full_ui_case! {
fn blizzard_money_frame_virtual_templates_do_not_leak_as_globals(env: &WowLuaEnv) {
    for template in VIRTUAL_TEMPLATES_THAT_MUST_NOT_LEAK {
        let kind: String = env
            .eval(&format!("return type(_G.{template})"))
            .unwrap_or_else(|err| panic!("type({template}) probe failed: {err}"));
        assert_eq!(
            kind, "nil",
            "Virtual template `{template}` is declared `virtual=\"true\"` and MUST NOT leak \
             as a `_G.*` global. The 13 virtual templates register only in the XML \
             template registry; consumer addons reference them via \
             `inherits=\"{template}\"` at instantiation time. Leaking them as globals \
             would let addons accidentally mutate the template's properties at runtime"
        );
    }
}
}

prefork_full_ui_case! {
fn blizzard_money_frame_localization_table_does_not_leak_as_global(env: &WowLuaEnv) {
    let l10n_kind: String = env
        .eval("return type(_G.l10nTable)")
        .expect("l10nTable global probe succeeds");
    assert_eq!(
        l10n_kind, "nil",
        "The Mainline/Localization.lua `l10nTable` is declared `local` (line 1) — it MUST \
         NOT leak as a `_G.*` global. The local table is consumed by SetupLocalization \
         which dispatches the active locale's `localizeFrames` closure; the table itself \
         is single-use and must remain file-private"
    );
    let vadjust_kind: String = env
        .eval("return type(_G.MONEY_TEXT_VADJUST)")
        .expect("MONEY_TEXT_VADJUST probe succeeds");
    assert_eq!(
        vadjust_kind, "number",
        "MONEY_TEXT_VADJUST must remain a `_G.*` number after the localization pass — \
         enUS keeps the Shared default of 0; only zhCN / zhTW localizeFrames closures \
         override (to 2 and 1 respectively)"
    );
}
}
