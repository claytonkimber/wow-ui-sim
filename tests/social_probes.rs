//! Tests for social / character-sheet probe globals backed by SimState:
//!
//! - `GetNumTitles` / `GetTitleName(index)` / `IsTitleKnown` /
//!   `GetCurrentTitle` / `SetCurrentTitle`
//! - `GetNumClasses`
//! - `GetNumShapeshiftForms`

use wow_ui_sim::lua_api::WowLuaEnv;

fn env() -> WowLuaEnv {
    WowLuaEnv::new().expect("Failed to create Lua environment")
}

#[test]
fn get_num_titles_reports_sim_state_titles_len() {
    let env = env();
    let baseline: i32 = env.eval("return GetNumTitles()").unwrap();

    {
        let mut state = env.state().borrow_mut();
        state.titles.push("Jenkins".to_string());
        state.titles.push("of the Nightfall".to_string());
    }

    let after: i32 = env.eval("return GetNumTitles()").unwrap();
    assert_eq!(after, baseline + 2);
}

#[test]
fn get_title_name_indexes_one_based_and_nils_out_of_range() {
    let env = env();
    {
        let mut state = env.state().borrow_mut();
        state.titles.clear();
        state.titles.push("the Patient".to_string());
        state.titles.push("Jenkins".to_string());
    }

    let (first, second, below, above, zero): (String, String, bool, bool, bool) = env
        .eval(
            r#"
            return GetTitleName(1),
                   GetTitleName(2),
                   GetTitleName(-1) == nil,
                   GetTitleName(99) == nil,
                   GetTitleName(0) == nil
            "#,
        )
        .unwrap();

    assert_eq!(first, "the Patient");
    assert_eq!(second, "Jenkins");
    assert!(below, "negative index should return nil");
    assert!(above, "out-of-range index should return nil");
    assert!(zero, "zero index should return nil (1-based)");
}

#[test]
fn get_title_name_returns_player_title_flag() {
    let env = env();
    {
        let mut state = env.state().borrow_mut();
        state.titles.clear();
        state.titles.push("the Patient".to_string());
    }

    let (name, is_player_title): (String, bool) = env.eval("return GetTitleName(1)").unwrap();

    assert_eq!(name, "the Patient");
    assert!(
        is_player_title,
        "GetTitleName must return a truthy second value so GetKnownTitles populates"
    );
}

#[test]
fn is_title_known_matches_index_range() {
    let env = env();
    {
        let mut state = env.state().borrow_mut();
        state.titles.clear();
        state.titles.push("the Patient".to_string());
        state.titles.push("Jenkins".to_string());
    }

    let (one, two, three, zero, negative): (bool, bool, bool, bool, bool) = env
        .eval(
            r#"
            return IsTitleKnown(1),
                   IsTitleKnown(2),
                   IsTitleKnown(3),
                   IsTitleKnown(0),
                   IsTitleKnown(-1)
            "#,
        )
        .unwrap();

    assert!(one);
    assert!(two);
    assert!(!three, "out-of-range index should be unknown");
    assert!(!zero, "zero is not a valid 1-based index");
    assert!(!negative, "negative index should be unknown");
}

#[test]
fn set_current_title_persists_for_known_indices() {
    let env = env();
    {
        let mut state = env.state().borrow_mut();
        state.titles.clear();
        state.titles.push("the Patient".to_string());
        state.titles.push("Jenkins".to_string());
    }

    let initial: i32 = env.eval("return GetCurrentTitle()").unwrap();
    assert_eq!(initial, -1, "no title should be selected by default");

    let after_known: i32 = env
        .eval("SetCurrentTitle(2); return GetCurrentTitle()")
        .unwrap();
    assert_eq!(after_known, 2);

    let after_unknown: i32 = env
        .eval("SetCurrentTitle(99); return GetCurrentTitle()")
        .unwrap();
    assert_eq!(
        after_unknown, -1,
        "out-of-range title id should clear the selection"
    );

    let after_clear: i32 = env
        .eval("SetCurrentTitle(-1); return GetCurrentTitle()")
        .unwrap();
    assert_eq!(after_clear, -1);
}

#[test]
fn set_current_title_dispatches_unit_name_update_for_player() {
    let env = env();
    {
        let mut state = env.state().borrow_mut();
        state.titles.clear();
        state.titles.push("the Patient".to_string());
    }

    let unit: String = env
        .eval(
            r#"
            local listener = CreateFrame("Frame")
            local seen
            listener:RegisterEvent("UNIT_NAME_UPDATE")
            listener:SetScript("OnEvent", function(self, event, unit)
                seen = unit
            end)

            SetCurrentTitle(1)
            return seen or "missing"
            "#,
        )
        .unwrap();

    assert_eq!(unit, "player");
}

#[test]
fn get_num_classes_returns_thirteen() {
    let env = env();
    let n: i32 = env.eval("return GetNumClasses()").unwrap();
    assert_eq!(n, 13, "retail has 13 classes (includes Evoker)");
}

#[test]
fn get_num_shapeshift_forms_reports_sim_state_len() {
    let env = env();
    let before: i32 = env.eval("return GetNumShapeshiftForms()").unwrap();
    assert_eq!(before, 3, "seeded Paladin exposes its three aura forms");

    {
        use wow_ui_sim::lua_api::state::ShapeshiftForm;
        let mut state = env.state().borrow_mut();
        state.shapeshift_forms.push(ShapeshiftForm {
            name: "Bear Form".to_string(),
            texture: "Interface/Icons/Ability_Racial_BearForm".to_string(),
            spell_id: 5487,
            is_active: false,
            is_castable: true,
        });
        state.shapeshift_forms.push(ShapeshiftForm {
            name: "Cat Form".to_string(),
            texture: "Interface/Icons/Ability_Druid_CatForm".to_string(),
            spell_id: 768,
            is_active: false,
            is_castable: true,
        });
    }

    let after: i32 = env.eval("return GetNumShapeshiftForms()").unwrap();
    assert_eq!(after, 5);
}

#[test]
fn shapeshift_form_id_defaults_to_nil() {
    let env = env();
    let is_nil: bool = env.eval("return GetShapeshiftFormID() == nil").unwrap();
    assert!(is_nil);
}
