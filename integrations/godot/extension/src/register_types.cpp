#include "register_types.h"

#include "abi_typegen_client.h"
#include "abi_typegen_codec.h"
#include "abi_typegen_request.h"

#include <godot_cpp/godot.hpp>

using namespace godot;

void initialize_abi_typegen_module(ModuleInitializationLevel p_level) {
    if (p_level != MODULE_INITIALIZATION_LEVEL_SCENE) {
        return;
    }
    ClassDB::register_class<AbiTypegenCodec>();
    ClassDB::register_class<AbiTypegenRequest>();
    ClassDB::register_class<AbiTypegenClient>();
}

void uninitialize_abi_typegen_module(ModuleInitializationLevel p_level) {
    if (p_level != MODULE_INITIALIZATION_LEVEL_SCENE) {
        return;
    }
}

extern "C" {
GDExtensionBool GDE_EXPORT abi_typegen_library_init(
    GDExtensionInterfaceGetProcAddress p_get_proc_address,
    const GDExtensionClassLibraryPtr p_library,
    GDExtensionInitialization *r_initialization) {
    GDExtensionBinding::InitObject init_obj(p_get_proc_address, p_library, r_initialization);
    init_obj.register_initializer(initialize_abi_typegen_module);
    init_obj.register_terminator(uninitialize_abi_typegen_module);
    init_obj.set_minimum_library_initialization_level(MODULE_INITIALIZATION_LEVEL_SCENE);
    return init_obj.init();
}
}
