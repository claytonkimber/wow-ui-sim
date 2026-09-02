local addonName = ...

local MAX_RECORDS = 128
local MAX_STRING_LENGTH = 160
local FLUSH_REMINDER = "Reload or log out to flush StrictRemovalTimingProbeDB."

local SURFACE_PATHS = {
    { name = "C_DyeColor.GetDyeColorForItemLocation", keys = { "C_DyeColor", "GetDyeColorForItemLocation" } },
    { name = "C_DyeColor.GetDyeColorForItem", keys = { "C_DyeColor", "GetDyeColorForItem" } },
    { name = "C_Housing.IsInsideOwnHouse", keys = { "C_Housing", "IsInsideOwnHouse" } },
    { name = "C_Ping.GetContextualPingTypeForUnit", keys = { "C_Ping", "GetContextualPingTypeForUnit" } },
    { name = "C_RecruitAFriend.IsEnabled", keys = { "C_RecruitAFriend", "IsEnabled" } },
    { name = "C_SuperTrack.GetNextWaypointForMap", keys = { "C_SuperTrack", "GetNextWaypointForMap" } },
    { name = "C_UnitAuras.TriggerPrivateAuraShowDispelType", keys = { "C_UnitAuras", "TriggerPrivateAuraShowDispelType" } },
    { name = "Enum.EditModeUnitFrameSetting.IconSize", keys = { "Enum", "EditModeUnitFrameSetting", "IconSize" } },
    { name = "GetInventorySlotInfo", keys = { "GetInventorySlotInfo" } },
    { name = "GetCVar", keys = { "GetCVar" } },
    { name = "GetCVarDefault", keys = { "GetCVarDefault" } },
    { name = "C_CVar.GetCVar", keys = { "C_CVar", "GetCVar" } },
}

local CVAR_APIS = {
    { name = "GetCVar", keys = { "GetCVar" } },
    { name = "GetCVarDefault", keys = { "GetCVarDefault" } },
    { name = "C_CVar.GetCVar", keys = { "C_CVar", "GetCVar" } },
}

local CVAR_NAMES = {
    "lastLockedDelvesCompanionAbilities",
    "slugSuperSampling",
}

local runtime = {
    addonName = addonName,
    dbReady = false,
    pending = {},
    droppedRecords = 0,
    hookAttempted = false,
    hookInstalled = false,
    hookError = nil,
    hookCalls = 0,
    lastHookAddon = nil,
    ownAddonLoaded = false,
    lastStateSignature = nil,
}
StrictRemovalTimingProbeRuntime = runtime

local function boundedString(value)
    if value == nil then
        return nil
    end

    local text = tostring(value)
    if #text > MAX_STRING_LENGTH then
        return text:sub(1, MAX_STRING_LENGTH)
    end
    return text
end

local function typePresence(value)
    return {
        present = value ~= nil,
        type = type(value),
    }
end

local function primitiveSummary(value)
    local luaType = type(value)
    local summary = typePresence(value)

    if luaType == "boolean" or luaType == "number" then
        summary.value = value
    elseif luaType == "string" then
        summary.value = boundedString(value)
    end

    return summary
end

local function directPath(keys)
    local current = _G
    for index = 1, #keys do
        if current == nil then
            return nil
        end

        local key = keys[index]
        local ok, nextValue = pcall(function()
            return current[key]
        end)
        if not ok then
            return nil
        end
        current = nextValue
    end
    return current
end

local function rawPath(keys)
    local current = _G
    for index = 1, #keys do
        if type(current) ~= "table" then
            return nil
        end
        current = rawget(current, keys[index])
    end
    return current
end

local function pathObservation(path)
    return {
        direct = typePresence(directPath(path.keys)),
        raw = typePresence(rawPath(path.keys)),
    }
end

local function captureSurface()
    local result = {}
    for _, path in ipairs(SURFACE_PATHS) do
        result[path.name] = pathObservation(path)
    end
    return result
end

local function callCVar(api, cvarName)
    local functionValue = directPath(api.keys)
    if type(functionValue) ~= "function" then
        return {
            ok = false,
            error = "not callable (" .. type(functionValue) .. ")",
        }
    end

    local ok, value = pcall(functionValue, cvarName)
    if not ok then
        return {
            ok = false,
            error = boundedString(value),
        }
    end

    return {
        ok = true,
        result = primitiveSummary(value),
    }
end

local function captureCVars()
    local result = {}
    for _, cvarName in ipairs(CVAR_NAMES) do
        local cvarResult = {}
        for _, api in ipairs(CVAR_APIS) do
            cvarResult[api.name] = callCVar(api, cvarName)
        end
        result[cvarName] = cvarResult
    end
    return result
end

local function stateSignature(surface, cvars)
    local parts = {}

    for _, path in ipairs(SURFACE_PATHS) do
        local observation = surface[path.name]
        parts[#parts + 1] = table.concat({
            path.name,
            tostring(observation.direct.present),
            observation.direct.type,
            tostring(observation.raw.present),
            observation.raw.type,
        }, ":")
    end

    for _, cvarName in ipairs(CVAR_NAMES) do
        for _, api in ipairs(CVAR_APIS) do
            local observation = cvars[cvarName][api.name]
            local result = observation.result or {}
            parts[#parts + 1] = table.concat({
                cvarName,
                api.name,
                tostring(observation.ok),
                tostring(result.present),
                tostring(result.type),
                tostring(result.value),
                tostring(observation.error),
            }, ":")
        end
    end

    return table.concat(parts, "|")
end

local function syncMetadata()
    if not runtime.dbReady then
        return
    end

    StrictRemovalTimingProbeDB.hookAttempted = runtime.hookAttempted
    StrictRemovalTimingProbeDB.hookInstalled = runtime.hookInstalled
    StrictRemovalTimingProbeDB.hookError = runtime.hookError
    StrictRemovalTimingProbeDB.hookCalls = runtime.hookCalls
    StrictRemovalTimingProbeDB.lastHookAddon = runtime.lastHookAddon
    StrictRemovalTimingProbeDB.droppedRecords = runtime.droppedRecords
end

local function appendToDatabase(entry)
    local records = StrictRemovalTimingProbeDB.records
    if #records >= MAX_RECORDS then
        runtime.droppedRecords = runtime.droppedRecords + 1
        return
    end
    records[#records + 1] = entry
end

local function bindSavedVariables()
    if runtime.dbReady then
        return
    end

    if type(StrictRemovalTimingProbeDB) ~= "table" then
        StrictRemovalTimingProbeDB = {}
    end
    if type(StrictRemovalTimingProbeDB.records) ~= "table" then
        StrictRemovalTimingProbeDB.records = {}
    end
    while #StrictRemovalTimingProbeDB.records > MAX_RECORDS do
        table.remove(StrictRemovalTimingProbeDB.records, 1)
    end

    local version, build, buildDate, tocVersion = GetBuildInfo()
    StrictRemovalTimingProbeDB.schema = 1
    StrictRemovalTimingProbeDB.addonName = boundedString(addonName)
    StrictRemovalTimingProbeDB.version = boundedString(version)
    StrictRemovalTimingProbeDB.build = boundedString(build)
    StrictRemovalTimingProbeDB.buildDate = boundedString(buildDate)
    StrictRemovalTimingProbeDB.interface = boundedString(tocVersion)
    runtime.dbReady = true

    for _, entry in ipairs(runtime.pending) do
        appendToDatabase(entry)
    end
    runtime.pending = {}
    syncMetadata()
end

local function recordPhase(phase, extra)
    local surface = captureSurface()
    local cvars = captureCVars()
    local signature = stateSignature(surface, cvars)
    if extra and extra.onlyIfChanged and signature == runtime.lastStateSignature then
        return nil
    end
    runtime.lastStateSignature = signature

    local entry = {
        phase = boundedString(phase),
        capturedAt = time(),
        surface = surface,
        cvars = cvars,
    }

    if extra then
        if extra.addon ~= nil then
            entry.addon = boundedString(extra.addon)
        end
        if extra.label ~= nil then
            entry.label = boundedString(extra.label)
        end
    end

    if runtime.dbReady then
        appendToDatabase(entry)
    elseif #runtime.pending < MAX_RECORDS then
        runtime.pending[#runtime.pending + 1] = entry
    else
        runtime.droppedRecords = runtime.droppedRecords + 1
    end
    syncMetadata()
    return entry
end

runtime.recordPhase = recordPhase

local function installLoadAddonHook()
    runtime.hookAttempted = true

    if type(hooksecurefunc) ~= "function" then
        runtime.hookError = "hooksecurefunc unavailable"
        return
    end

    local cAddOns = directPath({ "C_AddOns" })
    if type(cAddOns) ~= "table" or type(cAddOns.LoadAddOn) ~= "function" then
        runtime.hookError = "C_AddOns.LoadAddOn unavailable"
        return
    end

    local ok, errorMessage = pcall(function()
        hooksecurefunc(cAddOns, "LoadAddOn", function(loadedAddon)
            runtime.hookCalls = runtime.hookCalls + 1
            runtime.lastHookAddon = boundedString(loadedAddon)
            syncMetadata()
        end)
    end)
    runtime.hookInstalled = ok
    if not ok then
        runtime.hookError = boundedString(errorMessage)
    end
end

local function resetCapture()
    runtime.pending = {}
    runtime.droppedRecords = 0
    if runtime.dbReady then
        StrictRemovalTimingProbeDB.records = {}
        syncMetadata()
    end
end

local function printStatus()
    bindSavedVariables()
    local db = StrictRemovalTimingProbeDB
    print(("%s: %d/%d records; dropped=%d; hook installed=%s; hook calls=%d"):format(
        addonName,
        #db.records,
        MAX_RECORDS,
        db.droppedRecords or 0,
        tostring(db.hookInstalled),
        db.hookCalls or 0
    ))
    if db.hookError then
        print("  hook error: " .. tostring(db.hookError))
    end
    print(FLUSH_REMINDER)
end

local frame = CreateFrame("Frame")
frame:RegisterEvent("ADDON_LOADED")
frame:RegisterEvent("VARIABLES_LOADED")
frame:RegisterEvent("PLAYER_LOGIN")
frame:RegisterEvent("PLAYER_ENTERING_WORLD")
frame:SetScript("OnEvent", function(_, event, ...)
    if event == "ADDON_LOADED" then
        local loadedAddon = ...
        if loadedAddon == addonName then
            runtime.ownAddonLoaded = true
            bindSavedVariables()
            recordPhase("addon-loaded:self")
        elseif runtime.ownAddonLoaded then
            recordPhase("addon-loaded:state-change", { addon = loadedAddon, onlyIfChanged = true })
        end
        return
    end

    if event == "VARIABLES_LOADED" then
        bindSavedVariables()
        recordPhase("variables-loaded")
    elseif event == "PLAYER_LOGIN" then
        bindSavedVariables()
        recordPhase("player-login")
    elseif event == "PLAYER_ENTERING_WORLD" then
        bindSavedVariables()
        recordPhase("player-entering-world")
    end
end)

SLASH_STRICTREMOVALTIMINGPROBE1 = "/srtp"
SLASH_STRICTREMOVALTIMINGPROBE2 = "/strictremovalprobe"
SlashCmdList.STRICTREMOVALTIMINGPROBE = function(message)
    local action, rest = (message or ""):match("^%s*(%S*)%s*(.-)%s*$")
    action = action:lower()

    if action == "snapshot" then
        bindSavedVariables()
        recordPhase("manual", { label = rest ~= "" and rest or "snapshot" })
        print(addonName .. ": manual snapshot captured")
        print(FLUSH_REMINDER)
    elseif action == "reset" then
        resetCapture()
        print(addonName .. ": records reset")
        print(FLUSH_REMINDER)
    elseif action == "status" or action == "" then
        printStatus()
    else
        print("Usage: /srtp snapshot [label] | /srtp status | /srtp reset")
    end
end

installLoadAddonHook()
recordPhase("bootstrap-file")
