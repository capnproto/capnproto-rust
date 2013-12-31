@0xee82762a0e12b9f0;

struct Observation {
    timestamp  @0 : Int64;
    x          @1 : Float32;
    y          @2 : Float32;
    red        @3 : UInt8;
    green      @4 : UInt8;
    blue       @5 : UInt8;

    diagnostic : union {
       ok @6 : Void;
       warning @7 : Text;
    }
}