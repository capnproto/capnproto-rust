@0xa074fbab61132cbd;

interface Subscription {}

interface Publisher(T) {
  subscribe @0 (subscriber: Subscriber(T)) -> (subscription: Subscription);
}

interface Subscriber(T) {
  pushValue @0 (value: T) -> ();
}
