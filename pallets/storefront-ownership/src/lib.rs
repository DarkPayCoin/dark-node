#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{
    decl_error, decl_event, decl_module, decl_storage,
    ensure,
    dispatch::DispatchResult,
    traits::Get
};
use sp_std::prelude::*;
use frame_system::{self as system, ensure_signed};

use pallet_storefronts::{Module as Storefronts, StorefrontById, StorefrontIdsByOwner};
use pallet_utils::{StorefrontId, vec_remove_on};

/// The pallet's configuration trait.
pub trait Trait: system::Trait
    + pallet_utils::Trait
    + pallet_storefronts::Trait
{
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

decl_error! {
  pub enum Error for Module<T: Trait> {
    /// The current storefront owner cannot transfer ownership to themself.
    CannotTranferToCurrentOwner,
    /// Account is already an owner of a storefront.
    AlreadyAStorefrontOwner,
    /// There is no pending ownership transfer for a given storefront.
    NoPendingTransferOnStorefront,
    /// Account is not allowed to accept ownership transfer.
    NotAllowedToAcceptOwnershipTransfer,
    /// Account is not allowed to reject ownership transfer.
    NotAllowedToRejectOwnershipTransfer,
  }
}

// This pallet's storage items.
decl_storage! {
    trait Store for Module<T: Trait> as StorefrontOwnershipModule {
        pub PendingStorefrontOwner get(fn pending_storefront_owner):
            map hasher(twox_64_concat) StorefrontId => Option<T::AccountId>;
    }
}

decl_event!(
    pub enum Event<T> where
        <T as system::Trait>::AccountId,
    {
        StorefrontOwnershipTransferCreated(/* current owner */ AccountId, StorefrontId, /* new owner */ AccountId),
        StorefrontOwnershipTransferAccepted(AccountId, StorefrontId),
        StorefrontOwnershipTransferRejected(AccountId, StorefrontId),
    }
);

// The pallet's dispatchable functions.
decl_module! {
  pub struct Module<T: Trait> for enum Call where origin: T::Origin {

    // Initializing errors
    type Error = Error<T>;

    // Initializing events
    fn deposit_event() = default;

    #[weight = 10_000 + T::DbWeight::get().reads_writes(1, 1)]
    pub fn transfer_storefront_ownership(origin, storefront_id: StorefrontId, transfer_to: T::AccountId) -> DispatchResult {
      let who = ensure_signed(origin)?;

      let storefront = Storefronts::<T>::require_storefront(storefront_id)?;
      storefront.ensure_storefront_owner(who.clone())?;

      ensure!(who != transfer_to, Error::<T>::CannotTranferToCurrentOwner);
      Storefronts::<T>::ensure_storefront_exists(storefront_id)?;

      <PendingStorefrontOwner<T>>::insert(storefront_id, transfer_to.clone());

      Self::deposit_event(RawEvent::StorefrontOwnershipTransferCreated(who, storefront_id, transfer_to));
      Ok(())
    }

    #[weight = 10_000 + T::DbWeight::get().reads_writes(2, 2)]
    pub fn accept_pending_ownership(origin, storefront_id: StorefrontId) -> DispatchResult {
      let new_owner = ensure_signed(origin)?;

      let mut storefront = Storefronts::require_storefront(storefront_id)?;
      ensure!(!storefront.is_owner(&new_owner), Error::<T>::AlreadyAStorefrontOwner);

      let transfer_to = Self::pending_storefront_owner(storefront_id).ok_or(Error::<T>::NoPendingTransferOnStorefront)?;
      ensure!(new_owner == transfer_to, Error::<T>::NotAllowedToAcceptOwnershipTransfer);

      // Here we know that the origin is eligible to become a new owner of this storefront.
      <PendingStorefrontOwner<T>>::remove(storefront_id);

      let old_owner = storefront.owner;
      storefront.owner = new_owner.clone();
      <StorefrontById<T>>::insert(storefront_id, storefront);

      // Remove storefront id from the list of storefronts by old owner
      <StorefrontIdsByOwner<T>>::mutate(old_owner.clone(), |storefront_ids| vec_remove_on(storefront_ids, storefront_id));

      // Add storefront id to the list of storefronts by new owner
      <StorefrontIdsByOwner<T>>::mutate(new_owner.clone(), |ids| ids.push(storefront_id));

      // TODO add a new owner as a storefront follower? See T::BeforeStorefrontCreated::before_storefront_created(new_owner.clone(), storefront)?;

      Self::deposit_event(RawEvent::StorefrontOwnershipTransferAccepted(new_owner, storefront_id));
      Ok(())
    }

    #[weight = 10_000 + T::DbWeight::get().reads_writes(2, 1)]
    pub fn reject_pending_ownership(origin, storefront_id: StorefrontId) -> DispatchResult {
      let who = ensure_signed(origin)?;

      let storefront = Storefronts::<T>::require_storefront(storefront_id)?;
      let transfer_to = Self::pending_storefront_owner(storefront_id).ok_or(Error::<T>::NoPendingTransferOnStorefront)?;
      ensure!(who == transfer_to || who == storefront.owner, Error::<T>::NotAllowedToRejectOwnershipTransfer);

      <PendingStorefrontOwner<T>>::remove(storefront_id);

      Self::deposit_event(RawEvent::StorefrontOwnershipTransferRejected(who, storefront_id));
      Ok(())
    }
  }
}
