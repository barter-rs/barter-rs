# barter-rs

Rust library containing high-performance trading engine & back-testing components.

## Todo Features:
- 'Statistics' portfolio component that keeps running totals of important stats - expose via api?

## Tech Debt:
1. Decide if I want to go full abstraction mode & have traits for all w/ generics (read 10.2 & 19.2 again)
2. Clean up Allocator allocate_order() method & make sure unwraps are sorted! -> return Result?
3. Test both method a & b in Portfolio.UpdateFromFill to see if they are equal!
4. Clean up differences in constructors, some using struct components & some using struct Config
4. Clean up serde parser w/ extracted methods, testing, cleaner solution, etc
5. Should I add a logger for internal lib logging? Or just useful errors?
6. Impl sugar methods for builders to allow passing &str instead of just String


Todo Now:
- Check inconsistency between Position & Component traits / method names eg/ PositionEnterer vs OrderGenerator.
- Clean up access modifiers as I go along.
- Think if a good way to deal with builder.build().unwrap -> currently in data & strategy trait methods...
