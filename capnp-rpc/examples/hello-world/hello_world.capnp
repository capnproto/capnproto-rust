@0x9663f4dd604afa35;

interface HelloWorld {
    struct HelloRequest {
        name @0 :Text;
    }

    struct HelloReply {
        message @0 :Text;
    }

    sayHello @0 (request: HelloRequest) -> (reply: HelloReply);
}