use crate::common;

use std::path::PathBuf;
use std::time::{Duration, Instant};

use wow_ui_sim::loader::{discover_blizzard_addons_for_screen, load_addon};
use wow_ui_sim::lua_api::WowLuaEnv;
use wow_ui_sim::screen::ScreenKind;
use wow_ui_sim::startup::settle_headless_startup;

const TALENT_PANEL_VISIBLE_ONUPDATE_ADDED_CEILING: usize = 192;
const TALENT_PANEL_ONUPDATE_AVG_BUDGET: Duration = Duration::from_millis(35);
const TALENT_PANEL_ONUPDATE_MAX_BUDGET: Duration = Duration::from_millis(75);
const TALENT_PANEL_ONUPDATE_DELTA_BUDGET: Duration = Duration::from_millis(20);
const BASELINE_TICKS: usize = 20;
const TALENT_STEADY_TICKS: usize = 24;
const TALENT_SETTLE_TICKS: usize = 10;

fn blizzard_ui_dir() -> PathBuf {
    wow_ui_sim::client_profile::blizzard_ui_addons_dir_under(std::path::Path::new(env!(
        "CARGO_MANIFEST_DIR"
    )))
}

fn load_settled_game_ui() -> WowLuaEnv {
    let env = WowLuaEnv::new().expect("Failed to create Lua environment");
    env.set_screen_size(1024.0, 768.0);
    env.set_screen_mode(ScreenKind::Game);
    env.state().borrow_mut().addon_base_paths = vec![blizzard_ui_dir()];

    let addons = discover_blizzard_addons_for_screen(&blizzard_ui_dir(), ScreenKind::Game);
    for (name, toc_path) in &addons {
        load_addon(&env.loader_env(), toc_path)
            .unwrap_or_else(|err| panic!("Failed to load Blizzard addon {name}: {err}"));
    }

    env.apply_post_load_workarounds();
    settle_headless_startup(&env);
    env
}

fn open_talent_panel(env: &WowLuaEnv) {
    env.exec("PlayerSpellsUtil.ToggleClassTalentFrame()")
        .expect("Talent panel toggle should succeed");
    let shown: bool = env
        .eval(
            "return PlayerSpellsFrame ~= nil and PlayerSpellsFrame:IsShown() and PlayerSpellsFrame.TalentsFrame ~= nil and PlayerSpellsFrame.TalentsFrame:IsShown()",
        )
        .expect("Talent panel visibility query should succeed");
    assert!(shown, "Talent panel should be visible after opening");
}

fn capture_visible_onupdate(env: &WowLuaEnv) -> (usize, Vec<String>) {
    let state = env.state().borrow();
    let visible_ids = state.visible_on_update_cache.clone().unwrap_or_default();
    let names = visible_ids
        .iter()
        .map(|id| {
            state
                .widgets
                .get(*id)
                .and_then(|frame| frame.name.clone().or_else(|| frame.parent_key.clone()))
                .unwrap_or_else(|| format!("id:{id}"))
        })
        .collect::<Vec<_>>();
    (visible_ids.len(), names)
}

fn sample_on_update_ticks(env: &WowLuaEnv, samples: usize) -> Vec<Duration> {
    let mut durations = Vec::with_capacity(samples);
    for _ in 0..samples {
        durations.push(run_headless_tick(env));
    }
    durations
}

fn run_headless_tick(env: &WowLuaEnv) -> Duration {
    let started = Instant::now();
    env.process_timers().unwrap();
    env.state().borrow_mut().ensure_layout_rects();
    env.fire_on_update(0.016).unwrap();
    started.elapsed()
}

fn mean_duration(samples: &[Duration]) -> Duration {
    let total = samples
        .iter()
        .copied()
        .fold(Duration::ZERO, |sum, elapsed| sum + elapsed);
    total / u32::try_from(samples.len()).expect("sample set should fit u32")
}

fn max_duration(samples: &[Duration]) -> Duration {
    samples
        .iter()
        .copied()
        .max()
        .expect("sample set should not be empty")
}

#[test]
fn talent_panel_open_visible_onupdate_handlers_stay_within_ceiling() {
    perf_test_timeout! {
            let env = load_settled_game_ui();
            run_headless_tick(&env);
            let before_count = capture_visible_onupdate(&env).0;

            open_talent_panel(&env);
            for _ in 0..TALENT_SETTLE_TICKS {
                run_headless_tick(&env);
            }

            let (after_count, visible_names) = capture_visible_onupdate(&env);
            let added_count = after_count.saturating_sub(before_count);

            assert!(
                added_count <= TALENT_PANEL_VISIBLE_ONUPDATE_ADDED_CEILING,
                "talent-panel open should add at most {} visible OnUpdate handlers over settled baseline (before {}, after {}, added {}): {:?}",
                TALENT_PANEL_VISIBLE_ONUPDATE_ADDED_CEILING,
                before_count,
                after_count,
                added_count,
                visible_names
            );
    }
}

#[test]
fn talent_panel_open_onupdate_tick_latency_stays_under_budget() {
    perf_test_timeout! {
            let env = load_settled_game_ui();
            let baseline_samples = sample_on_update_ticks(&env, BASELINE_TICKS);
            let baseline_avg = mean_duration(&baseline_samples);

            open_talent_panel(&env);
            for _ in 0..TALENT_SETTLE_TICKS {
                run_headless_tick(&env);
            }

            let talent_samples = sample_on_update_ticks(&env, TALENT_STEADY_TICKS);
            let talent_avg = mean_duration(&talent_samples);
            let talent_max = max_duration(&talent_samples);

            eprintln!(
                "talent panel OnUpdate baseline avg={:.2?}, talent avg={:.2?}, talent max={:.2?} (budgets avg<{:.2?}, max<{:.2?}, delta<{:.2?})",
                baseline_avg,
                talent_avg,
                talent_max,
                TALENT_PANEL_ONUPDATE_AVG_BUDGET,
                TALENT_PANEL_ONUPDATE_MAX_BUDGET,
                TALENT_PANEL_ONUPDATE_DELTA_BUDGET
            );

            assert!(
                talent_avg <= TALENT_PANEL_ONUPDATE_AVG_BUDGET,
                "talent-panel open average OnUpdate tick took {:.2?}, exceeding budget {:.2?}",
                talent_avg,
                TALENT_PANEL_ONUPDATE_AVG_BUDGET
            );
            assert!(
                talent_max <= TALENT_PANEL_ONUPDATE_MAX_BUDGET,
                "talent-panel open max OnUpdate tick took {:.2?}, exceeding budget {:.2?}",
                talent_max,
                TALENT_PANEL_ONUPDATE_MAX_BUDGET
            );
            assert!(
                talent_avg <= baseline_avg + TALENT_PANEL_ONUPDATE_DELTA_BUDGET,
                "talent-panel open average OnUpdate tick regressed too far over baseline (baseline {:.2?}, talent {:.2?}, delta budget {:.2?})",
                baseline_avg,
                talent_avg,
                TALENT_PANEL_ONUPDATE_DELTA_BUDGET
            );
    }
}
