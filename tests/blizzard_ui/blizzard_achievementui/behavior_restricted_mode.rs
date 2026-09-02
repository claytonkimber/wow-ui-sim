//! Current retail contract for `AchievementFrame_SetRestrictedMode`.

use crate::common::blizzard_addon_harness::with_blizzard_addon_smoke_shape;

const ROOT: &str = "Blizzard_AchievementUI";
const RESTRICTED_CATEGORY_ID: i64 = 7;

const FAKE_FRAME_BUILDER: &str = r#"
    local function increment(captures, key)
        return function() captures[key] = (captures[key] or 0) + 1 end
    end

    local function set_shown(captures, key)
        return function(_, shown)
            captures[key] = (captures[key] or 0) + 1
            captures[key .. "_arg"] = tostring(shown)
        end
    end

    local function build_frame(captures)
        local search_box = {
            SetShown = set_shown(captures, "search_box_set_shown"),
            SearchProgressBar = {Hide = increment(captures, "progress_bar_hide")},
        }
        return {
            Header = {SetShown = set_shown(captures, "header_set_shown")},
            HeaderDetails = {Filters = {SearchBox = search_box}},
        }
    end

    local function signature(captures)
        return string.format(
            "category=%s header=%s search_box=%s progress=%d tabs=%s update=%s",
            tostring(captures.category),
            tostring(captures.header_set_shown_arg),
            tostring(captures.search_box_set_shown_arg),
            captures.progress_bar_hide or 0,
            tostring(captures.tabs_arg),
            tostring(captures.update_arg))
    end
"#;

#[test]
fn set_restricted_mode_uses_nested_search_box_and_forwards_category() {
    with_blizzard_addon_smoke_shape(&[ROOT], &[], |env, _loaded| {
        let observations: String = env
            .eval(&format!(
                r#"
                assert(type(AchievementFrame_SetRestrictedMode) == "function")
                {FAKE_FRAME_BUILDER}

                local function run(category)
                    local captures = {{}}
                    local frame = build_frame(captures)
                    PanelTemplates_SetAllTabsShown = function(_, shown)
                        captures.tabs_arg = tostring(shown)
                    end
                    AchievementFrame_UpdateAndSelectCategory = function(value)
                        captures.update_arg = tostring(value)
                    end
                    AchievementFrame_SetRestrictedMode(frame, category)
                    captures.category = frame.restrictedCategoryID
                    return signature(captures)
                end

                return string.format("restricted[%s] unrestricted[%s]",
                    run({RESTRICTED_CATEGORY_ID}), run(nil))
                "#
            ))
            .expect("SetRestrictedMode must use HeaderDetails.Filters.SearchBox");

        assert_eq!(
            observations,
            format!(
                "restricted[category={RESTRICTED_CATEGORY_ID} header=false search_box=false \
                 progress=1 tabs=false update={RESTRICTED_CATEGORY_ID}] \
                 unrestricted[category=nil header=true search_box=true progress=0 tabs=true update=nil]"
            ),
            "Retail SetRestrictedMode hides Header and HeaderDetails.Filters.SearchBox, hides \
             SearchBox.SearchProgressBar only in restricted mode, updates tab visibility, and \
             forwards the category."
        );
    });
}
