# MockForge

## Run instructions

cargo build
MOCKFORGE_LATENCY_ENABLED=true MOCKFORGE_FAILURES_ENABLED=false \
cargo run -p mockforge-cli -- --spec mockforge/examples/openapi-demo.json --http-port 3000 --ws-port 3001 --grpc-port 50051

## HTTP

curl <http://localhost:3000/ping>

## WS (scripted replay)

export MOCKFORGE_WS_REPLAY_FILE=mockforge/examples/ws-demo.jsonl

## then connect to ws://localhost:3001/ws and send "CLIENT_READY"

Using websocat (command line tool):
websocat ws://localhost:3001/ws
Then type CLIENT_READY and press Enter.

Using wscat (Node.js tool):
wscat -c ws://localhost:3001/ws
Then type CLIENT_READY and press Enter.

Using JavaScript in browser console:
const ws = new WebSocket('ws://localhost:3001/ws');
ws.onopen = () => ws.send('CLIENT_READY');
ws.onmessage = (event) => console.log('Received:', event.data);

Using curl (if server supports it):
curl --include --no-buffer --header "Connection: Upgrade" --header "Upgrade: websocket" --header
"Sec-WebSocket-Key: SGVsbG8sIHdvcmxkIQ==" --header "Sec-WebSocket-Version: 13" ws://localhost:3001/ws

## gRPC

grpcurl -plaintext -proto crates/mockforge-grpc/proto/gretter.proto -d '{"name":"Ray"}' localhost:50051 mockforge.greeter.Greeter/SayHello

echo -e '{"name":"one"}\n{"name":"two"}' | grpcurl -plaintext -proto crates/mockforge-grpc/proto/gretter.proto -d @ localhost:50051 mockforge.greeter.Greeter/SayHelloClientStream

echo -e '{"name":"first"}\n{"name":"second"}' | grpcurl -plaintext -proto crates/mockforge-grpc/proto/gretter.proto -d @ localhost:50051 mockforge.greeter.Greeter/Chat
