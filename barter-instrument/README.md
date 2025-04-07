# Barter-Instrument
Barter-Instrument provides core data structures and utilities for handling exchanges, instruments, and assets.
These abstractions provide a structured and intuitive way to organize exchanges, assets, and instruments, ensuring they are represented in a way that best fits their characteristics and usage needs.

The Barter-Instrument crate also includes IndexedInstruments, a structured collection of exchanges, assets, and instruments designed for fast lookups. 

**See: [`Barter`], [`Barter-Data`], [`Barter-Execution`] & [`Barter-Integration`] for
comprehensive documentation of other Barter libraries.**

[![Crates.io][crates-badge]][crates-url]
[![MIT licensed][mit-badge]][mit-url]
[![Discord chat][discord-badge]][discord-url]

[crates-badge]: https://img.shields.io/crates/v/barter.svg
[crates-url]: https://crates.io/crates/barter

[mit-badge]: https://img.shields.io/badge/license-MIT-blue.svg
[mit-url]: https://github.com/barter-rs/barter-rs/blob/develop/LICENSE

[discord-badge]: https://img.shields.io/discord/910237311332151317.svg?logo=discord&style=flat-square
[discord-url]: https://discord.gg/wE7RqhnQMV

[`Barter`]: https://crates.io/crates/barter
[`Barter-Data`]: https://crates.io/crates/barter-data
[`Barter-Execution`]: https://crates.io/crates/barter-execution
[`Barter-Integration`]: https://crates.io/crates/barter-integration
[API Documentation]: https://docs.rs/barter/latest/barter/
[Chat]: https://discord.gg/wE7RqhnQMV

## Overview  

**Barter-Instrument** provides the core data structures and utilities needed to model **exchanges, instruments, and assets**, ensuring a structured and efficient way to represent and interact with them.  

### Asset  
Utilities for representing assets in a flexible manner.  

### Instrument  
A comprehensive instrument model that includes all the necessary data for **subscribing to market data** and **generating correct orders**.  

### Exchange  
A unique identifier representing an execution server.  

### IndexedInstruments  
A structured collection of **exchanges, assets, and instruments** optimized for **fast lookups**. By assigning unique identifiers to each entity, it enables **efficient, constant-time (`O(1)`) retrieval**, making it a critical component in other **Barter-related crates** that require quick access to financial data. This indexing system **reduces computational overhead** and enhances the organization of financial data.

## Examples

#### Volume Collector using Indexed Instruments:
```rust,no_run
use barter_instrument::{
    Keyed, Underlying,
    asset::{Asset, AssetIndex, ExchangeAsset, name::AssetNameInternal},
    exchange::{ExchangeId, ExchangeIndex},
    index::IndexedInstruments,
    instrument::{Instrument, InstrumentIndex, name::InstrumentNameInternal},
};

use rust_decimal::{Decimal, prelude::Zero};

type VolumeSum = Decimal;
type FnvIndexMap<K, V> = indexmap::IndexMap<K, V, fnv::FnvBuildHasher>;

#[derive(Debug, Clone, PartialEq)]
struct VolumeCollector<K, V> {
    key: K,
    value: V,
    sum_of_volume: VolumeSum,
}

impl<K, V> VolumeCollector<K, V> {
    pub fn new(key: K, value: V) -> Self {
        Self {
            key,
            value,
            sum_of_volume: Decimal::zero(),
        }
    }
}

type InstrumentVolumeCollector =
    VolumeCollector<InstrumentIndex, Instrument<Keyed<ExchangeIndex, ExchangeId>, AssetIndex>>;
type ExchangeVolumeCollector = VolumeCollector<ExchangeIndex, ExchangeId>;
type AssetVolumeCollector = VolumeCollector<AssetIndex, ExchangeAsset<Asset>>;

fn generate_instrument_volume_collectors(
    indexed_instruments: &IndexedInstruments,
) -> FnvIndexMap<InstrumentNameInternal, InstrumentVolumeCollector> {
    indexed_instruments
        .instruments()
        .iter()
        .map(|instrument| {
            (
                instrument.value.name_internal.clone(),
                InstrumentVolumeCollector::new(instrument.key, instrument.value.clone()),
            )
        })
        .collect()
}

fn generate_asset_volume_collectors(
    indexed_instruments: &IndexedInstruments,
) -> FnvIndexMap<ExchangeAsset<AssetNameInternal>, AssetVolumeCollector> {
    indexed_instruments
        .assets()
        .iter()
        .map(|asset| {
            (
                ExchangeAsset::new(
                    asset.value.exchange,
                    asset.value.asset.name_internal.clone(),
                ),
                AssetVolumeCollector::new(asset.key, asset.value.clone()),
            )
        })
        .collect()
}

fn generate_exchange_volume_collectors(
    indexed_instruments: &IndexedInstruments,
) -> FnvIndexMap<ExchangeId, ExchangeVolumeCollector> {
    indexed_instruments
        .exchanges()
        .iter()
        .map(|exchange| {
            (
                exchange.value,
                ExchangeVolumeCollector::new(exchange.key, exchange.value),
            )
        })
        .collect()
}

#[derive(Default)]

struct TradeVolumeCollector {
    instrument_volumes: FnvIndexMap<InstrumentNameInternal, InstrumentVolumeCollector>,
    asset_volumes: FnvIndexMap<ExchangeAsset<AssetNameInternal>, AssetVolumeCollector>,
    exchange_volumes: FnvIndexMap<ExchangeId, ExchangeVolumeCollector>,
}

impl TradeVolumeCollector {
    pub fn new(indexed_instruments: &IndexedInstruments) -> Self {
        Self {
            instrument_volumes: generate_instrument_volume_collectors(indexed_instruments),
            asset_volumes: generate_asset_volume_collectors(indexed_instruments),
            exchange_volumes: generate_exchange_volume_collectors(indexed_instruments),
        }
    }

    pub fn collect_trade(
        &mut self,
        instrument_index: &InstrumentIndex,
        exchange_index: &ExchangeIndex,
        base_asset_index: &AssetIndex,
        quote_asset_index: &AssetIndex,
        volume: f64,
    ) {
        // Update instrument volume collector using instrument_index O(1)
        let instrument_collector = self
            .instrument_volumes
            .get_index_mut(instrument_index.index())
            .map(|(_key, state)| state)
            .unwrap_or_else(|| {
                panic!("InstrumentVolumes does not contain: {:?}", instrument_index)
            });
        instrument_collector.sum_of_volume += Decimal::try_from(volume).unwrap();

        // Update exchange volume collector using exchange_index O(1)
        let exchange_collector = self
            .exchange_volumes
            .get_index_mut(exchange_index.index())
            .map(|(_key, state)| state)
            .unwrap_or_else(|| panic!("ExchangeVolumes does not contain: {:?}", exchange_index));
        exchange_collector.sum_of_volume += Decimal::try_from(volume).unwrap();

        // Update asset volume collector using asset_index O(1)
        let asset_collector = self
            .asset_volumes
            .get_index_mut(base_asset_index.index())
            .map(|(_key, state)| state)
            .unwrap_or_else(|| panic!("AssetVolumes does not contain: {:?}", base_asset_index));
        asset_collector.sum_of_volume += Decimal::try_from(volume).unwrap();

        // Update asset volume collector using asset_index O(1)
        let asset_collector = self
            .asset_volumes
            .get_index_mut(quote_asset_index.index())
            .map(|(_key, state)| state)
            .unwrap_or_else(|| panic!("AssetVolumes does not contain: {:?}", quote_asset_index));
        asset_collector.sum_of_volume += Decimal::try_from(volume).unwrap();
    }

    pub fn trade_summary(&self) {
        println!("--- Trade Volume Summary ---");

        println!("\n📌 Per Instrument:");
        for (name, collector) in &self.instrument_volumes {
            println!(
                "Instrument: {}, Total Volume in USD: {}",
                name, collector.sum_of_volume
            );
        }

        println!("\n📌 Per Asset:");
        for (asset, collector) in &self.asset_volumes {
            println!(
                "Asset: {} on Exchange: {}, Total Volume in USD: {}",
                asset.asset, asset.exchange, collector.sum_of_volume
            );
        }

        println!("\n📌 Per Exchange:");
        for (exchange, collector) in &self.exchange_volumes {
            println!(
                "Exchange: {}, Total Volume in USD: {}",
                exchange, collector.sum_of_volume
            );
        }

        println!("----------------------------");
    }
}

fn main() {
    // Creating Indexed Instruments with the Indexed Instruments Builder
    let indexed_instruments = IndexedInstruments::builder()
        .add_instrument(Instrument::spot(
            ExchangeId::BinanceSpot,
            "binance_spot_btc_usdt",
            "BTCUSDT",
            Underlying::new("btc", "usdt"),
            None,
        ))
        .add_instrument(Instrument::spot(
            ExchangeId::BinanceSpot,
            "binance_spot_eth_usdt",
            "ETHUSDT",
            Underlying::new("eth", "usdt"),
            None,
        ))
        .add_instrument(Instrument::spot(
            ExchangeId::Coinbase,
            "coinbase_btc_usdt",
            "BTCUSDT",
            Underlying::new("btc", "usdt"),
            None,
        ))
        .build();

    // The Trade Volume Collector is built using IndexedInstruments, which contains
    // all the instruments, exchanges, and assets used throughout the system's lifetime.
    // Each property (instrument, exchange, or asset) has a unique corresponding ID
    // that matches its position in a vector of the respective elements.
    //
    // During construction, the Trade Volume Collector consumes the IndexedInstruments
    // and creates individual collectors for each exchange, instrument, and asset.
    // These collectors are then stored in an IndexedMap, where each property's ID
    // directly corresponds to its position in the respective IndexedMap.

    let mut traded_volume_collector = TradeVolumeCollector::new(&indexed_instruments);

    // Extracting the IDs of each property from the IndexedInstruments collection.
    // These IDs enable O(1) lookups within the system, allowing components
    // to efficiently retrieve the corresponding properties when initialized
    // with IndexedInstruments.
    let binance_spot_btc_usdt_index = indexed_instruments
        .find_instrument_index(
            ExchangeId::BinanceSpot,
            &InstrumentNameInternal::from("binance_spot_btc_usdt"),
        )
        .unwrap();
    let binance_spot_eth_usdt_index = indexed_instruments
        .find_instrument_index(
            ExchangeId::BinanceSpot,
            &InstrumentNameInternal::from("binance_spot_eth_usdt"),
        )
        .unwrap();
    let coinbase_spot_btc_usdt_index = indexed_instruments
        .find_instrument_index(
            ExchangeId::Coinbase,
            &InstrumentNameInternal::from("coinbase_btc_usdt"),
        )
        .unwrap();

    let binance_spot_exchange_index = indexed_instruments
        .find_exchange_index(ExchangeId::BinanceSpot)
        .unwrap();

    let coinbase_exchange_index = indexed_instruments
        .find_exchange_index(ExchangeId::Coinbase)
        .unwrap();

    let btc_asset_binance_spot_index = indexed_instruments
        .find_asset_index(ExchangeId::BinanceSpot, &AssetNameInternal::from("BTC"))
        .unwrap();
    let usdt_asset_binance_spot_index = indexed_instruments
        .find_asset_index(ExchangeId::BinanceSpot, &AssetNameInternal::from("USDT"))
        .unwrap();
    let eth_asset_binance_spot_index = indexed_instruments
        .find_asset_index(ExchangeId::BinanceSpot, &AssetNameInternal::from("ETH"))
        .unwrap();
    let btc_asset_coinbase_index = indexed_instruments
        .find_asset_index(ExchangeId::Coinbase, &AssetNameInternal::from("BTC"))
        .unwrap();
    let usdt_asset_coinbase_index = indexed_instruments
        .find_asset_index(ExchangeId::Coinbase, &AssetNameInternal::from("USDT"))
        .unwrap();

    // Using the extracted IDs to verify their consistency with those in the Trade Volume Collector.
    // Generating a few trades to update the Trade Volume Collector's state, ensuring that all updates
    // remain O(1).
    //
    // This is the most critical aspect: throughout the system's lifetime, the indexes must remain stable
    // to leverage IndexedInstruments and O(1) lookups.
    //
    // If new instruments are dynamically allocated, IndexedInstruments can no longer be used,
    // and traditional hash maps must be employed instead.

    traded_volume_collector.collect_trade(
        &binance_spot_btc_usdt_index,
        &binance_spot_exchange_index,
        &btc_asset_binance_spot_index,
        &usdt_asset_binance_spot_index,
        50.0,
    );
    traded_volume_collector.collect_trade(
        &binance_spot_btc_usdt_index,
        &binance_spot_exchange_index,
        &btc_asset_binance_spot_index,
        &usdt_asset_binance_spot_index,
        30.0,
    );
    traded_volume_collector.collect_trade(
        &binance_spot_eth_usdt_index,
        &binance_spot_exchange_index,
        &eth_asset_binance_spot_index,
        &usdt_asset_binance_spot_index,
        70.0,
    );
    traded_volume_collector.collect_trade(
        &coinbase_spot_btc_usdt_index,
        &coinbase_exchange_index,
        &btc_asset_coinbase_index,
        &usdt_asset_coinbase_index,
        20.0,
    );
    traded_volume_collector.collect_trade(
        &coinbase_spot_btc_usdt_index,
        &coinbase_exchange_index,
        &btc_asset_coinbase_index,
        &usdt_asset_coinbase_index,
        65.0,
    );

    //Displaying the trading summary
    traded_volume_collector.trade_summary();
}
```
[barter-examples]: https://github.com/barter-rs/barter-rs/tree/develop/barter/examples
**For a larger, "real world" example, see [here][barter-examples].**

## Getting Help
Firstly, see if the answer to your question can be found in the [API Documentation]. If the answer is not there, I'd be happy to help via [Chat]
and try answer your question via Discord.

## Support Barter Development
Help us advance Barter's capabilities by becoming a sponsor (or supporting me with a tip!).

Your contribution will allow me to dedicate more time to Barter, accelerating feature development and improvements.

**Please email *justastream.code@gmail.com* for all inquiries**

Please see [here](../README.md#support-barter-development) for more information.

## Contributing
Thanks in advance for helping to develop the Barter ecosystem! Please do get hesitate to get touch via the Discord [Chat] to discuss development,
new features, and the future roadmap.

### Licence
This project is licensed under the [MIT license].

[MIT license]: https://github.com/barter-rs/barter-rs/blob/develop/LICENSE

### Contribution License Agreement

Any contribution you intentionally submit for inclusion in Barter workspace crates shall be:
1. Licensed under MIT
2. Subject to all disclaimers and limitations of liability stated below
3. Provided without any additional terms or conditions
4. Submitted with the understanding that the educational-only purpose and risk warnings apply

By submitting a contribution, you certify that you have the right to do so under these terms.

## LEGAL DISCLAIMER AND LIMITATION OF LIABILITY

PLEASE READ THIS DISCLAIMER CAREFULLY BEFORE USING THE SOFTWARE. BY ACCESSING OR USING THE SOFTWARE, YOU ACKNOWLEDGE AND AGREE TO BE BOUND BY THE TERMS HEREIN.

1. EDUCATIONAL PURPOSE
   This software and related documentation ("Software") are provided solely for educational and research purposes. The Software is not intended, designed, tested, verified or certified for commercial deployment, live trading, or production use of any kind.

2. NO FINANCIAL ADVICE
   Nothing contained in the Software constitutes financial, investment, legal, or tax advice. No aspect of the Software should be relied upon for trading decisions or financial planning. Users are strongly advised to consult qualified professionals for investment guidance specific to their circumstances.

3. ASSUMPTION OF RISK
   Trading in financial markets, including but not limited to cryptocurrencies, securities, derivatives, and other financial instruments, carries substantial risk of loss. Users acknowledge that:
   a) They may lose their entire investment;
   b) Past performance does not indicate future results;
   c) Hypothetical or simulated performance results have inherent limitations and biases.

4. DISCLAIMER OF WARRANTIES
   THE SOFTWARE IS PROVIDED "AS IS" WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED. TO THE MAXIMUM EXTENT PERMITTED BY LAW, THE AUTHORS AND COPYRIGHT HOLDERS EXPRESSLY DISCLAIM ALL WARRANTIES, INCLUDING BUT NOT LIMITED TO:
   a) MERCHANTABILITY
   b) FITNESS FOR A PARTICULAR PURPOSE
   c) NON-INFRINGEMENT
   d) ACCURACY OR RELIABILITY OF RESULTS
   e) SYSTEM INTEGRATION
   f) QUIET ENJOYMENT

5. LIMITATION OF LIABILITY
   IN NO EVENT SHALL THE AUTHORS, COPYRIGHT HOLDERS, CONTRIBUTORS, OR ANY AFFILIATED PARTIES BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING BUT NOT LIMITED TO PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES, LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.

6. REGULATORY COMPLIANCE
   The Software is not registered with, endorsed by, or approved by any financial regulatory authority. Users are solely responsible for:
   a) Determining whether their use complies with applicable laws and regulations
   b) Obtaining any required licenses, permits, or registrations
   c) Meeting any regulatory obligations in their jurisdiction

7. INDEMNIFICATION
   Users agree to indemnify, defend, and hold harmless the authors, copyright holders, and any affiliated parties from and against any claims, liabilities, damages, losses, and expenses arising from their use of the Software.

8. ACKNOWLEDGMENT
   BY USING THE SOFTWARE, USERS ACKNOWLEDGE THAT THEY HAVE READ THIS DISCLAIMER, UNDERSTOOD IT, AND AGREE TO BE BOUND BY ITS TERMS AND CONDITIONS.

THE ABOVE LIMITATIONS MAY NOT APPLY IN JURISDICTIONS THAT DO NOT ALLOW THE EXCLUSION OF CERTAIN WARRANTIES OR LIMITATIONS OF LIABILITY.