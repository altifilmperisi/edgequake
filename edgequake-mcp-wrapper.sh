#!/usr/bin/env bash
set -e

RUST_BINARY="/home/savasuyar/projects/edgequake/edgequake/target/release/edgequake"
MCP_SERVER="/home/savasuyar/projects/edgequake/mcp/dist/index.js"
API_PORT=8081

export DATABASE_URL="postgresql://edgequake:edgequake_secret@localhost:5432/edgequake?options=-c%20search_path%3Dpublic"
export OPENAI_API_KEY="docker-llm-key"
export OPENAI_BASE_URL="http://localhost:8098/v1"
export EDGEQUAKE_LLM_PROVIDER="openai"
export EDGEQUAKE_LLM_MODEL="gpt-5-mini"
export CHAT_MODEL="gpt-5-mini"
export EDGEQUAKE_DEFAULT_LLM_PROVIDER="openai"
export EDGEQUAKE_DEFAULT_LLM_MODEL="gpt-5-mini"
export EDGEQUAKE_EMBEDDING_PROVIDER="openai"
export EDGEQUAKE_EMBEDDING_MODEL="text-embedding-ada-002"
export EDGEQUAKE_EMBEDDING_DIMENSION="384"
export EDGEQUAKE_DEFAULT_EMBEDDING_PROVIDER="openai"
export EDGEQUAKE_DEFAULT_EMBEDDING_MODEL="text-embedding-ada-002"
export EDGEQUAKE_DEFAULT_EMBEDDING_DIMENSION="384"
export HOST="0.0.0.0"
export PORT=$API_PORT
export RUST_LOG="edgequake=info,edgequake_api=info,sqlx=warn"
export EDGEQUAKE_BASE_URL="http://localhost:$API_PORT"

# Start Rust API server in background
$RUST_BINARY &
RUST_PID=$!

# Wait for API to be ready
for i in $(seq 1 30); do
  if curl -s "http://localhost:$API_PORT/health" >/dev/null 2>&1; then
    break
  fi
  sleep 1
done

# Run MCP server (stdio) - when it exits, kill the Rust server
node "$MCP_SERVER"
MCP_EXIT=$?
kill $RUST_PID 2>/dev/null
exit $MCP_EXIT
