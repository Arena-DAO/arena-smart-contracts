#!/bin/bash

# Change directory to ./contracts
cd ./contracts

# Check if the directory change was successful
if [ $? -ne 0 ]; then
    echo "Failed to change directory to ./contracts. Exiting."
    exit 1
fi

# Iterate over each subdirectory in the current directory
for dir in */; do
    # Change directory to subdirectory
    cd "$dir"

    # Execute cargo schema
    echo "Executing cargo schema in $dir"
    cargo schema

    # Return to the parent directory
    cd ..

    echo "Completed $dir"
done

cd ../scripts
npm i
npm run gen