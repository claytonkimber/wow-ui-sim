#![cfg(feature = "gui")]

use crate::common;
use crate::perf_event_dispatch;

use std::time::Duration;

use perf_event_dispatch::measure_event_dispatch_throughput;

const EVENT_DISPATCH_THROUGHPUT_BUDGET: Duration = Duration::from_millis(10);

#[test]
fn event_dispatch_to_128_registered_frames_stays_under_budget() {
    perf_test_timeout! {
        let elapsed = measure_event_dispatch_throughput();
        eprintln!(
            "event dispatch throughput baseline: {:.2?} (budget {:.2?})",
            elapsed,
            EVENT_DISPATCH_THROUGHPUT_BUDGET
        );

        assert!(
            elapsed < EVENT_DISPATCH_THROUGHPUT_BUDGET,
            "event dispatch throughput took {:.2?}, exceeding budget {:.2?}",
            elapsed,
            EVENT_DISPATCH_THROUGHPUT_BUDGET
        );
    }
}
