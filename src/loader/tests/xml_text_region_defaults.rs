use super::*;

#[test]
fn button_text_without_anchors_uses_justify_h_default_point() {
    let ctx = load_test_xml(
        "button-text-justify-h-default-anchor",
        r#"
        <Ui xmlns="http://www.blizzard.com/wow/ui/">
            <Button name="DefaultButtonTextAnchorLeft" parent="UIParent">
                <ButtonText name="$parentText" inherits="GameFontNormal" text="BL" justifyH="LEFT" justifyV="TOP"/>
            </Button>
            <Button name="DefaultButtonTextAnchorCenter" parent="UIParent">
                <ButtonText name="$parentText" inherits="GameFontNormal" text="BC" justifyH="CENTER" justifyV="MIDDLE"/>
            </Button>
            <Button name="DefaultButtonTextAnchorRight" parent="UIParent">
                <ButtonText name="$parentText" inherits="GameFontNormal" text="BR" justifyH="RIGHT" justifyV="BOTTOM"/>
            </Button>
            <Button name="DefaultButtonTextAnchorTopOnly" parent="UIParent">
                <ButtonText name="$parentText" inherits="GameFontNormal" text="BT" justifyH="LEFT" justifyV="BOTTOM">
                    <Anchors><Anchor point="TOP"/></Anchors>
                </ButtonText>
            </Button>
        </Ui>
        "#,
    );

    ctx.assert_lua_str(
        r#"
        return (function()
            local cases = {
                { DefaultButtonTextAnchorLeftText, DefaultButtonTextAnchorLeft, "LEFT" },
                { DefaultButtonTextAnchorCenterText, DefaultButtonTextAnchorCenter, "CENTER" },
                { DefaultButtonTextAnchorRightText, DefaultButtonTextAnchorRight, "RIGHT" },
                { DefaultButtonTextAnchorTopOnlyText, DefaultButtonTextAnchorTopOnly, "TOP" },
            }

            local results = {}
            for index, case in ipairs(cases) do
                local region, parent, expected = case[1], case[2], case[3]
                local point, relativeTo, relativePoint, x, y = region:GetPoint(1)
                results[index] = tostring(
                    region:GetNumPoints() == 1
                        and point == expected
                        and relativeTo == parent
                        and relativePoint == expected
                        and x == 0
                        and y == 0
                )
            end

            return table.concat(results, "|")
        end)()
        "#,
        "true|true|true|true",
    );
}

#[test]
fn justify_probe_editbox_regions_cover_size_and_inset_variants() {
    let ctx = load_test_xml(
        "justify-probe-editbox-region-variants",
        r#"
        <Ui xmlns="http://www.blizzard.com/wow/ui/">
            <EditBox name="JustifyEditBoxNoSize" parent="UIParent">
                <Size x="180" y="32"/>
                <FontString name="$parentText" inherits="GameFontNormal" text="EditText"/>
            </EditBox>
            <EditBox name="JustifyEditBoxSized" parent="UIParent">
                <Size x="180" y="32"/>
                <FontString name="$parentText" inherits="GameFontNormal" text="EditSized">
                    <Size x="120" y="18"/>
                </FontString>
            </EditBox>
            <EditBox name="JustifyEditBoxInset" parent="UIParent">
                <Size x="180" y="32"/>
                <TextInsets left="7" right="11" top="13" bottom="17"/>
                <FontString name="$parentText" inherits="GameFontNormal" text="EditInset"/>
            </EditBox>
        </Ui>
        "#,
    );

    ctx.assert_lua_str(
        r#"
        return (function()
            local noSizeRegion = JustifyEditBoxNoSize:GetRegions()
            local sizedRegion = JustifyEditBoxSized:GetRegions()
            local insetRegion = JustifyEditBoxInset:GetRegions()
            local left, right, top, bottom = JustifyEditBoxInset:GetTextInsets()
            return table.concat({
                tostring(noSizeRegion ~= nil and noSizeRegion:GetObjectType() == "FontString"),
                tostring(sizedRegion ~= nil and sizedRegion:GetObjectType() == "FontString"),
                tostring(insetRegion ~= nil and insetRegion:GetObjectType() == "FontString"),
                tostring(noSizeRegion ~= nil and noSizeRegion:GetNumPoints() == 0),
                tostring(sizedRegion ~= nil and sizedRegion:GetNumPoints() == 0),
                tostring(insetRegion ~= nil and insetRegion:GetNumPoints() == 0),
                tostring(left == 7 and right == 11 and top == 13 and bottom == 17),
                tostring(JustifyEditBoxNoSizeText == nil),
                tostring(JustifyEditBoxSizedText == nil),
                tostring(JustifyEditBoxInsetText == nil),
            }, "|")
        end)()
        "#,
        "true|true|true|true|true|true|true|true|true|true",
    );
}

#[test]
fn justify_probe_message_regions_stay_absent_with_owner_insets() {
    let ctx = load_test_xml(
        "justify-probe-message-regions",
        r#"
        <Ui xmlns="http://www.blizzard.com/wow/ui/">
            <Frame name="JustifyMessageOwnerParent" parent="UIParent" hidden="true"/>
        </Ui>
        "#,
    );

    ctx.assert_lua_str(
        r#"
        return (function()
            local function createMessageFrame(frameType, name, withInsets)
                local frame = CreateFrame(frameType, name, JustifyMessageOwnerParent)
                frame:SetSize(180, 32)
                frame:SetFontObject(GameFontNormal)
                frame:SetJustifyH("RIGHT")
                frame:SetJustifyV("BOTTOM")
                if withInsets then
                    frame:SetTextInsets(7, 11, 13, 17)
                end
                frame:AddMessage(withInsets and "Inset" or "Text")
                return frame
            end

            local function hasFontStringRegion(frame)
                for _, region in ipairs({ frame:GetRegions() }) do
                    if region:GetObjectType() == "FontString" then
                        return true
                    end
                end
                return false
            end

            local message = createMessageFrame("MessageFrame", "JustifyMessageFrame", false)
            local messageInset = createMessageFrame("MessageFrame", "JustifyMessageFrameInset", true)
            local scrolling = createMessageFrame("ScrollingMessageFrame", "JustifyScrollingMessageFrame", false)
            local scrollingInset = createMessageFrame("ScrollingMessageFrame", "JustifyScrollingMessageFrameInset", true)
            local ml, mr, mt, mb = messageInset:GetTextInsets()
            local sl, sr, st, sb = scrollingInset:GetTextInsets()

            return table.concat({
                tostring(message:GetNumPoints() == 0),
                tostring(messageInset:GetNumPoints() == 0),
                tostring(scrolling:GetNumPoints() == 0),
                tostring(scrollingInset:GetNumPoints() == 0),
                tostring(not hasFontStringRegion(message)),
                tostring(not hasFontStringRegion(messageInset)),
                tostring(not hasFontStringRegion(scrolling)),
                tostring(not hasFontStringRegion(scrollingInset)),
                tostring(ml == 7 and mr == 11 and mt == 13 and mb == 17),
                tostring(sl == 7 and sr == 11 and st == 13 and sb == 17),
            }, "|")
        end)()
        "#,
        "true|true|true|true|true|true|true|true|true|true",
    );
}

#[test]
fn editbox_xml_text_insets_do_not_anchor_backing_fontstring() {
    let ctx = load_test_xml(
        "editbox-text-insets-fontstring-anchor",
        r#"
        <Ui xmlns="http://www.blizzard.com/wow/ui/">
            <EditBox name="DefaultEditBoxTextInsets" parent="UIParent">
                <Size><AbsDimension x="180" y="32"/></Size>
                <TextInsets left="7" right="11" top="13" bottom="17"/>
                <FontString name="$parentText" inherits="GameFontNormal" text="EditText" justifyH="RIGHT"/>
            </EditBox>
        </Ui>
        "#,
    );

    ctx.assert_lua_str(
        r#"
        return (function()
            local left, right, top, bottom = DefaultEditBoxTextInsets:GetTextInsets()
            local region = DefaultEditBoxTextInsets:GetRegions()
            return table.concat({
                tostring(left == 7),
                tostring(right == 11),
                tostring(top == 13),
                tostring(bottom == 17),
                tostring(region ~= nil and region:GetObjectType() == "FontString"),
                tostring(region ~= nil and region:GetNumPoints() == 0),
                tostring(DefaultEditBoxTextInsetsText == nil),
            }, "|")
        end)()
        "#,
        "true|true|true|true|true|true|true",
    );
}
