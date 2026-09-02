local addonName = ...
local PREFIX = "UASP"
local MAX_TEXT = 200
local MAX_FIELDS = 64
local MAX_EVENT_ENTRIES = 200
local MAX_ADDED_AURAS = 10

local pendingEvents = {}
local dbReady = false
local currentPhase = "startup"

local function pack(...)
    return { n = select("#", ...), ... }
end

local function boundedText(value)
    if #value <= MAX_TEXT then
        return value
    end
    return value:sub(1, MAX_TEXT) .. "..."
end

local function invoke(fn)
    local results = pack(pcall(fn))
    local values = { n = math.max(0, results.n - 1) }
    for index = 2, results.n do
        values[index - 1] = results[index]
    end
    return results[1], values
end

local function probeBoolean(fn)
    local ok, values = invoke(fn)
    local value
    if ok then
        value = values[1] == true
    end
    return {
        ok = ok,
        value = value,
    }
end

local function summarizeValue(value)
    local valueType = type(value)
    local secretSupported = type(issecretvalue) == "function"
    local accessSupported = type(canaccessvalue) == "function"
    local secret = secretSupported and probeBoolean(function()
        return issecretvalue(value)
    end) or { ok = false, unavailable = true }
    local accessible = accessSupported and probeBoolean(function()
        return canaccessvalue(value)
    end) or { ok = false, unavailable = true }
    local textOk, textValues = invoke(function()
        return tostring(value)
    end)
    local safeToRecord = secret.ok and secret.value == false
        and accessible.ok and accessible.value == true

    local summary = {
        type = valueType,
        secretProbeOk = secret.ok,
        secretProbeUnavailable = secret.unavailable,
        secret = secret.value,
        accessProbeOk = accessible.ok,
        accessProbeUnavailable = accessible.unavailable,
        accessible = accessible.value,
        tostringOk = textOk,
        tostring = textOk and safeToRecord and boundedText(textValues[1]) or nil,
        tostringRedacted = textOk and not safeToRecord or nil,
    }
    if textOk == false and textValues.n > 0 then
        local errorOk, errorValues = invoke(function()
            return tostring(textValues[1])
        end)
        summary.tostringError = errorOk and boundedText(errorValues[1]) or "<unprintable error>"
    end

    if safeToRecord then
        if valueType == "string" then
            summary.value = boundedText(value)
        elseif valueType == "number" or valueType == "boolean" then
            summary.value = value
        elseif valueType == "nil" then
            summary.value = "<nil>"
        end
    end
    return summary
end

local function summarizeInvocation(ok, values)
    local result = {
        ok = ok,
        returnCount = ok and values.n or 0,
        returns = {},
    }
    if ok then
        for index = 1, values.n do
            result.returns[index] = summarizeValue(values[index])
        end
    elseif values.n > 0 then
        result.error = summarizeValue(values[1])
    end
    return result
end

local function observeOperation(fn)
    local ok, values = invoke(fn)
    return summarizeInvocation(ok, values)
end

local function observeField(container, key, iteratedValue)
    local record = {
        key = summarizeValue(key),
        iteratedValue = summarizeValue(iteratedValue),
        directRead = observeOperation(function()
            return container[key]
        end),
        rawRead = observeOperation(function()
            return rawget(container, key)
        end),
        selfEquality = observeOperation(function()
            return iteratedValue == iteratedValue
        end),
        stringify = observeOperation(function()
            return tostring(iteratedValue)
        end),
        concatenate = observeOperation(function()
            return "" .. iteratedValue
        end),
        passThrough = observeOperation(function()
            return (function(value) return value end)(iteratedValue)
        end),
        tableStore = observeOperation(function()
            local storage = { iteratedValue }
            return storage[1]
        end),
    }
    if type(iteratedValue) == "number" then
        record.arithmetic = observeOperation(function()
            return iteratedValue + 0
        end)
    end
    return record
end

local function observeTable(value)
    local record = {
        value = summarizeValue(value),
        metatable = observeOperation(function()
            return getmetatable(value)
        end),
        fields = {},
    }
    local ok, values = invoke(function()
        local count = 0
        for key, child in pairs(value) do
            count = count + 1
            if count > MAX_FIELDS then
                return count, true
            end
            record.fields[count] = observeField(value, key, child)
        end
        return count, false
    end)
    record.iteration = summarizeInvocation(ok, values)
    return record
end

local function observeArray(value, inspectAuraTables)
    local record = {
        value = summarizeValue(value),
        entries = {},
    }
    local ok, values = invoke(function()
        local count = math.min(#value, inspectAuraTables and MAX_ADDED_AURAS or MAX_FIELDS)
        for index = 1, count do
            local entry = value[index]
            if inspectAuraTables and type(entry) == "table" then
                record.entries[index] = observeTable(entry)
            else
                record.entries[index] = summarizeValue(entry)
            end
        end
        return #value, count
    end)
    record.iteration = summarizeInvocation(ok, values)
    return record
end

local function observeAuraTable(value)
    if type(value) ~= "table" then
        return { value = summarizeValue(value) }
    end
    return observeTable(value)
end

local function captureCall(fn, inspectFirstTable)
    local ok, values = invoke(fn)
    local record = summarizeInvocation(ok, values)
    if ok and inspectFirstTable and values.n > 0 then
        record.firstValue = observeAuraTable(values[1])
    end
    return record, ok and values[1] or nil
end

local function readActualField(container, key)
    if type(container) ~= "table" then
        return false, nil
    end
    local ok, values = invoke(function()
        return container[key]
    end)
    return ok, values[1]
end

local function captureKnownUpdateFields(updateInfo)
    if type(updateInfo) ~= "table" then
        return nil
    end
    local result = {}
    for _, key in ipairs({
        "isFullUpdate",
        "updatedAuraInstanceIDs",
        "removedAuraInstanceIDs",
        "addedAuras",
    }) do
        local ok, value = readActualField(updateInfo, key)
        result[key] = {
            readOk = ok,
            value = summarizeValue(value),
        }
        if ok and type(value) == "table" then
            result[key].array = observeArray(value, key == "addedAuras")
        end
    end
    return result
end

local function buildInfo()
    local version, build, buildDate, interfaceVersion = GetBuildInfo()
    return {
        version = version,
        build = build,
        buildDate = buildDate,
        interface = interfaceVersion,
    }
end

local function bindSavedVariables()
    if dbReady then
        return
    end
    UnitAuraSecretProbeDB = UnitAuraSecretProbeDB or {}
    UnitAuraSecretProbeDB.schema = 1
    UnitAuraSecretProbeDB.addonName = addonName
    UnitAuraSecretProbeDB.build = buildInfo()
    UnitAuraSecretProbeDB.captures = UnitAuraSecretProbeDB.captures or {}
    UnitAuraSecretProbeDB.events = UnitAuraSecretProbeDB.events or {}
    UnitAuraSecretProbeDB.markers = UnitAuraSecretProbeDB.markers or {}
    UnitAuraSecretProbeDB.truncatedEvents = UnitAuraSecretProbeDB.truncatedEvents or 0
    dbReady = true
    for _, event in ipairs(pendingEvents) do
        if #UnitAuraSecretProbeDB.events < MAX_EVENT_ENTRIES then
            table.insert(UnitAuraSecretProbeDB.events, event)
        else
            UnitAuraSecretProbeDB.truncatedEvents = UnitAuraSecretProbeDB.truncatedEvents + 1
        end
    end
    pendingEvents = {}
end

local function appendEvent(record)
    if dbReady then
        if #UnitAuraSecretProbeDB.events < MAX_EVENT_ENTRIES then
            table.insert(UnitAuraSecretProbeDB.events, record)
        else
            UnitAuraSecretProbeDB.truncatedEvents = UnitAuraSecretProbeDB.truncatedEvents + 1
        end
    else
        table.insert(pendingEvents, record)
    end
end

local function apiPresence()
    local cUnitAuras = type(C_UnitAuras) == "table" and C_UnitAuras or nil
    local names = {
        "GetAuraDataByIndex",
        "GetAuraDataByAuraInstanceID",
        "GetAuraDataBySlot",
        "GetAuraDataBySpellName",
        "GetAuraSlots",
    }
    local presence = {
        UnitAura = type(UnitAura),
        UnitBuff = type(UnitBuff),
        UnitDebuff = type(UnitDebuff),
        C_UnitAuras = type(C_UnitAuras),
        issecretvalue = type(issecretvalue),
        canaccessvalue = type(canaccessvalue),
        canaccessallvalues = type(canaccessallvalues),
    }
    for _, name in ipairs(names) do
        presence["C_UnitAuras." .. name] = type(cUnitAuras and cUnitAuras[name])
    end
    return presence
end

local function findAuraIndex(unit, filter)
    if type(C_UnitAuras) ~= "table" or type(C_UnitAuras.GetAuraDataByIndex) ~= "function" then
        return nil, "missing"
    end
    for index = 1, 40 do
        local ok, values = invoke(function()
            return C_UnitAuras.GetAuraDataByIndex(unit, index, filter)
        end)
        if not ok then
            return index, "error"
        end
        if type(values[1]) ~= "nil" then
            return index, "present"
        end
    end
    return nil, "absent"
end

local function captureFilter(unit, filter)
    local index, discovery = findAuraIndex(unit, filter)
    local record = {
        filter = filter,
        discovery = discovery,
        index = index,
        calls = {},
        identity = {},
    }
    if index == nil then
        return record
    end

    if type(UnitAura) == "function" then
        record.calls.UnitAura = captureCall(function()
            return UnitAura(unit, index, filter)
        end, false)
    end
    if filter == "HELPFUL" and type(UnitBuff) == "function" then
        record.calls.UnitBuff = captureCall(function()
            return UnitBuff(unit, index, filter)
        end, false)
    end
    if filter == "HARMFUL" and type(UnitDebuff) == "function" then
        record.calls.UnitDebuff = captureCall(function()
            return UnitDebuff(unit, index, filter)
        end, false)
    end

    if type(C_UnitAuras) ~= "table" or type(C_UnitAuras.GetAuraDataByIndex) ~= "function" then
        return record
    end

    local firstRecord, firstAura = captureCall(function()
        return C_UnitAuras.GetAuraDataByIndex(unit, index, filter)
    end, true)
    record.calls.GetAuraDataByIndex = firstRecord

    local secondOk, secondValues = invoke(function()
        return C_UnitAuras.GetAuraDataByIndex(unit, index, filter)
    end)
    record.identity.repeatedLookup = summarizeInvocation(secondOk, secondValues)
    if firstAura ~= nil and secondOk then
        local secondAura = secondValues[1]
        record.identity.equals = observeOperation(function()
            return firstAura == secondAura
        end)
        record.identity.rawEquals = observeOperation(function()
            return rawequal(firstAura, secondAura)
        end)
    end

    local idOk, auraInstanceID = readActualField(firstAura, "auraInstanceID")
    if idOk and type(C_UnitAuras.GetAuraDataByAuraInstanceID) == "function" then
        record.calls.GetAuraDataByAuraInstanceID = captureCall(function()
            return C_UnitAuras.GetAuraDataByAuraInstanceID(unit, auraInstanceID)
        end, true)
    end

    local nameOk, auraName = readActualField(firstAura, "name")
    if nameOk and type(C_UnitAuras.GetAuraDataBySpellName) == "function" then
        record.calls.GetAuraDataBySpellName = captureCall(function()
            return C_UnitAuras.GetAuraDataBySpellName(unit, auraName, filter)
        end, true)
    end

    return record
end

local function captureUnit(unit, phase)
    bindSavedVariables()
    local record = {
        phase = phase or currentPhase,
        unit = unit,
        time = GetTime(),
        date = date("%Y-%m-%d %H:%M:%S"),
        exists = UnitExists(unit) == true,
        apiPresence = apiPresence(),
        filters = {
            captureFilter(unit, "HELPFUL"),
            captureFilter(unit, "HARMFUL"),
        },
    }
    table.insert(UnitAuraSecretProbeDB.captures, record)
    print(("%s captured unit=%s phase=%s; /reload or logout to flush SavedVariables"):format(
        PREFIX,
        tostring(unit),
        tostring(record.phase)
    ))
    return record
end

local function captureUnitAuraEvent(unitToken, updateInfo, ...)
    local arguments = pack(unitToken, updateInfo, ...)
    local record = {
        event = "UNIT_AURA",
        phase = currentPhase,
        time = GetTime(),
        date = date("%Y-%m-%d %H:%M:%S"),
        arguments = {},
        updateInfo = observeAuraTable(updateInfo),
        knownUpdateFields = captureKnownUpdateFields(updateInfo),
    }
    for index = 1, arguments.n do
        record.arguments[index] = summarizeValue(arguments[index])
    end
    appendEvent(record)
end

local function markPhase(label)
    bindSavedVariables()
    currentPhase = label
    table.insert(UnitAuraSecretProbeDB.markers, {
        label = label,
        time = GetTime(),
        date = date("%Y-%m-%d %H:%M:%S"),
    })
    print(PREFIX .. " phase=" .. label)
end

local function resetProbe()
    bindSavedVariables()
    UnitAuraSecretProbeDB.captures = {}
    UnitAuraSecretProbeDB.events = {}
    UnitAuraSecretProbeDB.markers = {}
    UnitAuraSecretProbeDB.truncatedEvents = 0
    currentPhase = "reset"
    print(PREFIX .. " capture data cleared")
end

UnitAuraSecretProbe = {
    Capture = captureUnit,
    Mark = markPhase,
    Reset = resetProbe,
}

SLASH_UNITAURASECRETPROBE1 = "/uasp"
SlashCmdList.UNITAURASECRETPROBE = function(command)
    local action, argument = command:match("^(%S*)%s*(.-)$")
    if action == "capture" then
        captureUnit(argument ~= "" and argument or "player", currentPhase)
    elseif action == "mark" and argument ~= "" then
        markPhase(argument)
    elseif action == "reset" then
        resetProbe()
    elseif action == "status" then
        bindSavedVariables()
        print(("%s captures=%d events=%d truncated=%d phase=%s"):format(
            PREFIX,
            #UnitAuraSecretProbeDB.captures,
            #UnitAuraSecretProbeDB.events,
            UnitAuraSecretProbeDB.truncatedEvents,
            currentPhase
        ))
    else
        print("UnitAuraSecretProbe: /uasp capture <unit> | mark <phase> | status | reset")
    end
end

local frame = CreateFrame("Frame")
frame:RegisterEvent("ADDON_LOADED")
frame:RegisterEvent("PLAYER_LOGIN")
frame:RegisterEvent("PLAYER_TARGET_CHANGED")
frame:RegisterEvent("UNIT_AURA")
frame:SetScript("OnEvent", function(_, event, ...)
    if event == "ADDON_LOADED" then
        local loadedName = ...
        if loadedName == addonName then
            bindSavedVariables()
        end
    elseif event == "PLAYER_LOGIN" then
        bindSavedVariables()
        markPhase("login")
        C_Timer.After(2, function()
            captureUnit("player", "login")
        end)
        print(PREFIX .. " ready: /uasp capture player|target, /uasp mark <phase>")
    elseif event == "PLAYER_TARGET_CHANGED" then
        markPhase("target-change")
        C_Timer.After(0.25, function()
            captureUnit("target", "target-change")
        end)
    elseif event == "UNIT_AURA" then
        captureUnitAuraEvent(...)
    end
end)
