using UnrealBuildTool;

public class AbiTypegenUnrealTestHostTests : ModuleRules
{
    public AbiTypegenUnrealTestHostTests(ReadOnlyTargetRules Target) : base(Target)
    {
        PCHUsage = PCHUsageMode.UseExplicitOrSharedPCHs;
        bEnableExceptions = false;
        PrivateDependencyModuleNames.AddRange(new[]
        {
            "Core", "CoreUObject", "Engine", "HTTP", "Json",
            "AbiTypegenRuntime", "AbiTypegenUnreal", "AbiTypegenUnrealTestHost"
        });
    }
}
