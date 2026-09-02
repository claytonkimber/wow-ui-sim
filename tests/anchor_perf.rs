#![cfg(feature = "gui")]

use crate::common;
use crate::perf_anchor;

use std::time::Duration;

use perf_anchor::measure_anchor_update_throughput;

const ANCHOR_UPDATE_THROUGHPUT_BUDGET: Duration = Duration::from_millis(100);

#[test]
fn anchor_update_throughput_for_1000_frames_stays_under_budget() {
    perf_test_timeout! {
        let elapsed = measure_anchor_update_throughput();
        eprintln!(
            "anchor update throughput baseline: {:.2?} (budget {:.2?})",
            elapsed,
            ANCHOR_UPDATE_THROUGHPUT_BUDGET
        );

        assert!(
            elapsed < ANCHOR_UPDATE_THROUGHPUT_BUDGET,
            "anchor update throughput took {:.2?}, exceeding budget {:.2?}",
            elapsed,
            ANCHOR_UPDATE_THROUGHPUT_BUDGET
        );
    }
}
