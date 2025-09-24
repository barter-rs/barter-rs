# 快速启动指南 Quick Start Guide

## 🚀 5分钟快速体验

### 1. 克隆并启动（无需API密钥）

```bash
# 克隆项目
git clone https://github.com/TaoSeekAI/barter-rs.git
cd barter-rs

# 快速启动测试模式
make env        # 创建配置文件
make up         # 启动所有服务
make logs       # 查看日志
```

### 2. 访问监控面板

- 📊 **Grafana**: http://localhost:3000 (admin/admin)
- 📈 **Prometheus**: http://localhost:9090
- 🔬 **Jupyter**: http://localhost:8888 (token: barter_jupyter)

## 🔑 使用真实API密钥

### 安全配置API密钥

```bash
# 运行配置向导
chmod +x scripts/setup_api_keys.sh
./scripts/setup_api_keys.sh

# 或手动编辑
nano .env
```

### API密钥获取指南

#### Binance (币安)

1. **测试网（推荐初始测试）**
   - 访问: https://testnet.binance.vision/
   - 注册测试账户
   - 生成API密钥（自动获得测试资金）

2. **主网**
   - 访问: https://www.binance.com/en/my/settings/api-management
   - 创建API密钥
   - 设置权限:
     - ✅ Enable Reading
     - ✅ Enable Spot & Margin Trading
     - ✅ Enable Futures
     - ❌ **禁用** Enable Withdrawals
   - 设置IP白名单（重要！）

#### OKX (欧易)

1. **模拟账户（推荐初始测试）**
   - 访问: https://www.okx.com/account/demo-trading
   - 申请模拟账户
   - 创建API密钥

2. **主网**
   - 访问: https://www.okx.com/account/my-api
   - 创建V5 API
   - 权限设置:
     - Read: ✅
     - Trade: ✅ (稍后启用)
     - Withdraw: ❌ **禁用**
   - 记录Passphrase（重要！）

## ✅ 验证系统

### 运行完整验证

```bash
# 给脚本执行权限
chmod +x scripts/verify_system.sh

# 运行验证（安全模式）
./scripts/verify_system.sh

# 运行验证（模拟交易）
./scripts/verify_system.sh -m paper

# 详细输出
./scripts/verify_system.sh -v
```

### 验证检查点

1. **基础系统** ✅
   ```bash
   docker-compose ps  # 所有服务应该是 "Up"
   ```

2. **数据连接** ✅
   ```bash
   # 查看实时数据流
   docker-compose logs -f barter-strategy | grep "Signal"
   ```

3. **交易逻辑** ✅
   ```bash
   # 查看交易决策
   docker-compose logs -f barter-strategy | grep "DECISION"
   ```

## 📊 测试交易策略

### 1. Paper Trading（模拟交易）

```bash
# 设置为模拟交易模式
export TRADING_MODE=paper

# 启动
make up

# 监控
make grafana
```

### 2. 回测历史数据

```bash
# 运行回测
docker-compose run --rm barter-strategy \
  cargo run --example aster_trading

# 查看结果
cat data/backtest_results.csv
```

### 3. 实时监控

```sql
-- 在PostgreSQL中查看交易
docker-compose exec postgres psql -U barter -d barter

-- 查看最近交易
SELECT * FROM trades ORDER BY timestamp DESC LIMIT 10;

-- 查看持仓
SELECT * FROM positions WHERE status = 'OPEN';

-- 查看每日收益
SELECT * FROM daily_performance;
```

## 🎯 ASTER/USDT 永续合约配置

### 特殊配置

```bash
# 编辑 .env
TRADING_SYMBOL=ASTER-USDT-SWAP  # OKX格式
# 或
TRADING_SYMBOL=ASTERUSDT         # Binance格式

TRADING_LEVERAGE=10              # 杠杆倍数
MAX_POSITION_SIZE=10000          # 最大仓位
STOP_LOSS_PCT=0.03              # 3%止损
TAKE_PROFIT_PCT=0.06            # 6%止盈
```

### 启动ASTER交易

```bash
# 确保配置正确
grep ASTER .env

# 启动交易系统
TRADING_MODE=paper make up

# 监控ASTER信号
docker-compose logs -f barter-strategy | grep ASTER
```

## 🛡️ 安全检查清单

### 启动前必查

- [ ] API密钥设置了IP白名单
- [ ] 禁用了提现权限
- [ ] 使用测试网/模拟账户测试
- [ ] 设置了合理的止损
- [ ] 配置了最大仓位限制
- [ ] 备份了配置文件

### 风险参数

```bash
# 保守配置（推荐初始）
MAX_POSITION_SIZE=1000    # $1000最大仓位
MAX_LEVERAGE=3           # 3倍杠杆
STOP_LOSS_PCT=0.02      # 2%止损
MAX_DAILY_LOSS=0.05     # 5%最大日损失

# 正常配置
MAX_POSITION_SIZE=10000   # $10000最大仓位
MAX_LEVERAGE=10          # 10倍杠杆
STOP_LOSS_PCT=0.03      # 3%止损
MAX_DAILY_LOSS=0.10     # 10%最大日损失
```

## 🔧 常用命令

```bash
# 服务管理
make up          # 启动服务
make down        # 停止服务
make restart     # 重启服务
make logs        # 查看日志
make status      # 服务状态

# 开发调试
make dev         # 开发模式
make test        # 运行测试
make shell       # 进入容器

# 数据管理
make db-backup   # 备份数据库
make clean       # 清理数据

# 监控
make grafana     # 打开Grafana
make metrics     # 查看指标
```

## ❓ 常见问题

### 1. Docker服务启动失败

```bash
# 检查Docker
docker --version
docker-compose --version

# 清理并重试
make clean
make build
make up
```

### 2. API连接失败

```bash
# 检查网络
ping api.binance.com
ping www.okx.com

# 验证API密钥
./scripts/verify_system.sh -v
```

### 3. 没有交易信号

```bash
# 检查数据流
docker-compose logs barter-strategy | tail -100

# 重启数据采集
docker-compose restart barter-strategy
```

## 📞 获取帮助

- 📖 [完整文档](./docs/)
- 🐛 [报告问题](https://github.com/TaoSeekAI/barter-rs/issues)
- 💬 [讨论区](https://github.com/TaoSeekAI/barter-rs/discussions)

## ⚠️ 风险提示

**加密货币交易风险极高，可能导致全部本金损失。**

- 先用测试网练习
- 从小额资金开始
- 设置严格止损
- 不要使用无法承受损失的资金
- 定期检查和更新策略

---

祝您交易顺利！ 🚀