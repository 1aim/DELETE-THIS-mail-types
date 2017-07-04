use types::shared::Item;
use types::components::data_types;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Unstructured {
    inner: Item,
    //FIXME check if needed
    component_slices: data_types::Unstructured
}