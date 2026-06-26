#!/usr/bin/env bash
set -e

echo "========================================="
echo " Starting Debian 12 Benchmarking Script"
echo "========================================="

# 1. Update packages and install build tools / gnuplot
echo "[*] Installing dependencies..."
sudo apt update
sudo apt install -y build-essential gnuplot curl

# 2. Install Rust if not present
if ! command -v cargo &> /dev/null; then
    echo "[*] Rust not found. Installing rustup..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
else
    echo "[*] Rust is already installed."
fi

# 3. Build and run benchmarks
echo "[*] Running benchmarks..."
cargo bench

echo "========================================="
echo " Benchmarking finished! HTML reports are at:"
echo " target/criterion/report/index.html"
echo "========================================="
