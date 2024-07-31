# Interactive Brokers as a Barter Exchange

## Development

* Open IBKR account
  * Needs to be IBKR Pro level, not IBKR Lite

* Install Client API
  * quickstart: https://interactivebrokers.github.io/cpwebapi/quickstart
  * requires JDK
* Make sure port 5000 is free and clear.

      lsof -i:5000

  On Mac, mine was not, so I [disabled Airplay Receiver](https://nono.ma/port-5000-used-by-control-center-in-macos-controlce).
* Login: https://localhost:5000/
  * 2FA with mobile app or SMS or ...

