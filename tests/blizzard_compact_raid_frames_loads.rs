#![cfg(feature = "client-retail")]
use std::path::PathBuf;

use wow_ui_sim::loader::discover_blizzard_addons_for_screen;
use wow_ui_sim::loader::load_addon;
use wow_ui_sim::lua_api::WowLuaEnv;
use wow_ui_sim::screen::ScreenKind;
use wow_ui_sim::startup::fire_startup_events_for_screen;
use wow_ui_sim::toc::TocFile;

fn blizzard_ui_dir() -> PathBuf {
    wow_ui_sim::client_profile::blizzard_ui_addons_dir_under(std::path::Path::new(env!(
        "CARGO_MANIFEST_DIR"
    )))
}

fn compact_raid_frames_toc() -> PathBuf {
    blizzard_ui_dir().join("Blizzard_CompactRaidFrames/Blizzard_CompactRaidFrames.toc")
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
fn blizzard_compact_raid_frames_toc_declares_raidframe_dependency() {
    let toc = TocFile::from_file(&compact_raid_frames_toc())
        .expect("Blizzard_CompactRaidFrames TOC should parse");
    assert!(
        !toc.is_load_on_demand(),
        "Blizzard_CompactRaidFrames is a non-LOD addon (it should auto-load on Game screen)"
    );
    let deps = toc.dependencies();
    assert!(
        deps.contains(&"Blizzard_RaidFrame".to_string()),
        "Blizzard_CompactRaidFrames should declare `## Dependencies: Blizzard_RaidFrame` (so its \
         `CompactUnitFrame_*` / `DefaultCompactUnitFrameSetup` references resolve), got {deps:?}"
    );
}

#[test]
fn blizzard_compact_raid_frames_appears_in_game_discovery() {
    let addons = discover_blizzard_addons_for_screen(&blizzard_ui_dir(), ScreenKind::Game);
    let in_game = addons
        .iter()
        .any(|(name, _)| name == "Blizzard_CompactRaidFrames");
    assert!(
        in_game,
        "Blizzard_CompactRaidFrames (non-LOD, no AllowLoadGameType restriction) should appear in \
         Game-screen auto-discovery"
    );
}

prefork_full_ui_case! {
fn blizzard_compact_raid_frames_loads_without_errors(env: &WowLuaEnv) {

    let addon_errors: Vec<String> = env
        .state()
        .borrow()
        .lua_errors
        .iter()
        .filter(|message| {
            message.contains("Blizzard_CompactRaidFrames")
                || message.contains("Blizzard_CompactRaidFrameContainer")
                || message.contains("Blizzard_CompactRaidFrameManager")
                || message.contains("Blizzard_CompactRaidFrameReservationManager")
        })
        .cloned()
        .collect();
    assert!(
        addon_errors.is_empty(),
        "Blizzard_CompactRaidFrames emitted Lua errors during load:\n  {}",
        addon_errors.join("\n  ")
    );
}
}

prefork_full_ui_case! {
fn blizzard_compact_raid_frames_toplevel_frames_are_defined(env: &WowLuaEnv) {

    let frames_present: bool = env
        .eval(
            "return type(_G.CompactRaidFrameContainer) == 'table' \
                and CompactRaidFrameContainer:GetParent() == UIParent \
                and type(_G.CompactRaidFrameManager) == 'table' \
                and CompactRaidFrameManager:GetParent() == UIParent",
        )
        .expect("toplevel frame query should succeed");
    assert!(
        frames_present,
        "Blizzard_CompactRaidFrames should define both top-level frames after load: \
         CompactRaidFrameContainer (parent UIParent, ResizeLayoutFrame + \
         EditModeUnitFrameSystemTemplate, frameStrata=HIGH, mixin=CompactRaidFrameContainerMixin) \
         and CompactRaidFrameManager (parent UIParent, frameStrata=MEDIUM, hidden=true, \
         toplevel=true)"
    );
}
}

prefork_full_ui_case! {
fn blizzard_compact_raid_frames_max_raid_groups_constant_is_eight(env: &WowLuaEnv) {

    let constant_present: bool = env
        .eval("return MAX_RAID_GROUPS == 8")
        .expect("MAX_RAID_GROUPS query should succeed");
    assert!(
        constant_present,
        "Blizzard_CompactRaidFrameContainer.lua should set the MAX_RAID_GROUPS = 8 global \
         (the cap used by RaidUtil_GetUsedGroups and the discrete-mode group iteration)"
    );
}
}

prefork_full_ui_case! {
fn blizzard_compact_raid_frames_container_mixin_methods_are_defined(env: &WowLuaEnv) {

    let methods_present: bool = env
        .eval(
            "return type(CompactRaidFrameContainerMixin) == 'table' \
                and type(CompactRaidFrameContainerMixin.OnLoad) == 'function' \
                and type(CompactRaidFrameContainerMixin.OnEvent) == 'function' \
                and type(CompactRaidFrameContainerMixin.OnSizeChanged) == 'function' \
                and type(CompactRaidFrameContainerMixin.SetGroupMode) == 'function' \
                and type(CompactRaidFrameContainerMixin.GetGroupMode) == 'function' \
                and type(CompactRaidFrameContainerMixin.SetFlowFilterFunction) == 'function' \
                and type(CompactRaidFrameContainerMixin.SetGroupFilterFunction) == 'function' \
                and type(CompactRaidFrameContainerMixin.SetFlowSortFunction) == 'function' \
                and type(CompactRaidFrameContainerMixin.SetDisplayPets) == 'function' \
                and type(CompactRaidFrameContainerMixin.SetDisplayMainTankAndAssist) == 'function' \
                and type(CompactRaidFrameContainerMixin.ApplyToFrames) == 'function' \
                and type(CompactRaidFrameContainerMixin.TryUpdate) == 'function' \
                and type(CompactRaidFrameContainerMixin.ReadyToUpdate) == 'function' \
                and type(CompactRaidFrameContainerMixin.LayoutFrames) == 'function' \
                and type(CompactRaidFrameContainerMixin.AddGroups) == 'function' \
                and type(CompactRaidFrameContainerMixin.AddGroup) == 'function' \
                and type(CompactRaidFrameContainerMixin.AddPlayers) == 'function' \
                and type(CompactRaidFrameContainerMixin.AddPets) == 'function' \
                and type(CompactRaidFrameContainerMixin.AddFlaggedUnits) == 'function' \
                and type(CompactRaidFrameContainerMixin.AddUnitFrame) == 'function' \
                and type(CompactRaidFrameContainerMixin.GetUnitFrame) == 'function' \
                and type(CompactRaidFrameContainerMixin.ReleaseAllReservedFrames) == 'function'",
        )
        .expect("CompactRaidFrameContainerMixin query should succeed");
    assert!(
        methods_present,
        "CompactRaidFrameContainerMixin should expose its 22 documented methods covering \
         lifecycle (OnLoad/OnEvent/OnSizeChanged), group-mode/filter/sort setters, layout \
         (TryUpdate/ReadyToUpdate/LayoutFrames), member iteration (AddGroups/AddGroup/AddPlayers/\
         AddPets/AddFlaggedUnits/AddUnitFrame/GetUnitFrame), and reservation cleanup \
         (ReleaseAllReservedFrames)"
    );
}
}

prefork_full_ui_case! {
fn blizzard_compact_raid_frames_manager_mixins_are_defined(env: &WowLuaEnv) {

    let mixins_present: bool = env
        .eval(
            "return type(CRFM_TooltipMixin) == 'table' \
                and type(CRFM_ButtonStateBehaviorMixin) == 'table' \
                and type(CRFM_ToolbarButtonMixin) == 'table' \
                and type(CRFM_DifficultyDropdownMixin) == 'table' \
                and type(RaidFrameToggleButtonMixin) == 'table' \
                and type(CRFManagerFilterRoleButtonMixin) == 'table' \
                and type(CRFManagerFilterGroupButtonMixin) == 'table' \
                and type(CRFManagerRoleMarkerCheckMixin) == 'table' \
                and type(CRFManagerRaidIconButtonMixin) == 'table' \
                and type(CRFManagerMarkerTabMixin) == 'table' \
                and type(CRFRaidMarkersMixin) == 'table' \
                and type(RaidFrameManagerRestrictPingsButtonMixin) == 'table' \
                and type(LeavePartyButtonMixin) == 'table' \
                and type(LeaveInstanceGroupButtonMixin) == 'table'",
        )
        .expect("manager mixin query should succeed");
    assert!(
        mixins_present,
        "Blizzard_CompactRaidFrames Mainline/Manager.lua should define all 14 manager mixins \
         (CRFM_TooltipMixin, CRFM_ButtonStateBehaviorMixin extending ButtonStateBehaviorMixin, \
         CRFM_ToolbarButtonMixin = CreateFromMixins(Tooltip, ButtonStateBehavior), \
         CRFM_DifficultyDropdownMixin extending CRFM_ToolbarButtonMixin, \
         RaidFrameToggleButtonMixin, CRFManagerFilterRoleButtonMixin, \
         CRFManagerFilterGroupButtonMixin, CRFManagerRoleMarkerCheckMixin, \
         CRFManagerRaidIconButtonMixin, CRFManagerMarkerTabMixin, CRFRaidMarkersMixin, \
         RaidFrameManagerRestrictPingsButtonMixin, LeavePartyButtonMixin, \
         LeaveInstanceGroupButtonMixin)"
    );
}
}

prefork_full_ui_case! {
fn blizzard_compact_raid_frames_sort_helpers_order_player_first(env: &WowLuaEnv) {

    let sort_helpers_correct: bool = env
        .eval(
            "return type(CRFSort_Group) == 'function' \
                and type(CRFSort_Role) == 'function' \
                and type(CRFSort_Alphabetical) == 'function' \
                and CRFSort_Group('player', 'party1') == true \
                and CRFSort_Group('party1', 'player') == false \
                and CRFSort_Group('party1', 'party2') == true",
        )
        .expect("CRFSort_Group query should succeed");
    assert!(
        sort_helpers_correct,
        "CRFSort_Group should sort the player above other party members and use string-compare \
         between same-prefix party tokens; CRFSort_Role and CRFSort_Alphabetical should also be \
         defined as globals (used as flowSortFunc by CompactRaidFrameContainer:SetFlowSortFunction)"
    );
}
}

prefork_full_ui_case! {
fn blizzard_compact_raid_frames_reservation_manager_round_trip(env: &WowLuaEnv) {

    let round_trip_works: bool = env
        .eval(
            "local released = {}; \
             local manager = CompactRaidFrameReservation_NewManager(function(f) \
               released[#released + 1] = f \
             end); \
             local frame_a = { inUse = true }; \
             local frame_b = { inUse = true }; \
             CompactRaidFrameReservation_RegisterReservation(manager, frame_a, 'guid-a'); \
             CompactRaidFrameReservation_RegisterReservation(manager, frame_b, 'guid-b'); \
             local lookup_a = CompactRaidFrameReservation_GetFrame(manager, 'guid-a'); \
             local lookup_b = CompactRaidFrameReservation_GetReservation(manager, 'guid-b'); \
             frame_a.inUse = false; \
             CompactRaidFrameReservation_ReleaseUnusedReservations(manager); \
             return lookup_a == frame_a \
                and lookup_b == frame_b \
                and #released == 1 \
                and released[1] == frame_a \
                and manager.reservations['guid-a'] == false \
                and manager.reservations['guid-b'] == frame_b \
                and #manager.unusedFrames == 1 \
                and manager.unusedFrames[1] == frame_a",
        )
        .expect("reservation manager round-trip query should succeed");
    assert!(
        round_trip_works,
        "CompactRaidFrameReservation_{{NewManager,RegisterReservation,GetFrame,GetReservation,\
         ReleaseUnusedReservations}} should give a key→frame reservation system with a release \
         callback: registered frames are recoverable by key, marking inUse=false then calling \
         ReleaseUnusedReservations should invoke the release callback once, blank the slot to \
         `false`, and push the frame onto unusedFrames so a future GetFrame for an unknown key \
         can recycle it"
    );
}
}
