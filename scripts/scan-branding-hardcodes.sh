#!/usr/bin/env bash
set -euo pipefail

# Scan for hardcoded branding and domain strings that should be environment-driven.

ROOT_DIR="${1:-.}"
OUTPUT_FILE="${2:-launch-branding-scan.txt}"

PATTERN='mockforge\.dev|app\.mockforge\.dev|support@mockforge\.dev|privacy@mockforge\.dev|legal@mockforge\.dev|sales@mockforge\.dev|dpo@mockforge\.dev'

echo "Scanning for hardcoded branding/domain strings..."
echo "Root: ${ROOT_DIR}"
echo "Output: ${OUTPUT_FILE}"
echo

rg -n --no-heading -S "${PATTERN}" \
  "${ROOT_DIR}" \
  -g '!target/**' \
  -g '!.git/**' \
  -g '!node_modules/**' \
  -g '!book/book/**' \
  > "${OUTPUT_FILE}" || true

MATCH_COUNT="$(wc -l < "${OUTPUT_FILE}" | tr -d ' ')"
echo "Matches found: ${MATCH_COUNT}"

if [[ "${MATCH_COUNT}" -eq 0 ]]; then
  echo "No hardcoded branding/domain strings found."
  exit 0
fi

echo
echo "Top files by match count:"
cut -d: -f1 "${OUTPUT_FILE}" | sort | uniq -c | sort -nr | head -n 20

echo
echo "First 30 matches:"
head -n 30 "${OUTPUT_FILE}"

echo
echo "Full report saved to ${OUTPUT_FILE}"
