# Docker Setup Guide

This guide explains how to run the Barter Trading System using Docker and Docker Compose.

## Prerequisites

- Docker Engine 20.10+
- Docker Compose 2.0+
- 4GB RAM minimum (8GB recommended)
- 10GB disk space

## Quick Start

### 1. Clone the repository
```bash
git clone https://github.com/TaoSeekAI/barter-rs.git
cd barter-rs
```

### 2. Set up environment variables
```bash
cp .env.example .env
# Edit .env with your configuration
```

### 3. Start the services
```bash
# Development mode
docker-compose -f docker-compose.yml -f docker-compose.dev.yml up

# Production mode
docker-compose -f docker-compose.yml -f docker-compose.prod.yml up -d

# Test mode (default)
docker-compose up
```

## Architecture

The system consists of the following services:

### Core Services
- **barter-strategy**: Main trading engine
- **fluvio**: Message streaming platform
- **redis**: Caching and state management
- **postgres**: Trade history and analytics database

### Monitoring Stack
- **prometheus**: Metrics collection
- **grafana**: Visualization dashboards (http://localhost:3000)
- **nginx**: Reverse proxy and load balancer

### Optional Services
- **jupyter**: Data analysis notebook (profile: analysis)

## Configuration

### Environment Variables

Key environment variables for `barter-strategy`:

```bash
# Trading Configuration
TRADING_MODE=test              # test, paper, live, backtest
TRADING_SYMBOL=ASTER-USDT-SWAP # Trading pair
TRADING_LEVERAGE=10            # Leverage multiplier
EXCHANGE_API_KEY=your_key      # Exchange API key (live mode)
EXCHANGE_API_SECRET=your_secret # Exchange API secret (live mode)

# Risk Management
MAX_POSITION_SIZE=50000         # Maximum position size in USDT
STOP_LOSS_PCT=0.03             # Stop loss percentage (3%)
TAKE_PROFIT_PCT=0.06           # Take profit percentage (6%)
CONFIDENCE_THRESHOLD=0.65      # Minimum confidence for trades

# Model Configuration
MODEL_NAME=mistral-7b-instruct # AI model name
USE_GPU=false                  # Enable GPU acceleration
PREDICTION_THRESHOLD=0.7       # Model prediction threshold

# System Configuration
RUST_LOG=info                  # Log level: debug, info, warn, error
FLUVIO_URL=fluvio:9003        # Fluvio connection URL
```

### Trading Modes

1. **test**: No real trades, uses simulated data
2. **paper**: Paper trading with real market data
3. **live**: Real trading with actual funds (requires API credentials)
4. **backtest**: Historical data testing

## Building Images

### Local Build
```bash
docker build -t barter-strategy:local .
```

### Multi-platform Build
```bash
docker buildx build --platform linux/amd64,linux/arm64 -t barter-strategy:latest .
```

## Deployment

### Development
```bash
# Start with hot-reload
docker-compose -f docker-compose.yml -f docker-compose.dev.yml up

# Run tests
docker-compose -f docker-compose.yml -f docker-compose.dev.yml run --rm test-runner

# Access logs
docker-compose logs -f barter-strategy
```

### Production
```bash
# Deploy with production settings
docker-compose -f docker-compose.yml -f docker-compose.prod.yml up -d

# Scale the service
docker-compose -f docker-compose.yml -f docker-compose.prod.yml up -d --scale barter-strategy=3

# Rolling update
docker-compose -f docker-compose.yml -f docker-compose.prod.yml up -d --no-deps barter-strategy
```

## Monitoring

### Grafana Dashboard
- URL: http://localhost:3000
- Default credentials: admin/admin
- Pre-configured dashboards:
  - Trading Performance
  - System Metrics
  - Market Analysis

### Prometheus Metrics
- URL: http://localhost:9090
- Available metrics:
  - Trade execution latency
  - Win/loss rates
  - Portfolio value
  - System resource usage

### Health Checks
```bash
# Check service health
docker-compose ps

# Check specific service
curl http://localhost/health

# View logs
docker-compose logs -f [service_name]
```

## Data Management

### Backup
```bash
# Backup database
docker-compose exec postgres pg_dump -U barter barter > backup.sql

# Backup volumes
docker run --rm -v barter-rs_postgres-data:/data -v $(pwd):/backup alpine tar czf /backup/postgres-backup.tar.gz /data
```

### Restore
```bash
# Restore database
docker-compose exec -T postgres psql -U barter barter < backup.sql

# Restore volumes
docker run --rm -v barter-rs_postgres-data:/data -v $(pwd):/backup alpine tar xzf /backup/postgres-backup.tar.gz -C /
```

## Troubleshooting

### Common Issues

1. **Container fails to start**
   ```bash
   # Check logs
   docker-compose logs barter-strategy

   # Verify configuration
   docker-compose config
   ```

2. **Connection issues**
   ```bash
   # Check network
   docker network ls
   docker network inspect barter-rs_barter-network
   ```

3. **Performance issues**
   ```bash
   # Check resource usage
   docker stats

   # Increase resources in docker-compose.yml
   ```

### Debug Mode
```bash
# Run with debug logging
RUST_LOG=debug docker-compose up

# Interactive shell
docker-compose exec barter-strategy /bin/bash
```

## Security

### SSL/TLS Setup
1. Generate certificates:
   ```bash
   mkdir -p nginx/ssl
   openssl req -x509 -nodes -days 365 -newkey rsa:2048 \
     -keyout nginx/ssl/key.pem -out nginx/ssl/cert.pem
   ```

2. Update nginx configuration with your domain

### Secrets Management
- Never commit `.env` files
- Use Docker secrets in production:
  ```yaml
  secrets:
    api_key:
      external: true
  ```

## CI/CD

The project includes GitHub Actions workflows for:
- Building and pushing images to GitHub Container Registry
- Running security scans with Trivy
- Automated deployment to staging/production

Images are automatically published to:
```
ghcr.io/taoseekai/barter-rs:latest
ghcr.io/taoseekai/barter-rs:develop
ghcr.io/taoseekai/barter-rs:v1.0.0
```

## Performance Tuning

### Docker Settings
```json
{
  "default-ulimits": {
    "nofile": {
      "Hard": 64000,
      "Soft": 64000
    }
  },
  "log-driver": "json-file",
  "log-opts": {
    "max-size": "10m",
    "max-file": "3"
  }
}
```

### Resource Limits
Configure in `docker-compose.prod.yml`:
```yaml
deploy:
  resources:
    limits:
      cpus: '2'
      memory: 2G
    reservations:
      cpus: '1'
      memory: 1G
```

## License

MIT License - See LICENSE file for details