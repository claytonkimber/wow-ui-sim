//! Integration tests for Blizzard_PlayerSpells spellbook flipbook behavior.

use crate::common;

use common::panel_fixtures::{
    clear_recorded_lua_errors, player_spells_panel_debug_snapshot, recorded_lua_errors, setup_env,
};
use wow_ui_sim::startup::run_extra_update_ticks;

fn open_spellbook(env: &wow_ui_sim::lua_api::WowLuaEnv) {
    env.exec(
        r#"
        assert(PlayerSpellsUtil and PlayerSpellsUtil.ToggleSpellBookFrame, "ToggleSpellBookFrame should exist")
        PlayerSpellsUtil.ToggleSpellBookFrame()
        assert(PlayerSpellsFrame and PlayerSpellsFrame:IsShown(), "PlayerSpellsFrame should be shown")
        assert(PlayerSpellsFrame.SpellBookFrame and PlayerSpellsFrame.SpellBookFrame:IsShown(), "SpellBookFrame should be shown")
        "#,
    )
    .expect("Failed to open spellbook");
}

const SPELLBOOK_CORNER_FLIPBOOK_FRAME_INDEX_LUA: &str = r#"
local flipbook = assert(
    PlayerSpellsFrame
        and PlayerSpellsFrame.SpellBookFrame
        and PlayerSpellsFrame.SpellBookFrame.BookCornerFlipbook,
    "BookCornerFlipbook should exist"
)
local atlas = assert(C_Texture.GetAtlasInfo("spellbook-corner-flipbook-evergreen"), "missing atlas info")
local tlx, tly, blx, bly, trx, try, brx, bry = flipbook:GetTexCoord()
local left = math.min(tlx, blx, trx, brx)
local right = math.max(tlx, blx, trx, brx)
local top = math.min(tly, bly, try, bry)
local bottom = math.max(tly, bly, try, bry)
local rows = 2
local cols = 4
local cell_width = (atlas.rightTexCoord - atlas.leftTexCoord) / cols
local cell_height = (atlas.bottomTexCoord - atlas.topTexCoord) / rows
local epsilon = 0.0001

local function close(a, b)
    return math.abs(a - b) <= epsilon
end

for index = 0, 7 do
    local col = index % cols
    local row = math.floor(index / cols)
    local expected_left = atlas.leftTexCoord + col * cell_width
    local expected_right = expected_left + cell_width
    local expected_top = atlas.topTexCoord + row * cell_height
    local expected_bottom = expected_top + cell_height

    if close(left, expected_left)
        and close(right, expected_right)
        and close(top, expected_top)
        and close(bottom, expected_bottom)
    then
        return index
    end
end

return -1
"#;

fn spellbook_corner_flipbook_frame_index(env: &wow_ui_sim::lua_api::WowLuaEnv) -> i32 {
    env.eval(SPELLBOOK_CORNER_FLIPBOOK_FRAME_INDEX_LUA)
        .expect("SpellBook corner flipbook frame probe should return")
}

#[test]
fn spellbook_corner_flipbook_plays_and_rewinds() {
    test_timeout! {
        let env = setup_env();
        common::install_error_collector(&env, "__spellbook_flipbook_errors");
        clear_recorded_lua_errors(&env);

        open_spellbook(&env);
        clear_recorded_lua_errors(&env);
        common::drain_string_table(&env, "__spellbook_flipbook_errors");

        assert_eq!(
            spellbook_corner_flipbook_frame_index(&env),
            0,
            "SpellBook corner flipbook should start on frame 0 after OnLoad"
        );

        env.exec(
            r#"
            PlayerSpellsFrame.SpellBookFrame.BookCornerFlipbook.Anim:Play()
            "#,
        )
        .expect("SpellBook corner flipbook play should succeed");

        run_extra_update_ticks(&env, 3);
        let forward_frame = spellbook_corner_flipbook_frame_index(&env);
        assert!(
            forward_frame > 0,
            "SpellBook corner flipbook should advance after play; got frame {forward_frame}"
        );

        env.exec(
            r#"
            PlayerSpellsFrame.SpellBookFrame.BookCornerFlipbook.Anim:Play(true)
            "#,
        )
        .expect("SpellBook corner flipbook reverse play should succeed");

        run_extra_update_ticks(&env, 3);
        assert_eq!(
            spellbook_corner_flipbook_frame_index(&env),
            0,
            "SpellBook corner flipbook should rewind to frame 0 when played in reverse"
        );

        let recorded_errors = recorded_lua_errors(&env);
        let handler_errors = common::drain_string_table(&env, "__spellbook_flipbook_errors");
        assert!(
            recorded_errors.is_empty(),
            "SpellBook flipbook regression produced {} recorded Lua error(s):\n{:#?}\nhandler_errors:\n{}\n{}",
            recorded_errors.len(),
            recorded_errors,
            handler_errors.join("\n"),
            player_spells_panel_debug_snapshot(&env),
        );
        assert!(
            handler_errors.is_empty(),
            "SpellBook flipbook regression produced {} Lua error(s):\n{}",
            handler_errors.len(),
            handler_errors.join("\n")
        );
    }
}
