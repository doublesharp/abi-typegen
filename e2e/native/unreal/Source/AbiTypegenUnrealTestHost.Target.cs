using UnrealBuildTool;
using System.Collections.Generic;

public class AbiTypegenUnrealTestHostTarget : TargetRules
{
    public AbiTypegenUnrealTestHostTarget(TargetInfo Target) : base(Target)
    {
        Type = TargetType.Game;
        DefaultBuildSettings = BuildSettingsVersion.Latest;
        IncludeOrderVersion = EngineIncludeOrderVersion.Latest;
        ExtraModuleNames.Add("AbiTypegenUnrealTestHost");
    }
}
