# Jackbot Sensor
Based on Barter is an algorithmic trading ecosystem of Rust libraries for building high-performance live-trading, paper-trading 
and back-testing systems.
* **Fast**: Written in native Rust. Minimal allocations. Data-oriented state management system with direct index lookups.
* **Robust**: Strongly typed. Thread safe. Extensive test coverage.
* **Customisable**: Plug and play Strategy and RiskManager components that facilitates most trading strategies (MarketMaking, StatArb, HFT, etc.).
* **Scalable**: Multithreaded architecture with modular design. Leverages Tokio for I/O. Memory efficient data structures.  

I expands Barter to support the exchanges of the Jackbot Terminal project:
* Binance (Great reference implementation on Barter)
* Bitget
* Bybit 
* Coinbase
* Kraken
* Kucoin
* OKX

## Overview
Jackbot Sensor is an algorithmic trading ecosystem of Rust libraries for building high-performance live-trading, paper-trading 
and back-testing systems. It is made up of several easy-to-use, extensible crates:
* **Barter**: Algorithmic trading Engine with feature rich state management system.
* **Barter-Instrument**: Exchange, Instrument and Asset data structures and utilities. 
* **Barter-Data**: Stream public market data from financial venues. Easily extensible via the MarketStream interface.
* **Barter-Execution**: Stream private account data and execute orders. Easily extensible via the ExecutionClient interface. 
* **Barter-Integration**: Low-level frameworks for flexible REST/WebSocket integrations.

## Notable Features
- Stream public market data from financial venues via the [`Barter-Data`] library. 
- Stream private account data, execute orders (live or mock)** via the [`Barter-Execution`] library.
- Plug and play Strategy and RiskManager components that facilitate most trading strategies. 
- Flexible Engine that facilitates trading strategies that execute on many exchanges simultaneously.
- Use mock MarketStream or Execution components to enable back-testing on a near-identical trading system as live-trading.  
- Centralised cache friendly state management system with O(1) constant lookups using indexed data structures.
- Robust Order management system
- Trading summaries with comprehensive performance metrics (PnL, Sharpe, Sortino, Drawdown, etc.).
- Turn on/off algorithmic trading from an external process (eg/ UI, Telegram, etc.) whilst still processing market/account data. 
- Issue Engine Commands from an external process (eg/ UI, Telegram, etc.) to initiate actions (CloseAllPositions, OpenOrders, CancelOrders, etc.).
- EngineState replica manager that processes the Engine AuditStream to facilitate non-hot path monitoring components (eg/ UI, Telegram, etc.).
- S3 data harvesting using parquet + iceberg for preserving data for later utilziation to build even better algos.
- Jackpot orderbook representation. Composed of a special kind of order sent from (UI, Telegram, etc.) that is not placeble in the current exchange orderbook because it is too out of money.

## Getting Help
Reach out via mail@jackbot.app

## Jackbot Sensor is Open Source 
Jackbot Terminal sensors are opensource to build trust with users. Here they can atest the code executing their order on the cloud.

## Contributing
If you use Jackbot Terminal and is a coder and want more exchange support or a new exchange that want exposure in the terminal send out a pull request and we are happy to integrate you into the project.

### Licence
This project is licensed under the MIT license.

### Contribution License Agreement

Any contribution you intentionally submit for inclusion in Jackbot Tterminal shall be:
1. Licensed under MIT
2. Subject to all disclaimers and limitations of liability stated below
3. Provided without any additional terms or conditions
4. Submitted with the understanding that the risk warnings apply and you're the sole responsible of the usage of this sensors loss made are yours and no one elses but also are the profits.

By submitting a contribution, you certify that you have the right to do so under these terms.

## LEGAL DISCLAIMER AND LIMITATION OF LIABILITY

PLEASE READ THIS DISCLAIMER CAREFULLY BEFORE USING THE SOFTWARE. BY ACCESSING OR USING THE SOFTWARE, YOU ACKNOWLEDGE AND AGREE TO BE BOUND BY THE TERMS HEREIN.

1. NO FINANCIAL ADVICE
   Nothing contained in the Software constitutes financial, investment, legal, or tax advice. No aspect of the Software should be relied upon for trading decisions or financial planning. Users are strongly advised to consult qualified professionals for investment guidance specific to their circumstances.

2. ASSUMPTION OF RISK
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
