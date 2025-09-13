# Interactive Mode

Interactive mode enables dynamic, real-time WebSocket communication where MockForge responds intelligently to client messages. Unlike replay mode's predetermined sequences, interactive mode supports complex conversational patterns, state management, and adaptive responses based on client input.

## Core Concepts

### Dynamic Response Logic
Interactive mode evaluates client messages and generates contextually appropriate responses using conditional logic, pattern matching, and state tracking.

### State Management
Connections maintain state across messages, enabling complex interactions like authentication flows, game mechanics, and multi-step processes.

### Message Processing Pipeline
1. **Receive** client message
2. **Parse** and validate input
3. **Evaluate** conditions and state
4. **Generate** appropriate response
5. **Update** connection state

## Basic Interactive Setup

### Simple Echo Server

```jsonl
{"ts":0,"dir":"out","text":"Echo server ready. Send me a message!"}
{"ts":0,"dir":"in","text":".*","response":"You said: {{request.ws.message}}"}
```

### Command Processor

```jsonl
{"ts":0,"dir":"out","text":"Available commands: HELP, TIME, ECHO <message>, QUIT"}
{"ts":0,"dir":"in","text":"^HELP$","response":"Commands: HELP, TIME, ECHO <msg>, QUIT"}
{"ts":0,"dir":"in","text":"^TIME$","response":"Current time: {{now}}"}
{"ts":0,"dir":"in","text":"^ECHO (.+)$","response":"Echo: {{request.ws.message.match(/^ECHO (.+)$/)[1]}}"}
{"ts":0,"dir":"in","text":"^QUIT$","response":"Goodbye!","close":true}
```

## Advanced Interactive Patterns

### Authentication Flow

```jsonl
{"ts":0,"dir":"out","text":"Welcome! Please login with: LOGIN <username> <password>"}
{"ts":0,"dir":"in","text":"^LOGIN (\\w+) (\\w+)$","response":"Authenticating {{request.ws.message.match(/^LOGIN (\\w+) (\\w+)$/)[1]}}...","state":"authenticating"}
{"ts":1000,"dir":"out","text":"Login successful! Welcome, {{request.ws.state.username}}!","condition":"{{request.ws.state.authenticating}}"}
{"ts":0,"dir":"out","text":"Login failed. Try again.","condition":"{{!request.ws.state.authenticating}}"}
```

### State-Based Conversations

```jsonl
{"ts":0,"dir":"out","text":"Welcome to the survey bot. What's your name?","state":"awaiting_name"}
{"ts":0,"dir":"in","text":".+","response":"Nice to meet you, {{request.ws.message}}! How old are you?","state":"awaiting_age","condition":"{{request.ws.state.awaiting_name}}"}
{"ts":0,"dir":"in","text":"^\\d+$","response":"Thanks! You're {{request.ws.message}} years old. Survey complete!","state":"complete","condition":"{{request.ws.state.awaiting_age}}"}
{"ts":0,"dir":"in","text":".*","response":"Please enter a valid age (numbers only).","condition":"{{request.ws.state.awaiting_age}}"}
```

### Game Mechanics

```jsonl
{"ts":0,"dir":"out","text":"Welcome to Number Guessing Game! I'm thinking of a number between 1-100.","state":"playing","game":{"target":42,"attempts":0}}
{"ts":0,"dir":"in","text":"^GUESS (\\d+)$","condition":"{{request.ws.state.playing}}","response":"{{#if (eq (parseInt request.ws.message.match(/^GUESS (\\d+)$/) [1]) request.ws.state.game.target)}}You won in {{request.ws.state.game.attempts + 1}} attempts!{{else}}{{#if (gt (parseInt request.ws.message.match(/^GUESS (\\d+)$/) [1]) request.ws.state.game.target)}}Too high!{{else}}Too low!{{/if}} Try again.{{/if}}","state":"{{#if (eq (parseInt request.ws.message.match(/^GUESS (\\d+)$/) [1]) request.ws.state.game.target)}}won{{else}}playing{{/if}}","game":{"target":"{{request.ws.state.game.target}}","attempts":"{{request.ws.state.game.attempts + 1}}"}}
```

## Message Processing Syntax

### Input Patterns

Interactive mode uses regex patterns to match client messages:

```jsonl
// Exact match
{"dir":"in","text":"hello","response":"Hi there!"}

// Case-insensitive match
{"dir":"in","text":"(?i)hello","response":"Hi there!"}

// Pattern with capture groups
{"dir":"in","text":"^NAME (.+)$","response":"Hello, {{request.ws.message.match(/^NAME (.+)$/)[1]}}!"}

// Optional elements
{"dir":"in","text":"^(HELP|help|\\?)$","response":"Available commands: ..."}
```

### Response Templates

Responses support the full MockForge template system:

```jsonl
{"dir":"in","text":".*","response":"Message received at {{now}}: {{request.ws.message}} (length: {{request.ws.message.length}})"}
```

### Conditions

Use template conditions to control when rules apply:

```jsonl
{"dir":"in","text":".*","condition":"{{request.ws.state.authenticated}}","response":"Welcome back!"}
{"dir":"in","text":".*","condition":"{{!request.ws.state.authenticated}}","response":"Please authenticate first."}
```

### State Updates

Modify connection state based on interactions:

```jsonl
// Set simple state
{"dir":"in","text":"START","response":"Starting...","state":"active"}

// Update complex state
{"dir":"in","text":"SCORE","response":"Current score: {{request.ws.state.score}}","state":"playing","score":"{{request.ws.state.score + 10}}"}
```

## Advanced Features

### Multi-Message Conversations

```jsonl
// Step 1: Greeting
{"ts":0,"dir":"out","text":"Hello! What's your favorite color?"}
{"ts":0,"dir":"in","text":".+","response":"{{request.ws.message}} is a great choice! What's your favorite food?","state":"asked_color","color":"{{request.ws.message}}","next":"food"}

// Step 2: Follow-up
{"ts":0,"dir":"out","text":"Based on your preferences, I recommend: ...","condition":"{{request.ws.state.next === 'complete'}}"}
{"ts":0,"dir":"in","text":".+","condition":"{{request.ws.state.next === 'food'}}","response":"Perfect! You like {{request.ws.state.color}} and {{request.ws.message}}. Here's a recommendation...","state":"complete"}
```

### Error Handling

```jsonl
{"ts":0,"dir":"out","text":"Enter a command:"}
{"ts":0,"dir":"in","text":"","response":"Empty input not allowed. Try again."}
{"ts":0,"dir":"in","text":"^.{100,}$","response":"Input too long (max 99 characters). Please shorten."}
{"ts":0,"dir":"in","text":"^INVALID.*","response":"Unknown command. Type HELP for available commands."}
{"ts":0,"dir":"in","text":".*","response":"Processing: {{request.ws.message}}"}
```

### Rate Limiting

```jsonl
{"ts":0,"dir":"in","text":".*","condition":"{{request.ws.state.messageCount < 10}}","response":"Message {{request.ws.state.messageCount + 1}}: {{request.ws.message}}","messageCount":"{{request.ws.state.messageCount + 1}}"}
{"ts":0,"dir":"in","text":".*","condition":"{{request.ws.state.messageCount >= 10}}","response":"Rate limit exceeded. Please wait."}
```

### Session Management

```jsonl
// Initialize session
{"ts":0,"dir":"out","text":"Session started: {{uuid}}","sessionId":"{{uuid}}","startTime":"{{now}}","messageCount":0}

// Track activity
{"ts":0,"dir":"in","text":".*","response":"Received","messageCount":"{{request.ws.state.messageCount + 1}}","lastActivity":"{{now}}","condition":"{{request.ws.state.active}}"}
```

## Template Functions for Interactive Mode

### Message Analysis

```jsonl
// Message properties
{"dir":"in","text":".*","response":"Length: {{request.ws.message.length}}, Uppercase: {{request.ws.message.toUpperCase()}}"}
```

### State Queries

```jsonl
// Check state existence
{"condition":"{{request.ws.state.userId}}","response":"Logged in as: {{request.ws.state.userId}}"}
{"condition":"{{!request.ws.state.userId}}","response":"Please log in first."}

// State comparisons
{"condition":"{{request.ws.state.score > 100}}","response":"High score achieved!"}
{"condition":"{{request.ws.state.level === 'expert'}}","response":"Expert mode enabled."}
```

### Time-based Logic

```jsonl
// Session timeout
{"condition":"{{request.ws.state.lastActivity && (now - request.ws.state.lastActivity) > 300000}}","response":"Session expired. Please reconnect.","close":true}

// Time-based greetings
{"response":"{{#if (gte (now.getHours()) 18)}}Good evening!{{else if (gte (now.getHours()) 12)}}Good afternoon!{{else}}Good morning!{{/if}}"}
```

## Creating Interactive Scenarios

### From Scratch

```bash
# Create a new interactive scenario
cat > interactive-chat.jsonl << 'EOF'
{"ts":0,"dir":"out","text":"ChatBot: Hello! How can I help you today?"}
{"ts":0,"dir":"in","text":"(?i).*help.*","response":"ChatBot: I can answer questions, tell jokes, or just chat. What would you like?"}
{"ts":0,"dir":"in","text":"(?i).*joke.*","response":"ChatBot: Why did the computer go to the doctor? It had a virus! 😂"}
{"ts":0,"dir":"in","text":"(?i).*bye.*","response":"ChatBot: Goodbye! Have a great day! 👋","close":true}
{"ts":0,"dir":"in","text":".*","response":"ChatBot: I'm not sure how to respond to that. Try asking for help!"}
EOF
```

### From Existing Logs

```bash
#!/bin/bash
# convert-logs-to-interactive.sh

# Extract conversation patterns from logs
grep "USER:" chat.log | sed 's/.*USER: //' | sort | uniq > user_patterns.txt
grep "BOT:" chat.log | sed 's/.*BOT: //' | sort | uniq > bot_responses.txt

# Generate interactive rules
paste user_patterns.txt bot_responses.txt | while IFS=$'\t' read -r user bot; do
  echo "{\"dir\":\"in\",\"text\":\"$(echo "$user" | sed 's/[^a-zA-Z0-9]/\\&/g')\",\"response\":\"$bot\"}"
done > interactive-from-logs.jsonl
```

### Testing Interactive Scenarios

```bash
#!/bin/bash
# test-interactive.sh

echo "Testing interactive WebSocket scenario..."

# Start MockForge with interactive file
mockforge serve --ws-replay-file interactive-test.jsonl &
SERVER_PID=$!
sleep 2

# Test conversation flow
node -e "
const WebSocket = require('ws');
const ws = new WebSocket('ws://localhost:3001/ws');

const conversation = [
  'Hello',
  'Tell me a joke',
  'What can you do?',
  'Goodbye'
];

let step = 0;

ws.on('open', () => {
  console.log('Connected, starting conversation...');
  ws.send(conversation[step++]);
});

ws.on('message', (data) => {
  const response = data.toString();
  console.log('Bot:', response);

  if (step < conversation.length) {
    setTimeout(() => {
      ws.send(conversation[step++]);
    }, 1000);
  } else {
    ws.close();
  }
});

ws.on('close', () => {
  console.log('Conversation complete');
  process.exit(0);
});

ws.on('error', (err) => {
  console.error('Error:', err);
  process.exit(1);
});
"

# Cleanup
kill $SERVER_PID
```

## Best Practices

### Design Principles

1. **Clear Conversation Flow**: Design conversations with clear paths and expectations
2. **Graceful Error Handling**: Provide helpful responses for unexpected input
3. **State Consistency**: Keep state updates predictable and logical
4. **Performance Awareness**: Avoid complex regex or template processing

### Pattern Guidelines

1. **Specific to General**: Order patterns from most specific to most general
2. **Anchored Regex**: Use `^` and `$` to avoid partial matches
3. **Case Handling**: Consider case sensitivity in user input
4. **Input Validation**: Validate and sanitize user input

### State Management

1. **Minimal State**: Store only necessary information in connection state
2. **State Validation**: Verify state consistency across interactions
3. **State Cleanup**: Clear state when conversations end
4. **State Persistence**: Consider state requirements for reconnection scenarios

### Debugging Interactive Scenarios

1. **Verbose Logging**: Enable detailed WebSocket logging
2. **State Inspection**: Log state changes during conversations
3. **Pattern Testing**: Test regex patterns independently
4. **Flow Tracing**: Track conversation paths through state changes

## Common Patterns

### Customer Support Chat

```jsonl
{"ts":0,"dir":"out","text":"Welcome to support! How can I help you? (Type your question or 'menu' for options)"}
{"ts":0,"dir":"in","text":"(?i)menu","response":"Options: 1) Password reset 2) Billing 3) Technical issue 4) Other","state":"menu"}
{"ts":0,"dir":"in","text":"(?i).*password.*","response":"I'll help you reset your password. What's your email address?","state":"password_reset","issue":"password"}
{"ts":0,"dir":"in","text":"(?i).*billing.*","response":"For billing questions, please visit our billing portal at billing.example.com","state":"billing"}
{"ts":0,"dir":"in","text":".*","response":"Thanks for your question: '{{request.ws.message}}'. A support agent will respond shortly. Your ticket ID is: {{uuid}}"}
```

### E-commerce Assistant

```jsonl
{"ts":0,"dir":"out","text":"Welcome to our store! What are you looking for?","state":"browsing"}
{"ts":0,"dir":"in","text":"(?i).*shirt.*","response":"We have various shirts: casual, formal, graphic. Which style interests you?","state":"shirt_selection","category":"shirts"}
{"ts":0,"dir":"in","text":"(?i).*size.*","response":"Available sizes: S, M, L, XL. Which size would you like?","state":"size_selection","condition":"{{request.ws.state.category}}"}
{"ts":0,"dir":"in","text":"(?i)(S|M|L|XL)","condition":"{{request.ws.state.size_selection}}","response":"Great! Adding {{request.ws.state.category}} in size {{request.ws.message.toUpperCase()}} to cart. Would you like to checkout or continue shopping?","state":"checkout_ready"}
```

### Game Server

```jsonl
{"ts":0,"dir":"out","text":"Welcome to the game server! Choose your character: WARRIOR, MAGE, ROGUE","state":"character_select"}
{"ts":0,"dir":"in","text":"(?i)^(warrior|mage|rogue)$","response":"Excellent choice! You selected {{request.ws.message.toUpperCase()}}. Your adventure begins now...","state":"playing","character":"{{request.ws.message.toLowerCase()}}","health":100,"level":1}
{"ts":0,"dir":"in","text":"(?i)stats","condition":"{{request.ws.state.playing}}","response":"Character: {{request.ws.state.character}}, Level: {{request.ws.state.level}}, Health: {{request.ws.state.health}}"}
{"ts":0,"dir":"in","text":"(?i)fight","condition":"{{request.ws.state.playing}}","response":"You encounter a monster! Roll for attack... {{randInt 1 20}}! {{#if (gte (randInt 1 20) 10)}}Victory!{{else}}Defeat!{{/if}}"}
```

## Integration Examples

### With Testing Frameworks

```javascript
// test-interactive.js
const WebSocket = require('ws');

class InteractiveWebSocketTester {
  constructor(url) {
    this.url = url;
    this.ws = null;
  }

  async connect() {
    return new Promise((resolve, reject) => {
      this.ws = new WebSocket(this.url);
      this.ws.on('open', () => resolve());
      this.ws.on('error', reject);
    });
  }

  async sendAndExpect(message, expectedResponse) {
    return new Promise((resolve, reject) => {
      const timeout = setTimeout(() => reject(new Error('Timeout')), 5000);

      this.ws.send(message);
      this.ws.once('message', (data) => {
        clearTimeout(timeout);
        const response = data.toString();
        if (response === expectedResponse) {
          resolve(response);
        } else {
          reject(new Error(`Expected "${expectedResponse}", got "${response}"`));
        }
      });
    });
  }

  close() {
    if (this.ws) this.ws.close();
  }
}

module.exports = InteractiveWebSocketTester;
```

### Load Testing Interactive Scenarios

```bash
#!/bin/bash
# load-test-interactive.sh

CONCURRENT_USERS=50
DURATION=300

echo "Load testing interactive WebSocket with $CONCURRENT_USERS concurrent users for ${DURATION}s"

# Start MockForge
mockforge serve --ws-replay-file interactive-load-test.jsonl &
SERVER_PID=$!
sleep 2

# Run load test
node load-test-interactive.js $CONCURRENT_USERS $DURATION

# Generate report
echo "Generating performance report..."
node analyze-results.js

# Cleanup
kill $SERVER_PID
```

Interactive mode transforms MockForge from a simple message player into an intelligent conversation partner, enabling sophisticated testing scenarios that adapt to client behavior and maintain complex interaction state.
