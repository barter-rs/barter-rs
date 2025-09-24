#!/bin/bash

# Secure API Key Setup Script
# This script helps you safely configure API keys

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

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

secure_input() {
    local prompt="$1"
    local var_name="$2"
    local is_secret="${3:-false}"

    echo -n "$prompt"
    if [ "$is_secret" = true ]; then
        read -s input
        echo
    else
        read input
    fi
    eval "$var_name='$input'"
}

validate_api_key() {
    local key="$1"
    local exchange="$2"

    # Basic validation - check length and format
    if [ -z "$key" ]; then
        return 1
    fi

    case "$exchange" in
        binance)
            if [ ${#key} -eq 64 ]; then
                return 0
            fi
            ;;
        okx)
            if [ ${#key} -eq 32 ] || [ ${#key} -eq 36 ]; then
                return 0
            fi
            ;;
    esac

    return 1
}

test_binance_connection() {
    local api_key="$1"
    local api_secret="$2"
    local testnet="$3"

    log_info "Testing Binance connection..."

    local base_url
    if [ "$testnet" = true ]; then
        base_url="https://testnet.binance.vision"
    else
        base_url="https://api.binance.com"
    fi

    # Test public endpoint first
    if curl -s "$base_url/api/v3/ping" | grep -q "{}"; then
        log_info "✓ Binance API is reachable"
    else
        log_error "✗ Cannot reach Binance API"
        return 1
    fi

    # Test authenticated endpoint (account info)
    local timestamp=$(date +%s000)
    local query="timestamp=$timestamp"
    local signature=$(echo -n "$query" | openssl dgst -sha256 -hmac "$api_secret" | cut -d' ' -f2)

    local response=$(curl -s -H "X-MBX-APIKEY: $api_key" \
        "$base_url/api/v3/account?$query&signature=$signature")

    if echo "$response" | grep -q "balances"; then
        log_info "✓ API credentials are valid"
        return 0
    else
        log_error "✗ API credentials are invalid"
        log_error "Response: $response"
        return 1
    fi
}

test_okx_connection() {
    local api_key="$1"
    local api_secret="$2"
    local passphrase="$3"
    local testnet="$4"

    log_info "Testing OKX connection..."

    local base_url
    if [ "$testnet" = true ]; then
        base_url="https://www.okx.com"  # OKX uses same URL for testnet
    else
        base_url="https://www.okx.com"
    fi

    # Test public endpoint
    if curl -s "$base_url/api/v5/public/time" | grep -q "ts"; then
        log_info "✓ OKX API is reachable"
    else
        log_error "✗ Cannot reach OKX API"
        return 1
    fi

    # For OKX, we would need to implement signature generation
    # This is more complex and requires proper HMAC-SHA256 with base64
    log_warn "⚠ OKX credential validation requires running the actual client"

    return 0
}

setup_exchange() {
    local exchange="$1"

    echo ""
    echo "========================================="
    echo "Setting up $exchange"
    echo "========================================="

    local use_testnet
    local api_key
    local api_secret
    local passphrase

    # Ask for testnet/mainnet
    echo "Do you want to use testnet/demo account?"
    echo "  1) Yes - Testnet (Recommended for initial testing)"
    echo "  2) No - Mainnet (Real trading)"
    read -p "Choice [1]: " choice
    choice=${choice:-1}

    if [ "$choice" = "1" ]; then
        use_testnet=true
        log_info "Using testnet/demo mode"
    else
        use_testnet=false
        log_warn "⚠ Using MAINNET - Real money at risk!"
        read -p "Type 'I UNDERSTAND' to continue: " confirm
        if [ "$confirm" != "I UNDERSTAND" ]; then
            log_info "Setup cancelled"
            return
        fi
    fi

    # Get credentials
    case "$exchange" in
        binance)
            secure_input "Enter Binance API Key: " api_key false
            secure_input "Enter Binance API Secret: " api_secret true

            if validate_api_key "$api_key" "binance"; then
                log_info "✓ API key format looks valid"
            else
                log_warn "⚠ API key format may be incorrect"
            fi

            # Test connection
            if test_binance_connection "$api_key" "$api_secret" "$use_testnet"; then
                # Save to .env
                if [ "$use_testnet" = true ]; then
                    echo "BINANCE_TESTNET_API_KEY=$api_key" >> .env
                    echo "BINANCE_TESTNET_API_SECRET=$api_secret" >> .env
                else
                    echo "BINANCE_API_KEY=$api_key" >> .env
                    echo "BINANCE_API_SECRET=$api_secret" >> .env
                fi
                log_info "✓ Binance credentials saved"
            else
                log_error "Failed to validate Binance credentials"
            fi
            ;;

        okx)
            secure_input "Enter OKX API Key: " api_key false
            secure_input "Enter OKX API Secret: " api_secret true
            secure_input "Enter OKX Passphrase: " passphrase true

            if validate_api_key "$api_key" "okx"; then
                log_info "✓ API key format looks valid"
            else
                log_warn "⚠ API key format may be incorrect"
            fi

            # Test connection
            if test_okx_connection "$api_key" "$api_secret" "$passphrase" "$use_testnet"; then
                # Save to .env
                if [ "$use_testnet" = true ]; then
                    echo "OKX_DEMO_API_KEY=$api_key" >> .env
                    echo "OKX_DEMO_API_SECRET=$api_secret" >> .env
                    echo "OKX_DEMO_PASSPHRASE=$passphrase" >> .env
                else
                    echo "OKX_API_KEY=$api_key" >> .env
                    echo "OKX_API_SECRET=$api_secret" >> .env
                    echo "OKX_PASSPHRASE=$passphrase" >> .env
                fi
                log_info "✓ OKX credentials saved"
            else
                log_error "Failed to validate OKX credentials"
            fi
            ;;
    esac
}

main() {
    echo "========================================="
    echo "   Barter API Key Configuration Tool    "
    echo "========================================="
    echo ""
    log_warn "⚠ SECURITY NOTICE:"
    echo "  - Never share your API keys"
    echo "  - Use IP whitelist on exchange"
    echo "  - Start with testnet/demo accounts"
    echo "  - Disable withdrawal permissions"
    echo ""

    # Check if .env exists
    if [ -f .env ]; then
        log_info "Found existing .env file"
        read -p "Do you want to backup it first? (y/n) [y]: " backup
        backup=${backup:-y}
        if [ "$backup" = "y" ]; then
            cp .env ".env.backup.$(date +%Y%m%d_%H%M%S)"
            log_info "Backup created"
        fi
    else
        log_info "Creating new .env file from template"
        cp .env.example .env
    fi

    # Main menu
    while true; do
        echo ""
        echo "Which exchange do you want to configure?"
        echo "  1) Binance"
        echo "  2) OKX"
        echo "  3) Both"
        echo "  4) Exit"
        read -p "Choice [1]: " choice
        choice=${choice:-1}

        case $choice in
            1)
                setup_exchange "binance"
                ;;
            2)
                setup_exchange "okx"
                ;;
            3)
                setup_exchange "binance"
                setup_exchange "okx"
                ;;
            4)
                break
                ;;
            *)
                log_error "Invalid choice"
                ;;
        esac
    done

    echo ""
    log_info "Configuration complete!"
    echo ""
    echo "Next steps:"
    echo "  1. Review your .env file"
    echo "  2. Run verification: ./scripts/verify_system.sh"
    echo "  3. Start paper trading: make dev"
    echo "  4. Monitor at http://localhost:3000"

    # Set secure permissions
    chmod 600 .env
    log_info "Set secure permissions on .env file (600)"
}

# Check dependencies
if ! command -v openssl &> /dev/null; then
    log_error "openssl is required but not installed"
    exit 1
fi

if ! command -v curl &> /dev/null; then
    log_error "curl is required but not installed"
    exit 1
fi

# Run main
main