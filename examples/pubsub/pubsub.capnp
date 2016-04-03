@0xa074fbab61132cbd;

interface Publisher {
  interface Listener {
    pushValues @0 (values: Float64) -> ();
  }
  registerListener @0 (listener: Listener) -> ();
}
