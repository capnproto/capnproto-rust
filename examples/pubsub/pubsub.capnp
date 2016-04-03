@0xa074fbab61132cbd;

interface Handle {}

interface Publisher {
  register @0 (subscriber: Subscriber) -> (handle: Handle);
}

interface Subscriber {
  pushValues @0 (values: Float64) -> ();
}
