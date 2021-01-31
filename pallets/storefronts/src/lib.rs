#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use frame_support::{
    decl_error, decl_event, decl_module, decl_storage, ensure,
    dispatch::{DispatchError, DispatchResult},
    traits::{Get, Currency, ExistenceRequirement},
};
use sp_runtime::RuntimeDebug;
use sp_std::prelude::*;
use frame_system::{self as system, ensure_signed};

use df_traits::{StorefrontForRoles, StorefrontForRolesProvider};
use df_traits::{PermissionChecker, StorefrontFollowsProvider};
use pallet_permissions::{StorefrontPermission, StorefrontPermissions, StorefrontPermissionsContext};
use pallet_utils::{Module as Utils, StorefrontId, WhoAndWhen, Content};

#[derive(Encode, Decode, Clone, Eq, PartialEq, RuntimeDebug)]
pub struct Storefront<T: Trait> {
    pub id: StorefrontId,
    pub created: WhoAndWhen<T>,
    pub updated: Option<WhoAndWhen<T>>,

    pub owner: T::AccountId,

    // Can be updated by the owner:
    pub parent_id: Option<StorefrontId>,
    pub handle: Option<Vec<u8>>,
    pub content: Content,
    pub hidden: bool,
    pub private: bool,

    pub products_count: u32,
    pub hidden_products_count: u32,
    pub private_products_count: u32,
    pub followers_count: u32,

    pub score: i32,

    /// Allows to override the default permissions for this storefront.
    pub permissions: Option<StorefrontPermissions>,
}

#[derive(Encode, Decode, Clone, Eq, PartialEq, RuntimeDebug)]
#[allow(clippy::option_option)]
pub struct StorefrontUpdate {
    pub parent_id: Option<Option<StorefrontId>>,
    pub handle: Option<Option<Vec<u8>>>,
    pub content: Option<Content>,
    pub hidden: Option<bool>,
    pub private: Option<bool>,
    pub permissions: Option<Option<StorefrontPermissions>>,
}

type BalanceOf<T> = <<T as pallet_utils::Trait>::Currency as Currency<<T as system::Trait>::AccountId>>::Balance;

/// The pallet's configuration trait.
pub trait Trait: system::Trait
    + pallet_utils::Trait
    + pallet_permissions::Trait
{
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;

    type Roles: PermissionChecker<AccountId=Self::AccountId>;

    type StorefrontFollows: StorefrontFollowsProvider<AccountId=Self::AccountId>;

    type BeforeStorefrontCreated: BeforeStorefrontCreated<Self>;

    type AfterStorefrontUpdated: AfterStorefrontUpdated<Self>;

    type StorefrontCreationFee: Get<BalanceOf<Self>>;
}

decl_error! {
  pub enum Error for Module<T: Trait> {
    /// Storefront was not found by id.
    StorefrontNotFound,
    /// Storefront handle is not unique.
    StorefrontHandleIsNotUnique,
    /// Nothing to update in storefront.
    NoUpdatesForStorefront,
    /// Only storefront owner can manage their storefront.
    NotAStorefrontOwner,
    /// User has no permission to update this storefront.
    NoPermissionToUpdateStorefront,
    /// User has no permission to create substorefronts in this storefront
    NoPermissionToCreateSubstorefronts,
    /// Storefront is at root level, no parent_id specified
    StorefrontIsAtRoot,
  }
}

// This pallet's storage items.
decl_storage! {
    trait Store for Module<T: Trait> as StorefrontsModule {

        pub NextStorefrontId get(fn next_storefront_id): StorefrontId = 1001;

        pub StorefrontById get(fn storefront_by_id) build(|config: &GenesisConfig<T>| {
          let mut storefronts: Vec<(StorefrontId, Storefront<T>)> = Vec::new();
          let endowed_account = config.endowed_account.clone();
          for id in 1..=1000 {
            storefronts.push((id, Storefront::<T>::new(id, None, endowed_account.clone(), Content::None, None)));
          }
          storefronts
        }):
            map hasher(twox_64_concat) StorefrontId => Option<Storefront<T>>;

        pub StorefrontIdByHandle get(fn storefront_id_by_handle):
            map hasher(blake2_128_concat) Vec<u8> => Option<StorefrontId>;

        pub StorefrontIdsByOwner get(fn storefront_ids_by_owner):
            map hasher(twox_64_concat) T::AccountId => Vec<StorefrontId>;
    }
    add_extra_genesis {
      config(endowed_account): T::AccountId;
    }
}

decl_event!(
    pub enum Event<T> where
        <T as system::Trait>::AccountId,
    {
        StorefrontCreated(AccountId, StorefrontId),
        StorefrontUpdated(AccountId, StorefrontId),
        StorefrontDeleted(AccountId, StorefrontId),
    }
);

// The pallet's dispatchable functions.
decl_module! {
  pub struct Module<T: Trait> for enum Call where origin: T::Origin {

    const StorefrontCreationFee: BalanceOf<T> = T::StorefrontCreationFee::get();

    // Initializing errors
    type Error = Error<T>;

    // Initializing events
    fn deposit_event() = default;

    #[weight = 500_000 + T::DbWeight::get().reads_writes(4, 4)]
    pub fn create_storefront(
      origin,
      parent_id_opt: Option<StorefrontId>,
      handle_opt: Option<Vec<u8>>,
      content: Content
    ) -> DispatchResult {
      let owner = ensure_signed(origin)?;

      Utils::<T>::is_valid_content(content.clone())?;

      let mut handle_in_lowercase: Vec<u8> = Vec::new();
      if let Some(original_handle) = handle_opt.clone() {
        handle_in_lowercase = Self::lowercase_and_validate_storefront_handle(original_handle)?;
      }

      // TODO: add tests for this case
      if let Some(parent_id) = parent_id_opt {
        let parent_storefront = Self::require_storefront(parent_id)?;

        Self::ensure_account_has_storefront_permission(
          owner.clone(),
          &parent_storefront,
          StorefrontPermission::CreateSubstorefronts,
          Error::<T>::NoPermissionToCreateSubstorefronts.into()
        )?;
      }

      <T as pallet_utils::Trait>::Currency::transfer(
        &owner,
        &Utils::<T>::treasury_account(),
        T::StorefrontCreationFee::get(),
        ExistenceRequirement::KeepAlive
      )?;

      let storefront_id = Self::next_storefront_id();
      let new_storefront = &mut Storefront::new(storefront_id, parent_id_opt, owner.clone(), content, handle_opt);

      T::BeforeStorefrontCreated::before_storefront_created(owner.clone(), new_storefront)?;

      <StorefrontById<T>>::insert(storefront_id, new_storefront);
      <StorefrontIdsByOwner<T>>::mutate(owner.clone(), |ids| ids.push(storefront_id));
      NextStorefrontId::mutate(|n| { *n += 1; });

      if !handle_in_lowercase.is_empty() {
        StorefrontIdByHandle::insert(handle_in_lowercase, storefront_id);
      }

      Self::deposit_event(RawEvent::StorefrontCreated(owner, storefront_id));
      Ok(())
    }

    #[weight = 500_000 + T::DbWeight::get().reads_writes(2, 3)]
    pub fn update_storefront(origin, storefront_id: StorefrontId, update: StorefrontUpdate) -> DispatchResult {
      let owner = ensure_signed(origin)?;

      let has_updates =
        update.parent_id.is_some() ||
        update.handle.is_some() ||
        update.content.is_some() ||
        update.hidden.is_some() ||
        update.private.is_some() ||
        update.permissions.is_some();

      ensure!(has_updates, Error::<T>::NoUpdatesForStorefront);

      let mut storefront = Self::require_storefront(storefront_id)?;

      Self::ensure_account_has_storefront_permission(
        owner.clone(),
        &storefront,
        StorefrontPermission::UpdateStorefront,
        Error::<T>::NoPermissionToUpdateStorefront.into()
      )?;

      let mut is_update_applied = false;
      let mut old_data = StorefrontUpdate::default();

      // TODO: add tests for this case
      if let Some(parent_id_opt) = update.parent_id {
        if parent_id_opt != storefront.parent_id {

          if let Some(parent_id) = parent_id_opt {
            let parent_storefront = Self::require_storefront(parent_id)?;

            Self::ensure_account_has_storefront_permission(
              owner.clone(),
              &parent_storefront,
              StorefrontPermission::CreateSubstorefronts,
              Error::<T>::NoPermissionToCreateSubstorefronts.into()
            )?;
          }

          old_data.parent_id = Some(storefront.parent_id);
          storefront.parent_id = parent_id_opt;
          is_update_applied = true;
        }
      }

      if let Some(content) = update.content {
        if content != storefront.content {
          Utils::<T>::is_valid_content(content.clone())?;

          old_data.content = Some(storefront.content);
          storefront.content = content;
          is_update_applied = true;
        }
      }

      if let Some(hidden) = update.hidden {
        if hidden != storefront.hidden {
          old_data.hidden = Some(storefront.hidden);
          storefront.hidden = hidden;
          is_update_applied = true;
        }
      }


      if let Some(private) = update.private {
        if private != storefront.private {
          old_data.private = Some(storefront.private);
          storefront.private = private;
          is_update_applied = true;
        }
      }


      if let Some(overrides_opt) = update.permissions {
        if storefront.permissions != overrides_opt {
          old_data.permissions = Some(storefront.permissions);

          if let Some(mut overrides) = overrides_opt.clone() {
            overrides.none = overrides.none.map(
              |mut none_permissions_set| {
                none_permissions_set.extend(T::DefaultStorefrontPermissions::get().none.unwrap_or_default());
                none_permissions_set
              }
            );

            storefront.permissions = Some(overrides);
          } else {
            storefront.permissions = overrides_opt;
          }

          is_update_applied = true;
        }
      }

      if let Some(handle_opt) = update.handle {
        if handle_opt != storefront.handle {
          if let Some(new_handle) = handle_opt.clone() {
            let handle_in_lowercase = Self::lowercase_and_validate_storefront_handle(new_handle)?;
            StorefrontIdByHandle::insert(handle_in_lowercase, storefront_id);
          }
          if let Some(old_handle) = storefront.handle.clone() {
            StorefrontIdByHandle::remove(old_handle);
          }
          old_data.handle = Some(storefront.handle);
          storefront.handle = handle_opt;
          is_update_applied = true;
        }
      }

      // Update this storefront only if at least one field should be updated:
      if is_update_applied {
        storefront.updated = Some(WhoAndWhen::<T>::new(owner.clone()));

        <StorefrontById<T>>::insert(storefront_id, storefront.clone());
        T::AfterStorefrontUpdated::after_storefront_updated(owner.clone(), &storefront, old_data);

        Self::deposit_event(RawEvent::StorefrontUpdated(owner, storefront_id));
      }
      Ok(())
    }
  }
}

impl<T: Trait> Storefront<T> {
    pub fn new(
        id: StorefrontId,
        parent_id: Option<StorefrontId>,
        created_by: T::AccountId,
        content: Content,
        handle: Option<Vec<u8>>,
    ) -> Self {
        Storefront {
            id,
            created: WhoAndWhen::<T>::new(created_by.clone()),
            updated: None,
            owner: created_by,
            parent_id,
            handle,
            content,
            hidden: false,
            private: false,
            products_count: 0,
            hidden_products_count: 0,
            private_products_count: 0,
            followers_count: 0,
            score: 0,
            permissions: None,
        }
    }

    pub fn is_owner(&self, account: &T::AccountId) -> bool {
        self.owner == *account
    }

    pub fn is_follower(&self, account: &T::AccountId) -> bool {
        T::StorefrontFollows::is_storefront_follower(account.clone(), self.id)
    }

    pub fn ensure_storefront_owner(&self, account: T::AccountId) -> DispatchResult {
        ensure!(self.is_owner(&account), Error::<T>::NotAStorefrontOwner);
        Ok(())
    }

    pub fn inc_products(&mut self) {
        self.products_count = self.products_count.saturating_add(1);
    }

    pub fn dec_products(&mut self) {
        self.products_count = self.products_count.saturating_sub(1);
    }

    pub fn inc_hidden_products(&mut self) {
        self.hidden_products_count = self.hidden_products_count.saturating_add(1);
    }

    pub fn dec_hidden_products(&mut self) {
        self.hidden_products_count = self.hidden_products_count.saturating_sub(1);
    }

    pub fn inc_private_products(&mut self) {
      self.private_products_count = self.private_products_count.saturating_add(1);
  }

   pub fn dec_private_products(&mut self) {
      self.private_products_count = self.private_products_count.saturating_sub(1);
  }

    pub fn inc_followers(&mut self) {
        self.followers_count = self.followers_count.saturating_add(1);
    }

    pub fn dec_followers(&mut self) {
        self.followers_count = self.followers_count.saturating_sub(1);
    }

    #[allow(clippy::comparison_chain)]
    pub fn change_score(&mut self, diff: i16) {
        if diff > 0 {
            self.score = self.score.saturating_add(diff.abs() as i32);
        } else if diff < 0 {
            self.score = self.score.saturating_sub(diff.abs() as i32);
        }
    }

    pub fn try_get_parent(&self) -> Result<StorefrontId, DispatchError> {
        self.parent_id.ok_or_else(|| Error::<T>::StorefrontIsAtRoot.into())
    }
}

impl Default for StorefrontUpdate {
    fn default() -> Self {
        StorefrontUpdate {
            parent_id: None,
            handle: None,
            content: None,
            hidden: None,
            private: None,
            permissions: None,
        }
    }
}

impl<T: Trait> Module<T> {

    /// Check that there is a `Storefront` with such `storefront_id` in the storage
    /// or return`StorefrontNotFound` error.
    pub fn ensure_storefront_exists(storefront_id: StorefrontId) -> DispatchResult {
        ensure!(<StorefrontById<T>>::contains_key(storefront_id), Error::<T>::StorefrontNotFound);
        Ok(())
    }

    /// Get `Storefront` by id from the storage or return `StorefrontNotFound` error.
    pub fn require_storefront(storefront_id: StorefrontId) -> Result<Storefront<T>, DispatchError> {
        Ok(Self::storefront_by_id(storefront_id).ok_or(Error::<T>::StorefrontNotFound)?)
    }

    pub fn lowercase_and_validate_storefront_handle(handle: Vec<u8>) -> Result<Vec<u8>, DispatchError> {
        let handle_in_lowercase = Utils::<T>::lowercase_and_validate_a_handle(handle)?;

        // Check if a handle is unique across all storefronts' handles:
        ensure!(Self::storefront_id_by_handle(handle_in_lowercase.clone()).is_none(), Error::<T>::StorefrontHandleIsNotUnique);

        Ok(handle_in_lowercase)
    }

    pub fn ensure_account_has_storefront_permission(
        account: T::AccountId,
        storefront: &Storefront<T>,
        permission: StorefrontPermission,
        error: DispatchError,
    ) -> DispatchResult {
        let is_owner = storefront.is_owner(&account);
        let is_follower = storefront.is_follower(&account);

        let ctx = StorefrontPermissionsContext {
            storefront_id: storefront.id,
            is_storefront_owner: is_owner,
            is_storefront_follower: is_follower,
            storefront_perms: storefront.permissions.clone(),
        };

        T::Roles::ensure_account_has_storefront_permission(
            account,
            ctx,
            permission,
            error,
        )
    }

    pub fn try_move_storefront_to_root(storefront_id: StorefrontId) -> DispatchResult {
        let mut storefront = Self::require_storefront(storefront_id)?;
        storefront.parent_id = None;

        StorefrontById::<T>::insert(storefront_id, storefront);
        Ok(())
    }
}

impl<T: Trait> StorefrontForRolesProvider for Module<T> {
    type AccountId = T::AccountId;

    fn get_storefront(id: StorefrontId) -> Result<StorefrontForRoles<Self::AccountId>, DispatchError> {
        let storefront = Module::<T>::require_storefront(id)?;

        Ok(StorefrontForRoles {
            owner: storefront.owner,
            permissions: storefront.permissions,
        })
    }
}

pub trait BeforeStorefrontCreated<T: Trait> {
    fn before_storefront_created(follower: T::AccountId, storefront: &mut Storefront<T>) -> DispatchResult;
}

impl<T: Trait> BeforeStorefrontCreated<T> for () {
    fn before_storefront_created(_follower: T::AccountId, _storefront: &mut Storefront<T>) -> DispatchResult {
        Ok(())
    }
}

#[impl_trait_for_tuples::impl_for_tuples(10)]
pub trait AfterStorefrontUpdated<T: Trait> {
    fn after_storefront_updated(sender: T::AccountId, storefront: &Storefront<T>, old_data: StorefrontUpdate);
}
