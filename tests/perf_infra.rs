#![cfg(feature = "gui")]

use crate::common;
use crate::perf_base_game;
use crate::perf_cases;
use crate::perf_game_ui;

use perf_cases::{PerfCase, run_game_ui_cases};

#[test]
fn shared_perf_cases_reuse_one_loaded_game_env() {
    perf_test_timeout! {
        let results = run_game_ui_cases(vec![
            PerfCase::new("seed shared state", |env| {
                let has_ui_parent: bool = env.eval("return UIParent ~= nil").unwrap();
                assert!(has_ui_parent, "shared perf env should load the game UI");

                env.exec("_G.__perf_shared_marker = (_G.__perf_shared_marker or 0) + 1")
                    .unwrap();

                env.eval::<i32>("return __perf_shared_marker").unwrap()
            }),
            PerfCase::new("read shared state", |env| {
                env.eval::<i32>("return __perf_shared_marker or 0").unwrap()
            }),
        ]);

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].name, "seed shared state");
        assert_eq!(results[0].output, 1);
        assert_eq!(results[1].name, "read shared state");
        assert!(
            results.iter().all(|result| result.elapsed.as_nanos() > 0),
            "perf cases should capture non-zero elapsed timings"
        );
        assert_eq!(
            results[1].output, 1,
            "second perf case should observe the shared Lua state from the first case"
        );
    }
}
