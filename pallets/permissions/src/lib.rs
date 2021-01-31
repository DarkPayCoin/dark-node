#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use frame_support::{
  decl_module,
  traits::Get
};
use sp_runtime::RuntimeDebug;
use sp_std::{
  collections::btree_set::BTreeSet,
  prelude::*
};
use frame_system::{self as system};

use pallet_utils::StorefrontId;

#[derive(Encode, Decode, Ord, PartialOrd, Clone, Eq, PartialEq, RuntimeDebug)]
pub enum StorefrontPermission {
  /// Create, update, delete, grant and revoke roles in this storefront.
  ManageRoles,

  /// Act on behalf of this storefront within this storefront.
  RepresentStorefrontInternally,
  /// Act on behalf of this storefront outside of this storefront.
  RepresentStorefrontExternally,

  /// Update this storefront.
  UpdateStorefront,

  // Related to substorefronts in this storefront:
  CreateSubstorefronts,
  UpdateOwnSubstorefronts,
  DeleteOwnSubstorefronts,
  HideOwnSubstorefronts,

  UpdateAnySubstorefront,
  DeleteAnySubstorefront,
  HideAnySubstorefront,

  // Related to products in this storefront:
  CreateProducts,
  UpdateOwnProducts,
  DeleteOwnProducts,
  HideOwnProducts,

  UpdateAnyProduct,
  DeleteAnyProduct,
  HideAnyProduct,

  // Related to comments in this storefront:
  CreateComments,
  UpdateOwnComments,
  DeleteOwnComments,
  HideOwnComments,

  // NOTE: It was made on purpose that it's not possible to update or delete not own comments.
  // Instead it's possible to allow to hide and block comments.
  HideAnyComment,

  /// Upvote any product or comment in this storefront.
  Upvote,
  /// Downvote any product or comment in this storefront.
  Downvote,
  /// Share any product or comment from this storefront to another outer storefront.
  Share,

  /// Override permissions per substorefront in this storefront.
  OverrideSubstorefrontPermissions,
  /// Override permissions per product in this storefront.
  OverrideProductPermissions,

  // Related to moderation pallet
  /// Suggest new entity status in storefront (whether it's blocked or allowed)
  SuggestEntityStatus,
  /// Update entity status in storefront
  UpdateEntityStatus,

  // Related to Storefront settings
  /// Update collection of storefront settings in different pallets
  UpdateStorefrontSettings,
}

pub type StorefrontPermissionSet = BTreeSet<StorefrontPermission>;

#[derive(Encode, Decode, Clone, Eq, PartialEq, RuntimeDebug)]
pub struct StorefrontPermissions {
  pub none: Option<StorefrontPermissionSet>,
  pub everyone: Option<StorefrontPermissionSet>,
  pub follower: Option<StorefrontPermissionSet>,
  pub storefront_owner: Option<StorefrontPermissionSet>,
}

impl Default for StorefrontPermissions {
  fn default() -> StorefrontPermissions {
    StorefrontPermissions {
      none: None,
      everyone: None,
      follower: None,
      storefront_owner: None,
    }
  }
}

#[derive(Encode, Decode, Clone, Eq, PartialEq, RuntimeDebug)]
pub struct StorefrontPermissionsContext {
  pub storefront_id: StorefrontId,
  pub is_storefront_owner: bool,
  pub is_storefront_follower: bool,
  pub storefront_perms: Option<StorefrontPermissions>
}

/// The pallet's configuration trait.
pub trait Trait: system::Trait {
  type DefaultStorefrontPermissions: Get<StorefrontPermissions>;
}

decl_module! {
  pub struct Module<T: Trait> for enum Call where origin: T::Origin {
    const DefaultStorefrontPermissions: StorefrontPermissions = T::DefaultStorefrontPermissions::get();
  }
}

impl StorefrontPermission {
  fn is_present_in_role(&self, perms_opt: Option<StorefrontPermissionSet>) -> bool {
    if let Some(perms) = perms_opt {
      if perms.contains(self) {
        return true
      }
    }
    false
  }
}

impl<T: Trait> Module<T> {

  fn get_overrides_or_defaults(
    overrides: Option<StorefrontPermissionSet>,
    defaults: Option<StorefrontPermissionSet>
  ) -> Option<StorefrontPermissionSet> {

    if overrides.is_some() {
      overrides
    } else {
      defaults
    }
  }

  fn resolve_storefront_perms(
    storefront_perms: Option<StorefrontPermissions>,
  ) -> StorefrontPermissions {

    let defaults = T::DefaultStorefrontPermissions::get();
    let overrides = storefront_perms.unwrap_or_default();

    StorefrontPermissions {
      none: Self::get_overrides_or_defaults(overrides.none, defaults.none),
      everyone: Self::get_overrides_or_defaults(overrides.everyone, defaults.everyone),
      follower: Self::get_overrides_or_defaults(overrides.follower, defaults.follower),
      storefront_owner: Self::get_overrides_or_defaults(overrides.storefront_owner, defaults.storefront_owner)
    }
  }

  pub fn has_user_a_storefront_permission(
    ctx: StorefrontPermissionsContext,
    permission: StorefrontPermission,
  ) -> Option<bool> {

    let perms_by_role = Self::resolve_storefront_perms(ctx.storefront_perms);

    // Check if this permission is forbidden:
    if permission.is_present_in_role(perms_by_role.none) {
      return Some(false)
    }

    let is_storefront_owner = ctx.is_storefront_owner;
    let is_follower = is_storefront_owner || ctx.is_storefront_follower;

    if
      permission.is_present_in_role(perms_by_role.everyone) ||
      is_follower && permission.is_present_in_role(perms_by_role.follower) ||
      is_storefront_owner && permission.is_present_in_role(perms_by_role.storefront_owner)
    {
      return Some(true)
    }

    None
  }
}