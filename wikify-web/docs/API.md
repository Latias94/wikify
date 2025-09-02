# Wikify Web API Documentation

## Overview

The Wikify Web API provides RESTful endpoints and WebSocket connections for repository analysis, chat functionality, and wiki generation.

## API Specifications

- **REST API**: [OpenAPI 3.0 Specification](/api-docs/openapi.yaml) | [Swagger UI](/docs)
- **WebSocket API**: [AsyncAPI 3.0 Specification](/api-docs/asyncapi.yaml)

## Base URL

```
http://localhost:8080/api
```

## Authentication

Currently, no authentication is required. Future versions will support JWT-based authentication.

## REST API Endpoints

### Health Check

**GET** `/health`

Check the server health status.

**Response:**
```json
{
  "status": "healthy",
  "timestamp": "2024-01-01T00:00:00Z",
  "version": "0.1.0"
}
```

### Repository Management

#### Initialize Repository

**POST** `/repositories`

Initialize a new repository for analysis.

**Request Body:**
```json
{
  "repository": "https://github.com/user/repo",
  "repo_type": "github",
  "access_token": "optional-token"
}
```

**Response:**
```json
{
  "session_id": "uuid-string",
  "status": "initialized",
  "repository": {
    "name": "repo",
    "url": "https://github.com/user/repo",
    "type": "github"
  }
}
```

#### Get Repository Info

**GET** `/repositories/{session_id}`

Get information about an initialized repository.

**Response:**
```json
{
  "session_id": "uuid-string",
  "repository": {
    "name": "repo",
    "url": "https://github.com/user/repo",
    "type": "github",
    "size_bytes": 1024000,
    "file_count": 150,
    "last_updated": "2024-01-01T00:00:00Z"
  },
  "status": "ready"
}
```

#### List Repositories (SQLite feature)

**GET** `/repositories`

List all repositories stored in the database.

**Response:**
```json
{
  "repositories": [
    {
      "id": "uuid-string",
      "name": "repo-name",
      "repo_path": "/path/to/repo",
      "repo_type": "github",
      "status": "indexed",
      "created_at": "2024-01-01T00:00:00Z",
      "last_indexed_at": "2024-01-01T01:00:00Z"
    }
  ],
  "count": 1
}
```

### Chat Functionality

#### Chat Query

**POST** `/chat`

Send a question about the repository.

**Request Body:**
```json
{
  "session_id": "uuid-string",
  "question": "How does authentication work in this codebase?",
  "context": "optional-context-string"
}
```

**Response:**
```json
{
  "answer": "The authentication system uses JWT tokens...",
  "sources": [
    {
      "file_path": "src/auth.rs",
      "content": "relevant code snippet",
      "line_start": 10,
      "line_end": 25,
      "score": 0.95
    }
  ],
  "session_id": "uuid-string",
  "timestamp": "2024-01-01T00:00:00Z"
}
```

#### Streaming Chat (Placeholder)

**POST** `/chat/stream`

Stream chat responses in real-time.

**Note:** Currently returns a placeholder response. Use WebSocket for real-time chat.

### Session Management (SQLite feature)

#### Get Sessions

**GET** `/sessions`

List all chat sessions.

**Response:**
```json
{
  "sessions": [
    {
      "id": "uuid-string",
      "repository_id": "repo-uuid",
      "created_at": "2024-01-01T00:00:00Z",
      "last_activity": "2024-01-01T01:00:00Z",
      "is_active": true
    }
  ],
  "count": 1
}
```

#### Get Query History

**GET** `/history/{repository_id}`

Get chat history for a specific repository.

**Response:**
```json
{
  "queries": [
    {
      "id": "uuid-string",
      "session_id": "session-uuid",
      "question": "How does this work?",
      "answer": "It works by...",
      "created_at": "2024-01-01T00:00:00Z"
    }
  ],
  "count": 1
}
```

### Wiki Generation

#### Generate Wiki

**POST** `/wiki/generate`

Generate wiki documentation for a repository.

**Request Body:**
```json
{
  "session_id": "uuid-string",
  "config": {
    "language": "en",
    "max_pages": 50,
    "include_diagrams": true,
    "comprehensive_view": false
  }
}
```

**Response:**
```json
{
  "wiki_id": "uuid-string",
  "status": "success",
  "pages_count": 25,
  "sections_count": 8
}
```

#### Get Generated Wiki

**GET** `/wiki/{session_id}`

Retrieve generated wiki content.

**Response:**
```json
{
  "id": "uuid-string",
  "title": "Repository Wiki",
  "description": "Comprehensive documentation",
  "pages": [...],
  "sections": [...],
  "metadata": {...}
}
```

## WebSocket Endpoints

### Unified WebSocket

**WS** `/ws/`

Unified real-time communication endpoint for all features including chat, wiki generation, indexing progress, and research updates.

**Message Types:**

**Chat Request:**
```json
{
  "type": "Chat",
  "session_id": "uuid-string",
  "question": "Your question here",
  "context": "optional-context"
}
```

**Chat Response:**
```json
{
  "type": "ChatResponse",
  "session_id": "uuid-string",
  "answer": "The answer to your question...",
  "sources": [...],
  "timestamp": "2024-01-01T00:00:00Z"
}
```

### Wiki Generation WebSocket

**WS** `/ws/wiki`

Real-time wiki generation progress.

**Message Types:**

**Wiki Generation Request:**
```json
{
  "type": "WikiGenerate",
  "session_id": "uuid-string",
  "config": {...}
}
```

**Progress Update:**
```json
{
  "type": "WikiProgress",
  "session_id": "uuid-string",
  "progress": 0.5,
  "current_step": "Analyzing code structure",
  "total_steps": 10,
  "completed_steps": 5
}
```

**Completion:**
```json
{
  "type": "WikiComplete",
  "session_id": "uuid-string",
  "wiki_id": "uuid-string",
  "pages_count": 25,
  "sections_count": 8
}
```

### Repository Indexing WebSocket

**WS** `/ws/index`

Real-time repository indexing progress.

**Progress Update:**
```json
{
  "type": "IndexProgress",
  "session_id": "uuid-string",
  "progress": 0.75,
  "files_processed": 75,
  "total_files": 100,
  "current_file": "src/main.rs"
}
```

## Error Responses

All endpoints may return error responses in the following format:

```json
{
  "error": "Error message",
  "code": "ERROR_CODE",
  "details": "Additional error details"
}
```

**Common HTTP Status Codes:**
- `200 OK` - Success
- `400 Bad Request` - Invalid request data
- `404 Not Found` - Resource not found
- `500 Internal Server Error` - Server error

## Rate Limiting

Currently, no rate limiting is implemented. Future versions will include rate limiting for API endpoints.

## Configuration

The server can be configured using environment variables or command-line arguments:

- `WIKIFY_HOST` - Server host (default: 127.0.0.1)
- `WIKIFY_PORT` - Server port (default: 8080)
- `WIKIFY_DATABASE_URL` - Database URL for persistence
- `WIKIFY_STATIC_DIR` - Static files directory
- `WIKIFY_DEV_MODE` - Enable development mode
