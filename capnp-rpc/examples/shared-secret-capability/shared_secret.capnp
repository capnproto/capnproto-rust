@0x9ab4d9cc53638ac6;

# This interface is for testing purposes. Do not use this interface
# for security because it is vulnerable to replay attacks among other
# things.
interface SharedSecretAuthenticated(T) {
# This interface is generic over an underlying protected interface T
  authenticate @0 (sharedSecret :Text) -> (authenticated :T);
  # The authenticate method, which returns a capability to the underlying
  # protected interface when the passed secret is valid.
}
