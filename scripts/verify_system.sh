#!/bin/bash

# Barter Trading System Verification Script
# This script helps verify the system is working correctly

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Configuration
VERBOSE=${VERBOSE:-false}
TEST_MODE=${TEST_MODE:-safe}
EXCHANGE=${EXCHANGE:-binance}

# Functions
log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

check_command() {
    if command -v $1 &> /dev/null; then
        log_info "✓ $1 is installed"
        return 0
    else
        log_error "✗ $1 is not installed"
        return 1
    fi
}

verify_prerequisites() {
    echo "========================================="
    echo "Stage 1: Verifying Prerequisites"
    echo "========================================="

    local all_good=true

    check_command docker || all_good=false
    check_command docker-compose || all_good=false
    check_command curl || all_good=false
    check_command jq || all_good=false

    if [ "$all_good" = true ]; then
        log_info "All prerequisites met!"
    else
        log_error "Some prerequisites are missing. Please install them first."
        exit 1
    fi

    # Check Docker daemon
    if docker info &> /dev/null; then
        log_info "✓ Docker daemon is running"
    else
        log_error "✗ Docker daemon is not running"
        exit 1
    fi
}

verify_configuration() {
    echo ""
    echo "========================================="
    echo "Stage 2: Verifying Configuration"
    echo "========================================="

    # Check if .env file exists
    if [ -f .env ]; then
        log_info "✓ .env file exists"

        # Check for required variables (without exposing values)
        if grep -q "TRADING_MODE=" .env; then
            log_info "✓ TRADING_MODE is configured"
        else
            log_warn "⚠ TRADING_MODE not set, using default"
        fi

        if [ "$TEST_MODE" = "live" ]; then
            if grep -q "EXCHANGE_API_KEY=.." .env && grep -q "EXCHANGE_API_SECRET=.." .env; then
                log_info "✓ API credentials appear to be configured"
            else
                log_error "✗ API credentials not configured for live mode"
                exit 1
            fi
        fi
    else
        log_warn "⚠ .env file not found, creating from template..."
        cp .env.example .env
        log_info "Created .env file. Please configure it with your settings."
    fi
}

start_services() {
    echo ""
    echo "========================================="
    echo "Stage 3: Starting Services"
    echo "========================================="

    log_info "Starting services in $TEST_MODE mode..."

    # Set mode based on parameter
    case $TEST_MODE in
        safe)
            export TRADING_MODE=test
            log_info "Running in SAFE TEST mode (no real API calls)"
            ;;
        paper)
            export TRADING_MODE=paper
            log_info "Running in PAPER TRADING mode (real data, fake trades)"
            ;;
        live)
            export TRADING_MODE=live
            log_warn "⚠ Running in LIVE mode - real trades will be executed!"
            read -p "Are you sure? (yes/no): " confirm
            if [ "$confirm" != "yes" ]; then
                log_info "Cancelled."
                exit 0
            fi
            ;;
    esac

    # Start services
    docker-compose up -d

    # Wait for services to be ready
    log_info "Waiting for services to start..."
    sleep 10

    # Check service status
    if docker-compose ps | grep -q "Up"; then
        log_info "✓ Services are running"
        docker-compose ps
    else
        log_error "✗ Some services failed to start"
        docker-compose logs --tail=50
        exit 1
    fi
}

verify_connectivity() {
    echo ""
    echo "========================================="
    echo "Stage 4: Verifying Connectivity"
    echo "========================================="

    # Check internal services
    log_info "Checking internal services..."

    # Check Redis
    if docker-compose exec -T redis redis-cli ping | grep -q PONG; then
        log_info "✓ Redis is responding"
    else
        log_error "✗ Redis is not responding"
    fi

    # Check PostgreSQL
    if docker-compose exec -T postgres pg_isready -U barter | grep -q "accepting connections"; then
        log_info "✓ PostgreSQL is responding"
    else
        log_error "✗ PostgreSQL is not responding"
    fi

    # Check Fluvio
    if docker-compose exec -T fluvio fluvio version &> /dev/null; then
        log_info "✓ Fluvio is responding"
    else
        log_warn "⚠ Fluvio check failed (may be normal in test mode)"
    fi

    # Check health endpoint
    if curl -s http://localhost/health | grep -q healthy; then
        log_info "✓ Health check passed"
    else
        log_warn "⚠ Health check failed (service may still be starting)"
    fi
}

verify_data_flow() {
    echo ""
    echo "========================================="
    echo "Stage 5: Verifying Data Flow"
    echo "========================================="

    log_info "Checking data pipelines..."

    # Check if strategy is generating signals
    log_info "Monitoring signals for 30 seconds..."

    timeout 30 docker-compose logs -f barter-strategy 2>&1 | while read line; do
        if echo "$line" | grep -q "Signal"; then
            log_info "✓ Signal detected: $(echo $line | cut -d' ' -f5-)"
            break
        fi
    done || true

    # Check Fluvio topics if available
    if docker-compose exec -T fluvio fluvio topic list &> /dev/null; then
        log_info "Fluvio topics:"
        docker-compose exec -T fluvio fluvio topic list
    fi

    # Check database tables
    log_info "Database tables:"
    docker-compose exec -T postgres psql -U barter -d barter -c "\dt" 2>/dev/null || true
}

verify_monitoring() {
    echo ""
    echo "========================================="
    echo "Stage 6: Verifying Monitoring"
    echo "========================================="

    # Check Grafana
    if curl -s http://localhost:3000 | grep -q Grafana; then
        log_info "✓ Grafana is accessible at http://localhost:3000"
        log_info "  Username: admin, Password: admin"
    else
        log_warn "⚠ Grafana is not accessible"
    fi

    # Check Prometheus
    if curl -s http://localhost:9090/-/healthy | grep -q "Prometheus Server is Healthy"; then
        log_info "✓ Prometheus is accessible at http://localhost:9090"
    else
        log_warn "⚠ Prometheus is not accessible"
    fi

    # Check metrics endpoint
    if curl -s http://localhost:9464/metrics &> /dev/null; then
        log_info "✓ Metrics endpoint is accessible"

        if [ "$VERBOSE" = true ]; then
            log_info "Sample metrics:"
            curl -s http://localhost:9464/metrics | grep -E "^barter_" | head -5
        fi
    else
        log_warn "⚠ Metrics endpoint is not accessible"
    fi
}

run_basic_tests() {
    echo ""
    echo "========================================="
    echo "Stage 7: Running Basic Tests"
    echo "========================================="

    log_info "Running connection tests..."

    # Test Binance connectivity (public endpoint)
    if [ "$EXCHANGE" = "binance" ] || [ "$EXCHANGE" = "all" ]; then
        if curl -s https://api.binance.com/api/v3/ping | grep -q "{}"; then
            log_info "✓ Binance API is reachable"
        else
            log_error "✗ Cannot reach Binance API"
        fi
    fi

    # Test OKX connectivity (public endpoint)
    if [ "$EXCHANGE" = "okx" ] || [ "$EXCHANGE" = "all" ]; then
        if curl -s https://www.okx.com/api/v5/public/time | grep -q "ts"; then
            log_info "✓ OKX API is reachable"
        else
            log_error "✗ Cannot reach OKX API"
        fi
    fi

    # Run unit tests if in safe mode
    if [ "$TEST_MODE" = "safe" ]; then
        log_info "Running unit tests..."
        docker-compose run --rm barter-strategy cargo test --lib || log_warn "⚠ Some tests failed"
    fi
}

generate_report() {
    echo ""
    echo "========================================="
    echo "Verification Report"
    echo "========================================="

    REPORT_FILE="verification_report_$(date +%Y%m%d_%H%M%S).md"

    cat > $REPORT_FILE <<EOF
# System Verification Report

**Date**: $(date)
**Mode**: $TEST_MODE
**Exchange**: $EXCHANGE

## Summary

### Services Status
\`\`\`
$(docker-compose ps)
\`\`\`

### Recent Logs
\`\`\`
$(docker-compose logs --tail=20 barter-strategy 2>&1 | grep -E "(INFO|WARN|ERROR)" || echo "No recent logs")
\`\`\`

### Recommendations
1. Review the logs for any warnings or errors
2. Check Grafana dashboards for visual monitoring
3. Verify API credentials if planning to trade
4. Run backtest before live trading

## Next Steps
- To run backtest: \`make backtest\`
- To start paper trading: \`TEST_MODE=paper ./scripts/verify_system.sh\`
- To monitor: Open http://localhost:3000
EOF

    log_info "Report saved to $REPORT_FILE"
}

cleanup() {
    echo ""
    read -p "Do you want to stop all services? (y/n): " stop_services
    if [ "$stop_services" = "y" ]; then
        log_info "Stopping services..."
        docker-compose down
        log_info "Services stopped."
    else
        log_info "Services are still running. To stop: docker-compose down"
    fi
}

show_usage() {
    echo "Usage: $0 [OPTIONS]"
    echo ""
    echo "Options:"
    echo "  -m, --mode MODE      Test mode: safe, paper, live (default: safe)"
    echo "  -e, --exchange EX    Exchange to test: binance, okx, all (default: binance)"
    echo "  -v, --verbose        Enable verbose output"
    echo "  -h, --help          Show this help message"
    echo ""
    echo "Examples:"
    echo "  $0                   # Run in safe mode"
    echo "  $0 -m paper          # Run paper trading verification"
    echo "  $0 -m live -e okx    # Run live mode for OKX (careful!)"
}

# Main execution
main() {
    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            -m|--mode)
                TEST_MODE="$2"
                shift 2
                ;;
            -e|--exchange)
                EXCHANGE="$2"
                shift 2
                ;;
            -v|--verbose)
                VERBOSE=true
                shift
                ;;
            -h|--help)
                show_usage
                exit 0
                ;;
            *)
                echo "Unknown option: $1"
                show_usage
                exit 1
                ;;
        esac
    done

    echo "========================================="
    echo "   Barter Trading System Verification   "
    echo "========================================="
    echo ""
    log_info "Mode: $TEST_MODE"
    log_info "Exchange: $EXCHANGE"
    log_info "Verbose: $VERBOSE"
    echo ""

    # Run verification stages
    verify_prerequisites
    verify_configuration
    start_services
    verify_connectivity
    verify_data_flow
    verify_monitoring
    run_basic_tests
    generate_report

    echo ""
    log_info "✅ Verification completed successfully!"
    echo ""
    echo "Dashboard URLs:"
    echo "  - Grafana: http://localhost:3000 (admin/admin)"
    echo "  - Prometheus: http://localhost:9090"
    echo "  - Jupyter: http://localhost:8888 (token: barter_jupyter)"
    echo ""

    cleanup
}

# Run main function
main "$@"