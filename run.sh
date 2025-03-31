#!/bin/bash

# Check if Rust is installed
if ! command -v cargo &> /dev/null; then
    echo "Rust/Cargo not found. Please install from https://rustup.rs/"
    exit 1
fi

# Check for data files
if [ ! -d "data" ] || [ ! -f "data/RoadVerticesAMS.txt" ] || [ ! -f "data/RoadEdgesAMS.txt" ] || [ ! -f "data/RestaurantsAMS.txt" ]; then
    echo "Warning: Required data files may be missing."
    echo "Please ensure the following files exist in the data/ directory:"
    echo "  - RoadVerticesAMS.txt"
    echo "  - RoadEdgesAMS.txt"
    echo "  - RestaurantsAMS.txt"
    
    # Prompt to continue
    read -p "Continue anyway? (y/n): " answer
    if [[ "$answer" != "y" && "$answer" != "Y" ]]; then
        exit 1
    fi
fi

echo "Building and running Personal Shopper in release mode..."
cargo run --release

# Check exit status
if [ $? -eq 0 ]; then
    echo "Program executed successfully!"
else
    echo "Program execution failed."
fi 