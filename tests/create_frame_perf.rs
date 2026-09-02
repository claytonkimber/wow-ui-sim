#![cfg(feature = "gui")]

use crate::common;
use crate::perf_create_frame;

use std::time::Duration;

use perf_create_frame::measure_create_frame_throughput;

const CREATE_FRAME_THROUGHPUT_BUDGET: Duration = Duration::from_millis(200);

#[test]
fn create_frame_throughput_for_1000_frames_stays_under_budget() {
    perf_test_timeout! {
        let elapsed = measure_create_frame_throughput();
        eprintln!(
            "CreateFrame throughput baseline: {:.2?} (budget {:.2?})",
            elapsed,
            CREATE_FRAME_THROUGHPUT_BUDGET
        );

        assert!(
            elapsed < CREATE_FRAME_THROUGHPUT_BUDGET,
            "CreateFrame throughput took {:.2?}, exceeding budget {:.2?}",
            elapsed,
            CREATE_FRAME_THROUGHPUT_BUDGET
        );
    }
}
