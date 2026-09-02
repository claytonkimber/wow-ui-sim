#![cfg(feature = "gui")]

use crate::common;
use crate::perf_text;

use std::time::Duration;

use perf_text::measure_glyph_text_shaping_for_representative_strings;

const GLYPH_TEXT_SHAPING_BUDGET: Duration = Duration::from_millis(40);

#[test]
fn glyph_text_shaping_for_representative_strings_stays_under_budget() {
    perf_test_timeout! {
        let shaping_elapsed = measure_glyph_text_shaping_for_representative_strings();
        eprintln!(
            "glyph text shaping baseline: {:.2?} (budget {:.2?})",
            shaping_elapsed,
            GLYPH_TEXT_SHAPING_BUDGET
        );

        assert!(
            shaping_elapsed < GLYPH_TEXT_SHAPING_BUDGET,
            "glyph text shaping took {:.2?}, exceeding budget {:.2?}",
            shaping_elapsed,
            GLYPH_TEXT_SHAPING_BUDGET
        );
    }
}
