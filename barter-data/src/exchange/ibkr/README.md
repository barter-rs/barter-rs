# Interactive Brokers as a Barter Exchange

## Development

* Open IBKR account
  * Needs to be IBKR Pro level, not IBKR Lite

* Install Web API v1.0
  * As of August 2024, individual accounts have to use Client Portal Gateway, v1.0 of the API.  That is what this barter-data exchange uses.
  * https://www.interactivebrokers.com/campus/ibkr-api-page/cpapi-v1/
  * requires a JDK
* Make sure port 5000 is free and clear.

      lsof -i:5000

  On Mac, mine was not, so I [disabled Airplay Receiver](https://nono.ma/port-5000-used-by-control-center-in-macos-controlce).
  > tl;dr - `System Preferences -> Airplay Receiver -> Uncheck "AirPlay Receiver"`.
* Start Client Portal Gateway

      bin/run.sh root/conf.yaml

* Login: https://localhost:5000/
  * 2FA with mobile app or SMS or ...

