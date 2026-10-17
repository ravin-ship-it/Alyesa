#!/bin/bash
# Alyesa Survival Knowledge Base Downloader
ROOT="${ALYESA_ROOT:-$HOME/nomi/alyesa}"
KB_DIR="$ROOT/data/survival_kb"
mkdir -p "$KB_DIR"

echo "🌍 Fetching Army Survival Manual (Markdown)..."
git clone https://github.com/ligi/SurvivalManual.wiki.git "$KB_DIR/manual" --depth 1

echo "📦 Fetching Technical Survival Data..."
git clone https://github.com/PR0M3TH3AN/Survival-Data.git "$KB_DIR/tech" --depth 1

echo "✅ Survival Knowledge Base downloaded to $KB_DIR"
echo "Run 'alyesa --survival-ingest' to index this data into the brain."
