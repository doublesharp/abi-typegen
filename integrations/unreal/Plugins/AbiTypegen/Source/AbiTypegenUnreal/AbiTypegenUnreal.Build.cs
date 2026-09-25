using UnrealBuildTool;

public class AbiTypegenUnreal : ModuleRules
{
    public AbiTypegenUnreal(ReadOnlyTargetRules Target) : base(Target)
    {
        PCHUsage = PCHUsageMode.UseExplicitOrSharedPCHs;
        bEnableExceptions = false;
        bUseRTTI = false;

        PublicDependencyModuleNames.AddRange(new[]
        {
            "Core",
            "CoreUObject",
            "Engine",
            "HTTP",
            "Json",
            "AbiTypegenRuntime"
        });
    }
}
