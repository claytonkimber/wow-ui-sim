use wow_ui_sim::lua_api::WowLuaEnv;

#[test]
fn test_cross_frame_show_recursion_does_not_overflow() {
    let env = WowLuaEnv::new().unwrap();
    env.eval::<()>(
        r#"
        local a = CreateFrame("Frame", "RecurseA", UIParent)
        local b = CreateFrame("Frame", "RecurseB", UIParent)
        a:Hide()
        b:Hide()
        a:SetScript("OnShow", function() b:Show() end)
        b:SetScript("OnShow", function() a:Show() end)
        a:Show()
        "#,
    )
    .unwrap();
}

#[test]
fn test_onshow_hide_preserves_handler_selected_hidden_state() {
    let env = WowLuaEnv::new().unwrap();
    let (log, is_shown, is_visible): (String, bool, bool) = env
        .eval(
            r#"
            local log = {}
            local f = CreateFrame("Frame", "OneWayVisibilityFrame", UIParent)
            f:Hide()
            f:SetScript("OnShow", function(self)
                table.insert(log, self:IsVisible() and "show-visible" or "show-hidden")
                self:Hide()
                table.insert(log, self:IsVisible() and "after-visible" or "after-hidden")
            end)
            f:SetScript("OnHide", function(self)
                table.insert(log, self:IsVisible() and "hide-visible" or "hide-hidden")
            end)

            f:Show()
            return table.concat(log, ","), f:IsShown(), f:IsVisible()
        "#,
        )
        .unwrap();

    assert_eq!(log, "show-visible,after-hidden,hide-hidden");
    assert!(!is_shown, "OnShow-selected hidden state must persist");
    assert!(!is_visible, "hidden frame must not remain visible");
}

#[test]
fn test_cross_frame_show_recursion_stops_at_dispatch_depth_limit() {
    let env = WowLuaEnv::new().unwrap();
    let (fired, last_shown): (i32, bool) = env
        .eval(
            r#"
            local frames = {}
            local fired = 0
            for i = 1, 41 do
                frames[i] = CreateFrame("Frame", "DepthFrame" .. i, UIParent)
                frames[i]:Hide()
                frames[i]:SetScript("OnShow", function()
                    fired = fired + 1
                    if frames[i + 1] then
                        frames[i + 1]:Show()
                    end
                end)
            end

            frames[1]:Show()
            return fired, frames[41]:IsShown()
        "#,
        )
        .unwrap();

    assert_eq!(fired, 40, "cross-frame dispatch must stop at depth 40");
    assert!(
        last_shown,
        "the depth-limited frame still receives its requested state"
    );
}

#[test]
fn test_onshow_onhide_mutual_recursion_terminates_with_reference_order() {
    let env = WowLuaEnv::new().unwrap();
    let log: String = env
        .eval(
            r#"
            local log = {}
            local f = CreateFrame("Frame", "MutualVisibilityFrame", UIParent)
            f:SetScript("OnShow", function(self)
                table.insert(log, self:IsVisible() and "A" or "a")
                self:Hide()
                table.insert(log, self:IsVisible() and "B" or "b")
            end)
            f:SetScript("OnHide", function(self)
                table.insert(log, self:IsVisible() and "C" or "c")
                self:Show()
                table.insert(log, self:IsVisible() and "D" or "d")
            end)

            f:Hide()
            return table.concat(log)
        "#,
        )
        .unwrap();

    assert_eq!(
        log,
        "cDAb".repeat(6),
        "OnShow/OnHide mutual recursion should unwind iteratively with wowless/master ordering"
    );
}

#[test]
fn test_child_onshow_fires_when_parent_becomes_visible() {
    let env = WowLuaEnv::new().unwrap();
    let fired: i32 = env
        .eval(
            r#"
            local parent = CreateFrame("Frame", "ChildOnShowParent", UIParent)
            local child = CreateFrame("Frame", "ChildOnShowChild", parent)
            parent:Hide()
            child:Hide()

            local fired = 0
            child:SetScript("OnShow", function()
                fired = fired + 1
            end)

            child:Show()
            parent:Show()
            return fired
        "#,
        )
        .unwrap();
    assert_eq!(
        fired, 1,
        "child OnShow should fire when a hidden parent becomes visible"
    );
}

#[test]
fn test_child_onhide_fires_when_parent_becomes_hidden() {
    let env = WowLuaEnv::new().unwrap();
    let fired: i32 = env
        .eval(
            r#"
            local parent = CreateFrame("Frame", "ChildOnHideParent", UIParent)
            local child = CreateFrame("Frame", "ChildOnHideChild", parent)

            local fired = 0
            child:SetScript("OnHide", function()
                fired = fired + 1
            end)

            parent:Hide()
            return fired
        "#,
        )
        .unwrap();
    assert_eq!(
        fired, 1,
        "child OnHide should fire when a visible parent becomes hidden"
    );
}
