# NL2CMD - Natural Language to Command

Convert natural language descriptions into safe, executable Linux/Unix commands using AI.

## What is NL2CMD?

NL2CMD is a microservice that translates human-readable text into command-line commands. Instead of remembering complex command syntax, you can describe what you want to do in plain English, and the AI will generate the appropriate command.

### Example

**Input:**
```
"show disk usage"
```

**Output:**
```json
{
  "result": {
    "cmd": "df -h",
    "runnable": true
  }
}
```

## Why is it Useful?

- 🚀 **Faster Development**: No need to look up command syntax
- 📚 **Learning Tool**: Great for beginners learning command-line
- 🛡️ **Safe**: Blocks destructive commands automatically
- 🤖 **AI-Powered**: Uses Claude AI for intelligent command generation
- 🔧 **Microservice**: Easy to integrate into larger applications

## Prerequisites

- Go 1.25.1 or higher
- Claude API key from [Anthropic](https://console.anthropic.com/)

## Installation

1. **Clone and navigate to the directory**
   ```bash
   cd /Users/chandrakanthkannam/Git/run-it/nl2cmd
   ```

2. **Install dependencies**
   ```bash
   go mod download
   ```

3. **Set your Claude API key**
   ```bash
   export CLAUDE_API_KEY="your-api-key-here"
   ```
   
   Or add to your `~/.zshrc` for persistence:
   ```bash
   echo 'export CLAUDE_API_KEY="your-api-key-here"' >> ~/.zshrc
   source ~/.zshrc
   ```

## Running the Server

### Method 1: Using Go Run (Development)

```bash
go run main.go
```

The server will start on `http://127.0.0.1:3400`

You should see output like:
```
2025/10/04 00:40:16 INFO server listening on 127.0.0.1:3400
```

### Method 2: Build and Run (Production)

```bash
# Build the binary
go build -o nl2cmd main.go

# Run the binary
./nl2cmd
```

### Method 3: Using Genkit Server Command

If you have Genkit CLI installed:

```bash
# Install Genkit CLI (if not already installed)
npm install -g @genkit-ai/cli

# Run with Genkit dev server
genkit start -- go run main.go
```

This will start the Genkit development UI at `http://localhost:4000` for testing flows.

## Usage Examples

### Using curl

**Valid Command Request:**
```bash
curl -X POST "http://localhost:3400/nl2CmdFlow" \
  -H "Content-Type: application/json" \
  -d '{"data": {"nl2cmd": "list all files"}}'
```

Response:
```json
{
  "result": {
    "cmd": "ls -la",
    "runnable": true
  }
}
```

**Invalid/Unclear Request:**
```bash
curl -X POST "http://localhost:3400/nl2CmdFlow" \
  -H "Content-Type: application/json" \
  -d '{"data": {"nl2cmd": "that is asdfs"}}'
```

Response:
```json
{
  "result": {
    "cmd": "Invalid command request",
    "runnable": false
  }
}
```

**Destructive Command (Blocked):**
```bash
curl -X POST "http://localhost:3400/nl2CmdFlow" \
  -H "Content-Type: application/json" \
  -d '{"data": {"nl2cmd": "delete all files"}}'
```

Response:
```json
{
  "result": {
    "cmd": "Command restricted - contact administrator",
    "runnable": false
  }
}
```

### Using HTTPie (if installed)

```bash
http POST http://localhost:3400/nl2CmdFlow \
  data:='{"nl2cmd": "show disk usage"}'
```

### Integration Examples

**From Rust (using reqwest):**
```rust
let client = reqwest::Client::new();
let request_body = serde_json::json!({
    "data": {
        "nl2cmd": "list all files"
    }
});

let response = client
    .post("http://localhost:3400/nl2CmdFlow")
    .json(&request_body)
    .send()
    .await?;

let result: serde_json::Value = response.json().await?;
```

**From JavaScript/Node.js:**
```javascript
const response = await fetch('http://localhost:3400/nl2CmdFlow', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({
    data: { nl2cmd: 'show disk usage' }
  })
});

const result = await response.json();
console.log(result.result.cmd); // "df -h"
```

## API Reference

### Endpoint
```
POST /nl2CmdFlow
```

### Request Format
```json
{
  "data": {
    "nl2cmd": "string - natural language command description"
  }
}
```

### Response Format
```json
{
  "result": {
    "cmd": "string - generated command or error message",
    "runnable": "boolean - true if safe to execute, false if blocked/invalid"
  }
}
```

### Status Codes
- `200 OK` - Request processed successfully
- `500 Internal Server Error` - AI processing failed

## Project Structure

```
nl2cmd/
├── main.go              # Main server application
├── prompts/
│   └── nl2cmd.prompt    # AI prompt configuration (controls AI behavior)
├── go.mod               # Go dependencies
├── go.sum               # Dependency checksums
└── README.md            # This file
```

## Configuration

### Environment Variables

| Variable | Description | Required | Default |
|----------|-------------|----------|---------|
| `CLAUDE_API_KEY` | Anthropic API key | Yes | - |

### Customizing the Prompt

Edit `prompts/nl2cmd.prompt` to customize:
- AI model (currently Claude 3.5 Haiku)
- System instructions
- Safety rules
- Output format

```yaml
---
model: anthropic/claude-3-5-haiku-20241022
input:
  schema:
    nl2cmd: string
output:
  format: json
  schema:
    cmd: string
    runnable: boolean
---
{{role "system"}}
Your custom instructions here...
```

## Troubleshooting

### Server won't start

**Error: `bind: address already in use`**
```bash
# Find process using port 3400
lsof -ti:3400

# Kill the process
kill -9 $(lsof -ti:3400)

# Try starting again
go run main.go
```

### API Key Issues

**Error: `Invalid Anthropic API Key`**
```bash
# Check if API key is set
echo $CLAUDE_API_KEY

# If empty, set it
export CLAUDE_API_KEY="your-api-key-here"

# Restart the server
go run main.go
```

### JSON Parsing Errors

**Error: `model failed to generate output matching expected schema`**

This was fixed in the current version. If you still see it:
1. Ensure `format: json` is in `prompts/nl2cmd.prompt`
2. Check that the prompt explicitly requires JSON output
3. Restart the server after prompt changes

### Debug Logging

The server logs all requests and AI responses. Check the console output:
```
2025/10/04 00:44:04 Received input: &{NL2Cmd:show disk usage}
2025/10/04 00:44:04 Executing prompt...
2025/10/04 00:44:07 Raw model response: ...
2025/10/04 00:44:07 Response text: {"cmd":"df -h","runnable":true}
```

## Development

### Running Tests
```bash
go test ./...
```

### Building for Production
```bash
go build -o nl2cmd main.go
```

### Hot Reload (Development)
Install Air for hot reloading:
```bash
go install github.com/cosmtrek/air@latest
air
```

## Safety Features

✅ **Built-in Protection:**
- Blocks delete operations
- Blocks destructive commands (rm, rmdir, etc.)
- Returns error messages for invalid input
- All commands are validated before being marked as runnable

⚠️ **Important:** Always review generated commands before execution in production environments.

## Use Cases

1. **CLI Learning Tool**: Help new users learn command-line syntax
2. **DevOps Automation**: Natural language interface for common tasks
3. **Documentation**: Convert task descriptions to executable commands
4. **Integration**: Embed in larger applications needing command generation

## Technology Stack

- **Go 1.25.1** - Server runtime
- **Firebase Genkit** - AI workflow framework
- **Anthropic Claude 3.5 Haiku** - Fast, efficient AI model
- **REST API** - Simple HTTP JSON interface

## License

[Add your license here]

## Support

For issues or questions:
1. Check the Troubleshooting section
2. Review server logs for error messages
3. Verify API key and environment setup

---

**Note**: This service requires an active internet connection and a valid Anthropic API key to function.
