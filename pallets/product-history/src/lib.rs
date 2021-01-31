#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use frame_support::{decl_module, decl_storage};
use sp_runtime::RuntimeDebug;
use sp_std::prelude::Vec;
use frame_system::{self as system};

use pallet_products::{ProductId, Product, ProductUpdate, AfterProductUpdated};
use pallet_utils::WhoAndWhen;

#[derive(Encode, Decode, Clone, Eq, PartialEq, RuntimeDebug)]
pub struct ProductHistoryRecord<T: Trait> {
    pub edited: WhoAndWhen<T>,
    pub old_data: ProductUpdate,
}

/// The pallet's configuration trait.
pub trait Trait: system::Trait
    + pallet_utils::Trait
    + pallet_products::Trait
{}

// This pallet's storage items.
decl_storage! {
    trait Store for Module<T: Trait> as ProductHistoryModule {
        pub EditHistory get(fn edit_history):
            map hasher(twox_64_concat) ProductId => Vec<ProductHistoryRecord<T>>;
    }
}

decl_module! {
  pub struct Module<T: Trait> for enum Call where origin: T::Origin {}
}

impl<T: Trait> ProductHistoryRecord<T> {
    fn new(updated_by: T::AccountId, old_data: ProductUpdate) -> Self {
        ProductHistoryRecord {
            edited: WhoAndWhen::<T>::new(updated_by),
            old_data
        }
    }
}

impl<T: Trait> AfterProductUpdated<T> for Module<T> {
    fn after_product_updated(sender: T::AccountId, product: &Product<T>, old_data: ProductUpdate) {
        <EditHistory<T>>::mutate(product.id, |ids|
            ids.push(ProductHistoryRecord::<T>::new(sender, old_data)));
    }
}
