#include "AbiTypegenTypes.h"

#include "Misc/Char.h"

namespace
{
    int32 HexNibble(TCHAR C)
    {
        if (C >= TEXT('0') && C <= TEXT('9')) return C - TEXT('0');
        if (C >= TEXT('a') && C <= TEXT('f')) return C - TEXT('a') + 10;
        if (C >= TEXT('A') && C <= TEXT('F')) return C - TEXT('A') + 10;
        return -1;
    }
}

FAbiTypegenResult::~FAbiTypegenResult()
{
    if (Ptr) atg_result_free(Ptr);
}

FAbiTypegenResult::FAbiTypegenResult(FAbiTypegenResult&& Other) noexcept : Ptr(Other.Ptr)
{
    Other.Ptr = nullptr;
}

FAbiTypegenResult& FAbiTypegenResult::operator=(FAbiTypegenResult&& Other) noexcept
{
    if (this != &Other)
    {
        if (Ptr) atg_result_free(Ptr);
        Ptr = Other.Ptr;
        Other.Ptr = nullptr;
    }
    return *this;
}

bool FAbiTypegenResult::IsOk() const
{
    return Ptr && atg_result_error(Ptr) == nullptr &&
        CanCopyBytes(atg_result_len(Ptr), atg_result_data(Ptr));
}

bool FAbiTypegenResult::CanCopyBytes(size_t Len, const uint8* Data)
{
    return Len <= static_cast<size_t>(MAX_int32) && (Len == 0 || Data != nullptr);
}

FString FAbiTypegenResult::Error() const
{
    if (!Ptr) return TEXT("abi-typegen runtime returned null result");
    const char* Message = atg_result_error(Ptr);
    if (Message) return UTF8_TO_TCHAR(Message);
    if (!CanCopyBytes(atg_result_len(Ptr), atg_result_data(Ptr)))
        return TEXT("abi-typegen result bytes exceed Unreal's array limit or have no storage");
    return FString();
}

const atg_value* FAbiTypegenResult::Value() const
{
    return Ptr ? atg_result_value(Ptr) : nullptr;
}

TArray<uint8> FAbiTypegenResult::CopyBytes() const
{
    TArray<uint8> Out;
    if (!Ptr || atg_result_error(Ptr)) return Out;
    const size_t Len = atg_result_len(Ptr);
    const uint8* Data = atg_result_data(Ptr);
    if (!CanCopyBytes(Len, Data)) return Out;
    Out.Append(Data, static_cast<int32>(Len));
    return Out;
}

atg_result* FAbiTypegenResult::Release()
{
    atg_result* Out = Ptr;
    Ptr = nullptr;
    return Out;
}

bool AbiTypegen::HexToBytes(const FString& Input, TArray<uint8>& Out, FString& OutError)
{
    FString Hex = Input;
    Hex.TrimStartAndEndInline();
    if (Hex.StartsWith(TEXT("0x"), ESearchCase::IgnoreCase)) Hex.RightChopInline(2);
    if ((Hex.Len() & 1) != 0)
    {
        OutError = TEXT("hex string must contain an even number of digits");
        return false;
    }
    Out.Reset(Hex.Len() / 2);
    for (int32 Index = 0; Index < Hex.Len(); Index += 2)
    {
        const int32 Hi = HexNibble(Hex[Index]);
        const int32 Lo = HexNibble(Hex[Index + 1]);
        if (Hi < 0 || Lo < 0)
        {
            OutError = TEXT("hex string contains a non-hex character");
            Out.Reset();
            return false;
        }
        Out.Add(static_cast<uint8>((Hi << 4) | Lo));
    }
    return true;
}

FString AbiTypegen::BytesToHex(const uint8* Data, int32 Num, bool bPrefix)
{
    static const TCHAR Digits[] = TEXT("0123456789abcdef");
    FString Out;
    Out.Reserve((bPrefix ? 2 : 0) + Num * 2);
    if (bPrefix) Out += TEXT("0x");
    for (int32 I = 0; I < Num; ++I)
    {
        Out.AppendChar(Digits[(Data[I] >> 4) & 0xF]);
        Out.AppendChar(Digits[Data[I] & 0xF]);
    }
    return Out;
}

bool AbiTypegen::ParseAddress(const FString& Hex, atg_address& Out, FString& OutError)
{
    TArray<uint8> Bytes;
    if (!HexToBytes(Hex, Bytes, OutError)) return false;
    if (Bytes.Num() != 20)
    {
        OutError = TEXT("address must contain exactly 20 bytes");
        return false;
    }
    FMemory::Memcpy(Out.bytes, Bytes.GetData(), 20);
    return true;
}

FString AbiTypegen::AddressToHex(const atg_address& Address)
{
    return BytesToHex(Address.bytes, 20, true);
}

FAbiTypegenWord256 FAbiTypegenWord256::FromWord(const atg_word& Word)
{
    FAbiTypegenWord256 Out;
    Out.Hex = AbiTypegen::BytesToHex(Word.bytes, 32, true);
    return Out;
}

FAbiTypegenWord256 FAbiTypegenWord256::FromUInt64(uint64 Value)
{
    atg_word Word{};
    for (int32 I = 31; I >= 24; --I)
    {
        Word.bytes[I] = static_cast<uint8>(Value & 0xFF);
        Value >>= 8;
    }
    return FromWord(Word);
}

bool FAbiTypegenWord256::TryToWord(atg_word& Out, FString& OutError) const
{
    TArray<uint8> Bytes;
    if (!AbiTypegen::HexToBytes(Hex, Bytes, OutError)) return false;
    if (Bytes.IsEmpty())
    {
        OutError = TEXT("ABI word must contain at least one hex byte");
        return false;
    }
    if (Bytes.Num() > 32)
    {
        OutError = TEXT("ABI word exceeds 256 bits");
        return false;
    }
    FMemory::Memzero(Out.bytes, 32);
    if (Bytes.Num()) FMemory::Memcpy(Out.bytes + (32 - Bytes.Num()), Bytes.GetData(), Bytes.Num());
    return true;
}

bool FAbiTypegenWord256::IsCanonical() const
{
    atg_word Word{};
    FString ErrorText;
    return TryToWord(Word, ErrorText) && Hex == FromWord(Word).Hex;
}

FAbiTypegenAddress FAbiTypegenAddress::FromAddress(const atg_address& Address)
{
    FAbiTypegenAddress Out;
    Out.Hex = AbiTypegen::AddressToHex(Address);
    return Out;
}

bool FAbiTypegenAddress::TryToAddress(atg_address& Out, FString& OutError) const
{
    return AbiTypegen::ParseAddress(Hex, Out, OutError);
}
