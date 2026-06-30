#!/usr/bin/env bash
set -euo pipefail

export DATABASE_URL="postgresql://edgequake:edgequake_secret@localhost:5432/edgequake?options=-c%20search_path%3Dpublic"
export OPENAI_API_KEY="sk-placeholder"
export OPENAI_BASE_URL="http://localhost:8099/v1"
export EDGEQUAKE_EMBEDDING_BASE_URL="http://localhost:1234/v1"
export EDGEQUAKE_EMBEDDING_API_KEY="sk-placeholder"
export EDGEQUAKE_LLM_PROVIDER="openai"
export EDGEQUAKE_LLM_MODEL="gpt-5-mini"
export EDGEQUAKE_DEFAULT_LLM_PROVIDER="openai"
export EDGEQUAKE_DEFAULT_LLM_MODEL="gpt-5-mini"
export EDGEQUAKE_EMBEDDING_PROVIDER="openai"
export EDGEQUAKE_EMBEDDING_MODEL="all-MiniLM-L6-v2"
export EDGEQUAKE_EMBEDDING_DIMENSION="384"
export EDGEQUAKE_DEFAULT_EMBEDDING_PROVIDER="openai"
export EDGEQUAKE_DEFAULT_EMBEDDING_MODEL="all-MiniLM-L6-v2"
export EDGEQUAKE_DEFAULT_EMBEDDING_DIMENSION="384"
export CHAT_MODEL="gpt-5-mini"
export HOST="0.0.0.0"
export PORT=8081
export RUST_LOG="edgequake=info,edgequake_api=info,sqlx=warn"

BINARY="/home/savasuyar/projects/edgequake/edgequake/target/release/edgequake"
nohup "$BINARY" > /tmp/edgequake.log 2>&1 &
echo "EdgeQuake started (PID $!)"
