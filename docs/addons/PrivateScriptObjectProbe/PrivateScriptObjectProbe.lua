local addonName = ...

local PREFIX = "PSOP"
local MAX_CAPTURES = 12
local MAX_RETURNS = 12
local MAX_TEXT = 160
local SCRIPT_NAME = "OnShow"
local DELEGATE_NAME = "GetAuraFrameCount"

local PUBLIC_KEYS = {
    "AddAuraFilter",
    "ClearAuraFilters",
    "GetAuraFrame",
    "GetAuraFrameCount",
    "auraFilters",
    "auraFrames",
    "unitToken",
}

local PRIVATE_KEYS = {
    "OnLoad_Intrinsic",
    "OnUnitAuraUpdate",
    "AddAuraFrameInternal",
    "RemoveAuraFrameInternal",
    "RefreshAuraFrames",
    "UpdateAllAuras",
}

local function pack(...)
    return { n = select("#", ...), ... }
end

local function boundedText(value)
    local ok, text = pcall(tostring, value)
    if not ok then
        return "<tostring error>"
    end
    if #text <= MAX_TEXT then
        return text
    end
    return text:sub(1, MAX_TEXT) .. "..."
end

local function invoke(fn, ...)
    local packed = pack(pcall(fn, ...))
    local values = { n = packed.n - 1 }
    for index = 2, packed.n do
        values[index - 1] = packed[index]
    end
    return packed[1], values
end

local function summarizeValue(value)
    local valueType = type(value)
    local result = {
        luaType = valueType,
    }

    if type(issecretvalue) == "function" then
        local secretOk, secretValues = invoke(issecretvalue, value)
        local secretValue
        if secretOk then
            secretValue = secretValues[1] == true
        end
        result.isSecretValue = {
            available = true,
            ok = secretOk,
            value = secretValue,
        }
    else
        result.isSecretValue = { available = false }
    end

    if type(canaccessvalue) == "function" then
        local accessOk, accessValues = invoke(canaccessvalue, value)
        local accessible
        if accessOk then
            accessible = accessValues[1] == true
        end
        result.canAccessValue = {
            available = true,
            ok = accessOk,
            value = accessible,
        }
    else
        result.canAccessValue = { available = false }
    end

    local tostringOk, tostringValues = invoke(function()
        return tostring(value)
    end)
    result.tostringOk = tostringOk

    local safeToRecordText = result.isSecretValue.ok
        and result.isSecretValue.value == false
        and result.canAccessValue.ok
        and result.canAccessValue.value == true
    if tostringOk and safeToRecordText then
        result.tostring = boundedText(tostringValues[1])
    elseif tostringOk then
        result.tostringRedacted = true
    elseif tostringValues.n > 0 then
        result.tostringError = "<unprintable>"
    end

    return result
end

local function summarizeError(value)
    local summary = summarizeValue(value)
    return {
        luaType = type(value),
        message = summary.tostring,
        redacted = summary.tostringRedacted,
    }
end

local function summarizeCall(ok, values)
    local result = {
        ok = ok,
        returnCount = ok and values.n or 0,
        returns = {},
    }

    if ok then
        for index = 1, math.min(values.n, MAX_RETURNS) do
            result.returns[index] = summarizeValue(values[index])
        end
        result.returnsTruncated = values.n > MAX_RETURNS
    elseif values.n > 0 then
        result.error = summarizeError(values[1])
    end

    return result
end

local function observe(fn, ...)
    local ok, values = invoke(fn, ...)
    return summarizeCall(ok, values)
end

local function observeSecureCaller()
    if type(issecure) ~= "function" then
        return { available = false }
    end
    return {
        available = true,
        call = observe(issecure),
    }
end

local function observeKey(object, key)
    return {
        direct = observe(function()
            return object[key]
        end),
        raw = observe(function()
            return rawget(object, key)
        end),
    }
end

local function observeKeys(object, keys)
    local result = {}
    for _, key in ipairs(keys) do
        result[key] = observeKey(object, key)
    end
    return result
end

local function observeObject(label, object)
    return {
        label = label,
        value = summarizeValue(object),
        selfEquality = observe(function()
            return object == object
        end),
        selfRawequal = observe(function()
            return rawequal(object, object)
        end),
        metatable = observe(function()
            return getmetatable(object)
        end),
        publicKeys = observeKeys(object, PUBLIC_KEYS),
        privateKeys = observeKeys(object, PRIVATE_KEYS),
    }
end

local function observeObjectPair(publicObject, forbiddenObject)
    return {
        equality = observe(function()
            return publicObject == forbiddenObject
        end),
        rawequal = observe(function()
            return rawequal(publicObject, forbiddenObject)
        end),
        public = observeObject("public", publicObject),
        forbidden = observeObject("forbidden", forbiddenObject),
    }
end

local function observeChildren(label, object)
    return {
        label = label,
        traversal = observe(function()
            return object:GetChildren()
        end),
    }
end

local function observeScriptSurface(label, object)
    local result = { label = label }
    result.getScript = observe(function()
        return object:GetScript(SCRIPT_NAME)
    end)
    result.setScript = observe(function()
        object:SetScript(SCRIPT_NAME, function() end)
        return true
    end)
    result.clearScript = observe(function()
        return object:SetScript(SCRIPT_NAME, nil)
    end)
    result.hookScript = observe(function()
        return object:HookScript(SCRIPT_NAME, function() end)
    end)

    if type(hooksecurefunc) == "function" then
        result.hooksecurefunc = observe(function()
            return hooksecurefunc(object, DELEGATE_NAME, function() end)
        end)
    else
        result.hooksecurefunc = { available = false }
    end

    return result
end

local function observeExtractedDelegate(publicObject, forbiddenObject)
    local lookupOk, lookupValues = invoke(function()
        return publicObject[DELEGATE_NAME]
    end)
    local delegate = lookupValues[1]
    local result = {
        name = DELEGATE_NAME,
        lookup = summarizeCall(lookupOk, lookupValues),
        available = lookupOk and type(delegate) == "function",
    }

    if not result.available then
        return result
    end

    result.delegate = summarizeValue(delegate)
    result.publicReceiver = observe(delegate, publicObject)
    result.uiParentReceiver = observe(delegate, UIParent)
    result.forbiddenReceiver = observe(delegate, forbiddenObject)
    result.noReceiver = observe(delegate)
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

local function ensureDatabase()
    PrivateScriptObjectProbeDB = PrivateScriptObjectProbeDB or {}
    PrivateScriptObjectProbeDB.schema = 1
    PrivateScriptObjectProbeDB.addonName = addonName
    PrivateScriptObjectProbeDB.captures = PrivateScriptObjectProbeDB.captures or {}
    return PrivateScriptObjectProbeDB
end

local function appendCapture(capture)
    local database = ensureDatabase()
    table.insert(database.captures, capture)
    while #database.captures > MAX_CAPTURES do
        table.remove(database.captures, 1)
    end
end

local function stoppedCapture(run, reason)
    run.stopReason = reason
    appendCapture(run)
    print(PREFIX, "stopped:", reason, "Use /psop status; /reload or logout to flush SavedVariables")
    return run
end

local function captureTemplate()
    local run = {
        schema = 1,
        addonName = addonName,
        capturedAt = time(),
        build = buildInfo(),
        secureCaller = observeSecureCaller(),
        load = {},
    }

    if type(PrivateScriptObjectProbeHost) ~= "table" then
        return stoppedCapture(run, "fixture PrivateScriptObjectProbeHost unavailable")
    end

    if type(C_AddOns) ~= "table" or type(C_AddOns.LoadAddOn) ~= "function" then
        return stoppedCapture(run, "public C_AddOns.LoadAddOn API unavailable")
    end

    run.load.blizzardAuraContainer = observe(function()
        return C_AddOns.LoadAddOn("Blizzard_AuraContainer")
    end)

    if type(CreateFrame) ~= "function" then
        return stoppedCapture(run, "CreateFrame unavailable")
    end

    if type(GetForbiddenObjectTable) ~= "function" then
        return stoppedCapture(run, "GetForbiddenObjectTable unavailable")
    end

    local createOk, createValues = invoke(function()
        return CreateFrame(
            "AuraContainer",
            nil,
            PrivateScriptObjectProbeHost,
            "CustomAuraContainerTemplate"
        )
    end)
    run.template = summarizeCall(createOk, createValues)
    if not createOk or createValues.n < 1 or createValues[1] == nil then
        return stoppedCapture(run, "CustomAuraContainerTemplate unavailable")
    end

    local publicObject = createValues[1]
    run.template.objectType = observe(function()
        return publicObject:GetObjectType()
    end)

    local forbiddenOk, forbiddenValues = invoke(function()
        return GetForbiddenObjectTable(publicObject)
    end)
    run.forbiddenObject = summarizeCall(forbiddenOk, forbiddenValues)
    if not forbiddenOk or forbiddenValues.n < 1 or forbiddenValues[1] == nil then
        return stoppedCapture(run, "forbidden object table unavailable")
    end

    local forbiddenObject = forbiddenValues[1]
    run.objects = observeObjectPair(publicObject, forbiddenObject)

    local childOk, childValues = invoke(function()
        return CreateFrame("Frame", nil, publicObject)
    end)
    run.child = summarizeCall(childOk, childValues)
    run.children = {
        observeChildren("public", publicObject),
        observeChildren("forbidden", forbiddenObject),
    }

    run.scripts = {
        observeScriptSurface("public", publicObject),
        observeScriptSurface("forbidden", forbiddenObject),
    }

    run.delegate = observeExtractedDelegate(publicObject, forbiddenObject)
    run.secureCallerAfter = observeSecureCaller()
    appendCapture(run)
    print(PREFIX, "captured:", #ensureDatabase().captures, "Use /psop status; /reload or logout to flush SavedVariables")
    return run
end

local function recapture()
    local ok, result = pcall(captureTemplate)
    if not ok then
        local run = {
            schema = 1,
            addonName = addonName,
            capturedAt = time(),
            build = buildInfo(),
            secureCaller = observeSecureCaller(),
            stopReason = "probe error: " .. boundedText(result),
        }
        appendCapture(run)
        print(PREFIX, "stopped: probe error; Use /psop status; /reload or logout to flush SavedVariables")
    end
end

local function resetCaptures()
    PrivateScriptObjectProbeDB = {
        schema = 1,
        addonName = addonName,
        captures = {},
    }
    print(PREFIX, "reset; /reload or logout to flush SavedVariables")
end

local function status()
    local database = ensureDatabase()
    local count = #database.captures
    local latest = database.captures[count]
    print(PREFIX, "captures:", count, "latest:", latest and (latest.stopReason or "complete") or "none")
    print(PREFIX, "Use /psop capture, /psop reset, /psop status; /reload or logout to flush SavedVariables")
end

SLASH_PRIVATESCRIPTOBJECTPROBE1 = "/psop"
SlashCmdList.PRIVATESCRIPTOBJECTPROBE = function(message)
    message = string.lower(message or "")
    if message == "reset" then
        resetCaptures()
    elseif message == "status" then
        status()
    else
        recapture()
    end
end

local eventFrame = CreateFrame("Frame")
eventFrame:RegisterEvent("PLAYER_LOGIN")
eventFrame:SetScript("OnEvent", function()
    if type(C_Timer) == "table" and type(C_Timer.After) == "function" then
        C_Timer.After(1, recapture)
    else
        recapture()
    end
end)
