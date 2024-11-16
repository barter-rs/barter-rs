use crate::v2::engine::state::asset::AssetState;

pub trait AssetStateManager<AssetKey> {
    fn asset(&self, key: &AssetKey) -> &AssetState;
    fn asset_mut(&mut self, key: &AssetKey) -> &mut AssetState;
}
