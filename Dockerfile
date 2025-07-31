# ---- Stage 1: Build Frontend ----
FROM node:22-alpine AS frontend-builder

# Set working directory for frontend
WORKDIR /app/frontend

# Copy package files and install dependencies
COPY frontend/package*.json ./
RUN npm install

# Copy the rest of the frontend code
COPY frontend/ ./
COPY locales ./public/locales

# Build the frontend application
RUN npm run build

# ---- Stage 2: Build Backend ----
FROM rust:1.87-slim-bookworm AS backend-builder

# Install build dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    protobuf-compiler \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Install cargo-chef for cached builds
RUN cargo install cargo-chef

WORKDIR /app

# Copy backend workspace files and prepare recipe
COPY backend/Cargo.toml backend/Cargo.lock ./backend/
COPY backend/crates ./backend/crates
COPY backend/migrations ./backend/migrations
COPY locales ./locales

# Generate the chef recipe
RUN cd backend && cargo chef prepare --recipe-path recipe.json

# Cook the dependencies
RUN cd backend && cargo chef cook --release --recipe-path recipe.json

# Copy frontend build artifacts from the first stage
COPY --from=frontend-builder /app/frontend/dist ./frontend/dist

# Copy the rest of the backend source code
# This is intentionally redundant to ensure any changes are picked up
# after the dependency cooking step.
COPY backend/crates ./backend/crates

# Build the backend application, embedding the frontend assets
RUN cd backend && cargo build --release -p nodenexus-server

# ---- Stage 3: Final Image ----
FROM debian:bookworm-slim AS runner

RUN apt-get update && apt-get install -y libssl3 && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Create a non-root user
RUN useradd --system --create-home appuser

# Copy the compiled binary from the backend-builder stage
COPY --from=backend-builder /app/backend/target/release/nodenexus-server .

# Set ownership to the new user
RUN chown appuser:appuser nodenexus-server

# Create and set ownership for data and log directories
RUN mkdir -p /app/data /app/logs && chown -R appuser:appuser /app/data /app/logs

# Switch to the non-root user
USER appuser

# Expose volumes for data and logs
VOLUME ["/app/data", "/app/logs"]

# Set environment variables
ENV RUNNING_IN_CONTAINER=true
ENV DATA_DIR=/app/data
ENV LOG_DIR=/app/logs

# Expose the port the server will run on
EXPOSE 8080

# Command to run the application
CMD ["./nodenexus-server"]