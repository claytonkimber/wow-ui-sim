#![cfg(feature = "gui")]

use crate::common;
use crate::perf_addon_loading;
use crate::perf_base_game;

use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;

use perf_addon_loading::{
    PerAddonLoadTiming, load_timed_game_addons_with_saved_vars,
    load_timed_game_addons_without_saved_vars,
};

const ADDON_LOADING_BUDGET: Duration = Duration::from_secs(25);
const XML_PARSE_BUDGET: Duration = Duration::from_millis(2500);
const LUA_COMPILE_BUDGET: Duration = Duration::from_millis(2000);
const LIFECYCLE_BUDGET: Duration = Duration::from_millis(7000);
const NO_SAVED_VARS_FRAME_SETUP_BUDGET: Duration = Duration::from_secs(10);
const NO_SAVED_VARS_FRAME_FINALIZE_BUDGET: Duration = Duration::from_secs(20);
const NO_SAVED_VARS_EXEC_LUA_BUDGET: Duration = Duration::from_secs(9);
const HEAVIEST_ADDON_COUNT: usize = 8;
const MIN_HEAVY_ADDON_ALLOWED_REGRESSION: Duration = Duration::from_millis(400);
const NEW_HEAVY_ADDON_OUTLIER_THRESHOLD: Duration = Duration::from_secs(1);
const KNOWN_HEAVY_ADDON_BUDGETS: &[(&str, Duration)] = &[
    ("Blizzard_ActionBar", Duration::from_millis(7700)),
    ("Blizzard_UIPanels_Game", Duration::from_millis(2700)),
    ("Blizzard_Communities", Duration::from_millis(1200)),
    ("Blizzard_UnitFrame", Duration::from_millis(1200)),
    ("Blizzard_FrameXML", Duration::from_millis(900)),
    ("Blizzard_GroupFinder", Duration::from_millis(700)),
    ("Blizzard_CompactRaidFrames", Duration::from_millis(800)),
];

#[test]
fn blizzard_addon_loading_reports_phase_breakdown_under_budget() {
    perf_test_timeout! {
            let loaded = load_timed_game_addons_with_saved_vars();
            let env = &loaded.env;
            let timing = &loaded.addon_timing;

            let addon_surface_ready: bool = env
                .eval("return UIParent ~= nil and PlayerFrame ~= nil and type(IsLoggedIn) == 'function'")
                .unwrap();
            assert!(
                addon_surface_ready,
                "timed addon loading should produce a real Blizzard game UI surface"
            );
            assert!(loaded.addon_count > 0, "expected Blizzard addons to be discovered");
            assert!(
                timing.xml_parse_time > Duration::ZERO,
                "xml parse timing should be non-zero across Blizzard addons"
            );
            assert!(
                timing.lua_compile_time > Duration::ZERO,
                "lua compile timing should be non-zero across Blizzard addons"
            );
            assert!(
                timing.lua_call_time > Duration::ZERO,
                "lua call timing should be non-zero across Blizzard addons"
            );
            assert!(
                timing.saved_vars_time > Duration::ZERO,
                "saved variables timing should be non-zero when loading through SavedVariablesManager"
            );
            assert_eq!(
                timing.lua_exec_time,
                timing.lua_compile_time + timing.lua_call_time,
                "lua exec timing should equal the sum of compile and call phases"
            );

            eprintln!(
                "blizzard addon loading baseline: {:.2?} total across {} addons (xml parse {:.2?}, lua compile {:.2?}, lua call {:.2?}, saved vars {:.2?}; budget {:.2?})",
                loaded.addon_elapsed,
                loaded.addon_count,
                timing.xml_parse_time,
                timing.lua_compile_time,
                timing.lua_call_time,
                timing.saved_vars_time,
                ADDON_LOADING_BUDGET
            );

            assert!(
                loaded.addon_elapsed < ADDON_LOADING_BUDGET,
                "blizzard addon loading took {:.2?}, exceeding budget {:.2?}",
                loaded.addon_elapsed,
                ADDON_LOADING_BUDGET
            );
    }
}

#[test]
fn loading_phase_breakdown_stays_within_budgets() {
    perf_test_timeout! {
            let loaded = load_timed_game_addons_with_saved_vars();
            let timing = &loaded.addon_timing;

            eprintln!(
                "loading phases: xml_parse={:.2?} lua_compile={:.2?} lifecycle={:.2?} layers={:.2?} frames={}",
                timing.xml_parse_time,
                timing.lua_compile_time,
                timing.frame_lifecycle_time,
                timing.frame_layer_children_time,
                timing.frame_count,
            );

            assert_under_budget("XML parse", timing.xml_parse_time, XML_PARSE_BUDGET);
            assert_under_budget("Lua compile", timing.lua_compile_time, LUA_COMPILE_BUDGET);
            assert_under_budget(
                "lifecycle scripts",
                timing.frame_lifecycle_time,
                LIFECYCLE_BUDGET,
            );
    }
}

#[test]
fn no_saved_vars_loader_hotspots_stay_within_budgets() {
    perf_test_timeout! {
            let loaded = load_timed_game_addons_without_saved_vars();
            let timing = &loaded.addon_timing;

            eprintln!(
                "no-saved-vars loader hotspots: setup={:.2?} finalize={:.2?} exec_lua={:.2?} lifecycle={:.2?}",
                timing.xml_frame_setup_time,
                timing.xml_frame_finalize_time,
                timing.frame_exec_lua_time,
                timing.frame_lifecycle_time,
            );

            assert_under_budget(
                "frame setup",
                timing.xml_frame_setup_time,
                NO_SAVED_VARS_FRAME_SETUP_BUDGET,
            );
            assert_under_budget(
                "frame finalize",
                timing.xml_frame_finalize_time,
                NO_SAVED_VARS_FRAME_FINALIZE_BUDGET,
            );
            assert_under_budget(
                "frame exec_lua",
                timing.frame_exec_lua_time,
                NO_SAVED_VARS_EXEC_LUA_BUDGET,
            );
    }
}

#[test]
fn heaviest_blizzard_addons_stay_within_per_addon_load_budgets() {
    perf_test_timeout! {
            let loaded = load_timed_game_addons_with_saved_vars();
            let heaviest = heaviest_addons(&loaded.per_addon_timings, HEAVIEST_ADDON_COUNT);
            assert_eq!(
                heaviest.len(),
                HEAVIEST_ADDON_COUNT,
                "expected at least {HEAVIEST_ADDON_COUNT} per-addon timing samples, got {}",
                heaviest.len()
            );

            let heaviest_report = format_heaviest_addon_report(&heaviest);
            eprintln!("heaviest Blizzard addon loads: {heaviest_report}");

            let timing_by_name = per_addon_timing_map(&loaded.per_addon_timings);
            let regressions = collect_heavy_addon_regressions(&heaviest, &timing_by_name);

            assert!(
                regressions.is_empty(),
                "heaviest Blizzard addon loading regressed:\n{}\nheaviest: [{}]",
                regressions.join("\n"),
                heaviest_report
            );
    }
}

fn format_heaviest_addon_report(heaviest: &[PerAddonLoadTiming]) -> String {
    heaviest
        .iter()
        .map(|timing| format!("{}={:.2?}", timing.name, timing.total_time))
        .collect::<Vec<_>>()
        .join(", ")
}

fn per_addon_timing_map(per_addon_timings: &[PerAddonLoadTiming]) -> BTreeMap<&str, Duration> {
    per_addon_timings
        .iter()
        .map(|timing| (timing.name.as_str(), timing.total_time))
        .collect()
}

fn collect_heavy_addon_regressions(
    heaviest: &[PerAddonLoadTiming],
    timing_by_name: &BTreeMap<&str, Duration>,
) -> Vec<String> {
    let mut regressions = collect_known_heavy_addon_budget_regressions(timing_by_name);
    regressions.extend(collect_new_heavy_addon_outliers(heaviest));
    regressions
}

fn collect_known_heavy_addon_budget_regressions(
    timing_by_name: &BTreeMap<&str, Duration>,
) -> Vec<String> {
    let mut regressions = Vec::new();
    for (name, budget) in KNOWN_HEAVY_ADDON_BUDGETS {
        let Some(total_time) = timing_by_name.get(name).copied() else {
            regressions.push(format!("missing per-addon timing sample for {name}"));
            continue;
        };

        let allowance = heavy_addon_allowed_regression(*budget);
        let limit = *budget + allowance;
        if total_time > limit {
            regressions.push(format!(
                "{} took {:.2?}, exceeding budget {:.2?} + regression allowance {:.2?}",
                name, total_time, budget, allowance
            ));
        }
    }
    regressions
}

fn collect_new_heavy_addon_outliers(heaviest: &[PerAddonLoadTiming]) -> Vec<String> {
    let known_heavy_addons: BTreeSet<&str> = KNOWN_HEAVY_ADDON_BUDGETS
        .iter()
        .map(|(name, _)| *name)
        .collect();
    heaviest
        .iter()
        .filter(|timing| !known_heavy_addons.contains(timing.name.as_str()))
        .filter(|timing| timing.total_time > NEW_HEAVY_ADDON_OUTLIER_THRESHOLD)
        .map(|timing| {
            format!(
                "{} became a new heavy-addon outlier at {:.2?}; add an explicit budget entry",
                timing.name, timing.total_time
            )
        })
        .collect()
}

fn heaviest_addons(
    per_addon_timings: &[PerAddonLoadTiming],
    count: usize,
) -> Vec<PerAddonLoadTiming> {
    let mut sorted = per_addon_timings.to_vec();
    sorted.sort_by(|left, right| {
        right
            .total_time
            .cmp(&left.total_time)
            .then_with(|| left.name.cmp(&right.name))
    });
    sorted.truncate(count);
    sorted
}

fn heavy_addon_allowed_regression(budget: Duration) -> Duration {
    let scaled = Duration::from_secs_f64(budget.as_secs_f64() * 0.2);
    scaled.max(MIN_HEAVY_ADDON_ALLOWED_REGRESSION)
}

fn assert_under_budget(label: &str, actual: Duration, budget: Duration) {
    assert!(
        actual < budget,
        "{label} took {actual:.2?}, exceeding budget {budget:.2?}"
    );
}
