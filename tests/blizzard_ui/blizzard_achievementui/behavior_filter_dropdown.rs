//! Current retail contract for the AchievementFrame filter dropdown.
//!
//! `AchievementFrame_HideFilterDropdown` always hides
//! `self.HeaderDetails.Filters.FilterDropdown` and lays out its `Filters`
//! container. `AchievementFrame_TryShowFilterDropdown` performs the same
//! layout only when `restrictedCategoryID` is nil.

use crate::common::blizzard_addon_harness::with_blizzard_addon_smoke_shape;

const ROOT: &str = "Blizzard_AchievementUI";

const FAKE_FRAME_BUILDER: &str = r#"
    local function increment(captures, key)
        return function() captures[key] = (captures[key] or 0) + 1 end
    end

    local function build_frame(captures)
        return {
            HeaderDetails = {
                Filters = {
                    FilterDropdown = {
                        Show = increment(captures, "show"),
                        Hide = increment(captures, "hide"),
                    },
                    Layout = increment(captures, "layout"),
                },
            },
        }
    end

    local function signature(captures)
        return string.format("show=%d hide=%d layout=%d",
            captures.show or 0, captures.hide or 0, captures.layout or 0)
    end
"#;

#[test]
fn try_show_filter_dropdown_gates_dropdown_and_layout_on_restricted_category() {
    with_blizzard_addon_smoke_shape(&[ROOT], &[], |env, _loaded| {
        let observations: String = env
            .eval(&format!(
                r#"
                assert(type(AchievementFrame_TryShowFilterDropdown) == "function")
                {FAKE_FRAME_BUILDER}

                local unrestricted = {{}}
                local unrestricted_frame = build_frame(unrestricted)
                AchievementFrame_TryShowFilterDropdown(unrestricted_frame)

                local restricted = {{}}
                local restricted_frame = build_frame(restricted)
                restricted_frame.restrictedCategoryID = 0
                AchievementFrame_TryShowFilterDropdown(restricted_frame)

                return string.format("unrestricted[%s] restricted[%s]",
                    signature(unrestricted), signature(restricted))
                "#
            ))
            .expect("TryShowFilterDropdown must drive HeaderDetails.Filters");

        assert_eq!(
            observations,
            "unrestricted[show=1 hide=0 layout=1] restricted[show=0 hide=0 layout=0]",
            "Retail TryShowFilterDropdown shows HeaderDetails.Filters.FilterDropdown and calls \
             HeaderDetails.Filters:Layout() only when restrictedCategoryID is nil."
        );
    });
}

#[test]
fn hide_filter_dropdown_hides_dropdown_and_lays_out_filters_unconditionally() {
    with_blizzard_addon_smoke_shape(&[ROOT], &[], |env, _loaded| {
        let observations: String = env
            .eval(&format!(
                r#"
                assert(type(AchievementFrame_HideFilterDropdown) == "function")
                {FAKE_FRAME_BUILDER}

                local captures = {{}}
                local frame = build_frame(captures)
                frame.restrictedCategoryID = 7
                AchievementFrame_HideFilterDropdown(frame)
                return signature(captures)
                "#
            ))
            .expect("HideFilterDropdown must drive HeaderDetails.Filters");

        assert_eq!(
            observations,
            "show=0 hide=1 layout=1",
            "Retail HideFilterDropdown always hides HeaderDetails.Filters.FilterDropdown and \
             calls HeaderDetails.Filters:Layout(), independent of restrictedCategoryID."
        );
    });
}
