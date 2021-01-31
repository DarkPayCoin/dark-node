#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use frame_support::{decl_module, decl_storage};
use sp_runtime::RuntimeDebug;
use sp_std::prelude::Vec;
use frame_system::{self as system};

use pallet_orders::{OrderId, Order, OrderUpdate, AfterOrderUpdated};
use pallet_utils::WhoAndWhen;

#[derive(Encode, Decode, Clone, Eq, PartialEq, RuntimeDebug)]
pub struct OrderHistoryRecord<T: Trait> {
    pub edited: WhoAndWhen<T>,
    pub old_data: OrderUpdate,
}

/// The pallet's configuration trait.
pub trait Trait: system::Trait
    + pallet_utils::Trait
    + pallet_orders::Trait
{}

// This pallet's storage items.
decl_storage! {
    trait Store for Module<T: Trait> as OrderHistoryModule {
        pub EditHistory get(fn edit_history):
            map hasher(twox_64_concat) OrderId => Vec<OrderHistoryRecord<T>>;
    }
}

decl_module! {
  pub struct Module<T: Trait> for enum Call where origin: T::Origin {}
}

impl<T: Trait> OrderHistoryRecord<T> {
    fn new(updated_by: T::AccountId, old_data: OrderUpdate) -> Self {
        OrderHistoryRecord {
            edited: WhoAndWhen::<T>::new(updated_by),
            old_data
        }
    }
}

impl<T: Trait> AfterOrderUpdated<T> for Module<T> {
    fn after_order_updated(sender: T::AccountId, order: &Order<T>, old_data: OrderUpdate) {
        <EditHistory<T>>::mutate(order.id, |ids|
            ids.push(OrderHistoryRecord::<T>::new(sender, old_data)));
    }
}
