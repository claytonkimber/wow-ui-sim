use crate::common;

use std::path::PathBuf;
use wow_ui_sim::loader::{discover_blizzard_addons, load_addon};
use wow_ui_sim::lua_api::WowLuaEnv;

fn blizzard_ui_dir() -> PathBuf {
    wow_ui_sim::client_profile::blizzard_ui_addons_dir_under(std::path::Path::new(env!(
        "CARGO_MANIFEST_DIR"
    )))
}

fn env_with_restricted_environment() -> WowLuaEnv {
    let env = WowLuaEnv::new().expect("create env");
    env.set_screen_size(1024.0, 768.0);

    let ui = blizzard_ui_dir();
    {
        let mut state = env.state().borrow_mut();
        state.addon_base_paths = vec![ui.clone()];
    }

    for (name, toc_path) in discover_blizzard_addons(&ui) {
        load_addon(&env.loader_env(), &toc_path)
            .unwrap_or_else(|error| panic!("{name} should load before group header test: {error}"));
        if name == "Blizzard_RestrictedAddOnEnvironment" {
            break;
        }
    }

    env
}

#[test]
fn secure_raid_group_header_spawns_raid_unit_children() {
    test_timeout! {
        let env = env_with_restricted_environment();

        let units: String = env.eval(
            r#"
            A_Admin.SetPartySize(7)
            local header = CreateFrame("Frame", "TestRaidHeader", UIParent, "SecureGroupHeaderTemplate")
            header:SetAttribute("showRaid", true)
            header:SetAttribute("template", "SecureUnitButtonTemplate")
            header:SetAttribute("point", "TOP")
            header:SetAttribute("xOffset", 0)
            header:SetAttribute("yOffset", -12)
            header:SetAttribute("unitsPerColumn", 4)
            header:SetAttribute("maxColumns", 2)
            header:SetAttribute("columnAnchorPoint", "LEFT")
            header:SetAttribute("columnSpacing", 20)
            header:Show()

            local out = {}
            for index = 1, 8 do
                local child = header:GetAttribute("child" .. index)
                out[index] = child and tostring(child:GetAttribute("unit")) or "nil"
            end

            local child2 = header:GetAttribute("child2")
            local child5 = header:GetAttribute("child5")
            local p2, _, rp2, x2, y2 = child2:GetPoint(1)
            local p5, _, rp5, x5, y5 = child5:GetPoint(1)
            local layout = table.concat({p2, rp2, tostring(x2), tostring(y2), p5, rp5, tostring(x5), tostring(y5)}, ":")
            return table.concat(out, ",") .. ";" .. layout
            "#,
        ).expect("raid header should update through Blizzard native Lua");

        assert_eq!(
            units,
            "raid1,raid2,raid3,raid4,raid5,raid6,raid7,raid8;TOP:BOTTOM:0:-12:LEFT:RIGHT:20:0"
        );
    }
}

#[test]
fn secure_group_pet_header_spawns_pet_child() {
    test_timeout! {
        let env = env_with_restricted_environment();

        let unit: String = env.eval(
            r#"
            A_Admin.SetPartySize(0)
            local header = CreateFrame("Frame", "TestPetHeader", UIParent, "SecureGroupPetHeaderTemplate")
            header:SetAttribute("showSolo", true)
            header:SetAttribute("template", "SecureUnitButtonTemplate")
            header:Show()

            local child1 = header:GetAttribute("child1")
            local child2 = header:GetAttribute("child2")
            return tostring(child1 and child1:GetAttribute("unit")) .. "," .. tostring(child2)
            "#,
        ).expect("pet header should update through Blizzard native Lua");

        assert_eq!(unit, "pet,nil");
    }
}
