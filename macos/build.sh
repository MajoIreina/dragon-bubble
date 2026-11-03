#!/bin/bash
# Run with bash build.sh; supports macOS's built-in Bash 3.2.
set -euo pipefail
cd "$(dirname "$0")"
root="$(pwd)"
if [[ "$(uname -s)" != Darwin ]]; then
    echo "App bundles must be built on macOS with Xcode Command Line Tools." >&2
    exit 1
fi
export MACOSX_DEPLOYMENT_TARGET="${MACOSX_DEPLOYMENT_TARGET:-11.0}"
skip_tests=false
universal=false
target="$(rustc -vV | sed -n 's/^host: //p')"
while [[ $# -gt 0 ]]; do
    case "$1" in
        --skip-tests) skip_tests=true; shift ;;
        --universal) universal=true; shift ;;
        --target)
            [[ $# -gt 1 ]] || { echo "--target requires a Rust target triple" >&2; exit 2; }
            target="$2"; shift 2 ;;
        *) echo "Unknown argument: $1" >&2; exit 2 ;;
    esac
done
case "$target" in
    aarch64-apple-darwin|x86_64-apple-darwin) ;;
    *) echo "Unsupported target: $target" >&2; exit 2 ;;
esac
if [[ "$universal" == true ]]; then
    targets=(aarch64-apple-darwin x86_64-apple-darwin)
    label=universal
else
    targets=("$target")
    label="$target"
fi
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets -- -D warnings
if [[ "$skip_tests" == false ]]; then
    cargo test --locked --workspace
fi
# Explicit target-dir makes the package location independent of user Cargo configuration.
for target in "${targets[@]}"; do
    cargo build --locked --release --target "$target" --target-dir "$root/target" -p pet-desktop
done
mkdir -p "$root/outputs"
stage="$(mktemp -d "$root/outputs/.stage.XXXXXX")"
app="$stage/My Dragon Bubble.app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
if [[ "$universal" == true ]]; then
    lipo -create "$root/target/aarch64-apple-darwin/release/dragon-bubble" \
        "$root/target/x86_64-apple-darwin/release/dragon-bubble" \
        -output "$app/Contents/MacOS/dragon-bubble"
else
    cp "$root/target/$target/release/dragon-bubble" "$app/Contents/MacOS/dragon-bubble"
fi
chmod +x "$app/Contents/MacOS/dragon-bubble"
cp Info.plist "$app/Contents/Info.plist"
minimum_os="/usr/libexec/PlistBuddy"
"$minimum_os" -c "Set :LSMinimumSystemVersion $MACOSX_DEPLOYMENT_TARGET" "$app/Contents/Info.plist"
cp assets/AppIcon.icns "$app/Contents/Resources/AppIcon.icns"
cp LICENSE-RENDERING.txt "$app/Contents/Resources/"
for resource in assets skins i18n converters; do
    cp -R "$resource" "$app/Contents/Resources/$resource"
done
plutil -lint "$app/Contents/Info.plist"
codesign --force --deep --sign - "$app"
codesign --verify --deep --strict "$app"
destination="$root/outputs/$label"
if [[ -e "$destination" ]]; then
    # Preserve the previous package; never recursively delete a user's output.
    mv "$destination" "$root/outputs/previous-$label-$(date +%Y%m%d-%H%M%S)-$$"
fi
mv "$stage" "$destination"
ditto -c -k --sequesterRsrc --keepParent \
    "$destination/My Dragon Bubble.app" "$destination/My Dragon Bubble.zip"
echo "Built: $destination/My Dragon Bubble.app"
