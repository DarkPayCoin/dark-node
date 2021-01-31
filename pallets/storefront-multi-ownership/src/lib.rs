#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use frame_support::{decl_error, decl_event, decl_module, decl_storage, ensure, traits::Get};
use pallet_utils::{StorefrontId, WhoAndWhen};
use sp_runtime::{RuntimeDebug, traits::Zero};
use sp_std::collections::{btree_map::BTreeMap, btree_set::BTreeSet};
use sp_std::prelude::*;
use frame_system::{self as system, ensure_signed};

pub mod functions;

// #[cfg(test)]
// mod tests;

#[derive(Encode, Decode, Clone, Eq, PartialEq, RuntimeDebug)]
pub struct StorefrontOwners<T: Trait> {
  pub created: WhoAndWhen<T>,
  pub storefront_id: StorefrontId,
  pub owners: Vec<T::AccountId>,
  pub threshold: u16,
  pub changes_count: u16,
}

#[derive(Encode, Decode, Clone, Eq, PartialEq, RuntimeDebug)]
pub struct Change<T: Trait> {
  pub created: WhoAndWhen<T>,
  pub id: ChangeId,
  pub storefront_id: StorefrontId,
  pub add_owners: Vec<T::AccountId>,
  pub remove_owners: Vec<T::AccountId>,
  pub new_threshold: Option<u16>,
  pub notes: Vec<u8>,
  pub confirmed_by: Vec<T::AccountId>,
  pub expires_at: T::BlockNumber,
}

type ChangeId = u64;

/// The pallet's configuration trait.
pub trait Trait: system::Trait
  + pallet_timestamp::Trait
{
  /// The overarching event type.
  type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;

  /// Minimum storefront owners allowed.
  type MinStorefrontOwners: Get<u16>;

  /// Maximum storefront owners allowed.
  type MaxStorefrontOwners: Get<u16>;

  /// Maximum length of change notes.
  type MaxChangeNotesLength: Get<u16>;

  /// Expiration time for change proposal.
  type BlocksToLive: Get<Self::BlockNumber>;

  /// Period in blocks for which change proposal is can remain in a pending state until deleted.
  type DeleteExpiredChangesPeriod: Get<Self::BlockNumber>;
}

decl_error! {
  pub enum Error for Module<T: Trait> {
    /// Storefront owners was not found by id
    StorefrontOwnersNotFound,
    /// Change was not found by id
    ChangeNotFound,
    /// Storefront owners already exist on this storefront
    StorefrontOwnersAlreadyExist,

    /// There can not be less owners than allowed
    NotEnoughOwners,
    /// There can not be more owners than allowed
    TooManyOwners,
    /// Account is not a storefront owner
    NotAStorefrontOwner,

    /// The threshold can not be less than 1
    ZeroThershold,
    /// The required confirmation count can not be greater than owners count"
    TooBigThreshold,
    /// Change notes are too long
    ChangeNotesOversize,
    /// No storefront owners will left in result of change
    NoStorefrontOwnersLeft,
    /// No updates proposed with this change
    NoUpdatesProposed,
    /// No fields update in result of change proposal
    NoFieldsUpdatedOnProposal,

    /// Account has already confirmed this change
    ChangeAlreadyConfirmed,
    /// There are not enough confirmations for this change
    NotEnoughConfirms,
    /// Change is already executed
    ChangeAlreadyExecuted,
    /// Change is not related to this storefront
    ChangeNotRelatedToStorefront,
    /// Pending change already exists
    PendingChangeAlreadyExists,
    /// Pending change doesn't exist
    PendingChangeDoesNotExist,

    /// Account is not a proposal creator
    NotAChangeCreator,

    /// Overflow when incrementing a counter of executed changes
    ChangesCountOverflow,
  }
}

// This pallet's storage items.
decl_storage! {
  trait Store for Module<T: Trait> as StorefrontOwnersModule {
    StorefrontOwnersByStorefrontById get(storefront_owners_by_storefront_id): map StorefrontId => Option<StorefrontOwners<T>>;
    StorefrontIdsOwnedByAccountId get(storefront_ids_owned_by_account_id): map T::AccountId => BTreeSet<StorefrontId> = BTreeSet::new();

    NextChangeId get(next_change_id): ChangeId = 1;
    ChangeById get(change_by_id): map ChangeId => Option<Change<T>>;
    PendingChangeIdByStorefrontId get(pending_change_id_by_storefront_id): map StorefrontId => Option<ChangeId>;
    PendingChangeIds get(pending_change_ids): BTreeSet<ChangeId> = BTreeSet::new();
    ExecutedChangeIdsByStorefrontId get(executed_change_ids_by_storefront_id): map StorefrontId => Vec<ChangeId>;
  }
}

// The pallet's dispatchable functions.
decl_module! {
  pub struct Module<T: Trait> for enum Call where origin: T::Origin {
    /// Minimum storefront owners allowed.
    const MinStorefrontOwners: u16 = T::MinStorefrontOwners::get();

    /// Maximum storefront owners allowed.
    const MaxStorefrontOwners: u16 = T::MaxStorefrontOwners::get();

    /// Maximum length of change notes.
    const MaxChangeNotesLength: u16 = T::MaxChangeNotesLength::get();

    /// Period in blocks for which change proposal is can remain in a pending state until deleted.
    const BlocksToLive: T::BlockNumber = T::BlocksToLive::get();

    /// Period in blocks to initialize deleting of pending changes that are outdated.
    const DeleteExpiredChangesPeriod: T::BlockNumber = T::DeleteExpiredChangesPeriod::get();

    // Initializing events
    fn deposit_event() = default;

    fn on_finalize(n: T::BlockNumber) {
      Self::delete_expired_changes(n);
    }

    pub fn create_storefront_owners(
      origin,
      storefront_id: StorefrontId,
      owners: Vec<T::AccountId>,
      threshold: u16
    ) {
      let who = ensure_signed(origin)?;

      ensure!(Self::storefront_owners_by_storefront_id(storefront_id).is_none(), Error::<T>::StorefrontOwnersAlreadyExist);

      let mut owners_map: BTreeMap<T::AccountId, bool> = BTreeMap::new();
      let mut unique_owners: Vec<T::AccountId> = Vec::new();

      for owner in owners.iter() {
        if !owners_map.contains_key(&owner) {
          owners_map.insert(owner.clone(), true);
          unique_owners.push(owner.clone());
        }
      }

      let owners_count = unique_owners.len() as u16;
      ensure!(owners_count >= T::MinStorefrontOwners::get(), Error::<T>::NotEnoughOwners);
      ensure!(owners_count <= T::MaxStorefrontOwners::get(), Error::<T>::TooManyOwners);

      ensure!(threshold <= owners_count, Error::<T>::TooBigThreshold);
      ensure!(threshold > 0, Error::<T>::ZeroThershold);

      let new_storefront_owners = StorefrontOwners {
        created: WhoAndWhen::<T>::new(who.clone()),
        storefront_id: storefront_id,
        owners: unique_owners.clone(),
        threshold,
        changes_count: 0
      };

      <StorefrontOwnersByStorefrontById<T>>::insert(storefront_id, new_storefront_owners);

      for owner in unique_owners.iter() {
        <StorefrontIdsOwnedByAccountId<T>>::mutate(owner.clone(), |ids| ids.insert(storefront_id));
      }

      Self::deposit_event(RawEvent::StorefrontOwnersCreated(who, storefront_id));
    }

    pub fn propose_change(
      origin,
      storefront_id: StorefrontId,
      add_owners: Vec<T::AccountId>,
      remove_owners: Vec<T::AccountId>,
      new_threshold: Option<u16>,
      notes: Vec<u8>
    ) {
      let who = ensure_signed(origin)?;

      let has_updates =
        !add_owners.is_empty() ||
        !remove_owners.is_empty() ||
        new_threshold.is_some();

      ensure!(has_updates, Error::<T>::NoUpdatesProposed);
      ensure!(notes.len() <= T::MaxChangeNotesLength::get() as usize, Error::<T>::ChangeNotesOversize);

      let storefront_owners = Self::storefront_owners_by_storefront_id(storefront_id).ok_or(Error::<T>::StorefrontOwnersNotFound)?;
      ensure!(Self::pending_change_id_by_storefront_id(storefront_id).is_none(), Error::<T>::PendingChangeAlreadyExists);

      let is_storefront_owner = storefront_owners.owners.iter().any(|owner| *owner == who.clone());
      ensure!(is_storefront_owner, Error::<T>::NotAStorefrontOwner);

      let mut fields_updated : u16 = 0;

      let result_owners = Self::transform_new_owners_to_vec(storefront_owners.owners.clone(), add_owners.clone(), remove_owners.clone());
      ensure!(!result_owners.is_empty(), Error::<T>::NoStorefrontOwnersLeft);
      if result_owners != storefront_owners.owners {
        fields_updated += 1;
      }

      if let Some(threshold) = new_threshold {
        if storefront_owners.threshold != threshold {
          ensure!(threshold as usize <= result_owners.len(), Error::<T>::TooBigThreshold);
          ensure!(threshold > 0, Error::<T>::ZeroThershold);
          fields_updated += 1;
        }
      }

      let change_id = Self::next_change_id();
      let mut new_change = Change {
        created: WhoAndWhen::<T>::new(who.clone()),
        id: change_id,
        storefront_id,
        add_owners: add_owners,
        remove_owners: remove_owners,
        new_threshold: new_threshold,
        notes,
        confirmed_by: Vec::new(),
        expires_at: <system::Module<T>>::block_number() + T::BlocksToLive::get()
      };

      if fields_updated > 0 {
        new_change.confirmed_by.push(who.clone());
        <ChangeById<T>>::insert(change_id, new_change);
        PendingChangeIdByStorefrontId::insert(storefront_id, change_id);
        PendingChangeIds::mutate(|set| set.insert(change_id));
        NextChangeId::mutate(|n| { *n += 1; });

        Self::deposit_event(RawEvent::ChangeProposed(who, storefront_id, change_id));
      } else {
        return Err(Error::<T>::NoFieldsUpdatedOnProposal.into());
      }
    }

    pub fn confirm_change(
      origin,
      storefront_id: StorefrontId,
      change_id: ChangeId
    ) {
      let who = ensure_signed(origin)?;

      let storefront_owners = Self::storefront_owners_by_storefront_id(storefront_id).ok_or(Error::<T>::StorefrontOwnersNotFound)?;

      let is_storefront_owner = storefront_owners.owners.iter().any(|owner| *owner == who.clone());
      ensure!(is_storefront_owner, Error::<T>::NotAStorefrontOwner);

      let mut change = Self::change_by_id(change_id).ok_or(Error::<T>::ChangeNotFound)?;

      let pending_change_id = Self::pending_change_id_by_storefront_id(storefront_id).ok_or(Error::<T>::PendingChangeDoesNotExist)?;
      ensure!(pending_change_id == change_id, Error::<T>::ChangeNotRelatedToStorefront);

      // Check whether sender confirmed change or not
      ensure!(!change.confirmed_by.iter().any(|account| *account == who.clone()), Error::<T>::ChangeAlreadyConfirmed);

      change.confirmed_by.push(who.clone());

      if change.confirmed_by.len() == storefront_owners.threshold as usize {
        Self::update_storefront_owners(who.clone(), storefront_owners, change)?;
      } else {
        <ChangeById<T>>::insert(change_id, change);
      }

      Self::deposit_event(RawEvent::ChangeConfirmed(who, storefront_id, change_id));
    }

    pub fn cancel_change(
      origin,
      storefront_id: StorefrontId,
      change_id: ChangeId
    ) {
      let who = ensure_signed(origin)?;

      let storefront_owners = Self::storefront_owners_by_storefront_id(storefront_id).ok_or(Error::<T>::StorefrontOwnersNotFound)?;

      let is_storefront_owner = storefront_owners.owners.iter().any(|owner| *owner == who.clone());
      ensure!(is_storefront_owner, Error::<T>::NotAStorefrontOwner);

      let pending_change_id = Self::pending_change_id_by_storefront_id(storefront_id).ok_or(Error::<T>::PendingChangeDoesNotExist)?;
      ensure!(pending_change_id == change_id, Error::<T>::ChangeNotRelatedToStorefront);

      let change = Self::change_by_id(change_id).ok_or(Error::<T>::ChangeNotFound)?;
      ensure!(change.created.account == who, Error::<T>::NotAChangeCreator);

      <ChangeById<T>>::remove(change_id);
      PendingChangeIdByStorefrontId::remove(storefront_id);
      PendingChangeIds::mutate(|set| set.remove(&change_id));

      Self::deposit_event(RawEvent::ProposalCanceled(who, storefront_id));
    }
  }
}

decl_event!(
  pub enum Event<T> where
    <T as system::Trait>::AccountId,
   {
    StorefrontOwnersCreated(AccountId, StorefrontId),
    ChangeProposed(AccountId, StorefrontId, ChangeId),
    ProposalCanceled(AccountId, StorefrontId),
    ChangeConfirmed(AccountId, StorefrontId, ChangeId),
    StorefrontOwnersUpdated(AccountId, StorefrontId, ChangeId),
  }
);
