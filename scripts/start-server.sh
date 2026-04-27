#!/bin/bash
# Alyesa Background Server Daemon
# Usage: start-server.sh [model_path] [port] [slot_name]

ROOT="${ALYESA_ROOT:-$HOME/nomi/alyesa}"
MODEL_PATH="${1:-$ROOT/models/llama-3.2-3b-instruct-q4_k_m.gguf}"
PORT="${2:-8080}"
SLOT="${3:-ceo}"

if [ ! -f "$MODEL_PATH" ]; then
    echo "Error: Model not found at $MODEL_PATH"
    exit 1
fi

# Ensure port is clear for the new model
pkill -f "llama-server --port $PORT" >/dev/null 2>&1
sleep 1

# Note: Modern llama-server enables prompt caching by default.
# We use --cache-reuse to speed up similar prompts.

nohup /data/data/com.termux/files/home/llama.cpp/build/bin/llama-server \
  -m "$MODEL_PATH" \
  --port "$PORT" \
  -c 4096 \
  -t $(nproc) \
  --embedding \
  --cache-reuse 256 \
  --alias "alyesa-$SLOT" \
  >/dev/null 2>&1 &
