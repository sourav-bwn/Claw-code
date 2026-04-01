# Uclaw on Termux (Android)

Use Uclaw AI assistant directly on your Android phone!

## Quick Setup (5 minutes)

### Step 1: Install Termux
Get Termux from F-Droid (not Google Play - it's outdated):
https://f-droid.org/en/packages/com.termux/

### Step 2: Update Termux
```bash
pkg update && pkg upgrade -y
```

### Step 3: Install Python
```bash
pkg install python -y
```

### Step 4: Install Ollama (Local AI)
```bash
# Install curl and wget
pkg install curl wget -y

# Download Ollama for ARM64 (most Android devices)
wget https://github.com/ollama/ollama/releases/download/v0.5.4/ollama-linux-arm64.tar.gz

# Extract
tar -xzf ollama-linux-arm64.tar.gz
rm ollama-linux-arm64.tar.gz

# Move to PATH
mv ollama /data/data/com.termux/files/usr/bin/
```

### Step 5: Download a Free Model
```bash
# Pull Llama 3.2 (about 2GB, takes a few minutes)
./ollama pull llama3.2
```

### Step 6: Start Ollama Server
```bash
# In a new Termux session (swipe left, tap "New session")
./ollama serve
```

### Step 7: Run Uclaw
```bash
# Go back to first session
cd /sdcard/uc law/python  # Or wherever you cloned it

# Or clone fresh
git clone https://github.com/KING-SS-OFFICIAL/Claw-code.git

pip install httpx

python cli.py
```

## Alternative: Use Cloud AI (No Ollama)

If Ollama doesn't work on your device:

```bash
# Install dependencies
pip install httpx anthropic openai

# Use with API keys
export ANTHROPIC_API_KEY="your-key"
python cli.py --provider anthropic
```

## Termux File Access

Access your storage:
```bash
termux-setup-storage
```

This allows Uclaw to read/write files in your phone's storage.

## Troubleshooting

### Ollama shows "Illegal instruction"
Your device might need a different Ollama version. Try:
```bash
# Check your architecture
uname -m
# Then download appropriate version from:
# https://github.com/ollama/ollama/releases
```

### Out of memory
Use a smaller model:
```bash
./ollama pull phi
python cli.py --model phi
```

### Ollama won't start
Try running in background:
```bash
nohup ./ollama serve > ollama.log 2>&1 &
```

## Quick Reference

| Command | Purpose |
|---------|---------|
| `python cli.py` | Start interactive chat |
| `python cli.py "hello"` | Single message |
| `python cli.py --model codellama` | Use Code Llama |
| `exit` | Quit |

## Auto-Start Script

Create a shortcut:
```bash
cat > ~/uc law.sh << 'EOF'
#!/bin/bash
cd ~/uc law/python
python cli.py
EOF
chmod +x ~/uc law.sh
```

Now just run `bash ~/uc law.sh` anytime!
