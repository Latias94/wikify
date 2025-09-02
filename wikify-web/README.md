# 🌐 Wikify Web Server

A high-performance web server for Wikify, built with Rust and Axum. Provides REST API endpoints, WebSocket support, and serves the frontend application.

## ✨ Features

- 🚀 **High Performance**: Built with Rust and Axum for maximum performance
- 🔌 **REST API**: Complete API for repository and session management
- 🔄 **WebSocket Support**: Real-time communication for chat functionality
- 🗄️ **Database Integration**: SQLite support for data persistence
- 🔒 **Security**: Built-in security headers and rate limiting
- 🌍 **CORS Support**: Configurable cross-origin resource sharing
- 📊 **Metrics**: Performance monitoring and health checks
- 🎨 **Static Serving**: Serves frontend assets and SPA fallback

## 🛠️ Technology Stack

- **Axum** - Modern async web framework
- **Tokio** - Async runtime
- **SQLite** - Embedded database
- **Tower** - Middleware and services
- **Serde** - Serialization/deserialization
- **Tracing** - Structured logging

## 🚀 Quick Start

### Prerequisites

- Rust 1.70+ with Cargo
- SQLite (optional, for database features)

### Installation

```bash
# Clone the repository
git clone <repository-url>
cd wikify/wikify-web

# Build the project
cargo build --release
```

### Basic Usage

```bash
# Start development server
cargo run --bin wikify-web -- --dev --host localhost --port 8080

# Start production server
cargo run --bin wikify-web -- --host 0.0.0.0 --port 8080

# With custom configuration
cargo run --bin wikify-web -- --config ./config.toml
```

## ⚙️ Configuration

### Environment Variables

#### Basic Server Configuration

| Variable | Description | Default | Example |
|----------|-------------|---------|---------|
| `WIKIFY_HOST` | Server bind address | `127.0.0.1` | `localhost`, `0.0.0.0` |
| `WIKIFY_PORT` | Server port | `8080` | `3000`, `8080` |
| `WIKIFY_DEV_MODE` | Enable development mode | `false` | `true`, `false` |
| `WIKIFY_STATIC_DIR` | Static files directory | `static` | `./public`, `/var/www` |
| `DATABASE_URL` | Database connection URL | `sqlite:./data/wikify.db` | `sqlite:memory:` |

#### CORS Configuration

| Variable | Description | Default | Example |
|----------|-------------|---------|---------|
| `WIKIFY_CORS_ORIGINS` | Allowed origins (comma-separated) | `http://localhost:5173,http://127.0.0.1:5173` | `https://app.example.com,https://admin.example.com` |
| `WIKIFY_CORS_CREDENTIALS` | Allow credentials | `true` | `true`, `false` |
| `WIKIFY_CORS_METHODS` | Allowed HTTP methods (comma-separated) | `GET,POST,PUT,DELETE,OPTIONS` | `GET,POST` |
| `WIKIFY_CORS_HEADERS` | Allowed headers (comma-separated) | `Authorization,Accept,Content-Type` | `Content-Type,X-API-Key` |
| `WIKIFY_CORS_DEV_ALLOW_ALL` | Allow all origins in dev mode | `true` | `true`, `false` |

### Configuration Examples

#### Development Environment

```bash
# .env file for development
WIKIFY_HOST=localhost
WIKIFY_PORT=8080
WIKIFY_DEV_MODE=true

# CORS - Development (permissive for local development)
WIKIFY_CORS_DEV_ALLOW_ALL=true
WIKIFY_CORS_ORIGINS=http://localhost:5173,http://127.0.0.1:5173
WIKIFY_CORS_CREDENTIALS=true
WIKIFY_CORS_METHODS=GET,POST,PUT,DELETE,OPTIONS
WIKIFY_CORS_HEADERS=Authorization,Accept,Content-Type,X-Requested-With

# Database
DATABASE_URL=sqlite:./data/wikify.db
```

#### Production Environment

```bash
# .env file for production
WIKIFY_HOST=0.0.0.0
WIKIFY_PORT=8080
WIKIFY_DEV_MODE=false

# CORS - Production (strict security)
WIKIFY_CORS_DEV_ALLOW_ALL=false
WIKIFY_CORS_ORIGINS=https://your-domain.com,https://app.your-domain.com
WIKIFY_CORS_CREDENTIALS=true
WIKIFY_CORS_METHODS=GET,POST,PUT,DELETE
WIKIFY_CORS_HEADERS=Authorization,Accept,Content-Type

# Database
DATABASE_URL=sqlite:/var/lib/wikify/wikify.db
```

#### Testing Environment

```bash
# .env file for testing
WIKIFY_HOST=localhost
WIKIFY_PORT=8080
WIKIFY_DEV_MODE=false

# CORS - Testing (moderate security)
WIKIFY_CORS_DEV_ALLOW_ALL=false
WIKIFY_CORS_ORIGINS=http://test.example.com,http://staging.example.com
WIKIFY_CORS_CREDENTIALS=true
WIKIFY_CORS_METHODS=GET,POST,PUT,DELETE,OPTIONS
WIKIFY_CORS_HEADERS=Authorization,Accept,Content-Type

# Database
DATABASE_URL=sqlite:memory:
```

### Command Line Options

```bash
# Available command line options
cargo run --bin wikify-web -- --help

Options:
  --host <HOST>              Server host [default: 127.0.0.1]
  --port <PORT>              Server port [default: 8080]
  --dev                      Enable development mode
  --static-dir <DIR>         Static files directory
  --database-url <URL>       Database connection URL
  --config <FILE>            Configuration file path
  -h, --help                 Print help
  -V, --version              Print version
```

## 🔌 API Endpoints

### Health Check
- `GET /api/health` - Server health status

### Repositories
- `GET /api/repositories` - List all repositories
- `POST /api/repositories` - Add new repository
- `GET /api/repositories/{id}` - Get repository details
- `DELETE /api/repositories/{id}` - Remove repository

### Sessions
- `GET /api/sessions` - List chat sessions
- `POST /api/sessions` - Create new session
- `GET /api/sessions/{id}` - Get session details
- `DELETE /api/sessions/{id}` - Delete session

### Chat
- `POST /api/chat/query` - Send chat query
- `GET /api/chat/history/{session_id}` - Get chat history

### WebSocket
- `WS /ws/` - Unified real-time communication endpoint for all features

For detailed API documentation, see [API.md](docs/API.md).

## 🔒 Security Features

### CORS Protection
- Configurable allowed origins
- Support for credentials
- Method and header restrictions
- Development vs production modes

### Security Headers
- Content Security Policy (CSP)
- X-Frame-Options
- X-Content-Type-Options
- X-XSS-Protection
- Strict-Transport-Security

### Rate Limiting
- Per-IP request limiting
- Configurable limits and windows
- Automatic cleanup of old entries

## 📊 Monitoring

### Health Checks
```bash
# Check server health
curl http://localhost:8080/api/health

# Response
{
  "status": "healthy",
  "timestamp": "2025-01-28T10:30:00Z",
  "version": "0.2.0"
}
```

### Metrics
- Request count and latency
- Database connection status
- Memory usage
- Active WebSocket connections

## 🐳 Docker Deployment

```dockerfile
FROM rust:1.70 as builder
WORKDIR /app
COPY . .
RUN cargo build --release --bin wikify-web

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/wikify-web /usr/local/bin/
EXPOSE 8080
CMD ["wikify-web", "--host", "0.0.0.0", "--port", "8080"]
```

## 🧪 Testing

```bash
# Run unit tests
cargo test

# Run integration tests
cargo test --test integration_tests

# Run benchmarks
cargo bench

# Test with coverage
cargo tarpaulin --out html
```

## 🔧 Development

### Project Structure

```
wikify-web/
├── src/
│   ├── handlers/          # Request handlers
│   ├── middleware/        # Custom middleware
│   ├── routes/           # Route definitions
│   ├── lib.rs            # Library entry point
│   ├── main.rs           # Binary entry point
│   ├── server.rs         # Server implementation
│   ├── state.rs          # Application state
│   └── websocket.rs      # WebSocket handling
├── templates/            # HTML templates
├── static/              # Static assets
├── migrations/          # Database migrations
├── tests/              # Integration tests
└── benches/            # Benchmarks
```

### Adding New Endpoints

1. Define handler in `src/handlers/`
2. Add route in `src/routes/`
3. Update API documentation
4. Add tests

### Database Migrations

```bash
# Create new migration
sqlx migrate add <migration_name>

# Run migrations
sqlx migrate run
```

## 🤝 Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests
5. Update documentation
6. Submit a pull request

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](../LICENSE) file for details.

## 🆘 Support

- 📖 [API Documentation](docs/API.md)
- 🐛 [Issue Tracker](https://github.com/your-repo/wikify/issues)
- 💬 [Discussions](https://github.com/your-repo/wikify/discussions)

---

**Built with ❤️ in Rust**
