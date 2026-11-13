# Feature coverage matrix

`Supported` means an official product document describes the capability. `Unknown` means this harness has not established official documentation for that cell. Feature presence does not imply parity or equivalent implementation depth.

| Capability | MockForge | WireMock | Prism | Mockoon |
|---|---|---|---|---|
| HTTP/REST mocking | [HTTP](https://mockforge.dev/docs/protocols/http) | [Stubbing](https://wiremock.org/docs/stubbing/) | [Mocking](https://docs.stoplight.io/docs/prism/README.md) | [API mocking](https://mockoon.com/docs/latest/) |
| OpenAPI import/mock | [OpenAPI](https://mockforge.dev/docs/openapi) | Unknown | [OpenAPI](https://docs.stoplight.io/docs/prism/README.md) | [OpenAPI import](https://mockoon.com/docs/latest/import-export/data-import/) |
| Recording and replay | [Recording](https://mockforge.dev/docs/recording) | [Record/playback](https://wiremock.org/docs/record-playback/) | Unknown | [Proxy mode](https://mockoon.com/docs/latest/proxy/) |
| Stateful mocks | [Stateful behavior](https://mockforge.dev/docs/stateful-behavior) | [Scenarios](https://wiremock.org/docs/stateful-behaviour/) | Unknown | [Dynamic templating](https://mockoon.com/docs/latest/response-templates/overview/) |
| Fault injection | [Chaos](https://mockforge.dev/docs/chaos-engineering) | [Fault simulation](https://wiremock.org/docs/simulating-faults/) | Unknown | Unknown |
| gRPC | [gRPC](https://mockforge.dev/docs/protocols/grpc) | [Extensions](https://wiremock.org/docs/extensions/) | Unknown | Unknown |
| WebSocket | [WebSocket](https://mockforge.dev/docs/protocols/websocket) | [WebSockets](https://wiremock.org/docs/websockets/) | Unknown | [WebSockets](https://mockoon.com/docs/latest/api-endpoints/websockets/) |
| GraphQL | [GraphQL](https://mockforge.dev/docs/protocols/graphql) | [Extensions](https://wiremock.org/docs/extensions/) | Unknown | Unknown |
| Proxying | [Proxy](https://mockforge.dev/docs/proxy) | [Proxying](https://wiremock.org/docs/proxying/) | [Proxy](https://docs.stoplight.io/docs/prism/README.md) | [Proxy mode](https://mockoon.com/docs/latest/proxy/) |

URLs are vendor documentation entry points and should be reviewed before publishing a coverage claim. Unknown is intentionally not a negative capability claim.
