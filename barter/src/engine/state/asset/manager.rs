pub trait AssetStateManager<AssetKey> {
    type State;

    fn asset(&self, key: &AssetKey) -> &Self::State;
    fn asset_mut(&mut self, key: &AssetKey) -> &mut Self::State;
}
