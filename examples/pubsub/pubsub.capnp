@0xa074fbab61132cbd;

interface Subscription {}

interface Publisher {
  subscribe @0 (subscriber: Subscriber) -> (subscription: Subscription);
}

interface Subscriber {
  pushValue @0 (value: Float64) -> ();
}
