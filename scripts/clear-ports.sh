#!/bin/bash
# Clear MockForge ports script

echo "🔍 Checking for processes using MockForge ports..."

# Check which ports are in use
PORTS=(3000 3001 50051 9080)
PIDS=()

for port in "${PORTS[@]}"; do
    pid=$(lsof -ti:$port 2>/dev/null)
    if [ ! -z "$pid" ]; then
        echo "📌 Port $port is in use by PID: $pid"
        PIDS+=($pid)
    else
        echo "✅ Port $port is free"
    fi
done

if [ ${#PIDS[@]} -eq 0 ]; then
    echo "🎉 All MockForge ports are already free!"
    exit 0
fi

echo ""
echo "🔪 Killing processes using MockForge ports..."
for pid in "${PIDS[@]}"; do
    echo "   Killing PID: $pid"
    kill -9 $pid 2>/dev/null
done

echo ""
echo "🔍 Verifying ports are now free..."
for port in "${PORTS[@]}"; do
    pid=$(lsof -ti:$port 2>/dev/null)
    if [ -z "$pid" ]; then
        echo "✅ Port $port is now free"
    else
        echo "❌ Port $port is still in use by PID: $pid"
    fi
done

echo ""
echo "🚀 Ready to start MockForge!"
