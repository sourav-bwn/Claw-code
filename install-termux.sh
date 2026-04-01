#!/bin/bash
# Uclaw Termux Installer
# Run this single command in Termux:

echo "🔧 Installing Uclaw on Termux..."

# Update packages
pkg update && pkg upgrade -y

# Install Python and dependencies
pkg install python git curl wget -y

# Clone Uclaw
cd ~
git clone https://github.com/KING-SS-OFFICIAL/Claw-code.git
cd Claw-code/python

# Install Python packages
pip install httpx

# Download Ollama for ARM64
echo "📥 Downloading Ollama (this may take a few minutes)..."
cd ~
wget -q https://github.com/ollama/ollama/releases/download/v0.5.4/ollama-linux-arm64.tar.gz

echo "📦 Extracting Ollama..."
tar -xzf ollama-linux-arm64.tar.gz
rm ollama-linux-arm64.tar.gz
mv ollama /data/data/com.termux/files/usr/bin/

echo "🤖 Downloading AI model (Llama 3.2)..."
ollama pull llama3.2

echo ""
echo "✅ INSTALLATION COMPLETE!"
echo ""
echo "To use Uclaw:"
echo ""
echo "1. START OLLAMA SERVER (keep running):"
echo "   ollama serve"
echo ""
echo "2. OPEN NEW SESSION (swipe left, tap 'New session')"
echo ""
echo "3. RUN UCLAW:"
echo "   cd ~/Claw-code/python"
echo "   python cli.py"
echo ""
echo "💡 Use 'ollama pull codellama' for code-specific AI"
echo "📱 For file access: run 'termux-setup-storage'"
