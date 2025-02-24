use derive_more::{Display, From};
use rand::prelude::IndexedRandom;
use serde::{Deserialize, Serialize};
use smol_str::SmolStr;

#[derive(
    Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize, Display, From,
)]
pub struct ClientOrderId<T = SmolStr>(pub T);

impl ClientOrderId<SmolStr> {
    /// Construct a `ClientOrderId` from the specified string.
    ///
    /// Use [`Self::random`] to generate a random stack-allocated `ClientOrderId`.
    pub fn new<S: Into<SmolStr>>(id: S) -> Self {
        Self(id.into())
    }

    /// Construct a stack-allocated `ClientOrderId` backed by a 23 byte [`SmolStr`].
    pub fn random() -> Self {
        const LEN_URL_SAFE_SYMBOLS: usize = 64;
        const URL_SAFE_SYMBOLS: [char; LEN_URL_SAFE_SYMBOLS] = [
            '_', '-', '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'a', 'b', 'c', 'd', 'e',
            'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's', 't', 'u', 'v',
            'w', 'x', 'y', 'z', 'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M',
            'N', 'O', 'P', 'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z',
        ];
        // SmolStr can be up to 23 bytes long without allocating
        const LEN_NON_ALLOCATING_CID: usize = 23;

        let mut thread_rng = rand::rng();

        let random_utf8: [u8; LEN_NON_ALLOCATING_CID] = std::array::from_fn(|_| {
            let symbol = URL_SAFE_SYMBOLS
                .choose(&mut thread_rng)
                .expect("URL_SAFE_SYMBOLS slice is not empty");

            *symbol as u8
        });

        let random_utf8_str =
            std::str::from_utf8(&random_utf8).expect("URL_SAFE_SYMBOLS are valid utf8");

        Self(SmolStr::new_inline(random_utf8_str))
    }
}

impl Default for ClientOrderId<SmolStr> {
    fn default() -> Self {
        Self::random()
    }
}

#[derive(
    Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize, Display, From,
)]
pub struct OrderId<T = SmolStr>(pub T);

impl OrderId {
    pub fn new<S: AsRef<str>>(id: S) -> Self {
        Self(SmolStr::new(id))
    }
}

#[derive(
    Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize, Display, From,
)]
pub struct StrategyId(pub SmolStr);

impl StrategyId {
    pub fn new<S: AsRef<str>>(id: S) -> Self {
        Self(SmolStr::new(id))
    }

    pub fn unknown() -> Self {
        Self::new("unknown")
    }
}
