local addonName = ...
local PREFIX = "FASP"
local MAX_TEXT = 240
local MAX_RUNS = 10

local ASPECT_NAMES = {
    "UntrustedScriptExecution",
    "UntrustedLayoutScriptExecution",
    "EventRegistrations",
    "AlwaysPropagateInput",
    "ScriptedInput",
    "QueryFocus",
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

local function probeBoolean(fn)
    local ok, observed = pcall(fn)
    local value
    if ok then
        value = observed == true
    end
    return {
        ok = ok,
        value = value,
    }
end

local function summarize(value)
    local valueType = type(value)
    local secretSupported = type(issecretvalue) == "function"
    local accessSupported = type(canaccessvalue) == "function"
    local secret = secretSupported and probeBoolean(function()
        return issecretvalue(value)
    end) or { ok = false, unavailable = true }
    local accessible = accessSupported and probeBoolean(function()
        return canaccessvalue(value)
    end) or { ok = false, unavailable = true }
    local safeToRecord = secret.ok and secret.value == false
        and accessible.ok and accessible.value == true

    local result = {
        type = valueType,
        secretProbeOk = secret.ok,
        secretProbeUnavailable = secret.unavailable,
        secret = secret.value,
        accessProbeOk = accessible.ok,
        accessProbeUnavailable = accessible.unavailable,
        accessible = accessible.value,
    }
    local textOk, text = pcall(tostring, value)
    result.tostringOk = textOk
    if textOk and safeToRecord then
        result.tostring = boundedText(text)
    elseif textOk then
        result.tostringRedacted = true
    end

    if safeToRecord then
        if valueType == "string" then
            result.value = boundedText(value)
        elseif valueType == "number" or valueType == "boolean" then
            result.value = value
        elseif valueType == "nil" then
            result.value = "<nil>"
        end
    end
    return result
end

local function observeCall(fn, ...)
    if type(fn) ~= "function" then
        return { ok = false, error = "missing function" }
    end

    local arguments = pack(...)
    local results = pack(pcall(function()
        return fn(unpack(arguments, 1, arguments.n))
    end))
    local ok = results[1]
    local observation = {
        ok = ok,
        returnCount = ok and results.n - 1 or 0,
        returns = {},
    }
    if ok then
        for index = 2, results.n do
            observation.returns[index - 1] = summarize(results[index])
        end
    else
        observation.error = summarize(results[2])
    end
    return observation, results[2]
end

local function observeMethod(object, method, ...)
    local lookupOk, member = pcall(function()
        return object[method]
    end)
    if not lookupOk then
        return { ok = false, lookupError = summarize(member) }
    end
    if type(member) ~= "function" then
        return { ok = false, memberType = type(member), error = "not callable" }
    end
    return observeCall(function(...)
        return member(object, ...)
    end, ...)
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

local function enumValue(group, name)
    local ok, value = pcall(function()
        return Enum[group][name]
    end)
    return ok, value
end

local function createHiddenFrame(widgetType)
    local frame = CreateFrame(widgetType or "Frame", nil, UIParent)
    pcall(function() frame:SetSize(1, 1) end)
    pcall(function() frame:SetPoint("TOPLEFT", UIParent, "BOTTOMLEFT", -100, -100) end)
    pcall(function() frame:Hide() end)
    return frame
end

local function maskState(frame)
    local parentOk, parentInheritance = enumValue("ForbiddenAspectInheritance", "Parent")
    local layoutOk, layoutInheritance = enumValue("ForbiddenAspectInheritance", "Layout")
    return {
        direct = observeMethod(frame, "GetForbiddenAspects"),
        hasAny = observeMethod(frame, "HasAnyForbiddenAspects"),
        parent = parentOk and observeMethod(
            frame,
            "GetInheritableForbiddenAspects",
            parentInheritance
        ) or { ok = false, error = "Parent inheritance enum unavailable" },
        layout = layoutOk and observeMethod(
            frame,
            "GetInheritableForbiddenAspects",
            layoutInheritance
        ) or { ok = false, error = "Layout inheritance enum unavailable" },
    }
end

local function scriptOperations(frame)
    local calls = { onShow = 0 }
    local handler = function()
        calls.onShow = calls.onShow + 1
    end
    local result = {}
    result.set = observeMethod(frame, "SetScript", "OnShow", handler)
    result.get = observeMethod(frame, "GetScript", "OnShow")
    result.has = observeMethod(frame, "HasScript", "OnShow")
    result.hook = observeMethod(frame, "HookScript", "OnShow", handler)
    result.show = observeMethod(frame, "Show")
    result.hide = observeMethod(frame, "Hide")
    result.dispatched = calls.onShow
    result.clear = observeMethod(frame, "ClearScripts")
    result.afterClear = observeMethod(frame, "GetScript", "OnShow")
    return result
end

local function layoutOperations(aspect)
    local parent = createHiddenFrame("Frame")
    local child = createHiddenFrame("Frame")
    local anchor = createHiddenFrame("Frame")
    local anchored = createHiddenFrame("Frame")
    pcall(function() anchored:ClearAllPoints() end)

    local result = {}
    result.parentAdd = observeMethod(parent, "AddForbiddenAspects", aspect)
    result.setParentBeforeOwnMask = observeMethod(child, "SetParent", parent)
    result.childAdd = observeMethod(child, "AddForbiddenAspects", aspect)
    result.setParentAfterOwnMask = observeMethod(child, "SetParent", parent)
    result.anchorAdd = observeMethod(anchor, "AddForbiddenAspects", aspect)
    result.setPointBeforeOwnMask = observeMethod(
        anchored,
        "SetPoint",
        "CENTER",
        anchor,
        "CENTER",
        0,
        0
    )
    result.anchoredAdd = observeMethod(anchored, "AddForbiddenAspects", aspect)
    result.setPointAfterOwnMask = observeMethod(
        anchored,
        "SetPoint",
        "CENTER",
        anchor,
        "CENTER",
        0,
        0
    )
    result.clearPoints = observeMethod(anchored, "ClearAllPoints")
    result.setSize = observeMethod(anchored, "SetSize", 2, 2)
    result.setWidth = observeMethod(anchored, "SetWidth", 3)
    result.setHeight = observeMethod(anchored, "SetHeight", 4)
    return result
end

local function eventOperations(frame)
    local result = {}
    result.register = observeMethod(frame, "RegisterEvent", "UNIT_AURA")
    result.registerUnit = observeMethod(frame, "RegisterUnitEvent", "UNIT_HEALTH", "player")
    result.isRegistered = observeMethod(frame, "IsEventRegistered", "UNIT_AURA")
    result.unregister = observeMethod(frame, "UnregisterEvent", "UNIT_AURA")
    result.unregisterAll = observeMethod(frame, "UnregisterAllEvents")
    return result
end

local function propagationOperations(frame)
    local result = {}
    result.initial = observeMethod(frame, "GetPropagateKeyboardInput")
    result.setTrue = observeMethod(frame, "SetPropagateKeyboardInput", true)
    result.afterTrue = observeMethod(frame, "GetPropagateKeyboardInput")
    result.setFalse = observeMethod(frame, "SetPropagateKeyboardInput", false)
    result.afterFalse = observeMethod(frame, "GetPropagateKeyboardInput")
    return result
end

local function scriptedInputOperations(aspect)
    local button = createHiddenFrame("Button")
    local editBox = createHiddenFrame("EditBox")
    observeMethod(button, "AddForbiddenAspects", aspect)
    observeMethod(editBox, "AddForbiddenAspects", aspect)

    local clickCount = 0
    local inputHandler = function()
        clickCount = clickCount + 1
    end
    local buttonHandlers = {}
    for _, script in ipairs({ "OnClick", "OnMouseDown", "OnMouseUp", "OnEnter", "OnLeave" }) do
        local handlerResult = {}
        handlerResult.set = observeMethod(button, "SetScript", script, inputHandler)
        handlerResult.hook = observeMethod(button, "HookScript", script, inputHandler)
        buttonHandlers[script] = handlerResult
    end
    local editBoxHandlers = {}
    for _, script in ipairs({ "OnKeyDown", "OnKeyUp", "OnChar" }) do
        local handlerResult = {}
        handlerResult.set = observeMethod(editBox, "SetScript", script, inputHandler)
        handlerResult.hook = observeMethod(editBox, "HookScript", script, inputHandler)
        editBoxHandlers[script] = handlerResult
    end

    local result = {
        buttonHandlers = buttonHandlers,
        editBoxHandlers = editBoxHandlers,
        realInputTested = false,
    }
    result.enableMouse = observeMethod(button, "EnableMouse", true)
    result.enableKeyboard = observeMethod(editBox, "EnableKeyboard", true)
    result.click = observeMethod(button, "Click")
    result.scriptedDispatchCount = clickCount
    return result
end

local function focusOperations(frame)
    local globals = {}
    for _, name in ipairs({
        "GetMouseFocus",
        "GetMouseFoci",
        "GetCurrentKeyBoardFocus",
        "GetCurrentKeyboardFocus",
        "GetKeyboardFocus",
    }) do
        globals[name] = observeCall(_G[name])
    end
    return {
        globals = globals,
        isMouseOver = observeMethod(frame, "IsMouseOver"),
        hasFocus = observeMethod(frame, "HasFocus"),
        getFocus = observeMethod(frame, "GetFocus"),
        realFocusTransitionTested = false,
    }
end

local function captureAspect(name, aspect)
    local frame = createHiddenFrame("Frame")
    local result = {
        name = name,
        value = summarize(aspect),
        beforeMask = maskState(frame),
        addMask = observeMethod(frame, "AddForbiddenAspects", aspect),
        afterMask = maskState(frame),
    }

    if name == "UntrustedScriptExecution" then
        result.scripts = scriptOperations(frame)
    elseif name == "UntrustedLayoutScriptExecution" then
        result.layout = layoutOperations(aspect)
    elseif name == "EventRegistrations" then
        result.events = eventOperations(frame)
    elseif name == "AlwaysPropagateInput" then
        result.propagation = propagationOperations(frame)
    elseif name == "ScriptedInput" then
        result.scriptedInput = scriptedInputOperations(aspect)
    elseif name == "QueryFocus" then
        result.focus = focusOperations(frame)
    end
    return result
end

local function captureAllMask(aspects)
    local frame = createHiddenFrame("Frame")
    local arguments = {}
    for _, name in ipairs(ASPECT_NAMES) do
        if aspects[name] ~= nil then
            table.insert(arguments, aspects[name])
        end
    end
    local add = observeCall(function()
        return frame:AddForbiddenAspects(unpack(arguments))
    end)
    return {
        add = add,
        state = maskState(frame),
    }
end

local function ensureDatabase()
    ForbiddenAspectsProbeDB = ForbiddenAspectsProbeDB or {}
    ForbiddenAspectsProbeDB.schema = 1
    ForbiddenAspectsProbeDB.addonName = addonName
    ForbiddenAspectsProbeDB.runs = ForbiddenAspectsProbeDB.runs or {}
    return ForbiddenAspectsProbeDB
end

local function runProbe()
    local db = ensureDatabase()
    local run = {
        capturedAt = date("%Y-%m-%d %H:%M:%S"),
        time = GetTime(),
        build = buildInfo(),
        enum = {},
        aspects = {},
    }

    for _, name in ipairs(ASPECT_NAMES) do
        local ok, aspect = enumValue("ForbiddenScriptObjectAspect", name)
        run.enum[name] = {
            ok = ok,
            value = summarize(aspect),
        }
        if ok and type(aspect) == "number" then
            run.aspects[name] = captureAspect(name, aspect)
        else
            run.aspects[name] = {
                name = name,
                error = "enum unavailable",
            }
        end
    end
    run.allMask = captureAllMask((function()
        local values = {}
        for _, name in ipairs(ASPECT_NAMES) do
            local ok, aspect = enumValue("ForbiddenScriptObjectAspect", name)
            if ok and type(aspect) == "number" then
                values[name] = aspect
            end
        end
        return values
    end)())

    table.insert(db.runs, run)
    while #db.runs > MAX_RUNS do
        table.remove(db.runs, 1)
    end
    db.latest = run
    print(PREFIX .. " captured forbidden-aspect observations")
    print(PREFIX .. " /reload or log out to flush SavedVariables")
end

local function resetProbe()
    local db = ensureDatabase()
    db.runs = {}
    db.latest = nil
    print(PREFIX .. " capture data cleared")
end

ForbiddenAspectsProbe = {
    Capture = runProbe,
    Reset = resetProbe,
}

SLASH_FORBIDDENASPECTSPROBE1 = "/fasp"
SLASH_FORBIDDENASPECTSPROBE2 = "/forbiddenaspectsprobe"
SlashCmdList.FORBIDDENASPECTSPROBE = function(command)
    local action = command:match("^%s*(%S*)")
    if action == "reset" then
        resetProbe()
    elseif action == "status" then
        local db = ensureDatabase()
        print(("%s runs=%d latest=%s"):format(
            PREFIX,
            #db.runs,
            tostring(db.latest ~= nil)
        ))
    else
        runProbe()
    end
end

local frame = CreateFrame("Frame")
frame:RegisterEvent("PLAYER_LOGIN")
frame:SetScript("OnEvent", function(self)
    self:UnregisterAllEvents()
    C_Timer.After(1, runProbe)
end)
