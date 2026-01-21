#!/bin/bash

echo "🚀 Building Linux Studio Effects..."

# 1. Build Rust project
echo "📦 Building Rust binary..."
cargo build --release

if [ $? -eq 0 ]; then
    echo "✅ Rust build successful."
else
    echo "❌ Rust build failed."
    exit 1
fi

# 2. Package Gnome Extension
echo "🧩 Packaging Gnome Extension..."
cd gnome-extension
zip -r ../linux-studio-effects-extension.zip ./*
cd ..

echo "🎉 Build complete!"
echo "Binary: ./target/release/linux-studio-effects"
echo "Extension: ./linux-studio-effects-extension.zip"
