use std::path::PathBuf;

use wow_ui_sim::loader::{discover_blizzard_addons_for_screen, find_toc_file, load_addon};
use wow_ui_sim::lua_api::WowLuaEnv;
use wow_ui_sim::screen::ScreenKind;
use wow_ui_sim::toc::TocFile;

fn blizzard_ui_dir() -> PathBuf {
    wow_ui_sim::paths::default_blizzard_ui_addons_path().expect("Blizzard UI cache should be available")
}

fn pickup_dir() -> PathBuf {
    blizzard_ui_dir().join("Blizzard_SpellPickUpIndicator")
}

fn pickup_toc() -> PathBuf {
    pickup_dir().join("Blizzard_SpellPickUpIndicator.toc")
}

const PUBLISHED_MIXINS: &[&str] = &["SpellPickupIndicatorMixin", "SpellPickupDisplayMixin"];

const INDICATOR_METHODS: &[&str] = &[
    "OnLoad",
    "SetInventoryType",
    "UpdateOffensiveReminder",
    "HandleUpgradeNotification",
    "UpdateUtilityReminder",
    "UpdateItemReminder",
    "HandleEmptyAbilitySlots",
];

const DISPLAY_METHODS: &[&str] = &[
    "OnLoad",
    "OnUpdate",
    "OnShow",
    "OnWorldLootObjectTooltipShown",
    "OnWorldLootObjectTooltipHidden",
    "UpdatePosition",
];

const EXPECTED_BODY: &[&str] = &[
    "Blizzard_SpellPickUpIndicator.lua",
    "Blizzard_SpellPickUpIndicator.xml",
];

fn load_spell_pick_up_indicator(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &pickup_toc())
        .expect("Blizzard_SpellPickUpIndicator should load via explicit Rust loader call");
}

#[test]
fn find_toc_resolves_bare_variant() {
    let resolved =
        find_toc_file(&pickup_dir()).expect("Blizzard_SpellPickUpIndicator TOC should resolve");
    assert_eq!(
        resolved,
        pickup_toc(),
        "Blizzard_SpellPickUpIndicator ships exactly one bare TOC — \
         Plunderstorm-only world-loot pickup indicator with no per-flavor \
         variants because the addon is gated entirely off-flavor on \
         standard retail"
    );
}

#[test]
fn toc_declares_three_directives_with_no_dependencies() {
    let toc =
        TocFile::from_file(&pickup_toc()).expect("Blizzard_SpellPickUpIndicator TOC should parse");

    assert!(
        !toc.is_load_on_demand(),
        "Blizzard_SpellPickUpIndicator omits `## LoadOnDemand:` — under \
         the Plunderstorm client it auto-discovers on the Game screen \
         (DefaultState defaults to enabled when omitted) so the \
         WorldLootObjectTooltip.Shown / Hidden EventRegistry channels \
         have a registered listener before the player walks over their \
         first plunder pile; under standard retail the game-type filter \
         excludes it entirely"
    );
    assert!(!toc.is_load_first());
    assert!(!toc.is_secure_env());
    assert!(
        toc.dependencies().is_empty(),
        "Blizzard_SpellPickUpIndicator declares ZERO dependencies — every \
         API used (C_ActionBar / C_Spell / C_WorldLootObject / EventRegistry / \
         CallbackRegistrantMixin / BaseLayoutMixin / HorizontalLayoutFrame / \
         VerticalLayoutFrame) lives in Blizzard_FrameXML or \
         Blizzard_SharedXML which are themselves eager non-LOD addons \
         loaded before any plunderstorm-flavored addon. Got: {:?}",
        toc.dependencies()
    );
    assert!(toc.optional_deps().is_empty());
    assert!(
        toc.saved_variables().is_empty(),
        "Blizzard_SpellPickUpIndicator declares zero saved variables — \
         WorldLootObject pickup state is server-driven, no client-side \
         persistence"
    );
    assert!(toc.default_enabled());
}

#[test]
fn toc_is_plunderstorm_only_without_allow_load_directive() {
    let toc =
        TocFile::from_file(&pickup_toc()).expect("Blizzard_SpellPickUpIndicator TOC should parse");

    assert!(
        toc.is_game_type_restricted(),
        "Blizzard_SpellPickUpIndicator declares `## AllowLoadGameType: \
         plunderstorm` — does not match `mainline` or `standard`, so \
         is_game_type_restricted (src/toc.rs:294) returns true. The \
         auto-discovery sweep filters this addon out on standard retail; \
         only Plunderstorm clients pick it up automatically"
    );

    assert!(
        toc.allows_screen(ScreenKind::Game),
        "Without `## AllowLoad:` directive, the metadata-absent fallback \
         at src/toc.rs allows only the Game screen (None branch returns \
         `screen == ScreenKind::Game`). The world-loot pickup indicator \
         is anchored to the WorldLootObjectTooltip which only surfaces \
         in-match, never on glue screens"
    );
    assert!(!toc.allows_screen(ScreenKind::Login));
    assert!(!toc.allows_screen(ScreenKind::CharacterSelect));
    assert!(!toc.allows_screen(ScreenKind::CharacterCreate));
}

#[test]
fn raw_bytes_pin_three_metadata_directives() {
    let raw = std::fs::read_to_string(pickup_toc()).expect("TOC reads utf-8");

    let expected_directives = [
        "## Title: Blizzard Spell Pick Up Indicator",
        "## Author: Blizzard Entertainment",
        "## AllowLoadGameType: plunderstorm",
    ];

    for directive in expected_directives {
        assert!(
            raw.contains(directive),
            "Raw bytes MUST pin the `{directive}` directive — \
             SpellPickUpIndicator's TOC is minimal (3 metadata lines + 2 \
             body entries) so each directive is load-bearing. The display \
             title uses SPACE-separated `Spell Pick Up Indicator` while \
             the addon-directory name uses `SpellPickUpIndicator` (no \
             spaces, mixed-case `PickUp`); confusingly the Lua mixin \
             names use lowercase `Pickup` (SpellPickupIndicatorMixin / \
             SpellPickupDisplayMixin) — the addon ships THREE different \
             casings of the same concept"
        );
    }

    assert!(!raw.contains("## DefaultState"));
    assert!(!raw.contains("## Version"));
    assert!(!raw.contains("## LoadOnDemand"));
    assert!(!raw.contains("## LoadFirst"));
    assert!(!raw.contains("## SavedVariables"));
    assert!(!raw.contains("## RequiredDep"));
    assert!(!raw.contains("## OptionalDep"));
    assert!(!raw.contains("## Dependencies"));
    assert!(!raw.contains("## UseSecureEnvironment"));
    assert!(!raw.contains("## AllowLoad:"));
}

#[test]
fn body_lists_lua_before_xml() {
    let toc = TocFile::from_file(&pickup_toc()).expect("TOC parses");

    let body: Vec<String> = toc
        .files
        .iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect();

    assert_eq!(
        body.len(),
        EXPECTED_BODY.len(),
        "Body must contain exactly 2 entries — the addon ships one .lua + \
         one .xml. Got: {body:?}"
    );

    for (i, want) in EXPECTED_BODY.iter().enumerate() {
        assert_eq!(
            &body[i], want,
            "Body entry {i}: expected {want}, got {}",
            body[i]
        );
    }

    assert!(
        body[0].ends_with(".lua") && body[1].ends_with(".xml"),
        "Blizzard_SpellPickUpIndicator.lua MUST load BEFORE the .xml — \
         the XML's `mixin=\"SpellPickupIndicatorMixin\"` (on the \
         SpellPickupIndicatorTemplate virtual template) and \
         `mixin=\"SpellPickupDisplayMixin\"` (on the named SpellPickupDisplay \
         frame) attributes resolve the mixin tables at \
         template-registration time, so they MUST already be tables in \
         _G when the .xml chunk is processed"
    );
}

#[test]
fn excluded_from_every_screen_auto_discovery() {
    for screen in [
        ScreenKind::Game,
        ScreenKind::Login,
        ScreenKind::CharacterSelect,
        ScreenKind::CharacterCreate,
    ] {
        let addons = discover_blizzard_addons_for_screen(&blizzard_ui_dir(), screen);
        let found = addons
            .iter()
            .any(|(name, _)| name == "Blizzard_SpellPickUpIndicator");
        assert!(
            !found,
            "Blizzard_SpellPickUpIndicator must be filtered out of \
             auto-discovery on standard retail across every ScreenKind. \
             The TOC declares `## AllowLoadGameType: plunderstorm`, and \
             discover_blizzard_addons_for_screen skips game-type-restricted \
             addons unless the active game type matches. (Screen tested: \
             {screen:?})"
        );
    }
}

prefork_full_ui_case! {
fn explicit_load_emits_no_addon_specific_lua_errors(env: &WowLuaEnv) {
    load_spell_pick_up_indicator(env);

    let load_errors: Vec<String> = env
        .state()
        .borrow()
        .lua_errors
        .iter()
        .filter(|message| {
            message.contains("Blizzard_SpellPickUpIndicator")
                || message.contains("SpellPickupIndicatorMixin")
                || message.contains("SpellPickupDisplayMixin")
                || message.contains("SpellPickupIndicatorTemplate")
                || message.contains("SpellPickupDisplay")
        })
        .cloned()
        .collect();
    assert!(
        load_errors.is_empty(),
        "Blizzard_SpellPickUpIndicator emitted addon-specific Lua errors \
         during load:\n  {}",
        load_errors.join("\n  ")
    );
}
}

prefork_full_ui_case! {
fn is_addon_loaded_reports_true_after_explicit_load(env: &WowLuaEnv) {
    load_spell_pick_up_indicator(env);

    let loaded: bool = env
        .eval("return C_AddOns.IsAddOnLoaded('Blizzard_SpellPickUpIndicator')")
        .expect("IsAddOnLoaded probe should succeed");
    assert!(
        loaded,
        "C_AddOns.IsAddOnLoaded('Blizzard_SpellPickUpIndicator') must \
         return true after the explicit load_addon call — confirms the \
         loader registers the addon with the loaded-set even though the \
         auto-discovery sweep skipped it (plunderstorm gametype filter)"
    );
}
}

prefork_full_ui_case! {
fn publishes_two_mixin_tables_at_global_scope(env: &WowLuaEnv) {
    load_spell_pick_up_indicator(env);

    for mixin in PUBLISHED_MIXINS {
        let kind: String = env
            .eval(&format!("return type({mixin})"))
            .unwrap_or_else(|err| panic!("{mixin} type probe failed: {err}"));
        assert_eq!(
            kind, "table",
            "{mixin} must publish at `_G` as a table after \
             Blizzard_SpellPickUpIndicator loads — \
             Blizzard_SpellPickUpIndicator.lua creates the two empty mixin \
             tables at file scope (lines 1 and 165) before binding methods \
             to them"
        );
    }
}
}

prefork_full_ui_case! {
fn indicator_mixin_carries_seven_canonical_methods(env: &WowLuaEnv) {
    load_spell_pick_up_indicator(env);

    for method in INDICATOR_METHODS {
        let kind: String = env
            .eval(&format!(
                "return type(SpellPickupIndicatorMixin['{method}'])"
            ))
            .unwrap_or_else(|err| panic!("SpellPickupIndicatorMixin.{method} probe failed: {err}"));
        assert_eq!(
            kind, "function",
            "SpellPickupIndicatorMixin.{method} must publish as a function \
             — the per-indicator mixin owns 7 methods covering OnLoad \
             (binds KeyIcon atlas based on spellSlot: 0=leftClickPickupAtlas, \
             1=rightClickPickupAtlas), SetInventoryType (router that \
             dispatches on WorldLootTypeOffensive=31 / WorldLootTypeUtility=32 \
             / WorldLootTypeItem=0), UpdateOffensiveReminder (uses \
             OffensiveSlotOffset=61 + WOWLABS_MULTIACTIONBAR1BUTTON binding), \
             UpdateUtilityReminder (uses UtilitySlotOffset=49 + \
             WOWLABS_MULTIACTIONBAR2BUTTON binding), UpdateItemReminder \
             (PLUNDERSTORM_INTERACT_PICK_UP_REMINDER_TEXT label, hides \
             PickupArrow + SlotSpell), HandleUpgradeNotification (color-codes \
             BindingAction GRAY for at-max-quality items via \
             C_WorldLootObject.GetWorldLootObjectInfoByGUID, switches text \
             to PLUNDERSTORM_INTERACT_UPGRADE_REMINDER_TEXT), \
             HandleEmptyAbilitySlots (returns true and hides all four \
             child widgets when either C_ActionBar.HasAction(baseIndex) or \
             C_ActionBar.HasAction(baseIndex+1) returns false)"
        );
    }
}
}

prefork_full_ui_case! {
fn display_mixin_carries_six_canonical_methods(env: &WowLuaEnv) {
    load_spell_pick_up_indicator(env);

    for method in DISPLAY_METHODS {
        let kind: String = env
            .eval(&format!("return type(SpellPickupDisplayMixin['{method}'])"))
            .unwrap_or_else(|err| panic!("SpellPickupDisplayMixin.{method} probe failed: {err}"));
        assert_eq!(
            kind, "function",
            "SpellPickupDisplayMixin.{method} must publish as a function \
             — the display mixin owns 6 methods: OnLoad wires \
             EventRegistry.WorldLootObjectTooltip.Shown via \
             AddStaticEventMethod and .Hidden via AddDynamicEventMethod \
             (the static/dynamic distinction reflects whether the \
             callback persists across frame reuse — Shown stays bound, \
             Hidden re-registers per anchor); OnUpdate calls UpdatePosition; \
             OnShow chains CallbackRegistrantMixin.OnShow + \
             BaseLayoutMixin.OnShow; OnWorldLootObjectTooltipShown filters \
             out WorldLootTypeItem=0 (no longer surfaces items on this \
             prompt), then dispatches SetInventoryType to both \
             LeftSpellPickupIndicator (spellSlot=0) and \
             RightSpellPickupIndicator (spellSlot=1) child frames; \
             OnWorldLootObjectTooltipHidden gates on anchorTooltip \
             identity to avoid hiding when an unrelated tooltip closes; \
             UpdatePosition computes spaceBelow from \
             anchorTooltip:GetBottom() and flips the anchor between \
             TOP/BOTTOM (default) and BOTTOM/TOP (when there's <selfHeight+10 \
             below) so the indicator stays on-screen near the cursor"
        );
    }
}
}

prefork_full_ui_case! {
fn named_spell_pickup_display_publishes_with_two_indicator_children(env: &WowLuaEnv) {
    load_spell_pick_up_indicator(env);

    let kind: String = env
        .eval("return type(SpellPickupDisplay)")
        .expect("SpellPickupDisplay probe should succeed");
    assert_eq!(
        kind, "table",
        "SpellPickupDisplay must publish at `_G` as a table — declared at \
         Blizzard_SpellPickUpIndicator.xml:62 with name=\"SpellPickupDisplay\" \
         parent=\"UIParent\" inherits=\"CallbackRegistrantTemplate, \
         VerticalLayoutFrame\" mixin=\"SpellPickupDisplayMixin\" \
         toplevel=\"true\""
    );

    let probe = "local f = SpellPickupDisplay \
                 if not f then return 'frame nil' end \
                 local missing = {} \
                 if type(f.LeftSpellPickupIndicator) ~= 'table' then table.insert(missing, 'LeftSpellPickupIndicator:'..type(f.LeftSpellPickupIndicator)) end \
                 if type(f.RightSpellPickupIndicator) ~= 'table' then table.insert(missing, 'RightSpellPickupIndicator:'..type(f.RightSpellPickupIndicator)) end \
                 if #missing == 0 then return 'OK' else return table.concat(missing, ',') end";
    let report: String = env.eval(probe).expect("SpellPickupDisplay children probe");
    assert_eq!(
        report, "OK",
        "SpellPickupDisplay must materialize with both LeftSpellPickupIndicator \
         (spellSlot=0, layoutIndex=1) and RightSpellPickupIndicator \
         (spellSlot=1, layoutIndex=2, topPadding=10) parentKey children \
         — these inherit SpellPickupIndicatorTemplate which itself \
         inherits HorizontalLayoutFrame so the whole composition is a \
         vertical stack of two horizontal pickup indicator rows. \
         Report: {report}"
    );
}
}

prefork_full_ui_case! {
fn indicator_template_materializes_with_four_layered_children(env: &WowLuaEnv) {
    load_spell_pick_up_indicator(env);

    let probe = "local f = CreateFrame('Frame', 'PickupIndicatorProbe', UIParent, 'SpellPickupIndicatorTemplate') \
                 if not f then return 'frame nil' end \
                 local missing = {} \
                 if type(f.BindingAction) ~= 'table' then table.insert(missing, 'BindingAction:'..type(f.BindingAction)) end \
                 if type(f.KeyIcon) ~= 'table' then table.insert(missing, 'KeyIcon:'..type(f.KeyIcon)) end \
                 if type(f.PickupArrow) ~= 'table' then table.insert(missing, 'PickupArrow:'..type(f.PickupArrow)) end \
                 if type(f.SlotSpell) ~= 'table' then table.insert(missing, 'SlotSpell:'..type(f.SlotSpell)) end \
                 if type(f.BG) ~= 'table' then table.insert(missing, 'BG:'..type(f.BG)) end \
                 if #missing == 0 then return 'OK' else return table.concat(missing, ',') end";
    let report: String = env
        .eval(probe)
        .expect("SpellPickupIndicatorTemplate children probe");
    assert_eq!(
        report, "OK",
        "SpellPickupIndicatorTemplate must materialize via CreateFrame \
         with five parentKey children at distinct draw layers + layout \
         indices: BindingAction (ARTWORK FontString inherits \
         SystemFont_Shadow_Large2 layoutIndex=2 with NORMAL_FONT_COLOR), \
         KeyIcon (ARTWORK Texture default-atlas housing-hotkey-icon-leftclick \
         layoutIndex=1 30x30), PickupArrow (ARTWORK Texture atlas \
         plunderstorm-pickup-arrow layoutIndex=3 30x30), SlotSpell \
         (ARTWORK Texture atlas plunderstorm-icon-key layoutIndex=4 30x30 \
         — the SetTexture call in UpdateOffensiveReminder/UpdateUtilityReminder \
         overrides the placeholder atlas with the actual action-bar slot \
         icon via C_ActionBar.GetActionTexture), BG (BACKGROUND Texture \
         atlas plunderstorm-pickup-BG 188x35 with ignoreInLayout=true \
         centered on $parent.BindingAction with x=12 offset to bias the \
         pill-shaped backdrop toward the text rather than the icons). \
         Report: {report}"
    );
}
}

prefork_full_ui_case! {
fn left_indicator_carries_spell_slot_zero_keyvalue(env: &WowLuaEnv) {
    load_spell_pick_up_indicator(env);

    let left_slot: f64 = env
        .eval("return SpellPickupDisplay.LeftSpellPickupIndicator.spellSlot")
        .expect("Left spellSlot probe");
    assert_eq!(
        left_slot, 0.0,
        "LeftSpellPickupIndicator must carry spellSlot=0 KeyValue from \
         the XML so SpellPickupIndicatorMixin:OnLoad selects \
         leftClickPickupAtlas=plunderstorm-pickup-mouseclick-left for the \
         KeyIcon atlas. Got: {left_slot}"
    );

    let right_slot: f64 = env
        .eval("return SpellPickupDisplay.RightSpellPickupIndicator.spellSlot")
        .expect("Right spellSlot probe");
    assert_eq!(
        right_slot, 1.0,
        "RightSpellPickupIndicator must carry spellSlot=1 KeyValue so OnLoad \
         picks rightClickPickupAtlas=plunderstorm-pickup-mouseclick-right. \
         The 0/1 spellSlot indices map to the two ability slots in each \
         WoWLabs MultiActionBar (offensive bar 1 / utility bar 2 share \
         the same 0/1 slot semantics, with the OffensiveSlotOffset=61 vs \
         UtilitySlotOffset=49 base index distinguishing which bar is \
         being inspected at runtime)"
    );

    let right_padding: f64 = env
        .eval("return SpellPickupDisplay.RightSpellPickupIndicator.topPadding")
        .expect("Right topPadding probe");
    assert_eq!(
        right_padding, 10.0,
        "RightSpellPickupIndicator must carry topPadding=10 KeyValue — \
         the VerticalLayoutFrame parent honors topPadding on stacked \
         children so the second row sits 10px below the first"
    );
}
}

prefork_full_ui_case! {
fn indicator_template_inherits_horizontal_layout_frame(env: &WowLuaEnv) {
    load_spell_pick_up_indicator(env);

    let probe = "local f = CreateFrame('Frame', 'PickupIndicatorLayoutProbe', UIParent, 'SpellPickupIndicatorTemplate') \
                 if not f then return false end \
                 return type(f.Layout) == 'function' or \
                        type(f.GetMinimumSize) == 'function' or \
                        type(f.MarkDirty) == 'function'";
    let result: bool = env
        .eval(probe)
        .expect("SpellPickupIndicatorTemplate Layout probe");
    assert!(
        result,
        "SpellPickupIndicatorTemplate inherits=\"HorizontalLayoutFrame\" \
         so the materialized frame must expose the layout API (Layout / \
         GetMinimumSize / MarkDirty — at least one of which the simulator \
         provides). The mixin methods all call self:Layout() after \
         show/hide manipulations to rebuild the LEFT-to-RIGHT flow with \
         the four ARTWORK children sequenced by their layoutIndex KeyValues"
    );
}
}

prefork_full_ui_case! {
fn display_frame_inherits_callback_registrant_and_vertical_layout(env: &WowLuaEnv) {
    load_spell_pick_up_indicator(env);

    let probe = "local f = SpellPickupDisplay \
                 if not f then return false end \
                 return (type(f.AddStaticEventMethod) == 'function' or \
                         type(f.AddDynamicEventMethod) == 'function' or \
                         type(f.RegisterCallback) == 'function') and \
                        (type(f.Layout) == 'function' or \
                         type(f.GetMinimumSize) == 'function')";
    let result: bool = env
        .eval(probe)
        .expect("SpellPickupDisplay inheritance probe");
    assert!(
        result,
        "SpellPickupDisplay inherits=\"CallbackRegistrantTemplate, \
         VerticalLayoutFrame\" so it must expose BOTH the \
         CallbackRegistrant API (AddStaticEventMethod / \
         AddDynamicEventMethod / RegisterCallback — the OnLoad method \
         calls AddStaticEventMethod for WorldLootObjectTooltip.Shown and \
         AddDynamicEventMethod for .Hidden) AND the layout API (Layout / \
         GetMinimumSize)"
    );
}
}
