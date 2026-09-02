#![cfg(feature = "gui")]

use crate::common;
use crate::perf_base_game;

use std::time::Duration;

use wow_ui_sim::loader::{discover_blizzard_addons_for_screen, load_addon};
use wow_ui_sim::lua_api::WowLuaEnv;
use wow_ui_sim::screen::ScreenKind;
use wow_ui_sim::startup::fire_startup_events_for_screen;

const FULL_GAME_STARTUP_BUDGET: Duration = Duration::from_secs(20);

struct StartupPhaseTimings {
    addon_load_elapsed: Duration,
    post_load_workarounds_elapsed: Duration,
    startup_events_elapsed: Duration,
}

struct LoadedGameUi {
    env: WowLuaEnv,
    phase_timings: StartupPhaseTimings,
    startup_elapsed: Duration,
}

fn load_timed_game_ui() -> LoadedGameUi {
    let started = std::time::Instant::now();
    let env = perf_base_game::new_game_env();
    let addon_load_elapsed = load_game_ui_addons(&env);
    let post_load_workarounds_elapsed = apply_post_load_workarounds(&env);
    let startup_events_elapsed = fire_startup_events(&env);

    LoadedGameUi {
        env,
        phase_timings: StartupPhaseTimings {
            addon_load_elapsed,
            post_load_workarounds_elapsed,
            startup_events_elapsed,
        },
        startup_elapsed: started.elapsed(),
    }
}

fn fire_startup_events(env: &WowLuaEnv) -> Duration {
    let started = std::time::Instant::now();
    fire_startup_events_for_screen(env, ScreenKind::Game);
    started.elapsed()
}

fn apply_post_load_workarounds(env: &WowLuaEnv) -> Duration {
    let started = std::time::Instant::now();
    env.apply_post_load_workarounds();
    started.elapsed()
}

fn load_game_ui_addons(env: &WowLuaEnv) -> Duration {
    let started = std::time::Instant::now();
    let addons =
        discover_blizzard_addons_for_screen(&perf_base_game::blizzard_ui_dir(), ScreenKind::Game);
    for (name, toc_path) in &addons {
        load_addon(&env.loader_env(), toc_path)
            .unwrap_or_else(|err| panic!("Failed to load Blizzard addon {name}: {err}"));
    }
    started.elapsed()
}

#[test]
fn full_game_startup_stays_under_budget() {
    perf_test_timeout! {
            let loaded_ui = load_timed_game_ui();
            let env = &loaded_ui.env;
            let phase_timings = &loaded_ui.phase_timings;
            let startup_elapsed = loaded_ui.startup_elapsed;

            let startup_ready: bool = env
                .eval("return UIParent ~= nil and PlayerFrame ~= nil and IsLoggedIn()")
                .unwrap();
            assert!(
                startup_ready,
                "timed startup should produce a settled logged-in game UI"
            );

            eprintln!(
                "full game startup baseline: {:.2?} (addons {:.2?}, post-load {:.2?}, startup events {:.2?}; budget {:.2?})",
                startup_elapsed,
                phase_timings.addon_load_elapsed,
                phase_timings.post_load_workarounds_elapsed,
                phase_timings.startup_events_elapsed,
                FULL_GAME_STARTUP_BUDGET
            );

            assert!(
                startup_elapsed < FULL_GAME_STARTUP_BUDGET,
                "full game startup took {:.2?}, exceeding budget {:.2?}",
                startup_elapsed,
                FULL_GAME_STARTUP_BUDGET
            );
    }
}
