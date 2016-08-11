@0xa074fbab61132cbd;

interface Handle {}

interface Publisher {
  register @0 (subscriber: Subscriber) -> (handle: Handle);
}

interface Subscriber {
  pushValue @0 (value: Float64) -> ();
}
