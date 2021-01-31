use super::*;

use sp_std::collections::btree_set::BTreeSet;
use frame_support::{dispatch::DispatchResult};

impl<T: Trait> Module<T> {

  pub fn update_storefront_owners(who: T::AccountId, mut storefront_owners: StorefrontOwners<T>, change: Change<T>) -> DispatchResult {
    let storefront_id = storefront_owners.storefront_id;
    let change_id = change.id;

    ensure!(change.confirmed_by.len() >= storefront_owners.threshold as usize, Error::<T>::NotEnoughConfirms);
    Self::move_change_from_pending_state_to_executed(storefront_id, change_id)?;

    storefront_owners.changes_count = storefront_owners.changes_count.checked_add(1).ok_or(Error::<T>::ChangesCountOverflow)?;
    if !change.add_owners.is_empty() || !change.remove_owners.is_empty() {
      storefront_owners.owners = Self::transform_new_owners_to_vec(
        storefront_owners.owners.clone(), change.add_owners.clone(), change.remove_owners.clone());
    }

    if let Some(threshold) = change.new_threshold {
      storefront_owners.threshold = threshold;
    }

    for account in &change.add_owners {
      <StorefrontIdsOwnedByAccountId<T>>::mutate(account, |ids| ids.insert(storefront_id));
    }
    for account in &change.remove_owners {
      <StorefrontIdsOwnedByAccountId<T>>::mutate(account, |ids| ids.remove(&storefront_id));
    }

    <StorefrontOwnersByStorefrontById<T>>::insert(storefront_id, storefront_owners);
    <ChangeById<T>>::insert(change_id, change);
    Self::deposit_event(RawEvent::StorefrontOwnersUpdated(who, storefront_id, change_id));

    Ok(())
  }

  pub fn move_change_from_pending_state_to_executed(storefront_id: StorefrontId, change_id: ChangeId) -> DispatchResult {
    ensure!(Self::storefront_owners_by_storefront_id(storefront_id).is_some(), Error::<T>::StorefrontOwnersNotFound);
    ensure!(Self::change_by_id(change_id).is_some(), Error::<T>::ChangeNotFound);
    ensure!(!Self::executed_change_ids_by_storefront_id(storefront_id).iter().any(|&x| x == change_id), Error::<T>::ChangeAlreadyExecuted);

    PendingChangeIdByStorefrontId::remove(&storefront_id);
    PendingChangeIds::mutate(|set| set.remove(&change_id));
    ExecutedChangeIdsByStorefrontId::mutate(storefront_id, |ids| ids.push(change_id));

    Ok(())
  }

  pub fn transform_new_owners_to_vec(current_owners: Vec<T::AccountId>, add_owners: Vec<T::AccountId>, remove_owners: Vec<T::AccountId>) -> Vec<T::AccountId> {
    let mut owners_set: BTreeSet<T::AccountId> = BTreeSet::new();
    let mut new_owners_set: BTreeSet<T::AccountId> = BTreeSet::new();

    // Extract current storefront owners
    current_owners.iter().for_each(|x| { owners_set.insert(x.clone()); });
    // Extract owners that should be added
    add_owners.iter().for_each(|x| { new_owners_set.insert(x.clone()); });
    // Unite both sets
    owners_set = owners_set.union(&new_owners_set).cloned().collect();
    // Remove accounts that exist in remove_owners from set
    remove_owners.iter().for_each(|x| { owners_set.remove(x); });

    owners_set.iter().cloned().collect()
  }

  pub fn delete_expired_changes(block_number: T::BlockNumber) {
    if (block_number % T::DeleteExpiredChangesPeriod::get()).is_zero() {
      for change_id in Self::pending_change_ids() {
        if let Some(change) = Self::change_by_id(change_id) {
          if block_number >= change.expires_at {
            PendingChangeIdByStorefrontId::remove(&change.storefront_id);
            <ChangeById<T>>::remove(&change_id);
            PendingChangeIds::mutate(|set| set.remove(&change_id));
          }
        }
      }
    }
  }
}
