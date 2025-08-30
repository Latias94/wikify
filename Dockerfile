# Multi-stage build for Wikify
FROM node:18-alpine AS frontend-builder

# Build frontend
WORKDIR /app/web
COPY web/package.json web/pnpm-lock.yaml ./
RUN npm install -g pnpm && pnpm install --frozen-lockfile

COPY web/ ./
RUN pnpm build

# Rust builder stage
FROM rust:1.75-slim AS backend-builder

# Install system dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libsqlite3-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy Cargo files
COPY Cargo.toml Cargo.lock ./
COPY wikify-*/Cargo.toml ./

# Create dummy source files to cache dependencies
RUN mkdir -p wikify-core/src wikify-indexing/src wikify-rag/src \
    wikify-applications/src wikify-web/src wikify-cli/src \
    wikify-repo/src wikify-wiki/src && \
    echo "fn main() {}" > wikify-web/src/main.rs && \
    echo "fn main() {}" > wikify-cli/src/main.rs && \
    find . -name "*.toml" -path "*/wikify-*" -exec dirname {} \; | \
    xargs -I {} touch {}/src/lib.rs

# Build dependencies
RUN cargo build --release --bin wikify-web
RUN rm -rf wikify-*/src

# Copy actual source code
COPY wikify-core/ ./wikify-core/
COPY wikify-indexing/ ./wikify-indexing/
COPY wikify-rag/ ./wikify-rag/
COPY wikify-applications/ ./wikify-applications/
COPY wikify-web/ ./wikify-web/
COPY wikify-cli/ ./wikify-cli/
COPY wikify-repo/ ./wikify-repo/
COPY wikify-wiki/ ./wikify-wiki/
COPY repo-ref/ ./repo-ref/

# Build the application
RUN cargo build --release --bin wikify-web

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    libsqlite3-0 \
    git \
    && rm -rf /var/lib/apt/lists/*

# Create app user
RUN useradd -r -s /bin/false -m -d /app wikify

WORKDIR /app

# Copy binary
COPY --from=backend-builder /app/target/release/wikify-web ./wikify-web

# Copy frontend assets
COPY --from=frontend-builder /app/web/dist ./static/

# Copy configuration templates
COPY wikify-web/templates ./templates/

# Create data directory
RUN mkdir -p data && chown -R wikify:wikify /app

USER wikify

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/api/health || exit 1

EXPOSE 8080

CMD ["./wikify-web"]
