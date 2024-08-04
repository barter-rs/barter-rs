//
// use fnv::FnvHashMap;
// // Todo:
// //  - Should be able to construct using "global" InstrumentIds, or simply add instruments and
// //    generate an ephemeral / local InstrumentId
// //  - Build this once on startup, then generate market data subscriptions, execution subscriptions
// //  - Move exchange "market"s to barter-integration?
// //    '--> Or new crate "barter-instrument"? or add fetch instrument data in barter-data?
//
// #[derive(Debug)]
// pub struct InstrumentMap<InstrumentData, AssetData> {
//     pub instruments: FnvHashMap<InstrumentId, InstrumentData>,
//     pub assets: FnvHashMap<AssetId, AssetData>,
// }
//
// impl<InstrumentData, AssetData> InstrumentMap<InstrumentData, AssetData> {
//     pub fn new(
//         instruments: impl IntoIterator<Item = KeyedInstrument<InstrumentId, InstrumentData>>,
//         assets: impl IntoIterator<Item = KeyedAsset<AssetId, AssetData>>,
//     ) -> Self {
//         Self {
//             instruments: instruments
//                 .into_iter()
//                 .map(|instrument| (instrument.key, instrument.instrument))
//                 .collect(),
//             assets: assets
//                 .into_iter()
//                 .map(|asset| (asset.key, asset.asset))
//                 .collect(),
//         }
//     }
//
//     pub fn find_instrument(&self, id: &InstrumentId) -> Option<&InstrumentData> {
//         self.instruments.get(id)
//     }
//
//     pub fn find_asset(&self, id: &AssetId) -> Option<&AssetData> {
//         self.assets.get(id)
//     }
// }
//
// #[derive(Debug, Default)]
// pub struct InstrumentMapLocalBuilder {
//     pub instruments: Vec<Instrument>,
//     pub assets: Vec<Asset>,
// }
//
// impl InstrumentMapLocalBuilder {
//     pub fn add<I>(self, instrument: I) -> Self
//     where
//         I: Into<Instrument>,
//     {
//         let instrument = instrument.into();
//         // Todo;
//         self
//     }
//
//     pub fn build(self) -> InstrumentMap<(), ()> {
//         todo!()
//     }
// }
