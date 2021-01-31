#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use frame_support::{
    decl_error, decl_event, decl_module, decl_storage, ensure,
    dispatch::DispatchResult,
    traits::Get
};
use sp_runtime::RuntimeDebug;
use sp_std::prelude::*;
use frame_system::{self as system, ensure_signed};

use pallet_permissions::StorefrontPermission;
use pallet_products::{Module as Products, Product, ProductById, ProductId};
use pallet_storefronts::Module as Storefronts;
use pallet_utils::{vec_remove_on, WhoAndWhen};

pub type ReactionId = u64;

#[derive(Encode, Decode, Clone, Copy, Eq, PartialEq, RuntimeDebug)]
pub enum ReactionKind {
    Upvote,
    Downvote,
}

impl Default for ReactionKind {
    fn default() -> Self {
        ReactionKind::Upvote
    }
}

#[derive(Encode, Decode, Clone, Eq, PartialEq, RuntimeDebug)]
pub struct Reaction<T: Trait> {
    pub id: ReactionId,
    pub created: WhoAndWhen<T>,
    pub updated: Option<WhoAndWhen<T>>,
    pub kind: ReactionKind,
}

/// The pallet's configuration trait.
pub trait Trait: system::Trait
    + pallet_utils::Trait
    + pallet_products::Trait
    + pallet_storefronts::Trait
{
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;

    type ProductReactionScores: ProductReactionScores<Self>;
}

// This pallet's storage items.
decl_storage! {
    trait Store for Module<T: Trait> as ReactionsModule {
        pub NextReactionId get(fn next_reaction_id): ReactionId = 1;

        pub ReactionById get(fn reaction_by_id):
            map hasher(twox_64_concat) ReactionId => Option<Reaction<T>>;

        pub ReactionIdsByProductId get(fn reaction_ids_by_product_id):
            map hasher(twox_64_concat) ProductId => Vec<ReactionId>;

        pub ProductReactionIdByAccount get(fn product_reaction_id_by_account):
            map hasher(twox_64_concat) (T::AccountId, ProductId) => ReactionId;
    }
}

decl_event!(
    pub enum Event<T> where
        <T as system::Trait>::AccountId,
    {
        ProductReactionCreated(AccountId, ProductId, ReactionId),
        ProductReactionUpdated(AccountId, ProductId, ReactionId),
        ProductReactionDeleted(AccountId, ProductId, ReactionId),
    }
);

decl_error! {
    pub enum Error for Module<T: Trait> {
        /// Reaction was not found by id.
        ReactionNotFound,
        /// Account has already reacted to this product/comment.
        AccountAlreadyReacted,
        /// There is no reaction by account on this product/comment.
        ReactionByAccountNotFound,
        /// Only reaction owner can update their reaction.
        NotReactionOwner,
        /// New reaction kind is the same as old one on this product/comment.
        SameReaction,

        /// Not allowed to react on a product/comment in a hidden storefront.
        CannotReactWhenStorefrontHidden,
        /// Not allowed to react on a product/comment if a root product is hidden.
        CannotReactWhenProductHidden,

        /// User has no permission to upvote products/comments in this storefront.
        NoPermissionToUpvote,
        /// User has no permission to downvote products/comments in this storefront.
        NoPermissionToDownvote,
    }
}

decl_module! {
  pub struct Module<T: Trait> for enum Call where origin: T::Origin {

    // Initializing errors
    type Error = Error<T>;

    // Initializing events
    fn deposit_event() = default;

    #[weight = 10_000 + T::DbWeight::get().reads_writes(6, 5)]
    pub fn create_product_reaction(origin, product_id: ProductId, kind: ReactionKind) -> DispatchResult {
      let owner = ensure_signed(origin)?;

      let product = &mut Products::require_product(product_id)?;
      ensure!(
        !<ProductReactionIdByAccount<T>>::contains_key((owner.clone(), product_id)),
        Error::<T>::AccountAlreadyReacted
      );

      let storefront = product.get_storefront()?;
      ensure!(!storefront.hidden, Error::<T>::CannotReactWhenStorefrontHidden);
      ensure!(Products::<T>::is_root_product_visible(product_id)?, Error::<T>::CannotReactWhenProductHidden);

      let reaction_id = Self::insert_new_reaction(owner.clone(), kind);

      match kind {
        ReactionKind::Upvote => {
          Storefronts::ensure_account_has_storefront_permission(
            owner.clone(),
            &product.get_storefront()?,
            StorefrontPermission::Upvote,
            Error::<T>::NoPermissionToUpvote.into()
          )?;
          product.inc_upvotes();
        },
        ReactionKind::Downvote => {
          Storefronts::ensure_account_has_storefront_permission(
            owner.clone(),
            &product.get_storefront()?,
            StorefrontPermission::Downvote,
            Error::<T>::NoPermissionToDownvote.into()
          )?;
          product.inc_downvotes();
        }
      }

      if product.is_owner(&owner) {
        <ProductById<T>>::insert(product_id, product.clone());
      }

      T::ProductReactionScores::score_product_on_reaction(owner.clone(), product, kind)?;

      ReactionIdsByProductId::mutate(product.id, |ids| ids.push(reaction_id));
      <ProductReactionIdByAccount<T>>::insert((owner.clone(), product_id), reaction_id);

      Self::deposit_event(RawEvent::ProductReactionCreated(owner, product_id, reaction_id));
      Ok(())
    }

    #[weight = 10_000 + T::DbWeight::get().reads_writes(3, 2)]
    pub fn update_product_reaction(origin, product_id: ProductId, reaction_id: ReactionId, new_kind: ReactionKind) -> DispatchResult {
      let owner = ensure_signed(origin)?;

      ensure!(
        <ProductReactionIdByAccount<T>>::contains_key((owner.clone(), product_id)),
        Error::<T>::ReactionByAccountNotFound
      );

      let mut reaction = Self::reaction_by_id(reaction_id).ok_or(Error::<T>::ReactionNotFound)?;
      let product = &mut Products::require_product(product_id)?;

      ensure!(owner == reaction.created.account, Error::<T>::NotReactionOwner);
      ensure!(reaction.kind != new_kind, Error::<T>::SameReaction);

      let old_kind = reaction.kind;
      reaction.kind = new_kind;
      reaction.updated = Some(WhoAndWhen::<T>::new(owner.clone()));

      match new_kind {
        ReactionKind::Upvote => {
          product.inc_upvotes();
          product.dec_downvotes();
        },
        ReactionKind::Downvote => {
          product.inc_downvotes();
          product.dec_upvotes();
        },
      }

      T::ProductReactionScores::score_product_on_reaction(owner.clone(), product, old_kind)?;
      T::ProductReactionScores::score_product_on_reaction(owner.clone(), product, new_kind)?;

      <ReactionById<T>>::insert(reaction_id, reaction);
      <ProductById<T>>::insert(product_id, product);

      Self::deposit_event(RawEvent::ProductReactionUpdated(owner, product_id, reaction_id));
      Ok(())
    }

    #[weight = 10_000 + T::DbWeight::get().reads_writes(4, 4)]
    pub fn delete_product_reaction(origin, product_id: ProductId, reaction_id: ReactionId) -> DispatchResult {
      let owner = ensure_signed(origin)?;

      ensure!(
        <ProductReactionIdByAccount<T>>::contains_key((owner.clone(), product_id)),
        Error::<T>::ReactionByAccountNotFound
      );

      // TODO extract Self::require_reaction(reaction_id)?;
      let reaction = Self::reaction_by_id(reaction_id).ok_or(Error::<T>::ReactionNotFound)?;
      let product = &mut Products::require_product(product_id)?;

      ensure!(owner == reaction.created.account, Error::<T>::NotReactionOwner);

      match reaction.kind {
        ReactionKind::Upvote => product.dec_upvotes(),
        ReactionKind::Downvote => product.dec_downvotes(),
      }

      T::ProductReactionScores::score_product_on_reaction(owner.clone(), product, reaction.kind)?;

      <ProductById<T>>::insert(product_id, product.clone());
      <ReactionById<T>>::remove(reaction_id);
      ReactionIdsByProductId::mutate(product.id, |ids| vec_remove_on(ids, reaction_id));
      <ProductReactionIdByAccount<T>>::remove((owner.clone(), product_id));

      Self::deposit_event(RawEvent::ProductReactionDeleted(owner, product_id, reaction_id));
      Ok(())
    }
  }
}

impl<T: Trait> Module<T> {

    // FIXME: don't add reaction in storage before the checks in 'create_reaction' are done
    pub fn insert_new_reaction(account: T::AccountId, kind: ReactionKind) -> ReactionId {
        let id = Self::next_reaction_id();
        let reaction: Reaction<T> = Reaction {
            id,
            created: WhoAndWhen::<T>::new(account),
            updated: None,
            kind
        };

        <ReactionById<T>>::insert(id, reaction);
        NextReactionId::mutate(|n| { *n += 1; });

        id
    }
}

/// Handler that will be called right before the product reaction is toggled.
pub trait ProductReactionScores<T: Trait> {
    fn score_product_on_reaction(actor: T::AccountId, product: &mut Product<T>, reaction_kind: ReactionKind) -> DispatchResult;
}

impl<T: Trait> ProductReactionScores<T> for () {
    fn score_product_on_reaction(_actor: T::AccountId, _product: &mut Product<T>, _reaction_kind: ReactionKind) -> DispatchResult {
        Ok(())
    }
}
