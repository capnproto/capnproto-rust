@0xa074fbab61132cbd;

interface Publisher {
  register @0 (subscriber: Subscriber) -> ();
}

interface Subscriber {
  pushValues @0 (values: Float64) -> ();
}
