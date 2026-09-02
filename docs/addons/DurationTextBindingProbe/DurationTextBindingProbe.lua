local addonName = ...

local MAX_STRING_LENGTH = 160
local MAX_PAIR_ENTRIES = 8
local MAX_SAVED_RUNS = 10

local function pack(...)
    return { n = select("#", ...), ... }
end

local CANDIDATE_METHODS = {
    "Assign",
    "CanFormatText",
    "CanUpdateFontString",
    "ClearTextColorCurve",
    "Disable",
    "Enable",
    "GetClock",
    "GetDuration",
    "GetExpiredText",
    "GetFontString",
    "GetFormattedText",
    "GetFormattedTextColor",
    "GetTextColorCurve",
    "GetTimeModifier",
    "GetUpdateInterval",
    "GetZeroDurationText",
    "HasExpired",
    "HasSecretValues",
    "HasStarted",
    "IsActive",
    "IsEnabled",
    "SetClock",
    "SetDuration",
    "SetEnabled",
    "SetExpiredText",
    "SetFontString",
    "SetFormatter",
    "SetTextColorCurve",
    "SetTextFormat",
    "SetTimeModifier",
    "SetToDefaults",
    "SetUpdateInterval",
    "SetZeroDurationText",
    "UpdateFontString",
}

local STATE_GETTERS = {
    "CanFormatText",
    "CanUpdateFontString",
    "GetClock",
    "GetDuration",
    "GetExpiredText",
    "GetFontString",
    "GetFormattedText",
    "GetFormattedTextColor",
    "GetTextColorCurve",
    "GetTimeModifier",
    "GetUpdateInterval",
    "GetZeroDurationText",
    "HasExpired",
    "HasSecretValues",
    "HasStarted",
    "IsActive",
    "IsEnabled",
}

local function boundedString(value)
    local ok, text = pcall(tostring, value)
    if not ok then
        return "<tostring error>"
    end
    if #text > MAX_STRING_LENGTH then
        return text:sub(1, MAX_STRING_LENGTH) .. "..."
    end
    return text
end

local function summarize(value)
    local valueType = type(value)
    local result = { type = valueType }

    if valueType == "boolean" or valueType == "number" then
        result.value = value
    elseif valueType == "string" then
        result.value = boundedString(value)
    elseif valueType ~= "nil" then
        result.tostring = boundedString(value)
    end

    return result
end

local function observeCall(fn, ...)
    if type(fn) ~= "function" then
        return { ok = false, error = "missing function" }
    end

    local arguments = pack(...)
    local callResults = pack(pcall(function()
        return fn(unpack(arguments, 1, arguments.n))
    end))
    local ok = callResults[1]
    local result = {
        ok = ok,
        returnCount = ok and callResults.n - 1 or 0,
        returns = {},
    }
    if ok then
        for index = 2, callResults.n do
            result.returns[index - 1] = summarize(callResults[index])
        end
    else
        result.error = boundedString(callResults[2])
    end
    return result, callResults[2]
end

local function observeMethodLookup(object, method)
    local ok, member = pcall(function()
        return object[method]
    end)
    if not ok then
        return { ok = false, error = boundedString(member) }
    end
    return { ok = true, type = type(member) }
end

local function observeMethod(object, method, ...)
    local lookupOk, member = pcall(function()
        return object[method]
    end)
    if not lookupOk then
        return { ok = false, error = boundedString(member) }
    end
    if type(member) ~= "function" then
        return { ok = false, type = type(member), error = "not callable" }
    end

    return observeCall(function(...)
        return member(object, ...)
    end, ...)
end

local function observePairs(object)
    local ok, entries, truncated = pcall(function()
        local iterator, state, control = pairs(object)
        local result = {}
        local count = 0

        while count < MAX_PAIR_ENTRIES do
            local key, value = iterator(state, control)
            if key == nil then
                return result, false
            end
            count = count + 1
            result[count] = {
                key = summarize(key),
                value = summarize(value),
            }
            control = key
        end

        return result, true
    end)

    if not ok then
        return { ok = false, error = boundedString(entries) }
    end
    return { ok = true, entries = entries, truncated = truncated }
end

local function observeRawAccess(object)
    local markerKey = "__DurationTextBindingProbeMarker"
    local result = {
        rawget = observeCall(function()
            return rawget(object, "SetDuration")
        end),
        rawset = observeCall(function()
            rawset(object, markerKey, "probe")
            local marker = rawget(object, markerKey)
            rawset(object, markerKey, nil)
            return marker
        end),
    }
    return result
end

local function observeIdentity(object, other)
    local result = {
        equalsSelf = observeCall(function()
            return object == object
        end),
        rawEqualsSelf = observeCall(function()
            return rawequal(object, object)
        end),
    }

    if other ~= nil then
        result.equalsOther = observeCall(function()
            return object == other
        end)
        result.rawEqualsOther = observeCall(function()
            return rawequal(object, other)
        end)
    end

    return result
end

local function observeState(object)
    local state = {}
    for _, method in ipairs(STATE_GETTERS) do
        state[method] = observeMethod(object, method)
    end
    return state
end

local function observeBinding(object, other)
    if object == nil then
        return { present = false }
    end

    local result = {
        present = true,
        type = type(object),
        tostring = observeCall(function()
            return tostring(object)
        end),
        equality = observeIdentity(object, other),
        getmetatable = observeCall(function()
            return getmetatable(object)
        end),
        rawAccess = observeRawAccess(object),
        pairs = observePairs(object),
        methods = {},
        state = observeState(object),
    }

    for _, method in ipairs(CANDIDATE_METHODS) do
        result.methods[method] = observeMethodLookup(object, method)
    end

    return result
end

local function observeFactory(durationUtil, label, argumentProvided, argument)
    local factoryLookup, factory = observeCall(function()
        return durationUtil.CreateDurationTextBinding
    end)
    local result
    local object
    if argumentProvided then
        result, object = observeCall(factory, argument)
    else
        result, object = observeCall(factory)
    end
    return {
        label = label,
        argumentProvided = argumentProvided,
        argument = summarize(argument),
        factoryLookup = factoryLookup,
        call = result,
    }, object
end

local function observeResource(label, fn, ...)
    local observation, value = observeCall(fn, ...)
    return {
        label = label,
        call = observation,
    }, value
end

local function observeRetainedReference(object)
    if object == nil then
        return { present = false }
    end

    local retained = object
    local result = {
        present = true,
        beforeCollect = {
            type = type(retained),
            tostring = observeCall(function()
                return tostring(retained)
            end),
            selfEquality = observeCall(function()
                return retained == retained
            end),
        },
    }

    result.collectgarbage = observeCall(collectgarbage, "collect")
    result.afterCollect = {
        type = type(retained),
        tostring = observeCall(function()
            return tostring(retained)
        end),
        selfEquality = observeCall(function()
            return retained == retained
        end),
        getDuration = observeMethod(retained, "GetDuration"),
    }
    return result
end

local function observeAction(object, label, method, ...)
    local args = { ... }
    local result = {
        label = label,
        method = method,
        before = observeState(object),
    }

    local lookupOk, member = pcall(function()
        return object[method]
    end)
    if not lookupOk then
        result.call = { ok = false, error = boundedString(member) }
    elseif type(member) ~= "function" then
        result.call = { ok = false, type = type(member), error = "not callable" }
    else
        result.call = observeCall(function()
            return member(object, unpack(args))
        end)
    end

    result.after = observeState(object)
    return result
end

local function observeStateTransitions(binding, durationObject, manualClock, fontString)
    local transitions = {}

    if durationObject ~= nil then
        table.insert(transitions, observeAction(
            binding,
            "SetDuration(Duration)",
            "SetDuration",
            durationObject
        ))
    end
    if manualClock ~= nil then
        table.insert(transitions, observeAction(
            binding,
            "SetClock(ManualClock)",
            "SetClock",
            manualClock
        ))
    end
    if fontString ~= nil then
        table.insert(transitions, observeAction(
            binding,
            "SetFontString(FontString)",
            "SetFontString",
            fontString
        ))
    end

    table.insert(transitions, observeAction(binding, "SetExpiredText", "SetExpiredText", "expired"))
    table.insert(transitions, observeAction(binding, "SetZeroDurationText", "SetZeroDurationText", "zero"))
    table.insert(transitions, observeAction(binding, "SetTimeModifier", "SetTimeModifier", 1.5))
    table.insert(transitions, observeAction(binding, "SetUpdateInterval", "SetUpdateInterval", 0.25))
    table.insert(transitions, observeAction(binding, "SetEnabled(false)", "SetEnabled", false))
    table.insert(transitions, observeAction(binding, "SetEnabled(true)", "SetEnabled", true))
    table.insert(transitions, observeAction(binding, "Disable", "Disable"))
    table.insert(transitions, observeAction(binding, "Enable", "Enable"))

    if fontString ~= nil then
        table.insert(transitions, observeAction(binding, "UpdateFontString", "UpdateFontString"))
    end

    table.insert(transitions, observeAction(binding, "SetToDefaults", "SetToDefaults"))
    return transitions
end

local function observeResources(durationUtil)
    local resources = {}
    local durationObject
    local manualClock
    local fontString

    resources.duration, durationObject = observeResource(
        "Duration",
        durationUtil and durationUtil.CreateDuration
    )
    resources.manualClock, manualClock = observeResource(
        "ManualClock",
        durationUtil and durationUtil.CreateManualClock,
        5
    )

    local ownerObservation, owner = observeResource("FontStringOwner", CreateFrame, "Frame")
    resources.fontStringOwner = ownerObservation
    if owner ~= nil and type(owner.CreateFontString) == "function" then
        resources.fontString, fontString = observeResource(
            "FontString",
            function()
                return owner:CreateFontString(nil, "ARTWORK")
            end
        )
    else
        resources.fontString = {
            label = "FontString",
            call = { ok = false, error = "CreateFontString unavailable" },
        }
    end

    return resources, durationObject, manualClock, fontString
end

local function captureBuildInfo()
    local result = observeCall(GetBuildInfo)
    return result
end

local function ensureDatabase()
    if type(DurationTextBindingProbeDB) ~= "table" then
        DurationTextBindingProbeDB = {}
    end
    if type(DurationTextBindingProbeDB.runs) ~= "table" then
        DurationTextBindingProbeDB.runs = {}
    end
    return DurationTextBindingProbeDB
end

local function printReminder()
    print(addonName .. ": observations saved to DurationTextBindingProbeDB.latest.")
    print(addonName .. ": /reload or log out to flush SavedVariables to WTF/Account/<ACCOUNT>/SavedVariables/DurationTextBindingProbe.lua")
end

local function runProbe()
    local db = ensureDatabase()
    local run = {
        probeVersion = 1,
        addonName = addonName,
        capturedAt = observeCall(time),
        build = captureBuildInfo(),
        namespace = {
            type = type(C_DurationUtil),
            factory = observeMethodLookup(C_DurationUtil, "CreateDurationTextBinding"),
        },
    }

    local durationUtil = C_DurationUtil
    if durationUtil == nil then
        run.error = "C_DurationUtil unavailable"
    else
        run.factories = {}
        local durationObject
        local manualClock
        local fontString
        local noArgFactory
        run.factories.noArg, noArgFactory = observeFactory(durationUtil, "no-arg", false)
        run.factories.nilArgument = observeFactory(durationUtil, "nil", true, nil)
        run.factories.falseArgument = observeFactory(durationUtil, "false", true, false)
        run.factories.tableArgument = observeFactory(durationUtil, "table", true, {})

        run.resources, durationObject, manualClock, fontString = observeResources(durationUtil)
        run.noArgBinding = observeBinding(noArgFactory)

        local secondFactory
        local secondObservation
        secondObservation, secondFactory = observeFactory(durationUtil, "second-no-arg", false)
        run.factories.secondNoArg = secondObservation
        if noArgFactory ~= nil and secondFactory ~= nil then
            run.twoObjectIdentity = observeIdentity(noArgFactory, secondFactory)
        end

        run.retainedReferenceAfterCollect = observeRetainedReference(noArgFactory)
        if noArgFactory ~= nil then
            run.stateTransitions = observeStateTransitions(
                noArgFactory,
                durationObject,
                manualClock,
                fontString
            )
            run.afterTransitions = observeBinding(noArgFactory, secondFactory)
        end
    end

    table.insert(db.runs, run)
    while #db.runs > MAX_SAVED_RUNS do
        table.remove(db.runs, 1)
    end
    db.latest = run

    print(addonName .. ": captured DurationTextBinding observations; see DurationTextBindingProbeDB.latest")
    printReminder()
end

SLASH_DURATIONTEXTBINDINGPROBE1 = "/dtbprobe"
SLASH_DURATIONTEXTBINDINGPROBE2 = "/durationtextbindingprobe"
SlashCmdList.DURATIONTEXTBINDINGPROBE = runProbe

local loader = CreateFrame("Frame")
loader:RegisterEvent("PLAYER_LOGIN")
loader:SetScript("OnEvent", runProbe)
