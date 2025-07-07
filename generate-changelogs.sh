#!/bin/bash
set -e

# Cutoff commit - start changelog from this commit forward
CUTOFF_COMMIT="4956def"

# Array of crate names and their directories
declare -A crates=(
    ["barter"]="barter"
    ["barter-data"]="barter-data"
    ["barter-execution"]="barter-execution"
    ["barter-instrument"]="barter-instrument"
    ["barter-integration"]="barter-integration"
    ["barter-macro"]="barter-macro"
)

# Generate changelog for each crate
for crate_name in "${!crates[@]}"; do
    crate_dir="${crates[$crate_name]}"
    echo "Generating changelog for $crate_name since $CUTOFF_COMMIT..."
    
    # Generate changelog scoped to this crate's directory since cutoff commit
    ~/.cargo/bin/git-cliff \
        --include-path "$crate_dir/**/*" \
        --tag-pattern "${crate_name}-v*" \
        --output "$crate_dir/CHANGELOG.md" \
        "${CUTOFF_COMMIT}.."
    
    echo "✅ Generated $crate_dir/CHANGELOG.md"
done

echo "🎉 All changelogs generated!"