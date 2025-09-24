# Binance Testnet Trading Guide

## 🎯 快速开始

您的Binance测试网API密钥已配置完成！现在可以立即开始测试交易系统。

### 一键启动

```bash
# 最简单的方式 - 自动完成所有配置和启动
make testnet

# 或者手动步骤：
chmod +x start_testnet.sh
./start_testnet.sh
```

## 📋 已配置的API密钥

```
API Key: Wt104kkmijNETENuP4hpJfnGLZxjcjhpH7cYVckIvGAeeI6vxd24Vf8zGKs4lznM
Secret:  q7MCl5Fp3tILTDsoVA7rG6WzzV2lscHYWsYVp65RYZaXI5dnDGMqXMKDkaniP2Wx
```

**重要**: 这是Binance官方测试网密钥，仅用于测试环境，无真实资金风险。

## 🚀 使用方法

### 1. 测试连接

```bash
# 验证API连接
make testnet-test

# 应该看到：
# ✅ Server is reachable
# ✅ Authentication successful
# ✅ Account balances displayed
```

### 2. 启动交易系统

```bash
# 启动所有服务
make testnet-up

# 查看日志
make testnet-logs

# 运行示例交易
make testnet-trade
```

### 3. 监控系统

- **Grafana监控面板**: http://localhost:3001
  - 用户名: admin
  - 密码: admin
- **实时日志**: `make testnet-logs`
- **交易监控**: `docker-compose -f docker-compose.testnet.yml logs -f | grep TRADE`

## 💰 测试网资金

Binance测试网提供虚拟资金用于测试：

1. 访问 https://testnet.binance.vision/
2. 登录您的测试账户
3. 点击 "Faucet" 获取测试资金
4. 可获得的测试币：
   - BTC: 1 BTC
   - USDT: 10,000 USDT
   - ETH: 10 ETH
   - BNB: 500 BNB

## 📊 支持的交易对

测试网支持主要交易对：
- BTCUSDT (推荐 - 流动性最好)
- ETHUSDT
- BNBUSDT
- BTCBUSD
- ETHBUSD

## 🧪 测试场景

### 场景1: 基础交易测试

```bash
# 运行预配置的交易测试
docker-compose -f docker-compose.testnet.yml run --rm barter-strategy \
    cargo run --example binance_testnet_trading
```

### 场景2: 策略回测

```bash
# 使用测试网数据回测
docker-compose -f docker-compose.testnet.yml run --rm barter-strategy \
    cargo run --example backtest -- \
    --symbol BTCUSDT \
    --testnet
```

### 场景3: 实时Paper Trading

```bash
# 配置环境
export TRADING_MODE=paper
export TRADING_SYMBOL=BTCUSDT

# 启动
make testnet-up

# 监控
watch -n 1 'docker-compose -f docker-compose.testnet.yml logs --tail=20'
```

## 🛠️ 常用命令

```bash
# 服务管理
make testnet          # 完整启动流程
make testnet-up       # 启动服务
make testnet-down     # 停止服务
make testnet-logs     # 查看日志
make testnet-clean    # 清理所有数据

# 测试和验证
make testnet-test     # 测试API连接
make testnet-trade    # 运行交易示例

# 数据库查询
docker-compose -f docker-compose.testnet.yml exec postgres \
    psql -U barter -d barter_testnet -c "SELECT * FROM trades;"
```

## 📈 交易策略配置

编辑 `.env.testnet` 调整策略参数：

```bash
# 风险参数
MAX_POSITION_SIZE=1000    # 最大仓位
MAX_LEVERAGE=10          # 最大杠杆
STOP_LOSS_PCT=0.02       # 止损百分比
TAKE_PROFIT_PCT=0.04     # 止盈百分比

# 交易参数
CONFIDENCE_THRESHOLD=0.6  # 最小置信度
RISK_THRESHOLD=0.7       # 风险阈值
```

## 🔍 调试技巧

### 查看详细日志

```bash
# 开启DEBUG模式
export RUST_LOG=debug
make testnet-up
```

### 监控特定事件

```bash
# 只看交易信号
docker-compose -f docker-compose.testnet.yml logs -f | grep SIGNAL

# 只看决策
docker-compose -f docker-compose.testnet.yml logs -f | grep DECISION

# 只看订单
docker-compose -f docker-compose.testnet.yml logs -f | grep ORDER
```

### 性能监控

```bash
# 查看资源使用
docker stats

# 查看延迟
docker-compose -f docker-compose.testnet.yml logs | grep latency
```

## ⚠️ 注意事项

1. **测试网限制**:
   - API请求限制: 1200/分钟
   - WebSocket连接: 最多5个
   - 订单限制: 200个开放订单

2. **数据差异**:
   - 测试网价格可能与主网不同
   - 流动性较低
   - 可能有数据延迟

3. **最佳实践**:
   - 始终先在测试网验证策略
   - 测试各种市场条件
   - 记录所有测试结果

## 🎓 学习路径

1. **第1天**: 熟悉系统，运行基础测试
2. **第2-3天**: 测试不同交易策略
3. **第4-5天**: 优化参数，分析结果
4. **第6-7天**: 模拟极端市场条件
5. **第2周**: 准备转向主网

## 📊 查看测试结果

### PostgreSQL查询

```sql
-- 连接数据库
docker-compose -f docker-compose.testnet.yml exec postgres \
    psql -U barter -d barter_testnet

-- 查看所有交易
SELECT * FROM trades ORDER BY timestamp DESC;

-- 查看盈亏统计
SELECT
    DATE(timestamp) as date,
    COUNT(*) as trades,
    SUM(CASE WHEN pnl > 0 THEN 1 ELSE 0 END) as wins,
    SUM(pnl) as total_pnl
FROM trades
GROUP BY DATE(timestamp);

-- 查看当前持仓
SELECT * FROM positions WHERE status = 'OPEN';
```

### Grafana仪表板

1. 访问 http://localhost:3001
2. 使用 admin/admin 登录
3. 查看预配置的仪表板：
   - Trading Performance
   - System Metrics
   - Risk Analytics

## 🚨 故障排除

### 连接问题

```bash
# 检查网络
ping testnet.binance.vision

# 验证API密钥
make testnet-test

# 重启服务
make testnet-down
make testnet-up
```

### 清理和重置

```bash
# 完全清理
make testnet-clean

# 重新开始
make testnet
```

## 📞 获取帮助

- Binance测试网文档: https://testnet.binance.vision/
- 项目Issues: https://github.com/TaoSeekAI/barter-rs/issues
- 测试网状态: https://testnet.binance.vision/status

## ✅ 下一步

成功完成测试网测试后：

1. **分析结果**: 查看交易日志和性能指标
2. **优化策略**: 根据测试结果调整参数
3. **风险评估**: 确保理解所有风险
4. **小额实盘**: 从最小金额开始真实交易

---

**记住**: 测试网是您的安全练习场，充分利用它来完善您的交易策略！ 🚀