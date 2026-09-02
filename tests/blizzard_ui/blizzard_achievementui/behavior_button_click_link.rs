//! Behavior pin: PLAN-named `AchievementButton_OnClick` does NOT exist in
//! the Mainline source. The chat-link branch lives on the Mainline
//! mixin (`AchievementTemplateMixin:ProcessClick` at lua:1060-1077), uses
//! `IsModifiedClick("CHATLINK")` (NOT specifically "shift"), and feeds
//! the link into `ChatFrameUtil.InsertLink` (NOT `ChatEdit_InsertLink`).
//! This test pins the PLAN-named function as absent and the actual
//! Mainline call chain as present.
//!
//! Source overview:
//!
//! ```lua
//! -- Mainline/Blizzard_AchievementUI.lua:1060-1077 (the actual call site)
//! function AchievementTemplateMixin:ProcessClick(buttonName, down)
//!     local handled = false;
//!     if IsModifiedClick() then
//!         local elementData = self:GetElementData();
//!         if IsModifiedClick("CHATLINK") then
//!             local achievementLink = GetAchievementLink(elementData.id);
//!             if achievementLink then
//!                 handled = ChatFrameUtil.InsertLink(achievementLink);
//!                 ...
//!             end
//!         end
//!         ...
//!     end
//!     ...
//! end
//! ```
//!
//! ```lua
//! -- Cata/Blizzard_AchievementUI.lua:893 (the only place AchievementButton_OnClick exists)
//! function AchievementButton_OnClick (self, button, down, ignoreModifiers)
//!     if(IsModifiedClick() and not ignoreModifiers) then
//!         if ( IsModifiedClick("CHATLINK") and ChatFrameUtil.GetActiveWindow() ) then
//!             local achievementLink = GetAchievementLink(self.id);
//!             if ( achievementLink ) then
//!                 ChatFrameUtil.InsertLink(achievementLink);
//!             end
//!         ...
//! ```
//!
//! Five spec/source mismatches in PLAN's wording:
//!
//! 1. **`AchievementButton_OnClick` does not exist in Mainline.** It only
//!    appears in `Cata/Blizzard_AchievementUI.lua:893`. Mainline routes
//!    chat-link clicks through `AchievementTemplateMixin:ProcessClick`.
//! 2. **"shift modifier" is not specifically what gates the branch.** The
//!    actual gate is `IsModifiedClick("CHATLINK")`, a user-configurable
//!    binding (default SHIFT but can be ALT, CTRL, etc. depending on the
//!    `CHATLINK` keybind setting).
//! 3. **`ChatEdit_InsertLink` is not what the actual code calls.** Both
//!    Mainline and Cata call `ChatFrameUtil.InsertLink(achievementLink)`.
//!    `ChatEdit_InsertLink` is a deprecated alias defined at
//!    `Blizzard_DeprecatedChatInfo/Deprecated_ChatFrame.lua:43` as
//!    `ChatEdit_InsertLink = ChatFrameUtil.InsertLink`.
//! 4. **The depends-on tag `GetAchievementLink gap` is stale.** The C API
//!    is implemented at `src/lua_api/globals/missing_surface/achievement_info.rs:386`
//!    (registration) and `:671` (impl); it returns
//!    `|cffffff00|Hachievement:<id>:Player-1-00000001:1:1:15:2025:0:0:0:0|h[<name>]|h|r`
//!    when the id resolves against `SimState.achievements`, nil otherwise.
//! 5. **In the smoke-shape harness `ChatFrameUtil.InsertLink` is unset** —
//!    the simulator only registers `AddSystemMessage` and `OpenChat` on
//!    the Rust side (`src/lua_api/globals/chat_frame_util.rs:158-164`);
//!    `InsertLink` would be installed by Blizzard_ChatFrameBase Lua at
//!    `Mainline/ChatFrameUtilOverrides.lua:1`, which is not in this
//!    addon's dependency closure. The smoke shape would crash on the
//!    Cata `AchievementButton_OnClick` chat-link branch with "attempt to
//!    call a nil value (field 'InsertLink')". The Mainline mixin path
//!    has the same dependency, but the test pins the static surface (not
//!    a click drive-through) so this gap doesn't fire.
//!
//! Six assertions split presence/absence:
//!
//! - **Absence half** (3): `_G.AchievementButton_OnClick` is nil (Mainline
//!   does not define it); `_G.ChatEdit_InsertLink` is nil (deprecated
//!   alias is in Blizzard_DeprecatedChatInfo, not the smoke-shape
//!   closure); `ChatFrameUtil.InsertLink` is nil (the Mainline ChatFrameBase
//!   Lua override is not loaded by the smoke harness).
//! - **Presence half** (3): `_G.GetAchievementLink` is a function
//!   (depends-on stale); `AchievementTemplateMixin.ProcessClick` is a
//!   function (the actual Mainline call site); `GetAchievementLink(6)`
//!   returns a string containing `|Hachievement:6:` (the link payload
//!   the click branch would feed into the insertion API).

use crate::common::blizzard_addon_harness::with_blizzard_addon_smoke_shape;

const ROOT: &str = "Blizzard_AchievementUI";
const PLAN_NAMED_BUT_ABSENT_GLOBAL: &str = "AchievementButton_OnClick";
const PLAN_NAMED_BUT_ABSENT_INSERTION_API: &str = "ChatEdit_InsertLink";
const SEEDED_ACHIEVEMENT_ID: i64 = 6;

type ClickLinkProbe = (String, String, String, String, String, String);

#[test]
fn achievement_button_on_click_is_absent_but_mixin_process_click_and_link_api_work() {
    with_blizzard_addon_smoke_shape(&[ROOT], &[], |env, _loaded| {
        let observations: ClickLinkProbe = env
            .eval(
                r#"
                local plan_named_global_type = type(_G.AchievementButton_OnClick)
                local chat_edit_insert_link_type = type(_G.ChatEdit_InsertLink)
                local chat_frame_util_insert_link_type =
                    (type(_G.ChatFrameUtil) == "table" and type(_G.ChatFrameUtil.InsertLink))
                    or "no-chat-frame-util-table"
                local get_achievement_link_type = type(_G.GetAchievementLink)
                local mixin_process_click_type =
                    (type(_G.AchievementTemplateMixin) == "table"
                        and type(_G.AchievementTemplateMixin.ProcessClick))
                    or "no-mixin-table"
                local seeded_link =
                    (type(_G.GetAchievementLink) == "function"
                        and (GetAchievementLink(6) or ""))
                    or ""

                return plan_named_global_type,
                       chat_edit_insert_link_type,
                       chat_frame_util_insert_link_type,
                       get_achievement_link_type,
                       mixin_process_click_type,
                       seeded_link
                "#,
            )
            .expect("AchievementButton click-link probe must run cleanly");

        let (
            plan_named_global_type,
            chat_edit_insert_link_type,
            chat_frame_util_insert_link_type,
            get_achievement_link_type,
            mixin_process_click_type,
            seeded_link,
        ) = observations;

        assert_eq!(
            plan_named_global_type, "nil",
            "Expected `_G.{PLAN_NAMED_BUT_ABSENT_GLOBAL}` to be nil — the function exists ONLY in \
             `Cata/Blizzard_AchievementUI.lua:893`, not in Mainline. Got \
             `{plan_named_global_type}`. A non-nil reading would prove Blizzard ported the \
             Cata-style global into Mainline (the absence half should then be replaced by a \
             behavior probe that drives the click and asserts \
             `GetAchievementLink(self.id)` was called)."
        );

        assert_eq!(
            chat_edit_insert_link_type, "nil",
            "Expected `_G.{PLAN_NAMED_BUT_ABSENT_INSERTION_API}` to be nil — this name is the \
             deprecated alias defined at \
             `Blizzard_DeprecatedChatInfo/Deprecated_ChatFrame.lua:43` (`ChatEdit_InsertLink = \
             ChatFrameUtil.InsertLink`), and Blizzard_DeprecatedChatInfo is NOT in the \
             Blizzard_AchievementUI dependency closure that the smoke harness loads. Got \
             `{chat_edit_insert_link_type}`. The actual Mainline call at lua:1067 is \
             `ChatFrameUtil.InsertLink(achievementLink)`, NOT `ChatEdit_InsertLink`. A non-nil \
             reading means the deprecated bridge entered the closure (or was force-loaded)."
        );

        assert_eq!(
            chat_frame_util_insert_link_type, "function",
            "Expected `ChatFrameUtil.InsertLink` to be a function — the panel fixture now \
             source-correctly loads Blizzard_ChatFrameBase before Blizzard_MicroMenu, and \
             `Blizzard_ChatFrameBase/Mainline/ChatFrameUtilOverrides.lua:14` publishes the \
             Mainline insertion API. Got `{chat_frame_util_insert_link_type}`. The deprecated \
             `ChatEdit_InsertLink` alias remains absent because Blizzard_DeprecatedChatInfo is \
             not part of this fixture."
        );

        assert_eq!(
            get_achievement_link_type, "function",
            "Expected `_G.GetAchievementLink` to be a function (PLAN tags this as a gap, but \
             it's implemented at \
             `src/lua_api/globals/missing_surface/achievement_info.rs:386` (registration) and \
             `:671` (impl)). Got `{get_achievement_link_type}`. The depends-on tag is stale; if \
             this assertion fails the `local achievementLink = GetAchievementLink(elementData.id)` \
             call at lua:1065 (Mainline mixin) and `GetAchievementLink(self.id)` at lua:896 \
             (Cata global) would crash."
        );

        assert_eq!(
            mixin_process_click_type, "function",
            "Expected `AchievementTemplateMixin.ProcessClick` to be a function — declared at \
             `Mainline/Blizzard_AchievementUI.lua:1060`, this is the actual Mainline call site \
             that PLAN's `AchievementButton_OnClick` wording referred to. Got \
             `{mixin_process_click_type}`. A `nil` reading means the addon's mixin definition \
             never executed or the method was renamed."
        );

        let expected_link_substring = format!("|Hachievement:{SEEDED_ACHIEVEMENT_ID}:");
        assert!(
            seeded_link.contains(&expected_link_substring),
            "Expected `GetAchievementLink({SEEDED_ACHIEVEMENT_ID})` to contain \
             `{expected_link_substring}` — the seeded `Level 10` achievement at \
             `src/lua_api/state.rs:2178-2191` has id 6, and the link format at \
             `src/lua_api/globals/missing_surface/achievement_info.rs:681-683` is \
             `|cffffff00|Hachievement:<id>:Player-1-00000001:1:1:15:2025:0:0:0:0|h[<name>]|h|r`. \
             Got `{seeded_link:?}`. An empty reading means the seed was removed; a string \
             without `|Hachievement:` means the link format was changed in a way that breaks \
             the chat-link parser."
        );
    });
}
