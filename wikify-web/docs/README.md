# Wikify API Documentation

This directory contains comprehensive API documentation for the Wikify Web Server.

## Available Documentation

### 📖 API Overview
- **[API.md](./API.md)** - Complete API reference with examples

### 🔗 REST API (OpenAPI 3.0)
- **Specification**: Available at `/api-docs/openapi.yaml` and `/api-docs/openapi.json`
- **Interactive Documentation**: Available at `/docs` (Swagger UI)
- **Local URLs**:
  - YAML: http://localhost:8080/api-docs/openapi.yaml
  - JSON: http://localhost:8080/api-docs/openapi.json
  - Swagger UI: http://localhost:8080/docs

### ⚡ WebSocket API (AsyncAPI 3.0)
- **Specification**: Available at `/api-docs/asyncapi.yaml` and `/api-docs/asyncapi.json`
- **Local URLs**:
  - YAML: http://localhost:8080/api-docs/asyncapi.yaml
  - JSON: http://localhost:8080/api-docs/asyncapi.json

## WebSocket API Overview

The Wikify WebSocket API provides real-time communication for:

- **Chat Operations**: Send questions and receive AI-powered responses
- **Wiki Generation**: Real-time progress updates for wiki generation
- **Repository Indexing**: Progress tracking for repository analysis
- **Research Operations**: Deep research with iterative progress updates
- **System Notifications**: Error handling and status updates

### Connection
```javascript
const ws = new WebSocket('ws://localhost:8080/ws');
```

### Message Format
All messages follow a consistent JSON format with a `type` field:

```json
{
  "type": "Chat",
  "repository_id": "repo-123",
  "question": "How does authentication work?",
  "timestamp": "2024-01-01T12:00:00Z"
}
```

### Supported Message Types

#### Client → Server
- `Chat` - Send chat messages
- `WikiGenerate` - Request wiki generation
- `Ping` - Heartbeat ping

#### Server → Client
- `ChatResponse` - AI responses (including streaming)
- `ChatError` - Chat operation errors
- `WikiProgress` - Wiki generation progress
- `WikiComplete` - Wiki generation completion
- `WikiError` - Wiki generation errors
- `IndexStart` - Indexing started
- `IndexProgress` - Indexing progress
- `IndexComplete` - Indexing completion
- `IndexError` - Indexing errors
- `ResearchStart` - Research started
- `ResearchProgress` - Research progress
- `ResearchComplete` - Research completion
- `ResearchError` - Research errors
- `Pong` - Heartbeat response
- `Error` - General system errors

## Using the Documentation

### For Frontend Developers
1. Use the **AsyncAPI specification** to understand WebSocket message formats
2. Reference the **OpenAPI specification** for REST endpoints
3. Check the **API.md** for detailed examples and usage patterns

### For API Consumers
1. Start with the **Swagger UI** at `/docs` for interactive REST API exploration
2. Download the **AsyncAPI YAML** for WebSocket client generation
3. Use the **OpenAPI JSON** for REST client generation

### For Integration Testing
1. Use the specifications to generate test clients
2. Validate message formats against the schemas
3. Test both REST and WebSocket endpoints comprehensively

## Tools and Ecosystem

### AsyncAPI Tools
- **AsyncAPI Studio**: https://studio.asyncapi.com/ (paste the YAML URL)
- **AsyncAPI Generator**: Generate client code from the specification
- **AsyncAPI CLI**: Validate and work with AsyncAPI documents

### OpenAPI Tools
- **Swagger Editor**: https://editor.swagger.io/ (paste the YAML URL)
- **OpenAPI Generator**: Generate client SDKs
- **Postman**: Import OpenAPI specification for testing

## Development Workflow

1. **Design**: Update specifications when adding new endpoints or messages
2. **Validate**: Use AsyncAPI/OpenAPI tools to validate specifications
3. **Generate**: Use code generators for client libraries
4. **Test**: Validate implementations against specifications
5. **Document**: Keep API.md in sync with specification changes

## Notes

- All timestamps use ISO 8601 format (`YYYY-MM-DDTHH:mm:ssZ`)
- WebSocket connections support automatic reconnection
- Message IDs are optional but recommended for tracking
- Progress values are decimals between 0 and 1
- Error messages include optional details for debugging
