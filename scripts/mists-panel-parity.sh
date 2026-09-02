#!/usr/bin/env bash
# Scripted Mists panel parity runner.
#
# Reads docs/baselines/mists-panels.md, opens every panel row through wow-sim,
# records per-panel lua-errors JSON, dumps the root frame tree, and captures a
# filtered screenshot. A panel fails if its scripted root frame is missing,
# hidden, visually empty, materially different from its visual baseline, or
# emits Lua/exec errors.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DEFAULT_MISTS_CARGO_TARGET_DIR="${XDG_CACHE_HOME:-$HOME/.cache}/wow-ui-sim/cargo-targets/mists-panel-parity"
CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-${MISTS_CARGO_TARGET_DIR:-$DEFAULT_MISTS_CARGO_TARGET_DIR}}"
BASELINE="$REPO_ROOT/docs/baselines/mists-panels.md"
VISUAL_BASELINE="$REPO_ROOT/docs/baselines/mists-panel-visuals.tsv"
OUT_DIR="$REPO_ROOT/target/mists-panel-parity"
WOW_SIM_BIN="${WOW_SIM_BIN:-$CARGO_TARGET_DIR/debug/wow-sim}"
DEFAULT_VISUAL_METRICS_BIN="$CARGO_TARGET_DIR/debug/panel-visual-metrics"
PANEL_VISUAL_METRICS_BIN="${PANEL_VISUAL_METRICS_BIN:-$DEFAULT_VISUAL_METRICS_BIN}"
TIMEOUT_SECONDS=120
PANEL_FILTER=""
VALIDATE_ONLY=0
SKIP_BUILD=0
LOAD_SAVED_VARS=0
LOAD_THIRD_PARTY_ADDONS=0
UPDATE_VISUAL_BASELINE=0
SIGNAL_ONLY_VISUALS="${MISTS_PANEL_SIGNAL_ONLY:-0}"
VISUAL_UPDATE_FILE=""
OUT_DIR_SET=0

export CARGO_TARGET_DIR

usage() {
    sed -n '2,/^set -euo/p' "$0" | sed 's/^# \?//;$d'
}

while [ $# -gt 0 ]; do
    case "$1" in
        --baseline) BASELINE="$2"; shift 2 ;;
        --out-dir) OUT_DIR="$2"; OUT_DIR_SET=1; shift 2 ;;
        --visual-baseline) VISUAL_BASELINE="$2"; shift 2 ;;
        --panel) PANEL_FILTER="$2"; shift 2 ;;
        --timeout) TIMEOUT_SECONDS="$2"; shift 2 ;;
        --skip-build) SKIP_BUILD=1; shift ;;
        --with-saved-vars) LOAD_SAVED_VARS=1; shift ;;
        --with-addons) LOAD_THIRD_PARTY_ADDONS=1; shift ;;
        --update-visual-baseline) UPDATE_VISUAL_BASELINE=1; shift ;;
        --signal-only-visuals) SIGNAL_ONLY_VISUALS=1; shift ;;
        --validate-only) VALIDATE_ONLY=1; shift ;;
        --help|-h) usage; exit 0 ;;
        *) echo "ERROR: unknown argument '$1'" >&2; exit 2 ;;
    esac
done

if [ "$LOAD_SAVED_VARS" -eq 1 ] && [ "$OUT_DIR_SET" -eq 0 ]; then
    OUT_DIR="$REPO_ROOT/target/mists-panel-parity-with-saved-vars"
fi

case "$OUT_DIR" in
    /*) ;;
    *) OUT_DIR="$PWD/$OUT_DIR" ;;
esac

SIM_SAVED_VAR_ARGS=(--no-saved-vars)
if [ "$LOAD_SAVED_VARS" -eq 1 ]; then
    SIM_SAVED_VAR_ARGS=()
fi

SIM_ADDON_ARGS=(--no-addons)
if [ "$LOAD_THIRD_PARTY_ADDONS" -eq 1 ]; then
    SIM_ADDON_ARGS=()
fi

declare -A PANEL_SLUGS=()
declare -A PANEL_ROOTS=()
declare -A PANEL_OPENERS=()
declare -A PANEL_MIN_ROOT_AREAS=()
declare -A PANEL_MIN_FOREGROUND_PIXELS=()
declare -A PANEL_MIN_FOREGROUND_BBOX_AREAS=()

DEFAULT_MIN_ROOT_AREA=25000
DEFAULT_MIN_FOREGROUND_PIXELS=500
DEFAULT_MIN_FOREGROUND_BBOX_AREA=5000

add_panel() {
    local panel="$1" slug="$2" root="$3" opener="$4"
    PANEL_SLUGS["$panel"]="$slug"
    PANEL_ROOTS["$panel"]="$root"
    PANEL_OPENERS["$panel"]="$opener"
}

set_panel_signal_gate() {
    local slug="$1" min_root_area="$2" min_foreground_pixels="$3" min_foreground_bbox_area="$4"
    PANEL_MIN_ROOT_AREAS["$slug"]="$min_root_area"
    PANEL_MIN_FOREGROUND_PIXELS["$slug"]="$min_foreground_pixels"
    PANEL_MIN_FOREGROUND_BBOX_AREAS["$slug"]="$min_foreground_bbox_area"
}

add_panel "Character panel: paperdoll, stats, titles, equipment manager" "character" "CharacterFrame" 'ToggleCharacter("PaperDollFrame")'
add_panel "Spellbook and professions" "spellbook-professions" "SpellBookFrame" 'ToggleSpellBook(BOOKTYPE_SPELL)'
add_panel "Talents and glyphs" "talents-glyphs" "PlayerTalentFrame" 'ToggleTalentFrame()'
add_panel "Quest log and objective tracker" "quest-log" "QuestLogFrame" 'ToggleQuestLog()'
add_panel "World map" "world-map" "WorldMapFrame" 'ToggleWorldMap()'
add_panel "Mail: inbox, send, attachments, COD" "mail" "MailFrame" 'MailFrame_Show()'
add_panel "Auction House: browse, bid, post, cancel" "auction-house" "AuctionFrame" 'AuctionFrame_LoadUI(); AuctionFrame_Show()'
add_panel "AddOn list and UI management LoD panels" "addon-list" "AddonList" 'LoadAddOn("Blizzard_AddOnList"); AddonList_Show()'
add_panel "Bank, ReagentBank, Void Storage, Guild Bank" "bank-storage" "BankFrame" 'FireEvent("BANKFRAME_OPENED")'
add_panel "Trade window and \`TradePlayerInputMoneyFrame\`" "trade" "TradeFrame" 'A_Admin.SetMoney(100000); InitiateTrade("NPC"); FireEvent("TRADE_SHOW")'
add_panel "Friends, Who, Guild, Communities, Club Finder" "social" "FriendsFrame" 'ToggleFriendsFrame(1); FriendsFrame_ShowSubFrame("FriendsListFrame")'
add_panel "Inspect and guild control LoD panels" "inspect-guild-control" "InspectFrame" 'LoadAddOn("Blizzard_GuildControlUI"); ShowUIPanel(GuildControlUI); assertRenderableRoot("GuildControlUI"); LoadAddOn("Blizzard_InspectUI"); InspectFrame_Show("player"); FireEvent("INSPECT_READY", UnitGUID("player"))'
add_panel "PvP UI: HonorFrame, BG queue, Conquest" "pvp" "PVPQueueFrame" 'LoadAddOn("Blizzard_PVPUI"); PVEFrame_ShowFrame("PVPQueueFrame", "HonorQueueFrame")'
add_panel "LFG, LFR, Raid Browser" "lfg-lfr" "PVEFrame" 'LoadAddOn("Blizzard_PVEUI"); PVEFrame:Show(); GroupFinderFrame:Show()'
add_panel "Raid unit frames LoD panel" "raid-unit-frames" "RaidParentFrame" 'A_Admin.SetPartySize(6); A_Admin.SetInstanceInfo("Vault of Archavon", "raid", 16, 20); for i = 1, 6 do A_Admin.SetPartyMember(i, "Raider" .. i, ((i - 1) % 11) + 1, 90) end; LoadAddOn("Blizzard_RaidUI"); RaidParentFrame:Show(); RaidParentFrame_SetView(1); RaidFrame:Show(); RaidGroupFrame_Update(); for i = 1, 8 do local group = _G["RaidGroup" .. i]; if group then group:Show() end end'
add_panel "Arena enemy unit frames LoD panel" "arena-unit-frames" "ArenaEnemyFrame1" 'A_Admin.SetInstanceInfo("Nagrand Arena", "arena", 0, 5); LoadAddOn("Blizzard_ArenaUI"); ArenaEnemyFrames_Enable(ArenaEnemyFrames); for i = 1, 5 do local frame = _G["ArenaEnemyFrame" .. i]; if frame then ArenaEnemyFrame_SetMysteryPlayer(frame); frame:Show() end end'
add_panel "Battlefield map LoD panel" "battlefield-map" "BattlefieldMapFrame" 'ToggleBattlefieldMap(); BattlefieldMapFrame:RefreshAllDataProviders()'
add_panel "Collections: mounts, pets, toys, heirlooms, transmog" "collections" "CollectionsJournal" 'ToggleCollectionsJournal()'
add_panel "Pet Journal and Battle Pet UI" "pet-journal" "CollectionsJournal" 'ToggleCollectionsJournal(COLLECTIONS_JOURNAL_TAB_INDEX_PETS)'
add_panel "Achievements and Calendar" "achievements-calendar" "AchievementFrame" 'ToggleAchievementFrame()'
add_panel "Archaeology panel" "archaeology" "ArchaeologyFrame" 'ArchaeologyFrame_LoadUI(); ShowUIPanel(ArchaeologyFrame)'
add_panel "Craft panel" "craft" "CraftFrame" 'CraftFrame_LoadUI(); ShowUIPanel(CraftFrame)'
add_panel "TradeSkill panel" "trade-skill" "TradeSkillFrame" 'TradeSkillFrame_LoadUI(); ShowUIPanel(TradeSkillFrame)'
add_panel "Class trainer LoD panel" "class-trainer" "ClassTrainerFrame" 'ClassTrainerFrame_LoadUI(); ClassTrainerFrame_Show()'
add_panel "Encounter Journal" "encounter-journal" "EncounterJournal" 'ToggleEncounterJournal()'
add_panel "Challenge mode LoD panel" "challenges" "ChallengesFrame" 'LoadAddOn("Blizzard_ChallengesUI"); LoadAddOn("Blizzard_PVEUI"); PVEFrame:Show(); ChallengesFrame:Show()'
add_panel "Currency and Token UI" "currency-token" "TokenFrame" 'ToggleCharacter("TokenFrame")'
add_panel "Store, CatalogShop, WowToken, and SimpleCheckout" "store-commercial" "CatalogShopFrame" 'LoadAddOn("Blizzard_CatalogShop"); LoadAddOn("Blizzard_WowTokenUI"); LoadAddOn("Blizzard_SimpleCheckout"); CatalogShopFrame:Show(); CatalogShopFrame.ProductContainerFrame:Show(); SimpleCheckout:CalculateDesiredSize(); SimpleCheckout:RecalculateSize(); SimpleCheckout:Show(); assertRenderableRoot("SimpleCheckout"); SimpleCheckout:Hide()'
add_panel "Item socketing, reforging, and upgrade LoD panels" "item-services" "ItemUpgradeFrame" 'ItemSocketingFrame_LoadUI(); ShowUIPanel(ItemSocketingFrame); assertRenderableRoot("ItemSocketingFrame"); Reforging_LoadUI(); ReforgingFrame_Show(); assertRenderableRoot("ReforgingFrame"); ItemUpgrade_LoadUI(); ItemUpgradeFrame_Show()'
add_panel "NPC service LoD panels: barber and black market" "npc-services" "BlackMarketFrame" 'LoadAddOn("Blizzard_BarbershopUI"); ShowUIPanel(BarberShopFrame); assertRenderableRoot("BarberShopFrame"); BlackMarket_LoadUI(); BlackMarketFrame_Show()'
add_panel "Quest choice LoD dialog" "quest-choice" "QuestChoiceFrame" 'QuestChoice_LoadUI(); ShowUIPanel(QuestChoiceFrame)'
add_panel "Macro and key bindings" "macro-keybindings" "SettingsPanel" 'SettingsPanel:OpenToCategory(KEY_BINDINGS)'
add_panel "Interface options" "interface-options" "SettingsPanel" 'ToggleGameMenu(); GameMenuButtonOptions:Click(); SettingsPanel:OpenToCategory(Settings.INTERFACE_CATEGORY_ID)'
add_panel "Action bars, micro menu, bag bar, status bars" "action-bars" "MainMenuBar" 'MainMenuBar:Show()'
add_panel "Time manager and move pad LoD utilities" "time-move-utilities" "TimeManagerFrame" 'LoadAddOn("Blizzard_MovePad"); MovePadFrame:Show(); assertRenderableRoot("MovePadFrame"); LoadAddOn("Blizzard_TimeManager"); ShowUIPanel(TimeManagerFrame)'
add_panel "Nameplates" "nameplates" "MistsNamePlateRenderProbe" 'local plate = CreateFrame("Frame", "MistsNamePlateRenderProbe", UIParent); plate:SetSize(128, 32); plate:SetPoint("CENTER"); plate:Show(); NamePlateDriverFrame:OnNamePlateCreated(plate); NamePlateDriverFrame:AcquireUnitFrame(plate); CompactUnitFrame_SetUpFrame(plate.UnitFrame, DefaultCompactNamePlateEnemyFrameSetup); plate.UnitFrame:Show()'
add_panel "Loot, group loot, personal loot" "loot" "LootFrame" 'A_Admin.ClearLoot(); A_Admin.AddLootItem(6948, 1); FireEvent("LOOT_OPENED", false)'
add_panel "Game menu options" "game-menu-options" "SettingsPanel" 'ToggleGameMenu(); GameMenuButtonOptions:Click()'

set_panel_signal_gate "nameplates" 3000 10 500
set_panel_signal_gate "arena-unit-frames" 3000 10 500

trim() {
    local value="$*"
    value="${value#"${value%%[![:space:]]*}"}"
    value="${value%"${value##*[![:space:]]}"}"
    printf '%s' "$value"
}

read_baseline_panels() {
    local line panel status
    while IFS= read -r line || [ -n "$line" ]; do
        [[ "$line" == \|* ]] || continue
        IFS='|' read -r _ panel status _ <<< "$line"
        panel="$(trim "$panel")"
        status="$(trim "$status")"
        [ "$panel" = "Panel" ] && continue
        [ "$panel" = "---" ] && continue
        [ -z "$panel" ] && continue
        [ "$status" = "Pass" ] || continue
        printf '%s\n' "$panel"
    done < "$BASELINE"
}

matches_panel_filter() {
    local panel="$1" slug="$2"
    [ -z "$PANEL_FILTER" ] && return 0
    [ "$PANEL_FILTER" = "$slug" ] && return 0
    [[ "${panel,,}" == *"${PANEL_FILTER,,}"* ]]
}

validate_manifest() {
    [ -f "$BASELINE" ] || { echo "ERROR: baseline not found: $BASELINE" >&2; return 2; }
    if [ "$UPDATE_VISUAL_BASELINE" -eq 0 ] && [ "$SIGNAL_ONLY_VISUALS" -eq 0 ] && [ ! -f "$VISUAL_BASELINE" ]; then
        echo "ERROR: visual baseline not found: $VISUAL_BASELINE" >&2
        return 2
    fi

    local panel slug count=0 missing=0
    while IFS= read -r panel; do
        count=$((count + 1))
        local panel_key="$panel"
        slug="${PANEL_SLUGS[$panel_key]:-}"
        if [ -z "$slug" ]; then
            echo "ERROR: no runner case for panel row: $panel" >&2
            missing=$((missing + 1))
        fi
    done < <(read_baseline_panels)

    if [ "$count" -eq 0 ]; then
        echo "ERROR: no Pass panel rows found in $BASELINE" >&2
        return 2
    fi
    if [ "$missing" -gt 0 ]; then
        return 2
    fi

    echo "$count panel rows validated from $BASELINE"
}

visual_metrics_bin() {
    echo "$PANEL_VISUAL_METRICS_BIN"
}

build_visual_metrics() {
    if [ ! -x "$(visual_metrics_bin)" ]; then
        echo "Building panel-visual-metrics in $CARGO_TARGET_DIR"
        cargo build --bin panel-visual-metrics
    fi
}

write_panel_lua() {
    local panel="$1" root="$2" lua_file="$3"
    local panel_key="$panel"
    cat > "$lua_file" <<LUA
local function assertRenderableRoot(name)
    local frame = _G[name]
    if frame == nil then
        error(name .. " missing")
    end
    if frame.IsShown and not frame:IsShown() then
        error(name .. " hidden")
    end
    if frame.GetWidth and (frame:GetWidth() or 0) <= 0 then
        error(name .. " has no width")
    end
    if frame.GetHeight and (frame:GetHeight() or 0) <= 0 then
        error(name .. " has no height")
    end
end

${PANEL_OPENERS[$panel_key]}
assertRenderableRoot("$root")
LUA
}

run_wow_sim() {
    local env_args=()
    if [ "$LOAD_THIRD_PARTY_ADDONS" -eq 0 ]; then
        env_args+=(WOW_SIM_NO_ADDONS=1)
    fi
    if [ "$LOAD_SAVED_VARS" -eq 0 ]; then
        env_args+=(WOW_SIM_NO_SAVED_VARS=1)
    fi
    env "${env_args[@]}" timeout "$TIMEOUT_SECONDS" "$WOW_SIM_BIN" "$@"
}

fail_if_runtime_log_error() {
    local label="$1" stderr_file="$2" stdout_file="$3"
    if grep -qE 'Lua error|\[exec-lua\] error|\[TexMgr\] Load error' "$stderr_file" "$stdout_file"; then
        echo "ERROR: $label emitted runtime log errors" >&2
        return 1
    fi
}

verify_lua_errors_json() {
    local panel="$1" json_file="$2"
    local count
    count="$(jq 'length' "$json_file")"
    if [ "$count" != "0" ]; then
        echo "ERROR: $panel produced $count lua-errors" >&2
        jq '.' "$json_file" >&2
        return 1
    fi
}

verify_dump_tree() {
    local panel="$1" slug="$2" root="$3" dump_file="$4"
    if ! grep -qE "(^|[[:space:]])${root//\\/\\\\} \\[[^]]+\\].* visible " "$dump_file"; then
        echo "ERROR: $panel root $root is missing or hidden in dump tree" >&2
        return 1
    fi

    local min_root_area max_root_area
    local slug_key="$slug"
    min_root_area="${PANEL_MIN_ROOT_AREAS[$slug_key]:-$DEFAULT_MIN_ROOT_AREA}"
    max_root_area="$(max_visible_root_area "$root" "$dump_file")"
    if [ "$max_root_area" -lt "$min_root_area" ]; then
        echo "ERROR: $panel root $root bounding box area $max_root_area is below minimum $min_root_area" >&2
        return 1
    fi

    local visible_renderables
    visible_renderables="$(grep -Ec '\[(Texture|FontString|Button|StatusBar)\].* visible ' "$dump_file" || true)"
    if [ "$visible_renderables" -eq 0 ]; then
        echo "ERROR: $panel dump tree has no visible renderable descendants" >&2
        return 1
    fi
}

max_visible_root_area() {
    local root="$1" dump_file="$2"
    awk -v root="$root" '
        $0 ~ "(^|[[:space:]])" root " \\[[^]]+\\].* visible " {
            if (match($0, /\(([0-9]+)x([0-9]+)\)/, dimensions)) {
                area = dimensions[1] * dimensions[2]
                if (area > max) {
                    max = area
                }
            }
        }
        END {
            print max + 0
        }
    ' "$dump_file"
}

verify_screenshot() {
    local panel="$1" stderr_file="$2" screenshot_file="$3"
    local quads
    quads="$(sed -n 's/.*QuadBatch: \([0-9][0-9]*\) quads.*/\1/p' "$stderr_file" | tail -1)"
    if [ -z "$quads" ] || [ "$quads" -eq 0 ]; then
        echo "ERROR: $panel screenshot emitted an empty render batch" >&2
        return 1
    fi
    if [ ! -s "$screenshot_file" ]; then
        echo "ERROR: $panel screenshot was not written: $screenshot_file" >&2
        return 1
    fi
}

verify_visual_signal() {
    local slug="$1" screenshot_file="$2"
    local min_foreground_pixels min_foreground_bbox_area
    local slug_key="$slug"
    min_foreground_pixels="${PANEL_MIN_FOREGROUND_PIXELS[$slug_key]:-$DEFAULT_MIN_FOREGROUND_PIXELS}"
    min_foreground_bbox_area="${PANEL_MIN_FOREGROUND_BBOX_AREAS[$slug_key]:-$DEFAULT_MIN_FOREGROUND_BBOX_AREA}"
    "$(visual_metrics_bin)" signal \
        "$slug" \
        "$screenshot_file" \
        "$min_foreground_pixels" \
        "$min_foreground_bbox_area"
}

verify_visual_baseline() {
    local slug="$1" screenshot_file="$2"
    if [ "$SIGNAL_ONLY_VISUALS" -eq 1 ]; then
        return 0
    fi
    if [ "$UPDATE_VISUAL_BASELINE" -eq 1 ]; then
        "$(visual_metrics_bin)" record "$slug" "$screenshot_file" >> "$VISUAL_UPDATE_FILE"
    else
        "$(visual_metrics_bin)" compare "$VISUAL_BASELINE" "$slug" "$screenshot_file"
    fi
}

run_panel() {
    local panel="$1"
    local panel_key="$panel"
    local slug="${PANEL_SLUGS[$panel_key]}"
    local root="${PANEL_ROOTS[$panel_key]}"
    local panel_dir="$OUT_DIR/$slug"
    local lua_file="$panel_dir/open.lua"
    local json_file="$panel_dir/lua-errors.json"
    local lua_stderr="$panel_dir/lua-errors.stderr"
    local dump_file="$panel_dir/dump-tree.txt"
    local dump_stderr="$panel_dir/dump-tree.stderr"
    local screenshot_base="$panel_dir/screenshot"
    local screenshot_file="$panel_dir/screenshot.webp"
    local screenshot_stderr="$panel_dir/screenshot.stderr"
    local screenshot_stdout="$panel_dir/screenshot.stdout"

    mkdir -p "$panel_dir"
    write_panel_lua "$panel" "$root" "$lua_file"

    echo "=== $slug: $panel ==="
    run_wow_sim "${SIM_ADDON_ARGS[@]}" "${SIM_SAVED_VAR_ARGS[@]}" --exec-lua "@$lua_file" lua-errors \
        > "$json_file" 2> "$lua_stderr"
    fail_if_runtime_log_error "$panel lua-errors" "$lua_stderr" "$json_file"
    verify_lua_errors_json "$panel" "$json_file"

    run_wow_sim "${SIM_ADDON_ARGS[@]}" "${SIM_SAVED_VAR_ARGS[@]}" --exec-lua "@$lua_file" dump-tree --filter-key "$root" \
        > "$dump_file" 2> "$dump_stderr"
    fail_if_runtime_log_error "$panel dump-tree" "$dump_stderr" "$dump_file"
    verify_dump_tree "$panel" "$slug" "$root" "$dump_file"

    run_wow_sim "${SIM_ADDON_ARGS[@]}" "${SIM_SAVED_VAR_ARGS[@]}" --exec-lua "@$lua_file" screenshot \
        --filter "$root" --output "$screenshot_base" \
        > "$screenshot_stdout" 2> "$screenshot_stderr"
    fail_if_runtime_log_error "$panel screenshot" "$screenshot_stderr" "$screenshot_stdout"
    verify_screenshot "$panel" "$screenshot_stderr" "$screenshot_file"
    verify_visual_signal "$slug" "$screenshot_file"
    verify_visual_baseline "$slug" "$screenshot_file"
}

validate_manifest
if [ "$VALIDATE_ONLY" -eq 1 ]; then
    exit 0
fi

mkdir -p "$OUT_DIR"
if [ "$SKIP_BUILD" -eq 0 ]; then
    echo "Building Mists wow-sim in $CARGO_TARGET_DIR"
    cargo build --bin wow-sim --no-default-features --features "sound,gui,casc,client-mists"
fi
build_visual_metrics

if [ "$UPDATE_VISUAL_BASELINE" -eq 1 ]; then
    mkdir -p "$(dirname "$VISUAL_BASELINE")"
    VISUAL_UPDATE_FILE="$OUT_DIR/mists-panel-visuals.tsv"
    printf '# slug\twidth\theight\tactive_pixels\tluma_stddev_milli\tahash\n' > "$VISUAL_UPDATE_FILE"
fi

selected=0
while IFS= read -r panel; do
    panel_key="$panel"
    slug="${PANEL_SLUGS[$panel_key]}"
    if matches_panel_filter "$panel" "$slug"; then
        selected=$((selected + 1))
        run_panel "$panel"
    fi
done < <(read_baseline_panels)

if [ "$UPDATE_VISUAL_BASELINE" -eq 1 ]; then
    mv "$VISUAL_UPDATE_FILE" "$VISUAL_BASELINE"
    echo "Updated visual baseline: $VISUAL_BASELINE"
fi

if [ "$selected" -eq 0 ]; then
    echo "ERROR: no panel matched filter '$PANEL_FILTER'" >&2
    exit 2
fi

echo "Mists panel parity passed for $selected panel(s). Artifacts: $OUT_DIR"
