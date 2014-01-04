@0xee82762a0e12b9f0;

struct Observation {
    timestamp  @0 : Int64;          # seconds
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

struct Grid {
   cells @0 : List(List(Cell));
   numberOfUpdates @1 : UInt32;
   latestTimestamp @2 : Int64;

   struct Cell {
      latestTimestamp @0 : Int64;
      numberOfUpdates @1 : UInt32;
      meanRed         @2 : Float32;
      meanGreen       @3 : Float32;
      meanBlue        @4 : Float32;
   }
}