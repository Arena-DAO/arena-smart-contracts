#!/bin/bash

# Change directory to ./contracts
cd ./contracts || { echo "Failed to change directory to ./contracts. Exiting."; exit 1; }

# Set max number of parallel jobs (adjust for your CPU)
MAX_JOBS=4
JOBS=0

# Loop through all contract directories
for dir in */; do
    (
        cd "$dir" || exit
        echo "Executing cargo schema in $dir"
        cargo schema
        echo "Completed $dir"
    ) &

    ((JOBS++))

    # Wait if max parallel jobs are running
    if [ "$JOBS" -ge "$MAX_JOBS" ]; then
        wait -n  # Wait for any job to finish
        ((JOBS--))
    fi
done

# Wait for remaining background jobs
wait

# Continue with JS steps
cd ../scripts || exit
npm i
npm run gen
