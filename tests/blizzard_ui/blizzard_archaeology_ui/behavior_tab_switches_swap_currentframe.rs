//! Tab switching behavior for `Blizzard_ArchaeologyUI`.

use crate::common::blizzard_addon_harness::with_blizzard_addon_startup_shape;

const ROOT: &str = "Blizzard_ArchaeologyUI";

#[test]
fn archaeology_tabs_switch_visible_pages_and_layouts() {
    with_blizzard_addon_startup_shape(&[ROOT], &[], |env, _loaded| {
        let serialized_states: String = env
            .eval(TAB_SWITCH_PROBE_LUA)
            .expect("ArchaeologyFrame tab-switch probe must run cleanly");
        let states = parse_tab_states(&serialized_states);

        assert_help_tab_state(&states[0]);
        assert_summary_tab_state(&states[1]);
        assert_completed_tab_state(&states[2]);
    });
}

#[derive(Debug)]
struct TabState {
    label: String,
    current_summary: bool,
    current_completed: bool,
    help_shown: bool,
    summary_shown: bool,
    completed_shown: bool,
    artifact_shown: bool,
    summary_left: bool,
    summary_right: bool,
    completed_left: bool,
    completed_right: bool,
}

fn parse_tab_states(serialized_states: &str) -> Vec<TabState> {
    let states = serialized_states
        .lines()
        .map(parse_tab_state)
        .collect::<Vec<_>>();
    assert_eq!(
        states.len(),
        3,
        "probe must return help, summary, and completed states"
    );
    states
}

fn parse_tab_state(line: &str) -> TabState {
    let columns = line.split('\t').collect::<Vec<_>>();
    assert_eq!(columns.len(), 11, "each tab-state row must have 11 columns");
    TabState {
        label: columns[0].to_string(),
        current_summary: parse_bool(columns[1]),
        current_completed: parse_bool(columns[2]),
        help_shown: parse_bool(columns[3]),
        summary_shown: parse_bool(columns[4]),
        completed_shown: parse_bool(columns[5]),
        artifact_shown: parse_bool(columns[6]),
        summary_left: parse_bool(columns[7]),
        summary_right: parse_bool(columns[8]),
        completed_left: parse_bool(columns[9]),
        completed_right: parse_bool(columns[10]),
    }
}

fn parse_bool(value: &str) -> bool {
    match value {
        "true" => true,
        "false" => false,
        _ => panic!("expected Lua boolean string, got {value:?}"),
    }
}

fn assert_help_tab_state(state: &TabState) {
    assert_eq!(state.label, "help");
    assert!(
        state.current_summary,
        "help overlay keeps `currentFrame` on the previous summary page"
    );
    assert!(
        !state.current_completed,
        "help overlay must not switch `currentFrame` to completedPage"
    );
    assert!(state.help_shown, "infoButton must show helpPage");
    assert!(!state.summary_shown, "help overlay hides summaryPage");
    assert!(!state.completed_shown, "help overlay hides completedPage");
    assert!(!state.artifact_shown, "help overlay hides artifactPage");
    assert!(
        state.summary_left,
        "help overlay uses summary bgLeft texture"
    );
    assert!(
        state.summary_right,
        "help overlay uses summary bgRight texture"
    );
    assert!(
        !state.completed_left && !state.completed_right,
        "help overlay must not use completed-page backgrounds"
    );
}

fn assert_summary_tab_state(state: &TabState) {
    assert_eq!(state.label, "summary");
    assert!(
        state.current_summary,
        "tab1 must set currentFrame to summaryPage"
    );
    assert!(
        !state.current_completed,
        "tab1 must not leave currentFrame on completedPage"
    );
    assert!(!state.help_shown, "tab1 must hide helpPage");
    assert!(state.summary_shown, "tab1 must show summaryPage");
    assert!(!state.completed_shown, "tab1 must hide completedPage");
    assert!(!state.artifact_shown, "tab1 must hide artifactPage");
    assert!(state.summary_left, "tab1 must use summary bgLeft texture");
    assert!(state.summary_right, "tab1 must use summary bgRight texture");
}

fn assert_completed_tab_state(state: &TabState) {
    assert_eq!(state.label, "completed");
    assert!(
        !state.current_summary,
        "tab2 must not leave currentFrame on summaryPage"
    );
    assert!(
        state.current_completed,
        "tab2 must set currentFrame to completedPage"
    );
    assert!(!state.help_shown, "tab2 must hide helpPage");
    assert!(!state.summary_shown, "tab2 must hide summaryPage");
    assert!(state.completed_shown, "tab2 must show completedPage");
    assert!(!state.artifact_shown, "tab2 must hide artifactPage");
    assert!(
        !state.summary_left && !state.summary_right,
        "tab2 must not use summary-page backgrounds"
    );
    assert!(
        state.completed_left,
        "tab2 must use completed bgLeft texture"
    );
    assert!(
        state.completed_right,
        "tab2 must use completed bgRight texture"
    );
}

const TAB_SWITCH_PROBE_LUA: &str = r#"
local summaryLeftTextureID = 426721
local summaryRightTextureID = 426722
local completedLeftTextureID = 426719
local completedRightTextureID = 426720

local function boolText(value)
    return tostring(value == true)
end

local function pageState(label)
    return table.concat({
        label,
        boolText(ArchaeologyFrame.currentFrame == ArchaeologyFrame.summaryPage),
        boolText(ArchaeologyFrame.currentFrame == ArchaeologyFrame.completedPage),
        boolText(ArchaeologyFrame.helpPage:IsShown()),
        boolText(ArchaeologyFrame.summaryPage:IsShown()),
        boolText(ArchaeologyFrame.completedPage:IsShown()),
        boolText(ArchaeologyFrame.artifactPage:IsShown()),
        boolText(ArchaeologyFrame.bgLeft:GetTexture() == summaryLeftTextureID),
        boolText(ArchaeologyFrame.bgRight:GetTexture() == summaryRightTextureID),
        boolText(ArchaeologyFrame.bgLeft:GetTexture() == completedLeftTextureID),
        boolText(ArchaeologyFrame.bgRight:GetTexture() == completedRightTextureID),
    }, "\t")
end

local states = {}

ArchaeologyFrame.infoButton:Click()
table.insert(states, pageState("help"))

ArchaeologyFrame.tab1:Click()
table.insert(states, pageState("summary"))

ArchaeologyFrame.tab2:Click()
table.insert(states, pageState("completed"))

return table.concat(states, "\n")
"#;
