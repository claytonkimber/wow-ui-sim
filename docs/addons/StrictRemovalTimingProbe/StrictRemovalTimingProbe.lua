local runtime = StrictRemovalTimingProbeRuntime
if runtime and type(runtime.recordPhase) == "function" then
    runtime.recordPhase("normal-file")
end
