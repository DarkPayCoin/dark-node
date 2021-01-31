#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{
    decl_error, decl_event, decl_module, decl_storage, ensure,
    dispatch::DispatchResult,
    traits::Get
};
use sp_std::prelude::*;
use frame_system::{self as system, ensure_signed};

use df_traits::StorefrontFollowsProvider;
use pallet_profiles::{Module as Profiles, SocialAccountById};
use pallet_storefronts::{BeforeStorefrontCreated, Module as Storefronts, Storefront, StorefrontById};
use pallet_utils::{StorefrontId, vec_remove_on};

/// The pallet's configuration trait.
pub trait Trait: system::Trait
    + pallet_utils::Trait
    + pallet_storefronts::Trait
    + pallet_profiles::Trait
{
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;

    type BeforeStorefrontFollowed: BeforeStorefrontFollowed<Self>;

    type BeforeStorefrontUnfollowed: BeforeStorefrontUnfollowed<Self>;
}

decl_error! {
    pub enum Error for Module<T: Trait> {
        /// Social account was not found by id.
        SocialAccountNotFound,
        /// Account is already a storefront follower.
        AlreadyStorefrontFollower,
        /// Account is not a storefront follower.
        NotStorefrontFollower,
        /// Not allowed to follow a hidden storefront.
        CannotFollowHiddenStorefront,
    }
}

// This pallet's storage items.
decl_storage! {
    trait Store for Module<T: Trait> as StorefrontFollowsModule {
        pub StorefrontFollowers get(fn storefront_followers):
            map hasher(twox_64_concat) StorefrontId => Vec<T::AccountId>;

        pub StorefrontFollowedByAccount get(fn storefront_followed_by_account):
            map hasher(blake2_128_concat) (T::AccountId, StorefrontId) => bool;

        pub StorefrontsFollowedByAccount get(fn storefronts_followed_by_account):
            map hasher(blake2_128_concat) T::AccountId => Vec<StorefrontId>;
    }
}

decl_event!(
    pub enum Event<T> where
        <T as system::Trait>::AccountId,
    {
        StorefrontFollowed(/* follower */ AccountId, /* following */ StorefrontId),
        StorefrontUnfollowed(/* follower */ AccountId, /* unfollowing */ StorefrontId),
    }
);

// The pallet's dispatchable functions.
decl_module! {
  pub struct Module<T: Trait> for enum Call where origin: T::Origin {
    // Initializing errors
    type Error = Error<T>;

    // Initializing events
    fn deposit_event() = default;

    #[weight = 10_000 + T::DbWeight::get().reads_writes(5, 5)]
    pub fn follow_storefront(origin, storefront_id: StorefrontId) -> DispatchResult {
      let follower = ensure_signed(origin)?;

      ensure!(!Self::storefront_followed_by_account((follower.clone(), storefront_id)), Error::<T>::AlreadyStorefrontFollower);

      let storefront = &mut Storefronts::require_storefront(storefront_id)?;
      ensure!(!storefront.hidden, Error::<T>::CannotFollowHiddenStorefront);

      Self::add_storefront_follower(follower, storefront)?;
      <StorefrontById<T>>::insert(storefront_id, storefront);

      Ok(())
    }

    #[weight = 10_000 + T::DbWeight::get().reads_writes(5, 5)]
    pub fn unfollow_storefront(origin, storefront_id: StorefrontId) -> DispatchResult {
      let follower = ensure_signed(origin)?;

      ensure!(Self::storefront_followed_by_account((follower.clone(), storefront_id)), Error::<T>::NotStorefrontFollower);

      Self::unfollow_storefront_by_account(follower, storefront_id)
    }
  }
}

impl<T: Trait> Module<T> {
    fn add_storefront_follower(follower: T::AccountId, storefront: &mut Storefront<T>) -> DispatchResult {
        storefront.inc_followers();

        let mut social_account = Profiles::get_or_new_social_account(follower.clone());
        social_account.inc_following_storefronts();

        T::BeforeStorefrontFollowed::before_storefront_followed(
            follower.clone(), social_account.reputation, storefront)?;

        let storefront_id = storefront.id;
        <StorefrontFollowers<T>>::mutate(storefront_id, |followers| followers.push(follower.clone()));
        <StorefrontFollowedByAccount<T>>::insert((follower.clone(), storefront_id), true);
        <StorefrontsFollowedByAccount<T>>::mutate(follower.clone(), |storefront_ids| storefront_ids.push(storefront_id));
        <SocialAccountById<T>>::insert(follower.clone(), social_account);

        Self::deposit_event(RawEvent::StorefrontFollowed(follower, storefront_id));

        Ok(())
    }

    pub fn unfollow_storefront_by_account(follower: T::AccountId, storefront_id: StorefrontId) -> DispatchResult {
        let storefront = &mut Storefronts::require_storefront(storefront_id)?;
        storefront.dec_followers();

        let mut social_account = Profiles::social_account_by_id(follower.clone()).ok_or(Error::<T>::SocialAccountNotFound)?;
        social_account.dec_following_storefronts();

        T::BeforeStorefrontUnfollowed::before_storefront_unfollowed(follower.clone(), storefront)?;

        <StorefrontsFollowedByAccount<T>>::mutate(follower.clone(), |storefront_ids| vec_remove_on(storefront_ids, storefront_id));
        <StorefrontFollowers<T>>::mutate(storefront_id, |account_ids| vec_remove_on(account_ids, follower.clone()));
        <StorefrontFollowedByAccount<T>>::remove((follower.clone(), storefront_id));
        <SocialAccountById<T>>::insert(follower.clone(), social_account);
        <StorefrontById<T>>::insert(storefront_id, storefront);

        Self::deposit_event(RawEvent::StorefrontUnfollowed(follower, storefront_id));
        Ok(())
    }
}

impl<T: Trait> StorefrontFollowsProvider for Module<T> {
    type AccountId = T::AccountId;

    fn is_storefront_follower(account: Self::AccountId, storefront_id: StorefrontId) -> bool {
        Module::<T>::storefront_followed_by_account((account, storefront_id))
    }
}

impl<T: Trait> BeforeStorefrontCreated<T> for Module<T> {
    fn before_storefront_created(creator: T::AccountId, storefront: &mut Storefront<T>) -> DispatchResult {
        // Make a storefront creator the first follower of this storefront:
        Module::<T>::add_storefront_follower(creator, storefront)
    }
}

/// Handler that will be called right before the storefront is followed.
pub trait BeforeStorefrontFollowed<T: Trait> {
    fn before_storefront_followed(follower: T::AccountId, follower_reputation: u32, storefront: &mut Storefront<T>) -> DispatchResult;
}

impl<T: Trait> BeforeStorefrontFollowed<T> for () {
    fn before_storefront_followed(_follower: T::AccountId, _follower_reputation: u32, _storefront: &mut Storefront<T>) -> DispatchResult {
        Ok(())
    }
}

/// Handler that will be called right before the storefront is unfollowed.
pub trait BeforeStorefrontUnfollowed<T: Trait> {
    fn before_storefront_unfollowed(follower: T::AccountId, storefront: &mut Storefront<T>) -> DispatchResult;
}

impl<T: Trait> BeforeStorefrontUnfollowed<T> for () {
    fn before_storefront_unfollowed(_follower: T::AccountId, _storefront: &mut Storefront<T>) -> DispatchResult {
        Ok(())
    }
}
