use crate::common;

use common::blizzard_addon_harness::with_blizzard_addon_startup_shape;
use common::panel_fixtures::clear_recorded_lua_errors;

const ROOT: &str = "Blizzard_FrameXML";

#[test]
fn blizzard_loot_history_loads_and_shows_empty_state_without_lua_errors() {
    common::with_perf_lock(|| {
        common::with_timeout(240, || {
            with_blizzard_addon_startup_shape(&[ROOT], &[], |env, loaded| {
                assert!(
                    loaded.iter().any(|addon| addon == ROOT),
                    "startup closure must include the real `{ROOT}` addon from the active cache"
                );

                let (frame_exists, frame_shown): (bool, bool) = env
                    .eval(
                        r#"
                        return GroupLootHistoryFrame ~= nil,
                               GroupLootHistoryFrame and GroupLootHistoryFrame:IsShown() or false
                        "#,
                    )
                    .expect("GroupLootHistoryFrame startup-state probe must run cleanly");
                assert!(frame_exists, "GroupLootHistoryFrame must exist after startup");
                assert!(
                    !frame_shown,
                    "GroupLootHistoryFrame must start hidden before its first Show()"
                );

                clear_recorded_lua_errors(env);
                env.exec("GroupLootHistoryFrame:Show()")
                    .expect("GroupLootHistoryFrame:Show() dispatch must return cleanly");

                let (frame_shown, no_info_shown): (bool, bool) = env
                    .eval(
                        r#"
                        return GroupLootHistoryFrame:IsShown(),
                               GroupLootHistoryFrame.NoInfoString:IsShown()
                        "#,
                    )
                    .expect("GroupLootHistoryFrame shown-state probe must run cleanly");
                assert!(frame_shown, "GroupLootHistoryFrame must be shown after Show()");
                assert!(
                    no_info_shown,
                    "GroupLootHistoryFrame.NoInfoString must be shown for empty loot history"
                );

                let (errors, records, counts) = {
                    let state = env.state().borrow();
                    (
                        state.lua_errors.clone(),
                        state
                            .lua_error_records
                            .iter()
                            .map(|record| record.message.clone())
                            .collect::<Vec<_>>(),
                        state.lua_error_counts.clone(),
                    )
                };
                assert!(
                    errors.is_empty() && records.is_empty() && counts.is_empty(),
                    "GroupLootHistoryFrame:Show() must record no Lua errors; protected handler \
                     dispatch must not hide an OnShow crash. lua_errors:\n  {}\nlua_error_records:\n  {}\n\
                     lua_error_counts: {counts:?}",
                    errors.join("\n  "),
                    records.join("\n  ")
                );
            });
        });
    });
}
