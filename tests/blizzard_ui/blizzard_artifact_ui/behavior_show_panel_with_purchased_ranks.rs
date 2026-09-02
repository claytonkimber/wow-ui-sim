//! Positive show behavior for `Blizzard_ArtifactUI`.

use crate::common::blizzard_addon_harness::with_blizzard_addon_smoke_shape;
use wow_ui_sim::lua_api::state::ArtifactInfo;

const ROOT: &str = "Blizzard_ArtifactUI";
const ARTIFACT_ICON: &str = "Interface/Icons/inv_sword_2h_artifactashbringer_d_01";

#[test]
fn show_panel_with_purchased_ranks_opens_perks_tab_at_forge() {
    with_blizzard_addon_smoke_shape(&[], &[], |env, _loaded| {
        seed_viewed_artifact_at_forge(env);
        load_artifact_ui(env);

        let mismatches: Vec<String> = env
            .eval(SHOW_PURCHASED_RANKS_PANEL_PROBE)
            .expect("ArtifactUI purchased-ranks show probe should run cleanly");

        assert!(
            mismatches.is_empty(),
            "`{ROOT}` must open the perks tab for a viewed artifact at the forge; \
             mismatches: {mismatches:?}"
        );
    });
}

fn seed_viewed_artifact_at_forge(env: &wow_ui_sim::lua_api::WowLuaEnv) {
    let mut state = env.state().borrow_mut();
    state.viewed_artifact.info = Some(sample_artifact());
    state.viewed_artifact.total_purchased_ranks = 3;
    state.viewed_artifact.is_at_forge = true;
}

fn sample_artifact() -> ArtifactInfo {
    ArtifactInfo {
        item_id: 128_910,
        alt_item_id: 128_911,
        name: "Ashbringer".to_string(),
        icon: ARTIFACT_ICON.to_string(),
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
        "`{ROOT}` must load before purchased-ranks ShowUIPanel probe; error={error:?}"
    );
}

const SHOW_PURCHASED_RANKS_PANEL_PROBE: &str = r#"
local mismatches = {}

local function expect(condition, message)
    if not condition then
        table.insert(mismatches, message)
    end
end

local expectedIcon = 1109508
local originalPerksOnUIOpened = ArtifactFrame.PerksTab.OnUIOpened
ArtifactFrame.PerksTab.OnUIOpened = function() end

local ok, errorMessage = pcall(function()
    ShowUIPanel(ArtifactFrame)
end)

ArtifactFrame.PerksTab.OnUIOpened = originalPerksOnUIOpened

expect(ok, "ShowUIPanel error:" .. tostring(errorMessage))
expect(ArtifactFrame:IsShown(), "ArtifactFrame should be shown")
expect(ArtifactFrame.wasAtForge == true, "wasAtForge:" .. tostring(ArtifactFrame.wasAtForge))
expect(ArtifactFrame.AppearancesTabButton:IsShown(), "AppearancesTabButton should be shown")
expect(PanelTemplates_GetSelectedTab(ArtifactFrame) == 1, "selected tab:" .. tostring(PanelTemplates_GetSelectedTab(ArtifactFrame)))
expect(ArtifactFrame:GetWidth() == 896, "ArtifactFrame width:" .. tostring(ArtifactFrame:GetWidth()))
expect(ArtifactFrame.PerksTab:IsShown(), "PerksTab should be shown")
expect(not ArtifactFrame.AppearancesTab:IsShown(), "AppearancesTab should be hidden")
expect(
    ArtifactFrame.ForgeBadgeFrame.ItemIcon:GetTexture() == expectedIcon,
    "ForgeBadgeFrame.ItemIcon texture:" .. tostring(ArtifactFrame.ForgeBadgeFrame.ItemIcon:GetTexture())
)

return mismatches
"#;
