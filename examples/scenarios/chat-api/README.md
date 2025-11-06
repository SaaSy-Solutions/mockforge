# Chat API Scenario

Real-time chat API with typing indicators, message history, and user presence.

## Features

- **Message History**: Retrieve and send messages
- **Typing Indicators**: Real-time typing status updates
- **User Presence**: Track online/offline status
- **WebSocket Support**: Real-time messaging via WebSocket

## API Endpoints

### Messages
- `GET /api/messages?channelId={id}` - Get message history
- `POST /api/messages` - Send a message

### Typing Indicators
- `POST /api/typing` - Set typing indicator status

## Usage

1. Install the scenario:
   ```bash
   mockforge scenario install ./examples/scenarios/chat-api
   ```

2. Apply to your workspace:
   ```bash
   mockforge scenario use chat-api
   ```

3. Start the server:
   ```bash
   mockforge serve --config config.yaml
   ```

## WebSocket Events

Connect to `ws://localhost:3001` for real-time updates:

- `message` - New message received
- `typing` - User typing status changed
- `presence` - User presence changed
