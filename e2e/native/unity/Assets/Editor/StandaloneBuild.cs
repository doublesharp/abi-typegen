using System;
using System.IO;
using UnityEditor;
using UnityEditor.Build;
using UnityEditor.Build.Reporting;
using UnityEditor.SceneManagement;
using UnityEngine;

namespace AbiTypegen.Unity.E2E.Editor
{
    public static class StandaloneBuild
    {
        public static void BuildLinux64Il2Cpp()
        {
            BuildStandalone(BuildTarget.StandaloneLinux64, ScriptingImplementation.IL2CPP,
                "Build/IL2CPP/AbiTypegenUnitySmoke.x86_64");
        }

        public static void BuildLinux64Mono()
        {
            BuildStandalone(BuildTarget.StandaloneLinux64, ScriptingImplementation.Mono2x,
                "Build/Mono/AbiTypegenUnitySmoke.x86_64");
        }

        public static void BuildMacIl2Cpp()
        {
            BuildStandalone(BuildTarget.StandaloneOSX, ScriptingImplementation.IL2CPP,
                "Build/MacIL2CPP/AbiTypegenUnitySmoke.app");
        }

        public static void BuildMacMono()
        {
            BuildStandalone(BuildTarget.StandaloneOSX, ScriptingImplementation.Mono2x,
                "Build/MacMono/AbiTypegenUnitySmoke.app");
        }

        private static void BuildStandalone(BuildTarget buildTarget, ScriptingImplementation backend, string outputPath)
        {
            const string scenePath = "Assets/Scenes/AbiTypegenSmoke.unity";
            var projectRoot = Directory.GetParent(Application.dataPath).FullName;
            Directory.CreateDirectory(Path.Combine(projectRoot, "Assets", "Scenes"));
            var absoluteOutput = Path.Combine(projectRoot, outputPath);
            Directory.CreateDirectory(Path.GetDirectoryName(absoluteOutput) ?? projectRoot);

            var scene = EditorSceneManager.NewScene(NewSceneSetup.EmptyScene, NewSceneMode.Single);
            var root = new GameObject("AbiTypegenAotProbe");
            root.AddComponent<AbiTypegen.Unity.E2E.AotProbe>();
            if (!EditorSceneManager.SaveScene(scene, scenePath))
                throw new InvalidOperationException("Failed to save standalone smoke-test scene.");

            var target = NamedBuildTarget.Standalone;
            var previousBackend = PlayerSettings.GetScriptingBackend(target);
            var previousTarget = EditorUserBuildSettings.activeBuildTarget;
            try
            {
                if (previousTarget != buildTarget &&
                    !EditorUserBuildSettings.SwitchActiveBuildTarget(BuildTargetGroup.Standalone, buildTarget))
                    throw new InvalidOperationException($"Failed to activate build target: {buildTarget}");
                PlayerSettings.SetScriptingBackend(target, backend);
                var report = BuildPipeline.BuildPlayer(new BuildPlayerOptions
                {
                    scenes = new[] { scenePath },
                    locationPathName = absoluteOutput,
                    target = buildTarget,
                    options = BuildOptions.None
                });
                if (report.summary.result != BuildResult.Succeeded)
                    throw new InvalidOperationException($"Standalone build failed: {report.summary.result}");
            }
            finally
            {
                try
                {
                    PlayerSettings.SetScriptingBackend(target, previousBackend);
                }
                finally
                {
                    if (EditorUserBuildSettings.activeBuildTarget != previousTarget &&
                        !EditorUserBuildSettings.SwitchActiveBuildTarget(
                            BuildPipeline.GetBuildTargetGroup(previousTarget), previousTarget))
                        throw new InvalidOperationException($"Failed to restore build target: {previousTarget}");
                }
            }
        }
    }
}
