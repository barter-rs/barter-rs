# 交易所数据格式详解

## Binance数据格式

### 1. WebSocket连接

**连接地址:**
- Spot: `wss://stream.binance.com:9443/ws`
- Futures: `wss://fstream.binance.com/ws`

### 2. 订阅格式

```json
{
  "method": "SUBSCRIBE",
  "params": [
    "btcusdt@trade",      // 交易流
    "btcusdt@depth20",    // 订单簿深度
    "btcusdt@aggTrade",   // 聚合交易
    "btcusdt@ticker"      // 24小时ticker
  ],
  "id": 1
}
```

### 3. 数据格式详解

#### 3.1 交易数据 (Trade Stream)

**原始数据:**
```json
{
  "e": "trade",          // 事件类型
  "E": 1638360000000,    // 事件时间
  "s": "BTCUSDT",        // 交易对
  "t": 123456789,        // 交易ID
  "p": "50000.00",       // 价格
  "q": "0.001000",       // 数量
  "b": 88888888,         // 买方订单ID
  "a": 99999999,         // 卖方订单ID
  "T": 1638360000000,    // 交易时间
  "m": true,             // 是否是买方挂单
  "M": true              // 是否可忽略
}
```

**Rust结构体:**
```rust
#[derive(Debug, Deserialize)]
pub struct BinanceTrade {
    #[serde(rename = "s")]
    pub symbol: String,
    #[serde(rename = "p")]
    pub price: String,
    #[serde(rename = "q")]
    pub quantity: String,
    #[serde(rename = "T")]
    pub trade_time: i64,
    #[serde(rename = "m")]
    pub is_buyer_maker: bool,
}
```

#### 3.2 订单簿深度 (Order Book)

**L1数据 (最优买卖):**
```json
{
  "e": "bookTicker",
  "u": 400900000,        // 更新ID
  "s": "BTCUSDT",
  "b": "50000.00",       // 最优买价
  "B": "10.00000",       // 最优买量
  "a": "50001.00",       // 最优卖价
  "A": "5.00000"         // 最优卖量
}
```

**L2数据 (深度):**
```json
{
  "e": "depthUpdate",
  "E": 1638360000000,
  "s": "BTCUSDT",
  "U": 157,              // 第一个更新ID
  "u": 160,              // 最后一个更新ID
  "b": [                 // 买单
    ["50000.00", "1.0"],
    ["49999.00", "2.0"]
  ],
  "a": [                 // 卖单
    ["50001.00", "1.5"],
    ["50002.00", "3.0"]
  ]
}
```

#### 3.3 K线数据

```json
{
  "e": "kline",
  "E": 1638360000000,
  "s": "BTCUSDT",
  "k": {
    "t": 1638360000000,   // 开始时间
    "T": 1638360059999,   // 结束时间
    "s": "BTCUSDT",
    "i": "1m",            // 时间间隔
    "f": 100,             // 第一笔交易ID
    "L": 200,             // 最后一笔交易ID
    "o": "50000.00",      // 开盘价
    "c": "50100.00",      // 收盘价
    "h": "50200.00",      // 最高价
    "l": "49900.00",      // 最低价
    "v": "1000.00000",    // 成交量
    "n": 100,             // 成交笔数
    "x": false,           // K线是否完结
    "q": "50000000.00",   // 成交额
    "V": "500.00000",     // 主动买入成交量
    "Q": "25000000.00"    // 主动买入成交额
  }
}
```

### 4. 永续合约特殊数据

#### 4.1 资金费率

```json
{
  "e": "markPriceUpdate",
  "E": 1638360000000,
  "s": "BTCUSDT",
  "p": "50000.00000000",   // 标记价格
  "i": "50000.00000000",   // 指数价格
  "P": "50100.00000000",   // 预估结算价格
  "r": "0.00010000",       // 资金费率
  "T": 1638368000000       // 下次资金费率时间
}
```

#### 4.2 强平订单

```json
{
  "e": "forceOrder",
  "E": 1638360000000,
  "o": {
    "s": "BTCUSDT",
    "S": "SELL",           // 方向
    "o": "LIMIT",          // 订单类型
    "f": "IOC",            // 有效方式
    "q": "1.000",          // 数量
    "p": "49000.00",       // 价格
    "ap": "49500.00",      // 平均价格
    "X": "FILLED",         // 状态
    "l": "1.000",          // 最后成交量
    "z": "1.000",          // 累计成交量
    "T": 1638360000000     // 交易时间
  }
}
```

## OKX数据格式

### 1. WebSocket连接

**连接地址:**
- Public: `wss://ws.okx.com:8443/ws/v5/public`
- Private: `wss://ws.okx.com:8443/ws/v5/private`

### 2. 订阅格式

```json
{
  "op": "subscribe",
  "args": [
    {
      "channel": "trades",
      "instId": "BTC-USDT"
    },
    {
      "channel": "books",
      "instId": "BTC-USDT"
    },
    {
      "channel": "tickers",
      "instId": "BTC-USDT"
    }
  ]
}
```

### 3. 数据格式详解

#### 3.1 交易数据

```json
{
  "arg": {
    "channel": "trades",
    "instId": "BTC-USDT"
  },
  "data": [
    {
      "instId": "BTC-USDT",
      "tradeId": "242720720",
      "px": "50000.0",        // 价格
      "sz": "0.1",            // 数量
      "side": "buy",          // 方向
      "ts": "1638360000000",  // 时间戳
      "count": "1"            // 成交笔数
    }
  ]
}
```

#### 3.2 订单簿数据

```json
{
  "arg": {
    "channel": "books",
    "instId": "BTC-USDT"
  },
  "action": "snapshot",       // snapshot/update
  "data": [
    {
      "asks": [               // 卖单
        ["50001.0", "1.5", "0", "2"],
        ["50002.0", "3.0", "0", "1"]
      ],
      "bids": [               // 买单
        ["50000.0", "2.0", "0", "3"],
        ["49999.0", "5.0", "0", "2"]
      ],
      "ts": "1638360000000",
      "checksum": -1234567
    }
  ]
}
```

#### 3.3 Ticker数据

```json
{
  "arg": {
    "channel": "tickers",
    "instId": "BTC-USDT"
  },
  "data": [
    {
      "instType": "SWAP",
      "instId": "BTC-USDT-SWAP",
      "last": "50000.0",      // 最新价
      "lastSz": "0.1",        // 最新成交量
      "askPx": "50001.0",     // 卖一价
      "askSz": "10",          // 卖一量
      "bidPx": "50000.0",     // 买一价
      "bidSz": "20",          // 买一量
      "open24h": "49000.0",   // 24小时开盘价
      "high24h": "51000.0",   // 24小时最高价
      "low24h": "48000.0",    // 24小时最低价
      "volCcy24h": "1000000", // 24小时成交额
      "vol24h": "20.5",       // 24小时成交量
      "ts": "1638360000000",
      "sodUtc0": "49500.0",   // UTC0时开盘价
      "sodUtc8": "49600.0"    // UTC8时开盘价
    }
  ]
}
```

### 4. 永续合约特殊数据

#### 4.1 资金费率

```json
{
  "arg": {
    "channel": "funding-rate",
    "instId": "BTC-USDT-SWAP"
  },
  "data": [
    {
      "instType": "SWAP",
      "instId": "BTC-USDT-SWAP",
      "fundingRate": "0.0001",     // 当前资金费率
      "nextFundingRate": "0.00015", // 下期资金费率
      "fundingTime": "1638368000000", // 结算时间
      "nextFundingTime": "1638396800000"
    }
  ]
}
```

#### 4.2 标记价格

```json
{
  "arg": {
    "channel": "mark-price",
    "instId": "BTC-USDT-SWAP"
  },
  "data": [
    {
      "instType": "SWAP",
      "instId": "BTC-USDT-SWAP",
      "markPx": "50000.0",     // 标记价格
      "ts": "1638360000000"
    }
  ]
}
```

#### 4.3 持仓数据

```json
{
  "arg": {
    "channel": "positions",
    "instType": "SWAP"
  },
  "data": [
    {
      "instType": "SWAP",
      "instId": "BTC-USDT-SWAP",
      "posId": "1234567890",
      "posSide": "long",       // 持仓方向
      "pos": "10",             // 持仓数量
      "avgPx": "49500.0",      // 开仓均价
      "upl": "5000.0",         // 未实现盈亏
      "uplRatio": "0.1",       // 未实现盈亏率
      "lever": "10",           // 杠杆倍数
      "liqPx": "45000.0",      // 预估强平价
      "imr": "500.0",          // 初始保证金
      "margin": "500.0",       // 保证金
      "mgnRatio": "0.2",       // 保证金率
      "mmr": "50.0",           // 维持保证金
      "cTime": "1638350000000",
      "uTime": "1638360000000"
    }
  ]
}
```

## 数据标准化

### 1. 统一数据模型

```rust
// 统一交易数据
pub struct UnifiedTrade {
    pub exchange: Exchange,
    pub symbol: String,
    pub price: Decimal,
    pub amount: Decimal,
    pub side: TradeSide,
    pub timestamp: DateTime<Utc>,
    pub trade_id: String,
}

// 统一订单簿
pub struct UnifiedOrderBook {
    pub exchange: Exchange,
    pub symbol: String,
    pub bids: Vec<PriceLevel>,
    pub asks: Vec<PriceLevel>,
    pub timestamp: DateTime<Utc>,
    pub sequence: Option<u64>,
}

// 价格档位
pub struct PriceLevel {
    pub price: Decimal,
    pub amount: Decimal,
    pub count: Option<u32>,
}
```

### 2. 转换函数

```rust
impl From<BinanceTrade> for UnifiedTrade {
    fn from(trade: BinanceTrade) -> Self {
        UnifiedTrade {
            exchange: Exchange::Binance,
            symbol: trade.symbol,
            price: Decimal::from_str(&trade.price).unwrap(),
            amount: Decimal::from_str(&trade.quantity).unwrap(),
            side: if trade.is_buyer_maker {
                TradeSide::Sell
            } else {
                TradeSide::Buy
            },
            timestamp: DateTime::from_timestamp(trade.trade_time / 1000, 0).unwrap(),
            trade_id: trade.trade_id.to_string(),
        }
    }
}

impl From<OkxTrade> for UnifiedTrade {
    fn from(trade: OkxTrade) -> Self {
        UnifiedTrade {
            exchange: Exchange::Okx,
            symbol: trade.inst_id,
            price: Decimal::from_str(&trade.px).unwrap(),
            amount: Decimal::from_str(&trade.sz).unwrap(),
            side: match trade.side.as_str() {
                "buy" => TradeSide::Buy,
                "sell" => TradeSide::Sell,
                _ => TradeSide::Unknown,
            },
            timestamp: DateTime::from_timestamp(
                trade.ts.parse::<i64>().unwrap() / 1000,
                0
            ).unwrap(),
            trade_id: trade.trade_id,
        }
    }
}
```

## 性能优化建议

### 1. 数据压缩
- 使用二进制格式传输内部数据
- 压缩历史数据存储
- 使用增量更新减少带宽

### 2. 缓存策略
- 订单簿本地缓存
- 热点数据内存缓存
- 使用Redis做分布式缓存

### 3. 批处理
- 批量处理交易数据
- 聚合小额交易
- 定时批量更新指标

### 4. 并发处理
- 多线程处理不同交易对
- 异步IO处理网络请求
- 使用Channel进行线程间通信