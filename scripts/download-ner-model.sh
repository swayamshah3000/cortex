#!/usr/bin/env bash
# download-ner-model.sh — reproducible download of the bert-base-NER ONNX model
# bundle for Cortex Phase 6. Run from the project root (cortex/).
#
# Source (verified 2026-06-29):
#   https://huggingface.co/Xenova/bert-base-NER
#   - onnx/model_quantized.onnx  (~109 MB INT8, MIT)
#   - tokenizer.json
#   - config.json                (id2label for 9 NER labels)
#   - special_tokens_map.json    (defensive — referenced by some tokenizer.json files)
#
# Output: src-tauri/models/{bert-base-NER.onnx, tokenizer.json, config.json,
# special_tokens_map.json}. Re-running is idempotent — existing files are skipped.

set -euo pipefail

MODEL_DIR="src-tauri/models"
BASE_URL="https://huggingface.co/Xenova/bert-base-NER/resolve/main"

mkdir -p "$MODEL_DIR"

download_if_missing() {
  local url="$1"
  local target="$2"
  local label="$3"
  if [ -f "$target" ] && [ -s "$target" ]; then
    echo "skip: $label already present at $target"
    return 0
  fi
  echo "fetch: $label  ->  $target"
  curl -L --fail --silent --show-error "$url" -o "$target.tmp"
  mv "$target.tmp" "$target"
  local size
  size=$(wc -c < "$target")
  echo "  done ($size bytes)"
}

download_if_missing "$BASE_URL/onnx/model_quantized.onnx"   "$MODEL_DIR/bert-base-NER.onnx"        "bert-base-NER.onnx"
download_if_missing "$BASE_URL/tokenizer.json"              "$MODEL_DIR/tokenizer.json"            "tokenizer.json"
download_if_missing "$BASE_URL/config.json"                 "$MODEL_DIR/config.json"               "config.json"
download_if_missing "$BASE_URL/special_tokens_map.json"     "$MODEL_DIR/special_tokens_map.json"   "special_tokens_map.json"

echo ""
echo "Model bundle contents:"
ls -la "$MODEL_DIR"

echo ""
echo "SHA-256 of bert-base-NER.onnx (compare against HuggingFace LFS pointer):"
shasum -a 256 "$MODEL_DIR/bert-base-NER.onnx"
