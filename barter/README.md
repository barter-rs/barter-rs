# Barter
Barter core is a Rust framework for building high-performance live-trading, paper-trading and back-testing systems.
* **Fast**: Written in native Rust. Minimal allocations. Data-oriented state management system with direct index lookups.
* **Robust**: Strongly typed. Thread safe. Extensive test coverage.
* **Customisable**: Plug and play Strategy and RiskManager components that facilitates most trading strategies (MarketMaking, StatArb, HFT, etc.).
* **Scalable**: Multithreaded architecture with modular design. Leverages Tokio for I/O. Memory efficient data structures.

**See: [`Barter-Data`], [`Barter-Instrument`], [`Barter-Execution`] & [`Barter-Integration`] for
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

[`Barter-Instrument`]: https://crates.io/crates/barter-instrument
[`Barter-Data`]: https://crates.io/crates/barter-data
[`Barter-Execution`]: https://crates.io/crates/barter-execution
[`Barter-Integration`]: https://crates.io/crates/barter-integration
[API Documentation]: https://docs.rs/barter/latest/barter/
[Chat]: https://discord.gg/wE7RqhnQMV

## Overview
Barter core is a Rust framework for building professional grade live-trading, paper-trading and back-testing systems. The
central Engine facilitates executing on many exchanges simultaneously, and offers the flexibility to run most types of
trading strategies.  It allows turning algorithmic order generation on/off and can action Commands issued from external
processes (eg/ CloseAllPositions, OpenOrders, CancelOrders, etc.)

At a high-level, it provides a few major components:
* `Engine` with plug and play `Strategy` and `RiskManager` components.
* Centralised cache friendly `EngineState` management with O(1) constant lookups using indexed data structures.  
* `Strategy` interfaces for customising Engine behavior (AlgoStrategy, ClosePositionsStrategy, OnDisconnectStrategy, etc.).
* `RiskManager` interface for defining custom risk logic which checking generated algorithmic orders.
* Event-driven system that allows for Commands to be issued from external processes (eg/ CloseAllPositions, OpenOrders, CancelOrders, etc.),
  as well as turning algorithmic trading on/off.
* Comprehensive statistics package that provides a summary of key performance metrics (PnL, Sharpe, Sortino, Drawdown, etc.).

[barter-examples]: https://github.com/barter-rs/barter-rs/tree/develop/barter/examples

## Examples
* See [here][barter-examples] for compilable examples including imports.
* See other sub-crates for further examples of each library.

## Getting Help
Firstly, see if the answer to your question can be found in the [API Documentation]. If the answer is not there, I'd be happy to help via [Chat]
and try answer your question via Discord.

## Support Barter Development
Help us advance Barter's capabilities by becoming a sponsor (or supporting me with a tip!).

Your contribution will allow me to dedicate more time to Barter, accelerating feature development and improvements.

**Please email *justastream.code@gmail.com* for all inquiries**

Please see [here](../README.md#support-barter-development) for more information.

## Contributing
Thanks in advance for helping to develop the Barter ecosystem! Please do get hesitate to get touch via the Discord[Chat] to discuss development,
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