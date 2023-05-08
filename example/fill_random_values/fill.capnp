@0xa2e5682ebdc1a289;

struct I8Range  { min @0 :    Int8; max @1 :    Int8; }
struct I16Range { min @0 :   Int16; max @1 :   Int16; }
struct I32Range { min @0 :   Int32; max @1 :   Int32; }
struct I64Range { min @0 :   Int64; max @1 :   Int64; }
struct U8Range  { min @0 :   UInt8; max @1 :   UInt8; }
struct U16Range { min @0 :  UInt16; max @1 :  UInt16; }
struct U32Range { min @0 :  UInt32; max @1 :  UInt32; }
struct U64Range { min @0 :  UInt64; max @1 :  UInt64; }
struct F32Range { min @0 : Float32; max @1 : Float32; }
struct F64Range { min @0 : Float64; max @1 : Float64; }

annotation int8Range(field)  : I8Range;
annotation int16Range(field)  : I16Range;
annotation int32Range(field)  : I32Range;
annotation int64Range(field)  : I64Range;
annotation uint8Range(field)  : U8Range;
annotation uint16Range(field)  : U16Range;
annotation uint32Range(field)  : U32Range;
annotation uint64Range(field)  : U64Range;
annotation float32Range(field)  : F32Range;
annotation float64Range(field)  : F64Range;


annotation lengthRange(field) : U32Range;

struct SelectFrom(T) {
  annotation choices(field) : T;
  # Selects from a set of choices. `T` should be a list type `List(S)` where `S`
  # is the type of the field being annotated.
}

annotation phoneNumber(field) : Void;
# Indicates that a text field should have a value that looks like a phone number.

annotation nullProbability(field) : Float64;
# Specifies the rate at which a pointer field should be set as null.
