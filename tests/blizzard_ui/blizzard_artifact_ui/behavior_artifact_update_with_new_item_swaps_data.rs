//! New-item update behavior for `Blizzard_ArtifactUI`.

use crate::common::blizzard_addon_harness::with_blizzard_addon_smoke_shape;
use wow_ui_sim::lua_api::state::ArtifactInfo;

const ROOT: &str = "Blizzard_ArtifactUI";
const INITIAL_ARTIFACT_ICON: &str = "Interface/Icons/inv_sword_2h_artifactashbringer_d_01";
const UPDATED_ARTIFACT_ICON: &str = "Interface/Icons/inv_axe_2h_artifactmaw_d_01";

#[test]
fn artifact_update_with_new_item_refreshes_all_visible_artifact_data() {
    with_blizzard_addon_smoke_shape(&[], &[], |env, _loaded| {
        seed_viewed_artifact_at_forge(env);
        load_artifact_ui(env);
        show_panel_and_install_update_spies(env);

        env.state().borrow_mut().viewed_artifact.info = Some(updated_artifact());

        let mismatches: Vec<String> = env
            .eval(ARTIFACT_UPDATE_WITH_NEW_ITEM_PROBE)
            .expect("ArtifactUI new-item update probe should run cleanly");

        assert!(
            mismatches.is_empty(),
            "`{ROOT}` must refresh visible artifact data on ARTIFACT_UPDATE(true); \
             mismatches: {mismatches:?}"
        );
    });
}

fn seed_viewed_artifact_at_forge(env: &wow_ui_sim::lua_api::WowLuaEnv) {
    let mut state = env.state().borrow_mut();
    state.viewed_artifact.info = Some(initial_artifact());
    state.viewed_artifact.total_purchased_ranks = 3;
    state.viewed_artifact.is_at_forge = true;
}

fn initial_artifact() -> ArtifactInfo {
    sample_artifact("Ashbringer", INITIAL_ARTIFACT_ICON)
}

fn updated_artifact() -> ArtifactInfo {
    sample_artifact("Maw of the Damned", UPDATED_ARTIFACT_ICON)
}

fn sample_artifact(name: &str, icon: &str) -> ArtifactInfo {
    ArtifactInfo {
        item_id: 128_910,
        alt_item_id: 128_911,
        name: name.to_string(),
        icon: icon.to_string(),
        total_xp: 12_500,
        points_spent: 3,
        quality: 6,
        artifact_appearance_id: 41,
        appearance_mod_id: 0,
        item_appearance_id: 0,
        alt_item_appearance_id: 0,
        alt_on_top: false,
        tier: 1,
        maxed: false,
        disabled: false,
        category: 1,
    }
}

fn load_artifact_ui(env: &wow_ui_sim::lua_api::WowLuaEnv) {
    let (loaded, error): (bool, Option<String>) = env
        .eval(r#"return C_AddOns.LoadAddOn("Blizzard_ArtifactUI")"#)
        .expect("C_AddOns.LoadAddOn probe should run cleanly");
    assert!(
        loaded,
        "`{ROOT}` must load before new-item update probe; error={error:?}"
    );
}

fn show_panel_and_install_update_spies(env: &wow_ui_sim::lua_api::WowLuaEnv) {
    let mismatches: Vec<String> = env
        .eval(SHOW_PANEL_AND_INSTALL_UPDATE_SPIES)
        .expect("ArtifactUI spy setup probe should run cleanly");
    assert!(
        mismatches.is_empty(),
        "`{ROOT}` must show before update spies are installed; mismatches: {mismatches:?}"
    );
}

const SHOW_PANEL_AND_INSTALL_UPDATE_SPIES: &str = r#"
local mismatches = {}

local function expect(condition, message)
    if not condition then
        table.insert(mismatches, message)
    end
end

local originalPerksOnUIOpened = ArtifactFrame.PerksTab.OnUIOpened
ArtifactFrame.PerksTab.OnUIOpened = function() end
local showOk, showError = pcall(function()
    ShowUIPanel(ArtifactFrame)
end)
ArtifactFrame.PerksTab.OnUIOpened = originalPerksOnUIOpened

expect(showOk, "ShowUIPanel error:" .. tostring(showError))
expect(ArtifactFrame:IsShown(), "ArtifactFrame should be shown before spy install")

local probe = {
    evaulateForgeState = 0,
    refreshKnowledgeRanks = 0,
    setupPerArtifactData = 0,
    perksRefresh = 0,
    perksRefreshNewItem = nil,
    appearancesNewItemEquipped = 0,
}
ArtifactFrame.__newItemUpdateProbe = probe

local originalEvaulateForgeState = ArtifactFrame.EvaulateForgeState
ArtifactFrame.EvaulateForgeState = function(self, ...)
    probe.evaulateForgeState = probe.evaulateForgeState + 1
    return originalEvaulateForgeState(self, ...)
end

local originalRefreshKnowledgeRanks = ArtifactFrame.RefreshKnowledgeRanks
ArtifactFrame.RefreshKnowledgeRanks = function(self, ...)
    probe.refreshKnowledgeRanks = probe.refreshKnowledgeRanks + 1
    return originalRefreshKnowledgeRanks(self, ...)
end

local originalSetupPerArtifactData = ArtifactFrame.SetupPerArtifactData
ArtifactFrame.SetupPerArtifactData = function(self, ...)
    probe.setupPerArtifactData = probe.setupPerArtifactData + 1
    return originalSetupPerArtifactData(self, ...)
end

local originalPerksRefresh = ArtifactFrame.PerksTab.Refresh
ArtifactFrame.PerksTab.Refresh = function(self, newItem, ...)
    probe.perksRefresh = probe.perksRefresh + 1
    probe.perksRefreshNewItem = newItem
    return originalPerksRefresh(self, newItem, ...)
end

local originalAppearancesOnNewItemEquipped = ArtifactFrame.AppearancesTab.OnNewItemEquipped
ArtifactFrame.AppearancesTab.OnNewItemEquipped = function(self, ...)
    probe.appearancesNewItemEquipped = probe.appearancesNewItemEquipped + 1
    return originalAppearancesOnNewItemEquipped(self, ...)
end

return mismatches
"#;

const ARTIFACT_UPDATE_WITH_NEW_ITEM_PROBE: &str = r#"
local mismatches = {}

local function expect(condition, message)
    if not condition then
        table.insert(mismatches, message)
    end
end

local expectedIcon = 1121487
local ok, errorMessage = pcall(function()
    FireEvent("ARTIFACT_UPDATE", true)
end)
local probe = ArtifactFrame.__newItemUpdateProbe

expect(ok, "ARTIFACT_UPDATE dispatch error:" .. tostring(errorMessage))
expect(probe.evaulateForgeState == 1, "EvaulateForgeState count:" .. tostring(probe.evaulateForgeState))
expect(probe.refreshKnowledgeRanks == 1, "RefreshKnowledgeRanks count:" .. tostring(probe.refreshKnowledgeRanks))
expect(probe.setupPerArtifactData == 1, "SetupPerArtifactData count:" .. tostring(probe.setupPerArtifactData))
expect(probe.perksRefresh == 1, "PerksTab.Refresh count:" .. tostring(probe.perksRefresh))
expect(probe.perksRefreshNewItem == true, "PerksTab.Refresh newItem:" .. tostring(probe.perksRefreshNewItem))
expect(
    probe.appearancesNewItemEquipped == 1,
    "AppearancesTab.OnNewItemEquipped count:" .. tostring(probe.appearancesNewItemEquipped)
)
expect(
    ArtifactFrame.ForgeBadgeFrame.ItemIcon:GetTexture() == expectedIcon,
    "ForgeBadgeFrame.ItemIcon texture:" .. tostring(ArtifactFrame.ForgeBadgeFrame.ItemIcon:GetTexture())
)

return mismatches
"#;
