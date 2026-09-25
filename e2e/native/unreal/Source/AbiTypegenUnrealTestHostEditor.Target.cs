using UnrealBuildTool;
using System.Collections.Generic;

public class AbiTypegenUnrealTestHostEditorTarget : TargetRules
{
    public AbiTypegenUnrealTestHostEditorTarget(TargetInfo Target) : base(Target)
    {
        Type = TargetType.Editor;
        DefaultBuildSettings = BuildSettingsVersion.Latest;
        IncludeOrderVersion = EngineIncludeOrderVersion.Latest;
        ExtraModuleNames.AddRange(new[] { "AbiTypegenUnrealTestHost", "AbiTypegenUnrealTestHostTests" });
    }
}
