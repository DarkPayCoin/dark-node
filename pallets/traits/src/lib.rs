#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::dispatch::{DispatchError, DispatchResult};

use pallet_permissions::{
  StorefrontPermission,
  StorefrontPermissions,
  StorefrontPermissionsContext
};
use pallet_utils::{StorefrontId, User};

pub mod moderation;

/// Minimal set of fields from Storefront struct that are required by roles pallet.
pub struct StorefrontForRoles<AccountId> {
  pub owner: AccountId,
  pub permissions: Option<StorefrontPermissions>,
}

pub trait StorefrontForRolesProvider {
  type AccountId;

  fn get_storefront(id: StorefrontId) -> Result<StorefrontForRoles<Self::AccountId>, DispatchError>;
}

pub trait StorefrontFollowsProvider {
  type AccountId;

  fn is_storefront_follower(account: Self::AccountId, storefront_id: StorefrontId) -> bool;
}

pub trait PermissionChecker {
  type AccountId;

  fn ensure_user_has_storefront_permission(
    user: User<Self::AccountId>,
    ctx: StorefrontPermissionsContext,
    permission: StorefrontPermission,
    error: DispatchError,
  ) -> DispatchResult;

  fn ensure_account_has_storefront_permission(
    account: Self::AccountId,
    ctx: StorefrontPermissionsContext,
    permission: StorefrontPermission,
    error: DispatchError,
  ) -> DispatchResult {

    Self::ensure_user_has_storefront_permission(
      User::Account(account),
      ctx,
      permission,
      error
    )
  }
}
