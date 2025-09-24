#!/bin/bash

# Quick start script for Binance Testnet Trading
# Uses the provided testnet API keys

set -e

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

clear

echo -e "${BLUE}=========================================${NC}"
echo -e "${BLUE}   Binance Testnet Trading System${NC}"
echo -e "${BLUE}=========================================${NC}"
echo ""
echo -e "${GREEN}✅ Using Binance TESTNET API Keys${NC}"
echo -e "${YELLOW}⚠️  This is TESTNET - No real funds involved${NC}"
echo ""

# Step 1: Check prerequisites
echo -e "${GREEN}Step 1: Checking prerequisites...${NC}"
if ! command -v docker &> /dev/null; then
    echo "❌ Docker is not installed. Please install Docker first."
    exit 1
fi

if ! command -v docker-compose &> /dev/null; then
    echo "❌ Docker Compose is not installed. Please install Docker Compose first."
    exit 1
fi

echo "✅ Docker and Docker Compose are installed"

# Step 2: Test API connection
echo ""
echo -e "${GREEN}Step 2: Testing Binance Testnet connection...${NC}"
chmod +x scripts/test_binance_testnet.sh
./scripts/test_binance_testnet.sh

if [ $? -ne 0 ]; then
    echo "❌ Connection test failed. Please check your network."
    exit 1
fi

# Step 3: Setup environment
echo ""
echo -e "${GREEN}Step 3: Setting up environment...${NC}"

# Use testnet configuration
cp .env.testnet .env
echo "✅ Environment configured for testnet"

# Step 4: Start services
echo ""
echo -e "${GREEN}Step 4: Starting services...${NC}"

# Stop any existing services
docker-compose down 2>/dev/null || true

# Start testnet services
docker-compose -f docker-compose.testnet.yml up -d

echo "⏳ Waiting for services to start..."
sleep 10

# Check service status
if docker-compose -f docker-compose.testnet.yml ps | grep -q "Up"; then
    echo "✅ All services are running"
else
    echo "❌ Some services failed to start"
    docker-compose -f docker-compose.testnet.yml logs --tail=50
    exit 1
fi

# Step 5: Run initial test
echo ""
echo -e "${GREEN}Step 5: Running initial trading test...${NC}"

# Run the testnet example
docker-compose -f docker-compose.testnet.yml run --rm barter-strategy \
    cargo run --example binance_testnet_trading 2>&1 | tee testnet_trading.log

# Step 6: Display information
echo ""
echo -e "${BLUE}=========================================${NC}"
echo -e "${BLUE}       System Started Successfully!${NC}"
echo -e "${BLUE}=========================================${NC}"
echo ""
echo "📊 Access Points:"
echo "   Grafana Dashboard: http://localhost:3001 (admin/admin)"
echo "   Logs: docker-compose -f docker-compose.testnet.yml logs -f"
echo ""
echo "🔍 Useful Commands:"
echo "   View logs:        make logs"
echo "   Stop system:      docker-compose -f docker-compose.testnet.yml down"
echo "   Check trades:     docker-compose exec postgres psql -U barter -d barter_testnet"
echo ""
echo "📈 Monitor Trading:"
echo "   docker-compose -f docker-compose.testnet.yml logs -f barter-strategy | grep -E 'SIGNAL|DECISION|ORDER'"
echo ""
echo -e "${YELLOW}Remember: This is TESTNET mode. Perfect for learning and testing!${NC}"
echo ""

# Keep monitoring option
read -p "Do you want to monitor the logs now? (y/n): " monitor
if [ "$monitor" = "y" ]; then
    echo ""
    echo "Monitoring logs (Press Ctrl+C to stop)..."
    docker-compose -f docker-compose.testnet.yml logs -f barter-strategy
fi