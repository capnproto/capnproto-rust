@0xca7ae4b25d718dfc;

using Fill = import "fill.capnp";

const palette: List(Color) = [
 (red = 0x22, green = 0xd7, blue = 0xb5),
 (red = 0x11, green = 0xb1, blue = 0x86),
 (red = 0x7c, green = 0xa4, blue = 0xf5),
 (red = 0xe7, green = 0x60, blue = 0x1d),
 (red = 0x25, green = 0x23, blue = 0x25),
 (red = 0x89, green = 0x74, blue = 0x59),
];
# Try adding this annotation to the Color fields below:
# $Fill.SelectFrom(List(Color)).choices(.palette);

struct Color {
  red   @0 : UInt8;
  green @1 : UInt8;
  blue  @2 : UInt8;
}

struct Point {
  # A point in normalized coordinates. (0,0) is the upper-left of
  # the current subcanvas, and (1,1) is the lower-right of the current
  # subcanvas.

  x @0 : Float64;
  y @1 : Float64;
}

struct Line {
  start @0 : Point;
  end   @1 : Point;

  thickness @2 : Float64 $Fill.float64Range((min = 0.01, max = 0.95));
  # Stroke width, as a percent of the current subcanvas's diagonal length.

  color @3 : Color;
}

struct Circle {
  center @0 : Point;
  # The center of the circle.

  radius @1 : Float64 $Fill.float64Range((min = 0.01, max = 0.25));
  # The radius of the circle, as a proportion of the current
  # subcanvas's diagonal length.

  fillColor @2 : Color;
}

struct Subcanvas {
  # A canvas contained in a larger canvas.

  center @0 : Point;
  width @1 : Float64;
  height @2 : Float64;
  canvas @3 : Canvas;
}

struct Canvas {
  # A canvas containing some geometric elements.

  backgroundColor @0 : Color;
  lines @1 : List(Line) $Fill.lengthRange((max = 5));
  circles @2 : List(Circle) $Fill.lengthRange((max = 5));
  subcanvases @3 : List(Subcanvas) $Fill.lengthRange((max = 3));
}
