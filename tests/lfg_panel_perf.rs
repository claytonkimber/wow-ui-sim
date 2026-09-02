#![cfg(feature = "gui")]

use crate::common;

use crate::perf_base_game;

use std::time::Duration;
use std::time::Instant;

use wow_ui_sim::iced_app::benchmark_lfg_panel_open_in_gui;
use wow_ui_sim::loader::{discover_blizzard_addons_for_screen, load_addon};
use wow_ui_sim::lua_api::WowLuaEnv;
use wow_ui_sim::screen::ScreenKind;

const LFG_FIRST_OPEN_SETTLE_BUDGET: Duration = Duration::from_secs(12);
const LFG_FIRST_OPEN_DRAW_BUDGET: Duration = Duration::from_secs(10);
const LFG_SECOND_OPEN_SETTLE_BUDGET: Duration = Duration::from_secs(5);
const LFG_SETTLE_FRAME_BUDGET: usize = 256;

struct LoadedBenchmarkUi {
    env: WowLuaEnv,
    startup_elapsed: Duration,
}

fn load_benchmark_ui() -> LoadedBenchmarkUi {
    let started = Instant::now();
    let env = perf_base_game::new_game_env();
    load_game_ui_addons(&env);
    env.apply_post_load_workarounds();

    LoadedBenchmarkUi {
        env,
        startup_elapsed: started.elapsed(),
    }
}

fn load_game_ui_addons(env: &WowLuaEnv) {
    let addons =
        discover_blizzard_addons_for_screen(&perf_base_game::blizzard_ui_dir(), ScreenKind::Game);
    for (name, toc_path) in &addons {
        load_addon(&env.loader_env(), toc_path)
            .unwrap_or_else(|err| panic!("Failed to load Blizzard addon {name}: {err}"));
    }
}

#[test]
fn lfg_panel_open_gui_latency_stays_under_budget() {
    perf_test_timeout! {
            let loaded = load_benchmark_ui();
            let report = benchmark_lfg_panel_open_in_gui(loaded.env)
                .expect("LFG panel GUI benchmark should complete");

            eprintln!("startup load: {:?}", loaded.startup_elapsed);
            eprintln!("first open: {:?}", report.first_open);
            eprintln!("first close: {:?}", report.first_close);
            eprintln!("second open: {:?}", report.second_open);

            assert!(
                report.first_open.settle_elapsed <= LFG_FIRST_OPEN_SETTLE_BUDGET,
                "first LFG open should settle within {:?}, got {:?}",
                LFG_FIRST_OPEN_SETTLE_BUDGET,
                report.first_open.settle_elapsed
            );
            assert!(
                report.first_open.draw_elapsed <= LFG_FIRST_OPEN_DRAW_BUDGET,
                "first LFG open draw work should stay within {:?}, got {:?}",
                LFG_FIRST_OPEN_DRAW_BUDGET,
                report.first_open.draw_elapsed
            );
            assert!(
                report.second_open.settle_elapsed <= LFG_SECOND_OPEN_SETTLE_BUDGET,
                "second LFG open should settle within {:?}, got {:?}",
                LFG_SECOND_OPEN_SETTLE_BUDGET,
                report.second_open.settle_elapsed
            );
            assert!(
                report.first_open.frames <= LFG_SETTLE_FRAME_BUDGET,
                "first LFG open should settle within {LFG_SETTLE_FRAME_BUDGET} frames, got {}",
                report.first_open.frames
            );
    }
}
