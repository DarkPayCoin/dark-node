#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use frame_support::{
    decl_error, decl_event, decl_module, decl_storage,
    ensure,
    traits::Get,
    dispatch::DispatchResult
};
use sp_runtime::RuntimeDebug;
use sp_std::{collections::btree_set::BTreeSet, iter::FromIterator, prelude::*};
use frame_system::{self as system, ensure_signed};

use df_traits::{PermissionChecker, StorefrontFollowsProvider, StorefrontForRolesProvider};
use pallet_permissions::{Module as Permissions, StorefrontPermission, StorefrontPermissionSet};
use pallet_utils::{Module as Utils, StorefrontId, User, WhoAndWhen, Content};

pub mod functions;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

type RoleId = u64;

#[derive(Encode, Decode, Clone, Eq, PartialEq, RuntimeDebug)]
pub struct Role<T: Trait> {
    pub created: WhoAndWhen<T>,
    pub updated: Option<WhoAndWhen<T>>,
    pub id: RoleId,
    pub storefront_id: StorefrontId,
    pub disabled: bool,
    pub expires_at: Option<T::BlockNumber>,
    pub content: Content,
    pub permissions: StorefrontPermissionSet,
}

#[derive(Encode, Decode, Clone, Eq, PartialEq, RuntimeDebug)]
pub struct RoleUpdate {
    pub disabled: Option<bool>,
    pub content: Option<Content>,
    pub permissions: Option<StorefrontPermissionSet>,
}

/// The pallet's configuration trait.
pub trait Trait: system::Trait
    + pallet_permissions::Trait
    + pallet_utils::Trait
{
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;

    type MaxUsersToProcessPerDeleteRole: Get<u16>;

    type Storefronts: StorefrontForRolesProvider<AccountId=Self::AccountId>;

    type StorefrontFollows: StorefrontFollowsProvider<AccountId=Self::AccountId>;
}

decl_event!(
    pub enum Event<T> where
        <T as system::Trait>::AccountId
    {
        RoleCreated(AccountId, StorefrontId, RoleId),
        RoleUpdated(AccountId, RoleId),
        RoleDeleted(AccountId, RoleId),
        RoleGranted(AccountId, RoleId, Vec<User<AccountId>>),
        RoleRevoked(AccountId, RoleId, Vec<User<AccountId>>),
    }
);

decl_error! {
    pub enum Error for Module<T: Trait> {
        /// Role was not found by id.
        RoleNotFound,
        /// RoleId counter storage overflowed.
        RoleIdOverflow,
        /// Account has no permission to manage roles in this storefront.
        NoPermissionToManageRoles,
        /// Nothing to update in role.
        NoUpdatesProvided,
        /// No permissions provided when trying to create a new role.
        NoPermissionsProvided,
        /// No users provided when trying to grant them a role.
        NoUsersProvided,
        /// There are too many users with this role to delete it in a single tx.
        TooManyUsersToDelete,
        /// Cannot disable a role that is already disabled.
        RoleAlreadyDisabled,
        /// Cannot enable a role that is already enabled.
        RoleAlreadyEnabled,
    }
}

// This pallet's storage items.
decl_storage! {
    trait Store for Module<T: Trait> as PermissionsModule {

        /// The next role id.
        pub NextRoleId get(fn next_role_id): RoleId = 1;

        /// Get role details by its id.
        pub RoleById get(fn role_by_id):
            map hasher(twox_64_concat) RoleId => Option<Role<T>>;

        /// A list of all users (account or storefront ids) that have this role.
        pub UsersByRoleId get(fn users_by_role_id):
            map hasher(twox_64_concat) RoleId => Vec<User<T::AccountId>>;

        /// A list of all role ids available in this storefront.
        pub RoleIdsByStorefrontId get(fn role_ids_by_storefront_id):
            map hasher(twox_64_concat) StorefrontId => Vec<RoleId>;

        /// A list of all role ids granted to this user (account or storefront) within this storefront.
        pub RoleIdsByUserInStorefront get(fn role_ids_by_user_in_storefront):
            map hasher(blake2_128_concat) (User<T::AccountId>, StorefrontId) => Vec<RoleId>;
    }
}

// The pallet's dispatchable functions.
decl_module! {
  pub struct Module<T: Trait> for enum Call where origin: T::Origin {

    const MaxUsersToProcessPerDeleteRole: u16 = T::MaxUsersToProcessPerDeleteRole::get();

    // Initializing errors
    type Error = Error<T>;

    // Initializing events
    fn deposit_event() = default;

    /// Create a new role in a storefront with a list of permissions.
    /// `content` points to the off-chain content with such additional info about this role
    /// as its name, description, color, etc.
    /// Only the storefront owner or a user with `ManageRoles` permission call this dispatch.
    #[weight = 10_000 + T::DbWeight::get().reads_writes(2, 3)]
    pub fn create_role(
      origin,
      storefront_id: StorefrontId,
      time_to_live: Option<T::BlockNumber>,
      content: Content,
      permissions: Vec<StorefrontPermission>
    ) -> DispatchResult {
      let who = ensure_signed(origin)?;

      ensure!(!permissions.is_empty(), Error::<T>::NoPermissionsProvided);

      Utils::<T>::is_valid_content(content.clone())?;

      Self::ensure_role_manager(who.clone(), storefront_id)?;

      let permissions_set = BTreeSet::from_iter(permissions.into_iter());
      let new_role = Role::<T>::new(who.clone(), storefront_id, time_to_live, content, permissions_set)?;

      let next_role_id = new_role.id.checked_add(1).ok_or(Error::<T>::RoleIdOverflow)?;
      NextRoleId::put(next_role_id);

      <RoleById<T>>::insert(new_role.id, new_role.clone());
      RoleIdsByStorefrontId::mutate(storefront_id, |role_ids| { role_ids.push(new_role.id) });

      Self::deposit_event(RawEvent::RoleCreated(who, storefront_id, new_role.id));
      Ok(())
    }

    /// Update an existing role by its id.
    /// Only the storefront owner or a user with `ManageRoles` permission call this dispatch.
    #[weight = 10_000 + T::DbWeight::get().reads_writes(2, 1)]
    pub fn update_role(origin, role_id: RoleId, update: RoleUpdate) -> DispatchResult {
      let who = ensure_signed(origin)?;

      let has_updates =
        update.disabled.is_some() ||
        update.content.is_some() ||
        update.permissions.is_some();

      ensure!(has_updates, Error::<T>::NoUpdatesProvided);

      let mut role = Self::require_role(role_id)?;

      Self::ensure_role_manager(who.clone(), role.storefront_id)?;

      let mut is_update_applied = false;

      if let Some(disabled) = update.disabled {
        if disabled != role.disabled {
          role.set_disabled(disabled)?;
          is_update_applied = true;
        }
      }

      if let Some(content) = update.content {
        if content != role.content {
          Utils::<T>::is_valid_content(content.clone())?;

          role.content = content;
          is_update_applied = true;
        }
      }

      if let Some(permissions) = update.permissions {
        if !permissions.is_empty() {
          let permissions_diff: Vec<_> = permissions.symmetric_difference(&role.permissions).cloned().collect();

          if !permissions_diff.is_empty() {
            role.permissions = permissions;
            is_update_applied = true;
          }
        }
      }

      if is_update_applied {
        role.updated = Some(WhoAndWhen::<T>::new(who.clone()));

        <RoleById<T>>::insert(role_id, role);
        Self::deposit_event(RawEvent::RoleUpdated(who, role_id));
      }
      Ok(())
    }

    /// Delete a role from all associated storage items.
    /// Only the storefront owner or a user with `ManageRoles` permission call this dispatch.
    #[weight = 1_000_000 + T::DbWeight::get().reads_writes(6, 5)]
    pub fn delete_role(origin, role_id: RoleId) -> DispatchResult {
      let who = ensure_signed(origin)?;

      let role = Self::require_role(role_id)?;

      Self::ensure_role_manager(who.clone(), role.storefront_id)?;

      let users = Self::users_by_role_id(role_id);
      ensure!(
        users.len() <= T::MaxUsersToProcessPerDeleteRole::get() as usize,
        Error::<T>::TooManyUsersToDelete
      );

      let role_idx_by_storefront_opt = Self::role_ids_by_storefront_id(role.storefront_id).iter()
        .position(|x| { *x == role_id });

      if let Some(role_idx) = role_idx_by_storefront_opt {
        RoleIdsByStorefrontId::mutate(role.storefront_id, |n| { n.swap_remove(role_idx) });
      }

      role.revoke_from_users(users);

      <RoleById<T>>::remove(role_id);
      <UsersByRoleId<T>>::remove(role_id);

      Self::deposit_event(RawEvent::RoleDeleted(who, role_id));
      Ok(())
    }

    /// Grant a role to a list of users.
    /// Only the storefront owner or a user with `ManageRoles` permission call this dispatch.
    #[weight = 1_000_000 + T::DbWeight::get().reads_writes(4, 2)]
    pub fn grant_role(origin, role_id: RoleId, users: Vec<User<T::AccountId>>) -> DispatchResult {
      let who = ensure_signed(origin)?;

      ensure!(!users.is_empty(), Error::<T>::NoUsersProvided);
      let users_set: BTreeSet<User<T::AccountId>> = Utils::<T>::convert_users_vec_to_btree_set(users)?;

      let role = Self::require_role(role_id)?;

      Self::ensure_role_manager(who.clone(), role.storefront_id)?;

      for user in users_set.iter() {
        if !Self::users_by_role_id(role_id).contains(&user) {
          <UsersByRoleId<T>>::mutate(role_id, |users| { users.push(user.clone()); });
        }
        if !Self::role_ids_by_user_in_storefront((user.clone(), role.storefront_id)).contains(&role_id) {
          <RoleIdsByUserInStorefront<T>>::mutate((user.clone(), role.storefront_id), |roles| { roles.push(role_id); })
        }
      }

      Self::deposit_event(RawEvent::RoleGranted(who, role_id, users_set.iter().cloned().collect()));
      Ok(())
    }

    /// Revoke a role from a list of users.
    /// Only the storefront owner or a user with `ManageRoles` permission call this dispatch.
    #[weight = 1_000_000 + T::DbWeight::get().reads_writes(4, 2)]
    pub fn revoke_role(origin, role_id: RoleId, users: Vec<User<T::AccountId>>) -> DispatchResult {
      let who = ensure_signed(origin)?;

      ensure!(!users.is_empty(), Error::<T>::NoUsersProvided);

      let role = Self::require_role(role_id)?;

      Self::ensure_role_manager(who.clone(), role.storefront_id)?;

      role.revoke_from_users(users.clone());

      Self::deposit_event(RawEvent::RoleRevoked(who, role_id, users));
      Ok(())
    }
  }
}
