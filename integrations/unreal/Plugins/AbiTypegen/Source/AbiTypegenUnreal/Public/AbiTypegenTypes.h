#pragma once

#include "CoreMinimal.h"
#include "abi_typegen.h"
#include "AbiTypegenTypes.generated.h"

/** Lossless reflected 256-bit ABI word. Hex is canonical 0x + 64 lowercase hex digits. */
USTRUCT(BlueprintType)
struct ABITYPEGENUNREAL_API FAbiTypegenWord256
{
    GENERATED_BODY()

    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category="abi-typegen")
    FString Hex = TEXT("0x0000000000000000000000000000000000000000000000000000000000000000");

    static FAbiTypegenWord256 FromWord(const atg_word& Word);
    static FAbiTypegenWord256 FromUInt64(uint64 Value);
    bool TryToWord(atg_word& Out, FString& OutError) const;
    bool IsCanonical() const;
};

/** Lossless reflected Ethereum address. Hex is canonical 0x + 40 lowercase hex digits. */
USTRUCT(BlueprintType)
struct ABITYPEGENUNREAL_API FAbiTypegenAddress
{
    GENERATED_BODY()

    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category="abi-typegen")
    FString Hex;

    static FAbiTypegenAddress FromAddress(const atg_address& Address);
    bool TryToAddress(atg_address& Out, FString& OutError) const;
};

USTRUCT(BlueprintType)
struct ABITYPEGENUNREAL_API FAbiTypegenBytes
{
    GENERATED_BODY()

    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category="abi-typegen")
    TArray<uint8> Data;
};

/** Non-throwing RAII owner for atg_result. */
class ABITYPEGENUNREAL_API FAbiTypegenResult final
{
public:
    explicit FAbiTypegenResult(atg_result* In = nullptr) : Ptr(In) {}
    ~FAbiTypegenResult();

    FAbiTypegenResult(FAbiTypegenResult&& Other) noexcept;
    FAbiTypegenResult& operator=(FAbiTypegenResult&& Other) noexcept;
    FAbiTypegenResult(const FAbiTypegenResult&) = delete;
    FAbiTypegenResult& operator=(const FAbiTypegenResult&) = delete;

    bool IsValid() const { return Ptr != nullptr; }
    /** Whether a native byte span can be represented by Unreal's TArray. */
    static bool CanCopyBytes(size_t Len, const uint8* Data);
    bool IsOk() const;
    FString Error() const;
    const atg_value* Value() const;
    TArray<uint8> CopyBytes() const;
    atg_result* Get() const { return Ptr; }
    atg_result* Release();

private:
    atg_result* Ptr = nullptr;
};

namespace AbiTypegen
{
    ABITYPEGENUNREAL_API bool HexToBytes(const FString& Hex, TArray<uint8>& Out, FString& OutError);
    ABITYPEGENUNREAL_API FString BytesToHex(const uint8* Data, int32 Num, bool bPrefix = true);
    ABITYPEGENUNREAL_API bool ParseAddress(const FString& Hex, atg_address& Out, FString& OutError);
    ABITYPEGENUNREAL_API FString AddressToHex(const atg_address& Address);
}
