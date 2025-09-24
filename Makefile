.PHONY: help build up down logs shell test clean deploy

# Default target
help:
	@echo "Barter Trading System - Docker Commands"
	@echo ""
	@echo "Usage: make [target]"
	@echo ""
	@echo "Targets:"
	@echo "  help           Show this help message"
	@echo "  build          Build Docker images"
	@echo "  up             Start all services"
	@echo "  down           Stop all services"
	@echo "  logs           View logs"
	@echo "  shell          Open shell in strategy container"
	@echo "  test           Run tests"
	@echo "  clean          Clean up containers and volumes"
	@echo "  deploy         Deploy to production"
	@echo ""
	@echo "Development:"
	@echo "  dev            Start in development mode"
	@echo "  dev-logs       View development logs"
	@echo "  dev-test       Run tests in development"
	@echo ""
	@echo "Production:"
	@echo "  prod           Start in production mode"
	@echo "  prod-scale     Scale production services"
	@echo "  prod-backup    Backup production data"
	@echo ""
	@echo "Monitoring:"
	@echo "  grafana        Open Grafana dashboard"
	@echo "  prometheus     Open Prometheus UI"
	@echo "  metrics        Show current metrics"

# Build Docker images
build:
	docker-compose build --no-cache

build-prod:
	docker buildx build --platform linux/amd64,linux/arm64 -t ghcr.io/taoseekai/barter-strategy:latest --push .

# Start services
up:
	docker-compose up -d

down:
	docker-compose down

restart:
	docker-compose restart

# Development mode
dev:
	docker-compose -f docker-compose.yml -f docker-compose.dev.yml up

dev-logs:
	docker-compose -f docker-compose.yml -f docker-compose.dev.yml logs -f

dev-test:
	docker-compose -f docker-compose.yml -f docker-compose.dev.yml run --rm test-runner

dev-shell:
	docker-compose -f docker-compose.yml -f docker-compose.dev.yml exec barter-strategy /bin/bash

# Production mode
prod:
	docker-compose -f docker-compose.yml -f docker-compose.prod.yml up -d

prod-logs:
	docker-compose -f docker-compose.yml -f docker-compose.prod.yml logs -f

prod-scale:
	docker-compose -f docker-compose.yml -f docker-compose.prod.yml up -d --scale barter-strategy=3

prod-stop:
	docker-compose -f docker-compose.yml -f docker-compose.prod.yml down

# Logging
logs:
	docker-compose logs -f

logs-strategy:
	docker-compose logs -f barter-strategy

logs-fluvio:
	docker-compose logs -f fluvio

logs-db:
	docker-compose logs -f postgres

# Shell access
shell:
	docker-compose exec barter-strategy /bin/bash

shell-db:
	docker-compose exec postgres psql -U barter -d barter

shell-redis:
	docker-compose exec redis redis-cli

# Testing
test:
	docker-compose run --rm barter-strategy cargo test --all

test-integration:
	docker-compose run --rm barter-strategy cargo test --test integration_test

benchmark:
	docker-compose run --rm barter-strategy cargo bench

# Database operations
db-migrate:
	docker-compose exec postgres psql -U barter -d barter -f /docker-entrypoint-initdb.d/init.sql

db-backup:
	@mkdir -p backups
	docker-compose exec postgres pg_dump -U barter barter > backups/barter_$(shell date +%Y%m%d_%H%M%S).sql
	@echo "Database backed up to backups/barter_$(shell date +%Y%m%d_%H%M%S).sql"

db-restore:
	@read -p "Enter backup file name: " backup; \
	docker-compose exec -T postgres psql -U barter barter < backups/$$backup

# Monitoring
grafana:
	@echo "Opening Grafana dashboard..."
	@open http://localhost:3000 || xdg-open http://localhost:3000 || echo "Navigate to http://localhost:3000"

prometheus:
	@echo "Opening Prometheus UI..."
	@open http://localhost:9090 || xdg-open http://localhost:9090 || echo "Navigate to http://localhost:9090"

metrics:
	@curl -s http://localhost:9464/metrics | grep -E "^barter_"

health:
	@curl -s http://localhost/health | jq .

# Clean up
clean:
	docker-compose down -v
	docker system prune -f

clean-all:
	docker-compose down -v
	docker system prune -af
	rm -rf data/* logs/* backups/*

# Deployment
deploy:
	@echo "Deploying to production..."
	git push origin main
	@echo "GitHub Actions will build and push the image"
	@echo "Monitor deployment at: https://github.com/TaoSeekAI/barter-rs/actions"

deploy-staging:
	@echo "Deploying to staging..."
	git push origin develop

# Docker registry
login:
	echo $(GITHUB_TOKEN) | docker login ghcr.io -u $(GITHUB_USER) --password-stdin

push:
	docker tag barter-strategy:latest ghcr.io/taoseekai/barter-strategy:latest
	docker push ghcr.io/taoseekai/barter-strategy:latest

pull:
	docker pull ghcr.io/taoseekai/barter-strategy:latest

# Fluvio operations
fluvio-topics:
	docker-compose exec fluvio fluvio topic list

fluvio-produce:
	@read -p "Enter topic name: " topic; \
	read -p "Enter message: " message; \
	docker-compose exec fluvio fluvio produce $$topic --value "$$message"

fluvio-consume:
	@read -p "Enter topic name: " topic; \
	docker-compose exec fluvio fluvio consume $$topic --from-beginning

# Status and info
status:
	@echo "=== Service Status ==="
	@docker-compose ps
	@echo ""
	@echo "=== Resource Usage ==="
	@docker stats --no-stream
	@echo ""
	@echo "=== Network Info ==="
	@docker network ls | grep barter

info:
	@echo "=== Docker Info ==="
	@docker version
	@echo ""
	@echo "=== Image Info ==="
	@docker images | grep barter
	@echo ""
	@echo "=== Volume Info ==="
	@docker volume ls | grep barter

# Environment setup
env:
	@test -f .env || cp .env.example .env
	@echo ".env file ready. Please edit it with your configuration."

check-env:
	@test -f .env || (echo "Error: .env file not found. Run 'make env' first." && exit 1)

# SSL certificate generation
ssl:
	@mkdir -p nginx/ssl
	@openssl req -x509 -nodes -days 365 -newkey rsa:2048 \
		-keyout nginx/ssl/key.pem -out nginx/ssl/cert.pem \
		-subj "/C=US/ST=State/L=City/O=Organization/CN=localhost"
	@echo "SSL certificates generated in nginx/ssl/"

# Development utilities
fmt:
	docker-compose run --rm barter-strategy cargo fmt

clippy:
	docker-compose run --rm barter-strategy cargo clippy -- -D warnings

audit:
	docker-compose run --rm barter-strategy cargo audit

# Testnet commands
testnet:
	@echo "Starting Binance Testnet Trading System..."
	@chmod +x start_testnet.sh
	@./start_testnet.sh

testnet-up:
	docker-compose -f docker-compose.testnet.yml up -d

testnet-down:
	docker-compose -f docker-compose.testnet.yml down

testnet-logs:
	docker-compose -f docker-compose.testnet.yml logs -f

testnet-test:
	@chmod +x scripts/test_binance_testnet.sh
	@./scripts/test_binance_testnet.sh

testnet-trade:
	docker-compose -f docker-compose.testnet.yml run --rm barter-strategy \
		cargo run --example binance_testnet_trading

testnet-clean:
	docker-compose -f docker-compose.testnet.yml down -v
	rm -rf data/testnet/* logs/testnet/*

# Quick commands
qs: up logs-strategy  # Quick start
qd: down clean         # Quick down
qt: testnet           # Quick testnet

# Default target
.DEFAULT_GOAL := help