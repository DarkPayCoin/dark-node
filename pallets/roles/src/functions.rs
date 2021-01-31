use super::*;

use frame_support::dispatch::DispatchError;
use pallet_permissions::StorefrontPermissionsContext;
use pallet_utils::StorefrontId;

impl<T: Trait> Module<T> {

  /// Check that there is a `Role` with such `role_id` in the storage
  /// or return`RoleNotFound` error.
  pub fn ensure_role_exists(role_id: RoleId) -> DispatchResult {
      ensure!(<RoleById<T>>::contains_key(role_id), Error::<T>::RoleNotFound);
      Ok(())
  }

  /// Get `Role` by id from the storage or return `RoleNotFound` error.
  pub fn require_role(role_id: StorefrontId) -> Result<Role<T>, DispatchError> {
      Ok(Self::role_by_id(role_id).ok_or(Error::<T>::RoleNotFound)?)
  }

  pub fn ensure_role_manager(account: T::AccountId, storefront_id: StorefrontId) -> DispatchResult {
    Self::ensure_user_has_storefront_permission_with_load_storefront(
      User::Account(account),
      storefront_id,
      StorefrontPermission::ManageRoles,
      Error::<T>::NoPermissionToManageRoles.into()
    )
  }

  fn ensure_user_has_storefront_permission_with_load_storefront(
    user: User<T::AccountId>,
    storefront_id: StorefrontId,
    permission: StorefrontPermission,
    error: DispatchError,
  ) -> DispatchResult {

    let storefront = T::Storefronts::get_storefront(storefront_id)?;

    let mut is_owner = false;
    let mut is_follower = false;

    match &user {
      User::Account(account) => {
        is_owner = *account == storefront.owner;

        // No need to check if a user is follower, if they already are an owner:
        is_follower = is_owner || T::StorefrontFollows::is_storefront_follower(account.clone(), storefront_id);
      }
      User::Storefront(_) => (/* Not implemented yet. */),
    }

    Self::ensure_user_has_storefront_permission(
      user,
      StorefrontPermissionsContext {
        storefront_id,
        is_storefront_owner: is_owner,
        is_storefront_follower: is_follower,
        storefront_perms: storefront.permissions
      },
      permission,
      error
    )
  }

  fn ensure_user_has_storefront_permission(
    user: User<T::AccountId>,
    ctx: StorefrontPermissionsContext,
    permission: StorefrontPermission,
    error: DispatchError,
  ) -> DispatchResult {

    match Permissions::<T>::has_user_a_storefront_permission(
      ctx.clone(),
      permission.clone()
    ) {
      Some(true) => return Ok(()),
      Some(false) => return Err(error),
      _ => (/* Need to check in dynamic roles */)
    }

    Self::has_permission_in_storefront_roles(
      user,
      ctx.storefront_id,
      permission,
      error
    )
  }

  fn has_permission_in_storefront_roles(
    user: User<T::AccountId>,
    storefront_id: StorefrontId,
    permission: StorefrontPermission,
    error: DispatchError,
  ) -> DispatchResult {

    let role_ids = Self::role_ids_by_user_in_storefront((user, storefront_id));

    for role_id in role_ids {
      if let Some(role) = Self::role_by_id(role_id) {
        if role.disabled {
          continue;
        }

        let mut is_expired = false;
        if let Some(expires_at) = role.expires_at {
          if expires_at <= <system::Module<T>>::block_number() {
            is_expired = true;
          }
        }

        if !is_expired && role.permissions.contains(&permission) {
          return Ok(());
        }
      }
    }

    Err(error)
  }
}

impl<T: Trait> Role<T> {

  pub fn new(
    created_by: T::AccountId,
    storefront_id: StorefrontId,
    time_to_live: Option<T::BlockNumber>,
    content: Content,
    permissions: BTreeSet<StorefrontPermission>,
  ) -> Result<Self, DispatchError> {

    let role_id = Module::<T>::next_role_id();

    let mut expires_at: Option<T::BlockNumber> = None;
    if let Some(ttl) = time_to_live {
      expires_at = Some(ttl + <system::Module<T>>::block_number());
    }

    let new_role = Role::<T> {
      created: WhoAndWhen::new(created_by),
      updated: None,
      id: role_id,
      storefront_id,
      disabled: false,
      expires_at,
      content,
      permissions,
    };

    Ok(new_role)
  }

  pub fn set_disabled(&mut self, disable: bool) -> DispatchResult {
    if self.disabled && disable {
      return Err(Error::<T>::RoleAlreadyDisabled.into());
    } else if !self.disabled && !disable {
      return Err(Error::<T>::RoleAlreadyEnabled.into());
    }

    self.disabled = disable;

    Ok(())
  }

  pub fn revoke_from_users(&self, users: Vec<User<T::AccountId>>) {
    let mut users_by_role = <UsersByRoleId<T>>::take(self.id);

    for user in users.iter() {
      let role_idx_by_user_opt = Module::<T>::role_ids_by_user_in_storefront((&user, self.storefront_id)).iter()
        .position(|x| { *x == self.id });

      if let Some(role_idx) = role_idx_by_user_opt {
        <RoleIdsByUserInStorefront<T>>::mutate((user, self.storefront_id), |n| { n.swap_remove(role_idx) });
      }

      let user_idx_by_role_opt = users_by_role.iter().position(|x| { x == user });

      if let Some(user_idx) = user_idx_by_role_opt {
        users_by_role.swap_remove(user_idx);
      }
    }
    <UsersByRoleId<T>>::insert(self.id, users_by_role);
  }
}

impl<T: Trait> PermissionChecker for Module<T> {
  type AccountId = T::AccountId;

  fn ensure_user_has_storefront_permission(
    user: User<Self::AccountId>,
    ctx: StorefrontPermissionsContext,
    permission: StorefrontPermission,
    error: DispatchError,
  ) -> DispatchResult {

    Self::ensure_user_has_storefront_permission(
      user,
      ctx,
      permission,
      error
    )
  }
}
