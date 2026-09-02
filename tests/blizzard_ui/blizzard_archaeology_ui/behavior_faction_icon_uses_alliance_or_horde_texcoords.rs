//! Faction icon tex-coords for `Blizzard_ArchaeologyUI`.

use crate::common::blizzard_addon_harness::{
    load_blizzard_addon_closure_into_env, new_blizzard_addon_env,
};
use crate::common::panel_fixtures::{blizzard_ui_dir, load_panel_addons};
use wow_ui_sim::lua_api::WowLuaEnv;
use wow_ui_sim::startup::settle_headless_startup;

const ROOT: &str = "Blizzard_ArchaeologyUI";
const HUMAN_RACE_INDEX: usize = 0;
const ORC_RACE_INDEX: usize = 1;

const ALLIANCE_TAB_ICON_COORDS: TexCoords =
    TexCoords(0.31250000, 0.36914063, 0.79296875, 0.93359375);
const ALLIANCE_FRAME_ICON_COORDS: TexCoords =
    TexCoords(0.41992188, 0.47265625, 0.45703125, 0.58593750);
const HORDE_TAB_ICON_COORDS: TexCoords = TexCoords(0.21484375, 0.27343750, 0.79296875, 0.93750000);
const HORDE_FRAME_ICON_COORDS: TexCoords =
    TexCoords(0.41992188, 0.47265625, 0.32031250, 0.44921875);

#[test]
fn archaeology_faction_icons_use_player_faction_texcoords() {
    let alliance = load_archaeology_with_player_race(HUMAN_RACE_INDEX);
    assert_faction_icon_coords(
        &alliance,
        ALLIANCE_TAB_ICON_COORDS,
        ALLIANCE_FRAME_ICON_COORDS,
        "Alliance",
    );

    let horde = load_archaeology_with_player_race(ORC_RACE_INDEX);
    assert_faction_icon_coords(
        &horde,
        HORDE_TAB_ICON_COORDS,
        HORDE_FRAME_ICON_COORDS,
        "Horde",
    );
}

#[derive(Debug, Clone, Copy)]
struct TexCoords(f64, f64, f64, f64);

#[derive(Debug)]
struct FactionIconProbe {
    tab_icon: TexCoords,
    frame_icon: TexCoords,
}

fn load_archaeology_with_player_race(race_index: usize) -> FactionIconProbe {
    let ui_dir = blizzard_ui_dir();
    let env = new_blizzard_addon_env(&ui_dir);

    load_panel_addons(&env);
    set_player_race(&env, race_index);
    load_blizzard_addon_closure_into_env(&env, &ui_dir, &[ROOT], &[]);
    env.apply_post_load_workarounds();
    settle_headless_startup(&env);

    faction_icon_probe(&env)
}

fn set_player_race(env: &WowLuaEnv, race_index: usize) {
    let state = env.state();
    let mut sim = state.borrow_mut();
    sim.player.race_index = race_index;
}

fn faction_icon_probe(env: &WowLuaEnv) -> FactionIconProbe {
    let coords: (f64, f64, f64, f64, f64, f64, f64, f64) = env
        .eval(
            r#"
            local tabLeft, tabTop, _, tabBottom, tabRight =
                ArchaeologyFrame.tab1.factionIcon:GetTexCoord()
            local frameLeft, frameTop, _, frameBottom, frameRight =
                ArchaeologyFrame.factionIcon:GetTexCoord()

            return tabLeft, tabRight, tabTop, tabBottom,
                   frameLeft, frameRight, frameTop, frameBottom
            "#,
        )
        .expect("ArchaeologyFrame faction icon tex-coord probe must run cleanly");

    FactionIconProbe {
        tab_icon: TexCoords(coords.0, coords.1, coords.2, coords.3),
        frame_icon: TexCoords(coords.4, coords.5, coords.6, coords.7),
    }
}

fn assert_faction_icon_coords(
    probe: &FactionIconProbe,
    expected_tab: TexCoords,
    expected_frame: TexCoords,
    faction: &str,
) {
    assert_tex_coords(
        probe.tab_icon,
        expected_tab,
        &format!("{faction} summary-tab faction icon"),
    );
    assert_tex_coords(
        probe.frame_icon,
        expected_frame,
        &format!("{faction} main faction icon"),
    );
}

fn assert_tex_coords(actual: TexCoords, expected: TexCoords, label: &str) {
    assert_close(actual.0, expected.0, label, "left");
    assert_close(actual.1, expected.1, label, "right");
    assert_close(actual.2, expected.2, label, "top");
    assert_close(actual.3, expected.3, label, "bottom");
}

fn assert_close(actual: f64, expected: f64, label: &str, component: &str) {
    let delta = (actual - expected).abs();
    assert!(
        delta < 0.00001,
        "{label} {component} tex-coord must be {expected}, got {actual}"
    );
}
