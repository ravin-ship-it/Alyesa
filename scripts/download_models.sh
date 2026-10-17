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

# 1. CEO/Orchestrator: Llama-3.2-3B-Instruct
echo "[1/4] Downloading Llama-3.2-3B-Instruct (CEO)..." | tee -a "$LOG_FILE"
curl -L -o llama-3.2-3b-instruct-q4_k_m.gguf -C - "https://huggingface.co/bartowski/Llama-3.2-3B-Instruct-GGUF/resolve/main/Llama-3.2-3B-Instruct-Q4_K_M.gguf?download=true" >> "$LOG_FILE" 2>&1

# 2. Researcher: Phi-3-Mini (3.8B)
echo "[2/4] Downloading Phi-3-Mini-4k-Instruct (Researcher)..." | tee -a "$LOG_FILE"
curl -L -o phi-3-mini-4k-instruct-q4_k_m.gguf -C - "https://huggingface.co/bartowski/Phi-3-mini-4k-instruct-GGUF/resolve/main/Phi-3-mini-4k-instruct-Q4_K_M.gguf?download=true" >> "$LOG_FILE" 2>&1

# 3. Coder 1: Qwen2.5-Coder (3B)
echo "[3/4] Downloading Qwen2.5-Coder-3B-Instruct (Lead Architect)..." | tee -a "$LOG_FILE"
curl -L -o qwen2.5-coder-3b-instruct-q4_k_m.gguf -C - "https://huggingface.co/Qwen/Qwen2.5-Coder-3B-Instruct-GGUF/resolve/main/qwen2.5-coder-3b-instruct-q4_k_m.gguf?download=true" >> "$LOG_FILE" 2>&1

# 4. Coder 2: DeepSeek-Coder (1.3B)
echo "[4/5] Downloading DeepSeek-Coder-1.3B-Instruct (Reviewer)..." | tee -a "$LOG_FILE"
curl -L -o deepseek-coder-1.3b-instruct-q4_k_m.gguf -C - "https://huggingface.co/bartowski/deepseek-coder-1.3b-instruct-GGUF/resolve/main/deepseek-coder-1.3b-instruct-Q4_K_M.gguf?download=true" >> "$LOG_FILE" 2>&1

# 5. Survival Specialist: DeepSeek-R1-Distill-Qwen-1.5B
echo "[5/6] Downloading DeepSeek-R1-Distill-Qwen-1.5B (Survival Logic)..." | tee -a "$LOG_FILE"
curl -L -o deepseek-r1-distill-qwen-1.5b-q4_k_m.gguf -C - "https://huggingface.co/bartowski/DeepSeek-R1-Distill-Qwen-1.5B-GGUF/resolve/main/DeepSeek-R1-Distill-Qwen-1.5B-Q4_K_M.gguf?download=true" >> "$LOG_FILE" 2>&1

# 6. Gemma 4 E2B Instruct
echo "[6/6] Downloading Gemma-4-E2B-Instruct (Stable CEO)..." | tee -a "$LOG_FILE"
curl -L -o gemma-4-e2b-it-q4_k_m.gguf -C - "https://huggingface.co/unsloth/gemma-4-E2B-it-GGUF/resolve/main/gemma-4-E2B-it-Q4_K_M.gguf?download=true" >> "$LOG_FILE" 2>&1

echo "---------------------------------------------------" >> "$LOG_FILE"
echo "✅ All core Boardroom models downloaded successfully!" | tee -a "$LOG_FILE"
