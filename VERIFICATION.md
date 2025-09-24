# 系统验证指南

本指南将帮助您安全地验证Barter交易系统的可用性。

## ⚠️ 重要安全提示

1. **首先在测试网进行验证**
2. **使用最小权限的API密钥**
3. **设置IP白名单**
4. **启用只读权限进行初始测试**
5. **绝不在公共仓库提交API密钥**

## 1. 准备工作

### 1.1 获取API密钥

#### Binance (币安)
1. 登录 [Binance](https://www.binance.com)
2. 进入 API管理页面
3. 创建新的API密钥，建议命名为 "barter-test"
4. 权限设置：
   - ✅ 启用读取权限
   - ✅ 启用现货和合约交易（测试后期）
   - ❌ 禁用提现权限
   - ✅ 设置IP限制（添加您的服务器IP）

#### OKX (欧易)
1. 登录 [OKX](https://www.okx.com)
2. 进入 API页面
3. 创建V5 API密钥
4. 权限设置：
   - 权限：只读（初始测试）
   - 交易：稍后启用
   - Passphrase：记录并保管好

### 1.2 环境配置

```bash
# 克隆项目
git clone https://github.com/TaoSeekAI/barter-rs.git
cd barter-rs

# 创建环境配置
cp .env.example .env

# 编辑.env文件，添加您的API密钥
nano .env
```

## 2. 分阶段验证

### 阶段1: 基础系统验证（无API密钥）

```bash
# 1. 验证Docker环境
docker --version
docker-compose --version

# 2. 启动测试模式
TRADING_MODE=test make up

# 3. 检查服务状态
make status

# 4. 查看日志
make logs-strategy

# 5. 健康检查
make health

# 预期结果：
# - 所有服务正常启动
# - 无错误日志
# - 健康检查返回 {"status":"healthy"}
```

### 阶段2: 数据连接验证（只读API）

创建验证配置文件：

```bash
cat > config/verify.json <<EOF
{
  "mode": "verify",
  "exchanges": [
    {
      "name": "binance",
      "testnet": true,
      "api_key": "YOUR_BINANCE_TESTNET_KEY",
      "api_secret": "YOUR_BINANCE_TESTNET_SECRET"
    },
    {
      "name": "okx",
      "testnet": true,
      "api_key": "YOUR_OKX_DEMO_KEY",
      "api_secret": "YOUR_OKX_DEMO_SECRET",
      "passphrase": "YOUR_OKX_PASSPHRASE"
    }
  ]
}
EOF
```

运行数据验证：

```bash
# 启动数据验证模式
docker-compose run --rm barter-strategy \
  cargo run --example verify_connections

# 检查数据流
make fluvio-consume topic=market-data
```

### 阶段3: 策略验证（模拟交易）

```bash
# 1. 启动Paper Trading模式
TRADING_MODE=paper make up

# 2. 监控交易决策
docker-compose logs -f barter-strategy | grep -E "DECISION|SIGNAL|ACTION"

# 3. 查看Grafana监控
open http://localhost:3000
# 用户名: admin
# 密码: admin
```

### 阶段4: 回测验证

```bash
# 运行历史数据回测
docker-compose run --rm barter-strategy \
  cargo run --example backtest -- \
  --symbol BTCUSDT \
  --start 2024-01-01 \
  --end 2024-01-31 \
  --initial-capital 10000

# 查看回测结果
cat data/backtest_results.csv
```

## 3. 验证检查清单

### 基础功能验证

- [ ] Docker服务全部启动成功
- [ ] 日志无错误信息
- [ ] 健康检查通过
- [ ] Grafana可访问
- [ ] Prometheus指标正常

### 数据连接验证

- [ ] Binance WebSocket连接成功
- [ ] OKX WebSocket连接成功
- [ ] 实时价格数据接收正常
- [ ] 订单簿数据更新正常
- [ ] 数据写入Fluvio队列

### 交易逻辑验证

- [ ] 信号生成正常
- [ ] 技术指标计算正确
- [ ] AI模型推理执行
- [ ] 风险管理规则生效
- [ ] 模拟订单创建成功

### 性能验证

- [ ] 延迟 < 100ms
- [ ] CPU使用率 < 70%
- [ ] 内存使用稳定
- [ ] 无内存泄漏
- [ ] 消息队列无积压

## 4. 监控指标

访问 http://localhost:3000 查看Grafana仪表板：

### 关键指标

1. **系统健康度**
   - 服务可用性
   - API连接状态
   - 错误率

2. **交易性能**
   - 信号延迟
   - 决策速度
   - 执行延迟

3. **数据质量**
   - 数据完整性
   - 更新频率
   - 异常检测

## 5. 故障排查

### 常见问题

#### API连接失败
```bash
# 检查网络连接
docker-compose exec barter-strategy ping api.binance.com

# 验证API密钥
docker-compose exec barter-strategy \
  curl -X GET "https://api.binance.com/api/v3/account" \
  -H "X-MBX-APIKEY: YOUR_API_KEY"

# 查看详细错误
docker-compose logs barter-strategy | grep ERROR
```

#### 数据不更新
```bash
# 检查WebSocket连接
docker-compose exec barter-strategy netstat -an | grep 9443

# 重启数据采集
docker-compose restart barter-strategy

# 清理缓存
docker-compose exec redis redis-cli FLUSHALL
```

#### 高延迟问题
```bash
# 检查系统资源
docker stats

# 分析瓶颈
docker-compose exec barter-strategy \
  cargo run --bin profiler

# 优化配置
# 编辑 .env 减少 MODEL_BATCH_SIZE
```

## 6. 安全建议

### API密钥管理

1. **使用密钥管理服务**
```bash
# 使用Docker secrets
echo "YOUR_API_KEY" | docker secret create binance_api_key -
echo "YOUR_API_SECRET" | docker secret create binance_api_secret -
```

2. **定期轮换密钥**
```bash
# 创建密钥轮换脚本
cat > scripts/rotate_keys.sh <<'EOF'
#!/bin/bash
# 备份当前配置
cp .env .env.backup.$(date +%Y%m%d)
# 更新密钥
# ... 添加轮换逻辑
EOF
```

3. **监控API使用**
```sql
-- 查询API调用统计
SELECT
  date_trunc('hour', timestamp) as hour,
  exchange,
  count(*) as api_calls
FROM trades
GROUP BY hour, exchange
ORDER BY hour DESC;
```

## 7. 生产环境准备

### 从测试到生产

1. **逐步提升权限**
   - 第1周：只读权限
   - 第2周：模拟交易
   - 第3周：小额真实交易
   - 第4周：正常交易

2. **风险控制**
```yaml
# 生产环境配置
MAX_POSITION_SIZE: 1000  # 从小额开始
MAX_DAILY_LOSS: 0.02      # 2%最大日损失
STOP_LOSS_PCT: 0.01       # 1%止损
```

3. **监控告警**
```bash
# 设置告警
docker-compose exec grafana \
  grafana-cli alerting enable

# 配置通知渠道（Telegram/Email）
```

## 8. 性能基准

### 预期性能指标

| 指标 | 目标值 | 告警阈值 |
|------|--------|----------|
| 信号延迟 | <10ms | >50ms |
| 决策延迟 | <50ms | >200ms |
| 执行延迟 | <100ms | >500ms |
| 错误率 | <0.1% | >1% |
| 可用性 | >99.9% | <99% |

### 压力测试

```bash
# 运行压力测试
docker-compose run --rm barter-strategy \
  cargo bench

# 模拟高频数据
docker-compose run --rm barter-strategy \
  cargo run --example stress_test
```

## 9. 验证报告模板

```markdown
## 系统验证报告

**日期**: 2024-XX-XX
**版本**: v1.0.0
**测试环境**: Docker/Linux

### 测试结果摘要
- ✅ 基础系统: 通过
- ✅ 数据连接: 通过
- ✅ 策略逻辑: 通过
- ⚠️ 性能测试: 部分通过
- ❌ 压力测试: 需要优化

### 详细结果
[添加详细测试结果]

### 建议
[添加改进建议]
```

## 10. 下一步

验证成功后：

1. **优化配置**
   - 调整风险参数
   - 优化模型参数
   - 配置交易对

2. **部署生产**
   - 设置监控告警
   - 配置自动备份
   - 实施灾难恢复

3. **持续改进**
   - 分析交易日志
   - 优化策略
   - 更新模型

## 支持

如遇问题，请查看：
- [GitHub Issues](https://github.com/TaoSeekAI/barter-rs/issues)
- [文档](./docs/)