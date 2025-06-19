#!/bin/bash
set -euo pipefail

# Auto-detect CPU cores for optimal parallelism
MAX_JOBS=${MAX_JOBS:-$(nproc 2>/dev/null || sysctl -n hw.ncpu 2>/dev/null || echo 4)}

echo "Running schema generation with $MAX_JOBS parallel jobs..."

# Generate schemas in parallel using xargs
find ./contracts -maxdepth 1 -type d -name "*" ! -name "contracts" | \
    xargs -I {} -P "$MAX_JOBS" bash -c '
        dir=$(basename {})
        echo "Executing cargo schema in $dir"
        cd {} && cargo schema --quiet
        echo "✓ Completed $dir"
    '

echo "All schemas generated. Running codegen..."

# Continue with JS steps
cd scripts
npm ci --silent
npm run gen
