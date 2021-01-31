#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use frame_support::{decl_module, decl_storage};
use sp_runtime::RuntimeDebug;
use sp_std::prelude::Vec;
use frame_system::{self as system};

use pallet_utils::{StorefrontId, WhoAndWhen};
use pallet_storefronts::{Storefront, StorefrontUpdate, AfterStorefrontUpdated};

#[derive(Encode, Decode, Clone, Eq, PartialEq, RuntimeDebug)]
pub struct StorefrontHistoryRecord<T: Trait> {
    pub edited: WhoAndWhen<T>,
    pub old_data: StorefrontUpdate,
}

/// The pallet's configuration trait.
pub trait Trait: system::Trait
    + pallet_storefronts::Trait
    + pallet_utils::Trait
{}

// This pallet's storage items.
decl_storage! {
    trait Store for Module<T: Trait> as StorefrontHistoryModule {
        pub EditHistory get(fn edit_history):
            map hasher(twox_64_concat) StorefrontId => Vec<StorefrontHistoryRecord<T>>;
    }
}

// The pallet's dispatchable functions.
decl_module! {
  pub struct Module<T: Trait> for enum Call where origin: T::Origin {}
}

impl<T: Trait> StorefrontHistoryRecord<T> {
    fn new(updated_by: T::AccountId, old_data: StorefrontUpdate) -> Self {
        StorefrontHistoryRecord {
            edited: WhoAndWhen::<T>::new(updated_by),
            old_data
        }
    }
}

impl<T: Trait> AfterStorefrontUpdated<T> for Module<T> {
    fn after_storefront_updated(sender: T::AccountId, storefront: &Storefront<T>, old_data: StorefrontUpdate) {
        <EditHistory<T>>::mutate(storefront.id, |ids|
            ids.push(StorefrontHistoryRecord::<T>::new(sender, old_data)));
    }
}
