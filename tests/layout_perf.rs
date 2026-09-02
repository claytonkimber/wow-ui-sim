#![cfg(feature = "gui")]

use crate::common;
use crate::perf_base_game;
use crate::perf_game_ui;
use crate::perf_layout;

use std::time::Duration;

use perf_game_ui::load_timed_game_ui;
use perf_layout::{
    measure_dirty_tree_quad_rebuild, measure_full_quad_batch_build, measure_full_root_layout_pass,
    measure_incremental_anchor_change_layout_pass, measure_strata_bucket_rebuild,
    measure_tooltip_collect_and_quad_emission,
};

const FULL_ROOT_LAYOUT_BUDGET: Duration = Duration::from_millis(500);
const INCREMENTAL_LAYOUT_BUDGET: Duration = Duration::from_millis(15);
const STRATA_BUCKET_REBUILD_BUDGET: Duration = Duration::from_millis(50);
const QUAD_BATCH_BUILD_BUDGET: Duration = Duration::from_millis(100);
const DIRTY_TREE_QUAD_REBUILD_BUDGET: Duration = Duration::from_millis(5);
const TOOLTIP_RENDER_BUDGET: Duration = Duration::from_millis(25);

#[test]
fn full_root_layout_pass_stays_under_budget() {
    perf_test_timeout! {
        let loaded_ui = load_timed_game_ui();
        let env = &loaded_ui.env;
        assert!(
            loaded_ui.startup_elapsed.as_nanos() > 0,
            "shared perf UI startup timing should be captured"
        );

        let layout_elapsed = measure_full_root_layout_pass(env);
        eprintln!(
            "full root layout baseline: {:.2?} (budget {:.2?})",
            layout_elapsed,
            FULL_ROOT_LAYOUT_BUDGET
        );

        assert!(
            layout_elapsed < FULL_ROOT_LAYOUT_BUDGET,
            "full root layout took {:.2?}, exceeding budget {:.2?}",
            layout_elapsed,
            FULL_ROOT_LAYOUT_BUDGET
        );
    }
}

#[test]
fn incremental_anchor_change_layout_pass_stays_under_budget() {
    perf_test_timeout! {
        let loaded_ui = load_timed_game_ui();
        let env = &loaded_ui.env;
        assert!(
            loaded_ui.startup_elapsed.as_nanos() > 0,
            "shared perf UI startup timing should be captured"
        );

        let layout_elapsed = measure_incremental_anchor_change_layout_pass(env);
        eprintln!(
            "incremental anchor-change layout baseline: {:.2?} (budget {:.2?})",
            layout_elapsed,
            INCREMENTAL_LAYOUT_BUDGET
        );

        assert!(
            layout_elapsed < INCREMENTAL_LAYOUT_BUDGET,
            "incremental anchor-change layout took {:.2?}, exceeding budget {:.2?}",
            layout_elapsed,
            INCREMENTAL_LAYOUT_BUDGET
        );
    }
}

#[test]
fn strata_bucket_rebuild_stays_under_budget() {
    perf_test_timeout! {
        let loaded_ui = load_timed_game_ui();
        let env = &loaded_ui.env;
        assert!(
            loaded_ui.startup_elapsed.as_nanos() > 0,
            "shared perf UI startup timing should be captured"
        );

        let rebuild_elapsed = measure_strata_bucket_rebuild(env);
        eprintln!(
            "strata bucket rebuild baseline: {:.2?} (budget {:.2?})",
            rebuild_elapsed,
            STRATA_BUCKET_REBUILD_BUDGET
        );

        assert!(
            rebuild_elapsed < STRATA_BUCKET_REBUILD_BUDGET,
            "strata bucket rebuild took {:.2?}, exceeding budget {:.2?}",
            rebuild_elapsed,
            STRATA_BUCKET_REBUILD_BUDGET
        );
    }
}

#[test]
fn full_quad_batch_build_stays_under_budget() {
    perf_test_timeout! {
        let loaded_ui = load_timed_game_ui();
        let env = &loaded_ui.env;
        assert!(
            loaded_ui.startup_elapsed.as_nanos() > 0,
            "shared perf UI startup timing should be captured"
        );

        let quad_batch_elapsed = measure_full_quad_batch_build(env);
        eprintln!(
            "full quad batch baseline: {:.2?} (budget {:.2?})",
            quad_batch_elapsed,
            QUAD_BATCH_BUILD_BUDGET
        );

        assert!(
            quad_batch_elapsed < QUAD_BATCH_BUILD_BUDGET,
            "full quad batch build took {:.2?}, exceeding budget {:.2?}",
            quad_batch_elapsed,
            QUAD_BATCH_BUILD_BUDGET
        );
    }
}

#[test]
fn dirty_tree_quad_rebuild_stays_under_budget() {
    perf_test_timeout! {
        let loaded_ui = load_timed_game_ui();
        let env = &loaded_ui.env;
        assert!(
            loaded_ui.startup_elapsed.as_nanos() > 0,
            "shared perf UI startup timing should be captured"
        );

        let rebuild_elapsed = measure_dirty_tree_quad_rebuild(env);
        eprintln!(
            "dirty-tree quad rebuild baseline: {:.2?} (budget {:.2?})",
            rebuild_elapsed,
            DIRTY_TREE_QUAD_REBUILD_BUDGET
        );

        assert!(
            rebuild_elapsed < DIRTY_TREE_QUAD_REBUILD_BUDGET,
            "dirty-tree quad rebuild took {:.2?}, exceeding budget {:.2?}",
            rebuild_elapsed,
            DIRTY_TREE_QUAD_REBUILD_BUDGET
        );
    }
}

#[test]
fn tooltip_collect_and_quad_emission_stays_under_budget() {
    perf_test_timeout! {
        let loaded_ui = load_timed_game_ui();
        let env = &loaded_ui.env;
        assert!(
            loaded_ui.startup_elapsed.as_nanos() > 0,
            "shared perf UI startup timing should be captured"
        );

        let tooltip_elapsed = measure_tooltip_collect_and_quad_emission(env);
        eprintln!(
            "tooltip collect + quad emission baseline: {:.2?} (budget {:.2?})",
            tooltip_elapsed,
            TOOLTIP_RENDER_BUDGET
        );

        assert!(
            tooltip_elapsed < TOOLTIP_RENDER_BUDGET,
            "tooltip collect + quad emission took {:.2?}, exceeding budget {:.2?}",
            tooltip_elapsed,
            TOOLTIP_RENDER_BUDGET
        );
    }
}
