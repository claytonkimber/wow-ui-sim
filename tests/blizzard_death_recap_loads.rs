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

fn death_recap_toc() -> PathBuf {
    blizzard_ui_dir().join("Blizzard_DeathRecap/Blizzard_DeathRecap.toc")
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
fn blizzard_death_recap_toc_is_load_on_demand() {
    let toc = TocFile::from_file(&death_recap_toc()).expect("Blizzard_DeathRecap TOC should parse");
    assert!(
        toc.is_load_on_demand(),
        "Blizzard_DeathRecap declares `## LoadOnDemand: 1` (the recap window is brought in by \
         the OpenDeathRecap chat link / `/recap` slash command via UIParentLoadAddOn — must \
         NOT auto-load on Game-screen bring-up)"
    );
    assert!(
        !toc.is_secure_env(),
        "Blizzard_DeathRecap does not declare UseSecureEnvironment"
    );
    assert!(
        toc.dependencies().is_empty(),
        "Blizzard_DeathRecap has no `## Dependencies` line — it is a self-contained recap \
         window that consumes the standard ScrollUtil / WowScrollBoxList / MinimalScrollBar / \
         UIPanelCloseButton / UIPanelButtonTemplate templates from FrameXML"
    );
}

#[test]
fn blizzard_death_recap_is_absent_from_game_auto_discovery() {
    let addons = discover_blizzard_addons_for_screen(&blizzard_ui_dir(), ScreenKind::Game);
    let in_game = addons.iter().any(|(name, _)| name == "Blizzard_DeathRecap");
    assert!(
        !in_game,
        "Blizzard_DeathRecap is `## LoadOnDemand: 1`, so it must NOT appear in Game-screen \
         auto-discovery — it is loaded explicitly by OpenDeathRecap / the `/recap` slash \
         command via UIParentLoadAddOn"
    );
}

prefork_full_ui_case! {
fn blizzard_death_recap_loads_via_load_addon_without_errors(env: &WowLuaEnv) {

    {
        let mut state = env.state().borrow_mut();
        state.lua_errors.clear();
        state.lua_error_records.clear();
        state.lua_error_counts.clear();
    }

    load_addon(&env.loader_env(), &death_recap_toc())
        .expect("Blizzard_DeathRecap should load via Rust loader");

    let load_errors: Vec<String> = env.state().borrow().lua_errors.clone();
    assert!(
        load_errors.is_empty(),
        "Blizzard_DeathRecap emitted Lua errors during explicit load:\n  {}",
        load_errors.join("\n  ")
    );
}
}

prefork_full_ui_case! {
fn blizzard_death_recap_toplevel_frame_is_created_after_load(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &death_recap_toc())
        .expect("Blizzard_DeathRecap should load via Rust loader");

    let frame_present: bool = env
        .eval(
            "return type(DeathRecapFrame) == 'table' \
                and DeathRecapFrame:GetParent() == UIParent \
                and DeathRecapFrame:IsShown() == false \
                and type(DeathRecapFrame.OpenRecap) == 'function' \
                and type(DeathRecapFrame.GetRecapID) == 'function' \
                and type(DeathRecapFrame.BuildDataProvider) == 'function'",
        )
        .expect("DeathRecapFrame query should succeed");
    assert!(
        frame_present,
        "Blizzard_DeathRecap.xml line 96 should create the toplevel `DeathRecapFrame` \
         (parent=UIParent, frameStrata=HIGH, movable=true, clampedToScreen=true, hidden=true) \
         with the DeathRecapMixin attached. The frame owns the OpenRecap(recapID) entry point \
         called by OpenDeathRecap chat-link clicks"
    );
}
}

prefork_full_ui_case! {
fn blizzard_death_recap_main_mixin_methods_are_defined(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &death_recap_toc())
        .expect("Blizzard_DeathRecap should load via Rust loader");

    let mixin_present: bool = env
        .eval(
            "return type(DeathRecapMixin) == 'table' \
                and type(DeathRecapMixin.OnLoad) == 'function' \
                and type(DeathRecapMixin.OnHide) == 'function' \
                and type(DeathRecapMixin.OpenRecap) == 'function' \
                and type(DeathRecapMixin.GetRecapID) == 'function' \
                and type(DeathRecapMixin.InitializeScrollBox) == 'function' \
                and type(DeathRecapMixin.BuildDataProvider) == 'function' \
                and type(DeathRecapMixin.GetEntryFrameCount) == 'function' \
                and type(DeathRecapMixin.GetCloseButton) == 'function' \
                and type(DeathRecapMixin.GetDragButton) == 'function' \
                and type(DeathRecapMixin.GetDivider) == 'function' \
                and type(DeathRecapMixin.GetScrollBox) == 'function' \
                and type(DeathRecapMixin.GetScrollBar) == 'function' \
                and type(DeathRecapMixin.GetUnavailableFontString) == 'function'",
        )
        .expect("DeathRecapMixin method query should succeed");
    assert!(
        mixin_present,
        "Blizzard_DeathRecap.lua line 233 should publish DeathRecapMixin with 14 methods: 6 \
         child-frame accessors (GetCloseButton / GetDragButton / GetDivider / GetScrollBox / \
         GetScrollBar / GetUnavailableFontString); OnLoad wiring CloseButton OnClick→\
         HideUIPanel + DragButton OnDragStart/OnDragStop StartMoving/StopMovingOrSizing + \
         InitializeScrollBox; OnHide clearing self.recapID; OpenRecap(recapID) toggling the \
         frame visibility via ShowUIPanel/HideUIPanel and pushing C_DeathRecap.GetRecapEvents \
         into the scrollbox; GetRecapID; InitializeScrollBox setting up the \
         DeathRecapEntryTemplate ListLinearView with ScrollUtil.\
         AddManagedScrollBarVisibilityBehavior; GetEntryFrameCount; and BuildDataProvider \
         iterating the recap events to flag highestDamage / maxHealth / timeBeforeDeath / \
         causedDeath fields"
    );
}
}

prefork_full_ui_case! {
fn blizzard_death_recap_entry_mixin_methods_are_defined(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &death_recap_toc())
        .expect("Blizzard_DeathRecap should load via Rust loader");

    let mixin_present: bool = env
        .eval(
            "return type(DeathRecapEntryMixin) == 'table' \
                and type(DeathRecapEntryMixin.OnLoad) == 'function' \
                and type(DeathRecapEntryMixin.Init) == 'function' \
                and type(DeathRecapEntryMixin.GetEventInfo) == 'function' \
                and type(DeathRecapEntryMixin.GetHealthPercent) == 'function' \
                and type(DeathRecapEntryMixin.GetTimeBeforeDeath) == 'function' \
                and type(DeathRecapEntryMixin.GetDamageInfo) == 'function' \
                and type(DeathRecapEntryMixin.GetSpellInfo) == 'function' \
                and type(DeathRecapEntryMixin.GetTombstoneIcon) == 'function' \
                and type(DeathRecapEntryMixin.GetAvoidableIcon) == 'function' \
                and type(DeathRecapEntryMixin.GetDeadlyIcon) == 'function' \
                and type(DeathRecapEntryMixin.GetDamageInfoAmount) == 'function' \
                and type(DeathRecapEntryMixin.GetDamageInfoAmountLarge) == 'function' \
                and type(DeathRecapEntryMixin.GetSpellInfoCaster) == 'function' \
                and type(DeathRecapEntryMixin.GetSpellInfoIcon) == 'function' \
                and type(DeathRecapEntryMixin.GetSpellInfoName) == 'function'",
        )
        .expect("DeathRecapEntryMixin method query should succeed");
    assert!(
        mixin_present,
        "Blizzard_DeathRecap.lua line 1 should publish DeathRecapEntryMixin with 16 methods: 9 \
         child accessors (GetDamageInfo / GetSpellInfo / GetTombstoneIcon / GetAvoidableIcon / \
         GetDeadlyIcon / GetDamageInfoAmount / GetDamageInfoAmountLarge / GetSpellInfoCaster / \
         GetSpellInfoIcon / GetSpellInfoName); OnLoad wiring the per-region GameTooltip \
         OnEnter/OnLeave handlers that emit DEATH_RECAP_DAMAGE_TT / DEATH_RECAP_CAST_BY_TT / \
         DEATH_RECAP_AVOIDABLE_SPELL / DEATH_RECAP_DEADLY_SPELL / DEATH_RECAP_CURR_HP_TT / \
         DEATH_RECAP_DEATH_TT lines; GetEventInfo(eventData) decoding the spell→localized + \
         caster + flags+ icon tuple from the C_DeathRecap event payload; Init(elementData) \
         pulled by the scrollbox view's per-row initializer (sets the icon textures + \
         HP-percent text + tombstone visibility + avoidable/deadly markers + damage amount \
         font scale based on `highestDamage`); GetHealthPercent + GetTimeBeforeDeath cached on \
         the entry"
    );
}
}

prefork_full_ui_case! {
fn blizzard_death_recap_xml_template_is_registered(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &death_recap_toc())
        .expect("Blizzard_DeathRecap should load via Rust loader");

    assert!(
        wow_ui_sim::xml::get_template("DeathRecapEntryTemplate").is_some(),
        "DeathRecapEntryTemplate (`<Frame virtual=\"true\" mixin=\"DeathRecapEntryMixin\" \
         parentArray=\"DeathRecapEntry\">` from Blizzard_DeathRecap.xml line 4) should be \
         registered in the Frame template registry — DeathRecapMixin:InitializeScrollBox uses \
         `view:SetElementInitializer(\"DeathRecapEntryTemplate\", ...)` to spawn each row of \
         the recap list"
    );
}
}

prefork_full_ui_case! {
fn blizzard_death_recap_c_namespace_is_available(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &death_recap_toc())
        .expect("Blizzard_DeathRecap should load via Rust loader");

    let namespace_present: bool = env
        .eval(
            "return type(C_DeathRecap) == 'table' \
                and type(C_DeathRecap.GetKillingBlows) == 'function' \
                and type(C_DeathRecap.GetMostRecentDeathRecap) == 'function'",
        )
        .expect("C_DeathRecap namespace query should succeed");
    assert!(
        namespace_present,
        "C_DeathRecap should be registered with at least GetKillingBlows + \
         GetMostRecentDeathRecap (backed by SimState.death_recaps in \
         src/c_api/c_death_recap.rs). \
         DeathRecapMixin:BuildDataProvider also calls C_DeathRecap.GetRecapEvents(recapID) and \
         C_DeathRecap.GetRecapMaxHealth(recapID) — both guarded with `or {{}}` / nil fallback \
         so the addon load itself does not require those probes to be present"
    );
}
}

prefork_full_ui_case! {
fn blizzard_death_recap_open_recap_is_safe_with_no_recapped_data(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &death_recap_toc())
        .expect("Blizzard_DeathRecap should load via Rust loader");

    {
        let mut state = env.state().borrow_mut();
        state.lua_errors.clear();
        state.lua_error_records.clear();
        state.lua_error_counts.clear();
    }

    let recap_id_after_open: bool = env
        .eval(
            "do DeathRecapFrame:OpenRecap(123) end \
             return DeathRecapFrame:GetRecapID() == 123 and DeathRecapFrame:IsShown() == true",
        )
        .expect("OpenRecap query should succeed");
    assert!(
        recap_id_after_open,
        "DeathRecapFrame:OpenRecap(123) should set self.recapID=123 and ShowUIPanel(self) — \
         even when there is no recap data, BuildDataProvider's `local events = \
         C_DeathRecap.GetRecapEvents(recapID) or {{}}` handles the nil case so the call must \
         not error"
    );

    let post_open_errors: Vec<String> = env.state().borrow().lua_errors.clone();
    assert!(
        post_open_errors.is_empty(),
        "DeathRecapFrame:OpenRecap(123) should not emit Lua errors (no recap data is fine; \
         BuildDataProvider tolerates an empty events list):\n  {}",
        post_open_errors.join("\n  ")
    );
}
}
