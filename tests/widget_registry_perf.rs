#![cfg(feature = "gui")]

use crate::common;
use crate::perf_base_game;
use crate::perf_game_ui;
use crate::perf_widget_registry;

use perf_game_ui::load_timed_game_ui;
use perf_widget_registry::snapshot_widget_registry;

const WIDGET_REGISTRY_FRAME_COUNT_BUDGET: usize = 52_000;
const WIDGET_REGISTRY_STORAGE_BUDGET_BYTES: usize = 230_000_000;

#[test]
fn settled_game_ui_widget_registry_stays_within_size_budget() {
    perf_test_timeout! {
        let loaded_ui = load_timed_game_ui();
        assert!(
            loaded_ui.startup_elapsed.as_nanos() > 0,
            "shared perf UI startup timing should be captured"
        );

        let snapshot = {
            let state = loaded_ui.env.state().borrow();
            snapshot_widget_registry(&state.widgets)
        };
        eprintln!(
            "widget registry baseline: {} frames, {} bytes",
            snapshot.frame_count,
            snapshot.storage_estimate_bytes
        );

        assert!(
            snapshot.frame_count < WIDGET_REGISTRY_FRAME_COUNT_BUDGET,
            "widget registry frame count {} exceeds budget {}",
            snapshot.frame_count,
            WIDGET_REGISTRY_FRAME_COUNT_BUDGET
        );
        assert!(
            snapshot.storage_estimate_bytes < WIDGET_REGISTRY_STORAGE_BUDGET_BYTES,
            "widget registry storage estimate {} exceeds budget {}",
            snapshot.storage_estimate_bytes,
            WIDGET_REGISTRY_STORAGE_BUDGET_BYTES
        );
    }
}
