using UnrealBuildTool;

public class AbiTypegenUnrealTestHost : ModuleRules
{
    public AbiTypegenUnrealTestHost(ReadOnlyTargetRules Target) : base(Target)
    {
        PCHUsage = PCHUsageMode.UseExplicitOrSharedPCHs;
        bEnableExceptions = false;
        PublicDependencyModuleNames.AddRange(new[]
        {
            "Core", "CoreUObject", "Engine", "Json", "AbiTypegenRuntime", "AbiTypegenUnreal"
        });
    }
}
