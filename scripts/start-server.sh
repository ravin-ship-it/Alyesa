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
if command -v fuser >/dev/null 2>&1; then
    fuser -k "$PORT/tcp" >/dev/null 2>&1
else
    pkill -f "llama-server --port $PORT" >/dev/null 2>&1
fi
sleep 1

# GPU Offloading (Default to 99 if supported, or 0 for CPU only)
NGL="${ALYESA_NGL:-99}"

nohup /data/data/com.termux/files/home/llama.cpp/build/bin/llama-server \
  -m "$MODEL_PATH" \
  -ngl "$NGL" \
  --port "$PORT" \
  -c 4096 \
  -t 4 \
  --embedding \
  --cache-reuse 256 \
  --alias "alyesa-$SLOT" \
  --no-mmap \
  >> "$ROOT/server.log" 2>&1 &
