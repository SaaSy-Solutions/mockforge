#!/bin/bash -eu

# Navigate to project directory
cd $SRC/mockforge

# Build fuzz targets with cargo-fuzz
cd crates/mockforge-core

# Install cargo-fuzz if not present
cargo install cargo-fuzz --force

# Build each fuzz target
FUZZ_TARGETS=(
    "fuzz_json_validator"
    "fuzz_openapi_parser"
    "fuzz_template_engine"
)

for target in "${FUZZ_TARGETS[@]}"; do
    echo "Building fuzz target: $target"

    # Build the fuzz target
    cargo fuzz build $target --release

    # Copy the fuzz target to the output directory
    cp fuzz/target/x86_64-unknown-linux-gnu/release/$target $OUT/

    # Create seed corpus directory if it exists
    if [ -d "fuzz/corpus/$target" ]; then
        zip -j $OUT/${target}_seed_corpus.zip fuzz/corpus/$target/*
    fi

    # Copy dictionary if it exists
    if [ -f "fuzz/fuzz_targets/${target}.dict" ]; then
        cp fuzz/fuzz_targets/${target}.dict $OUT/${target}.dict
    fi
done

# Build coverage instrumented version
echo "Building coverage instrumented version"
export RUSTFLAGS="$RUSTFLAGS -Cinstrument-coverage"
cargo build --release

# Add any additional configuration
echo "Fuzz targets built successfully"
