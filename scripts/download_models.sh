#!/bin/bash
# Alyesa Boardroom Model Downloader
# This script downloads the required GGUF models for the Sequential Multi-Agent Framework.

ROOT="${ALYESA_ROOT:-$HOME/nomi/alyesa}"
MODELS_DIR="$ROOT/models"
LOG_FILE="$ROOT/data/download_progress.log"

mkdir -p "$MODELS_DIR"
mkdir -p "$ROOT/data"

echo "Starting Boardroom Model Downloads..." > "$LOG_FILE"
echo "Models will be saved to: $MODELS_DIR" >> "$LOG_FILE"
echo "---------------------------------------------------" >> "$LOG_FILE"

cd "$MODELS_DIR"

# 1. Researcher: Phi-3-Mini (3.8B)
echo "[1/3] Downloading Phi-3-Mini-4k-Instruct (Researcher)..." | tee -a "$LOG_FILE"
curl -L -o phi-3-mini-4k-instruct-q4_k_m.gguf -C - "https://huggingface.co/bartowski/Phi-3-mini-4k-instruct-GGUF/resolve/main/Phi-3-mini-4k-instruct-Q4_K_M.gguf?download=true" >> "$LOG_FILE" 2>&1

# 2. Coder 1: Qwen2.5-Coder (3B)
echo "[2/3] Downloading Qwen2.5-Coder-3B-Instruct (Lead Architect)..." | tee -a "$LOG_FILE"
curl -L -o qwen2.5-coder-3b-instruct-q4_k_m.gguf -C - "https://huggingface.co/Qwen/Qwen2.5-Coder-3B-Instruct-GGUF/resolve/main/qwen2.5-coder-3b-instruct-q4_k_m.gguf?download=true" >> "$LOG_FILE" 2>&1

# 3. Coder 2: DeepSeek-Coder (1.3B)
echo "[3/3] Downloading DeepSeek-Coder-1.3B-Instruct (Reviewer)..." | tee -a "$LOG_FILE"
curl -L -o deepseek-coder-1.3b-instruct-q4_k_m.gguf -C - "https://huggingface.co/TheBloke/deepseek-coder-1.3b-instruct-GGUF/resolve/main/deepseek-coder-1.3b-instruct.Q4_K_M.gguf?download=true" >> "$LOG_FILE" 2>&1

echo "---------------------------------------------------" >> "$LOG_FILE"
echo "✅ All core Boardroom models downloaded successfully!" | tee -a "$LOG_FILE"
