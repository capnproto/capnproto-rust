@0xf278bebf673978ed;

interface HttpProxy {
    newSession @0 (baseUrl :Text) -> (session :HttpSession);
}

interface HttpSession {
    get @0 (path :Text) -> (responseCode :UInt32, body :Data);
    post @1 (path :Text, body :Data) -> (responseCode: UInt32);
}
