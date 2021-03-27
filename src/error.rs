use thiserror::Error;

#[derive(Error, Debug)]
pub enum BarterError {
    #[error("Failed to build struct due to incomplete attributes provided")]
    BuilderIncomplete(),

    #[error("Provided builder attributes are invalid")]
    BuilderAttributesInvalid(),
}