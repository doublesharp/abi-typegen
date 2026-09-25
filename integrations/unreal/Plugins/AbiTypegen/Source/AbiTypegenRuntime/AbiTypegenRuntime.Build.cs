using System;
using System.IO;
using UnrealBuildTool;

public class AbiTypegenRuntime : ModuleRules
{
    public AbiTypegenRuntime(ReadOnlyTargetRules Target) : base(Target)
    {
        Type = ModuleType.External;

        PublicIncludePaths.Add(Path.Combine(ModuleDirectory, "include"));
        PublicDefinitions.Add("ATG_ABI_VERSION_EXPECTED=1");

        if (Target.Platform == UnrealTargetPlatform.Mac)
        {
            string RuntimeLibrary = Path.Combine(ModuleDirectory, "lib", "Mac", "libabi_typegen_runtime.a");
            if (!File.Exists(RuntimeLibrary))
            {
                throw new BuildException("Missing abi-typegen Rust runtime: " + RuntimeLibrary);
            }
            PublicAdditionalLibraries.Add(RuntimeLibrary);
            return;
        }

        if (Target.Platform != UnrealTargetPlatform.Win64)
        {
            throw new BuildException(
                "The abi-typegen Unreal build supports Mac and Win64. " +
                "A Rust runtime built for the selected architecture is required.");
        }

        string ImportLibrary = Path.Combine(ModuleDirectory, "lib", "Win64", "abi_typegen_runtime.lib");
        string RuntimeDll = Path.Combine(ModuleDirectory, "bin", "Win64", "abi_typegen_runtime.dll");

        PublicAdditionalLibraries.Add(ImportLibrary);
        PublicDelayLoadDLLs.Add("abi_typegen_runtime.dll");
        RuntimeDependencies.Add("$(TargetOutputDir)/abi_typegen_runtime.dll", RuntimeDll, StagedFileType.NonUFS);
    }
}
