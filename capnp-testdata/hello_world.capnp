@0xf1bd251134b6b183;

interface HelloWorld {
    helloWorld @0 (name: Text) -> (greeting: Text);
    # Responds with a greeting appropriate for name.

    broadcastHello @1 (names: List(Text)) -> (greetings: List(Text));
    # Responds with greetings for each of the names.

    connect @2 (name: Text) -> (connection: Connection);
    # Returns a Connection to the named entity.

    interface Connection {
        say @0 (text: Text) -> (response: Text);
        # Says something to the named person.  Returns their response.
    }
}