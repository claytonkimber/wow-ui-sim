//! Behavior pin: `AchievementFrameSearchProgressBar_OnUpdate(self, elapsed)`
//! reads `GetAchievementSearchProgress() / GetAchievementSearchSize()`
//! to compute a fill ratio, scales it by the bar's `maxValue`, and
//! writes the result via `:SetValue`. When the bar reaches `maxValue`,
//! it self-clears its `OnUpdate` script, resets to 0, and forwards to
//! `AchievementFrame_ShowSearchPreviewResults`.
//!
//! Source map:
//!
//! ```lua
//! -- lua:3318-3330 (the function under test)
//! function AchievementFrameSearchProgressBar_OnUpdate(self, elapsed)
//!     local _, maxValue = self:GetMinMaxValues();                                    -- line 3319
//!     local actualProgress =
//!         GetAchievementSearchProgress() / GetAchievementSearchSize() * maxValue;    -- line 3320
//!     local displayedProgress = self:GetValue();                                     -- line 3321 (DEAD)
//!
//!     self:SetValue(actualProgress);                                                 -- line 3323
//!
//!     if ( self:GetValue() >= maxValue ) then                                        -- line 3325
//!         self:SetScript("OnUpdate", nil);                                           -- line 3326
//!         self:SetValue(0);                                                          -- line 3327
//!         AchievementFrame_ShowSearchPreviewResults();                               -- line 3328
//!     end
//! end
//! ```
//!
//! ```lua
//! -- lua:3295-3296 (the install site, inside _SearchBox_OnUpdate)
//! if ( AchievementFrame.HeaderDetails.Filters.SearchBox.SearchProgressBar:GetScript("OnUpdate") == nil ) then
//!     AchievementFrame.HeaderDetails.Filters.SearchBox.SearchProgressBar:SetScript("OnUpdate",
//!         AchievementFrameSearchProgressBar_OnUpdate);
//!     ...
//! ```
//!
//! XML at xml:2501: `<StatusBar parentKey="searchProgressBar" hidden="false">`
//! anchored inside the `SearchPreviewContainer` (xml:2503-2504), with
//! a BACKGROUND `bg` Texture (xml:2508-2514) and an OVERLAY `Text`
//! FontString (xml:2517-2522).
//!
//! Simulator backing for the read globals lives at
//! `src/lua_api/globals/missing_surface/achievement_info.rs:878-887`
//! — `get_achievement_search_progress` returns
//! `state.achievement_search.progress` and `get_achievement_search_size`
//! returns `state.achievement_search.size`. Default state per
//! `state_types/collections.rs:295-308`: both fields are 0, so a
//! division-by-zero would yield NaN at lua:3320. The smoke harness
//! does not seed search state, so this test installs Lua-level spies
//! over `_G.GetAchievementSearchProgress` and `_G.GetAchievementSearchSize`
//! that read from `_G.__test_progress` and `_G.__test_size` — letting
//! the test set up two scenarios (partial fill, completion) without
//! touching Rust state.
//!
//! **PLAN's "drives the progress bar fill ratio" elides FOUR contract
//! details:**
//!
//! 1. **The ratio is scaled by `maxValue`**, not just the raw fraction.
//!    `actualProgress = progress / size * maxValue` at lua:3320 — a
//!    bar with `maxValue=100` and `progress/size=0.5` ends up at
//!    value 50, not 0.5.
//! 2. **The completion branch self-clears** `OnUpdate` at lua:3326 and
//!    resets value to 0 at lua:3327. This is the bar's exit condition;
//!    PLAN's wording would suggest the bar STAYS at maxValue.
//! 3. **`_ShowSearchPreviewResults` fires only on completion** at
//!    lua:3328, NOT on every tick. Coupling the search-results panel
//!    show to the partial-fill update would surface as the preview
//!    panel flashing on every tick.
//! 4. **`displayedProgress` at lua:3321 is dead code.** It is captured
//!    via `:GetValue()` then never read; the comparison at lua:3325
//!    re-reads `:GetValue()`. Likely a leftover from a smoothing /
//!    interpolation refactor; documented here so a reader does not
//!    chase a non-existent contract.
//!
//! Eight assertions split presence/behavior:
//!
//! - **Presence half** (5): `_G.AchievementFrameSearchProgressBar_OnUpdate`,
//!   `_G.GetAchievementSearchProgress`, `_G.GetAchievementSearchSize`,
//!   and `_G.AchievementFrame_ShowSearchPreviewResults` are all
//!   functions; `AchievementFrame.HeaderDetails.Filters.SearchBox.SearchProgressBar:GetObjectType() ==
//!   "StatusBar"` (the XML widget at xml:2501 resolved into a real
//!   StatusBar with `:GetMinMaxValues`/`:SetValue`/`:SetScript`).
//! - **Behavior partial** (1): driving `_OnUpdate(bar, 0)` with
//!   `progress=5`, `size=10`, `bar:SetMinMaxValues(0, 100)`, and
//!   `OnUpdate` pre-installed produces the signature
//!   `"value=50 onupdate=function show_called=0"` — proves the
//!   ratio math at lua:3320 (`5/10 * 100 == 50`), proves
//!   `:SetValue(50)` was applied at lua:3323, proves the completion
//!   branch at lua:3325 did NOT fire (50 < 100), proves
//!   `_ShowSearchPreviewResults` was NOT called.
//! - **Behavior completion** (1): driving `_OnUpdate(bar, 0)` again
//!   with `progress=10`, `size=10` (ratio = 1.0, scaled = 100, equals
//!   maxValue) produces the signature
//!   `"value=0 onupdate=nil show_called=1"` — proves the
//!   `>= maxValue` guard at lua:3325 fired, proves the OnUpdate clear
//!   at lua:3326, proves the value reset at lua:3327 (the value is 0,
//!   NOT the maxValue it briefly held), proves
//!   `_ShowSearchPreviewResults` was called exactly once.
//! - **Min/max round-trip** (1): `bar:GetMinMaxValues()` after
//!   `bar:SetMinMaxValues(0, 100)` returns `(0, 100)`. This pins the
//!   StatusBar's min/max read at lua:3319 — without it, the
//!   denominator scaling at lua:3320 would silently use whatever
//!   default the framework returns, hiding regressions where
//!   `SetMinMaxValues` doesn't actually persist its arguments.

use crate::common::blizzard_addon_harness::with_blizzard_addon_smoke_shape;

const ROOT: &str = "Blizzard_AchievementUI";
const EXPECTED_PARTIAL_SIGNATURE: &str = "value=50 onupdate=function show_called=0";
const EXPECTED_COMPLETION_SIGNATURE: &str = "value=0 onupdate=nil show_called=1";
const EXPECTED_MIN_MAX_SIGNATURE: &str = "min=0 max=100";

type SearchProgressProbe = (
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
);

#[test]
fn search_progress_bar_on_update_writes_ratio_times_max_then_self_clears_on_completion() {
    with_blizzard_addon_smoke_shape(&[ROOT], &[], |env, _loaded| {
        let observations: SearchProgressProbe = env
            .eval(
                r#"
                assert(AchievementFrame, "AchievementFrame must exist after addon load")
                assert(AchievementFrame.HeaderDetails.Filters.SearchBox.SearchProgressBar,
                    "AchievementFrame.HeaderDetails.Filters.SearchBox.SearchProgressBar must exist (xml:2501)")

                local bar = AchievementFrame.HeaderDetails.Filters.SearchBox.SearchProgressBar
                bar:SetMinMaxValues(0, 100)
                bar:SetValue(0)
                bar:SetScript("OnUpdate", AchievementFrameSearchProgressBar_OnUpdate)

                local show_called = 0
                local original_show = _G.AchievementFrame_ShowSearchPreviewResults
                _G.AchievementFrame_ShowSearchPreviewResults = function()
                    show_called = show_called + 1
                end

                local original_get_progress = _G.GetAchievementSearchProgress
                local original_get_size = _G.GetAchievementSearchSize
                local stub_progress, stub_size = 5, 10
                _G.GetAchievementSearchProgress = function() return stub_progress end
                _G.GetAchievementSearchSize = function() return stub_size end

                local function bar_signature()
                    local onupdate_label = (bar:GetScript("OnUpdate") and "function") or "nil"
                    return string.format("value=%s onupdate=%s show_called=%d",
                        tostring(bar:GetValue()), onupdate_label, show_called)
                end

                AchievementFrameSearchProgressBar_OnUpdate(bar, 0)
                local partial_signature = bar_signature()

                stub_progress, stub_size = 10, 10
                AchievementFrameSearchProgressBar_OnUpdate(bar, 0)
                local completion_signature = bar_signature()

                local minv, maxv = bar:GetMinMaxValues()
                local min_max_signature = string.format(
                    "min=%s max=%s", tostring(minv), tostring(maxv))

                _G.GetAchievementSearchProgress = original_get_progress
                _G.GetAchievementSearchSize = original_get_size
                _G.AchievementFrame_ShowSearchPreviewResults = original_show

                return type(_G.AchievementFrameSearchProgressBar_OnUpdate),
                       type(original_get_progress),
                       type(original_get_size),
                       type(original_show),
                       bar:GetObjectType(),
                       partial_signature,
                       completion_signature,
                       min_max_signature
                "#,
            )
            .expect("setup + double-drive + capture must run cleanly");

        let (
            on_update_type,
            progress_getter_type,
            size_getter_type,
            show_results_type,
            bar_object_type,
            partial_signature,
            completion_signature,
            min_max_signature,
        ) = observations;

        assert_eq!(
            on_update_type, "function",
            "Expected `_G.AchievementFrameSearchProgressBar_OnUpdate` to be a function (declared \
             at `Mainline/Blizzard_AchievementUI.lua:3318`). Got `{on_update_type}`. A `nil` \
             reading means the `:SetScript(\"OnUpdate\", AchievementFrameSearchProgressBar_OnUpdate)` \
             install at lua:3296 (inside `_SearchBox_OnUpdate`) would crash, leaving the search \
             progress bar permanently empty when the search runs."
        );

        assert_eq!(
            progress_getter_type, "function",
            "Expected the original `_G.GetAchievementSearchProgress` (captured pre-spy as \
             `__test_original_get_progress`) to be a function — backed by \
             `src/lua_api/globals/missing_surface/achievement_info.rs:878-882` returning \
             `state.achievement_search.progress`. Got `{progress_getter_type}`. A `nil` reading \
             means the achievement-search-globals registration at \
             `register_search_getters` (lines 267-292) did not run, breaking the numerator at \
             lua:3320."
        );

        assert_eq!(
            size_getter_type, "function",
            "Expected the original `_G.GetAchievementSearchSize` (captured pre-spy as \
             `__test_original_get_size`) to be a function — backed by \
             `achievement_info.rs:884-888` returning `state.achievement_search.size`. Got \
             `{size_getter_type}`. A `nil` reading means the denominator at lua:3320 would \
             crash, breaking the entire fill-ratio computation. Default state is `size = 0` \
             (per `state_types/collections.rs:301-304`), so a real call without spy override \
             would yield `0/0 == NaN` at lua:3320."
        );

        assert_eq!(
            show_results_type, "function",
            "Expected the original `_G.AchievementFrame_ShowSearchPreviewResults` (captured \
             pre-spy as `__test_original_show`) to be a function (declared at lua:3332). Got \
             `{show_results_type}`. A `nil` reading means the completion-branch forward at \
             lua:3328 would crash whenever the search finishes, leaving the search preview \
             panel hidden and the progress bar stuck at maxValue."
        );

        assert_eq!(
            bar_object_type, "StatusBar",
            "Expected `AchievementFrame.HeaderDetails.Filters.SearchBox.SearchProgressBar:GetObjectType()` to be `\"StatusBar\"` \
             — declared at xml:2501 as `<StatusBar parentKey=\"searchProgressBar\" \
             hidden=\"false\">`. Got `{bar_object_type:?}`. A `\"Frame\"` reading would mean the \
             XML parser fell back to a generic Frame (StatusBar-specific methods like \
             `:GetMinMaxValues`/`:SetValue` would still be defined but the object type \
             metadata would mismatch — the `:GetMinMaxValues` call at lua:3319 returns \
             `(min, max)`, where on a non-StatusBar the second return could be `nil`, leading \
             to a downstream `nil * number` crash at lua:3320)."
        );

        assert_eq!(
            partial_signature, EXPECTED_PARTIAL_SIGNATURE,
            "Expected partial-fill drive (progress=5, size=10, max=100) to produce the \
             signature `{EXPECTED_PARTIAL_SIGNATURE}`. Got `{partial_signature}`. A `value=` \
             other than 50 means the math at lua:3320 (`progress / size * maxValue`) \
             diverged — Blizzard could have changed the formula to `progress / size` (raw \
             fraction, value=0.5), or to use `min` as an offset (value=50+min). An \
             `onupdate=nil` here means the completion branch at lua:3325 fired despite \
             `50 < 100` — the comparison was likely inverted or the threshold lowered. A \
             `show_called=` other than 0 means `_ShowSearchPreviewResults` is now invoked on \
             every tick, which would surface as the search preview panel flashing during \
             active scans."
        );

        assert_eq!(
            completion_signature, EXPECTED_COMPLETION_SIGNATURE,
            "Expected completion drive (progress=10, size=10, max=100, ratio=1.0, scaled=100 \
             == maxValue) to produce the signature `{EXPECTED_COMPLETION_SIGNATURE}`. Got \
             `{completion_signature}`. A `value=100` reading means lua:3327 \
             (`self:SetValue(0)`) did not fire — the bar would stay pinned at maxValue \
             instead of resetting. An `onupdate=function` reading means lua:3326 \
             (`self:SetScript(\"OnUpdate\", nil)`) did not fire — the bar would keep ticking \
             OnUpdate on a finished search, repeatedly calling \
             `_ShowSearchPreviewResults`. A `show_called=0` reading means the forward at \
             lua:3328 was severed; `show_called` > 1 means re-entry happened (likely the \
             OnUpdate clear at lua:3326 didn't take effect before the next tick, but the \
             test only drives once after completion so >1 would be a deeper bug)."
        );

        assert_eq!(
            min_max_signature, EXPECTED_MIN_MAX_SIGNATURE,
            "Expected `bar:GetMinMaxValues()` to return `(0, 100)` after \
             `bar:SetMinMaxValues(0, 100)` (signature `{EXPECTED_MIN_MAX_SIGNATURE}`). Got \
             `{min_max_signature}`. A different reading means the StatusBar's min/max \
             round-trip is broken — without persistence, the denominator scaling at lua:3319 \
             (`local _, maxValue = self:GetMinMaxValues()`) would silently use whatever \
             default the framework returns (typically `(0, 1)`), making `actualProgress` \
             always equal the raw ratio `progress/size`. The completion branch at lua:3325 \
             would then fire as soon as `progress >= size`, but the partial-fill assertion \
             would have failed first."
        );
    });
}
