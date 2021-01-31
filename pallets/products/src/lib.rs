#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use frame_support::{
    decl_error, decl_event, decl_module, decl_storage,
    dispatch::{DispatchError, DispatchResult}, ensure, traits::Get,
};
use sp_runtime::RuntimeDebug;
use sp_std::prelude::*;
use frame_system::{self as system, ensure_signed};

use pallet_permissions::StorefrontPermission;
use pallet_storefronts::{Module as Storefronts, Storefront, StorefrontById};
use pallet_utils::{Module as Utils, StorefrontId, WhoAndWhen, Content};

pub mod functions;

pub type ProductId = u64;

#[derive(Encode, Decode, Clone, Eq, PartialEq, RuntimeDebug)]
pub struct Product<T: Trait> {
    pub id: ProductId,
    pub created: WhoAndWhen<T>,
    pub updated: Option<WhoAndWhen<T>>,

    pub owner: T::AccountId,

    pub extension: ProductExtension,

    pub storefront_id: Option<StorefrontId>,
    pub content: Content,
    pub hidden: bool,

    pub replies_count: u16,
    pub hidden_replies_count: u16,

    pub shares_count: u16,
    pub upvotes_count: u16,
    pub downvotes_count: u16,

    pub score: i32,
}

#[derive(Encode, Decode, Clone, Eq, PartialEq, RuntimeDebug)]
pub struct ProductUpdate {
    pub storefront_id: Option<StorefrontId>,
    pub content: Option<Content>,
    pub hidden: Option<bool>,
}

#[derive(Encode, Decode, Clone, Copy, Eq, PartialEq, RuntimeDebug)]
pub enum ProductExtension {
    RegularProduct,
    Comment(Comment),
    SharedProduct(ProductId),
}

#[derive(Encode, Decode, Clone, Copy, Eq, PartialEq, RuntimeDebug)]
pub struct Comment {
    pub parent_id: Option<ProductId>,
    pub root_product_id: ProductId,
}

impl Default for ProductExtension {
    fn default() -> Self {
        ProductExtension::RegularProduct
    }
}

/// The pallet's configuration trait.
pub trait Trait: system::Trait
    + pallet_utils::Trait
    + pallet_storefronts::Trait
{
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;

    /// Max comments depth
    type MaxCommentDepth: Get<u32>;

    type ProductScores: ProductScores<Self>;

    type AfterProductUpdated: AfterProductUpdated<Self>;
}

pub trait ProductScores<T: Trait> {
    fn score_product_on_new_share(account: T::AccountId, original_product: &mut Product<T>) -> DispatchResult;
    fn score_root_product_on_new_comment(account: T::AccountId, root_product: &mut Product<T>) -> DispatchResult;
}

impl<T: Trait> ProductScores<T> for () {
    fn score_product_on_new_share(_account: T::AccountId, _original_product: &mut Product<T>) -> DispatchResult {
        Ok(())
    }
    fn score_root_product_on_new_comment(_account: T::AccountId, _root_product: &mut Product<T>) -> DispatchResult {
        Ok(())
    }
}

#[impl_trait_for_tuples::impl_for_tuples(10)]
pub trait AfterProductUpdated<T: Trait> {
    fn after_product_updated(account: T::AccountId, product: &Product<T>, old_data: ProductUpdate);
}

// This pallet's storage items.
decl_storage! {
    trait Store for Module<T: Trait> as ProductsModule {
        pub NextProductId get(fn next_product_id): ProductId = 1;

        pub ProductById get(fn product_by_id): map hasher(twox_64_concat) ProductId => Option<Product<T>>;

        pub ReplyIdsByProductId get(fn reply_ids_by_product_id):
            map hasher(twox_64_concat) ProductId => Vec<ProductId>;

        pub ProductIdsByStorefrontId get(fn product_ids_by_storefront_id):
            map hasher(twox_64_concat) StorefrontId => Vec<ProductId>;

        // TODO rename 'Shared...' to 'Sharing...'
        pub SharedProductIdsByOriginalProductId get(fn shared_product_ids_by_original_product_id):
            map hasher(twox_64_concat) ProductId => Vec<ProductId>;
    }
}

decl_event!(
    pub enum Event<T> where
        <T as system::Trait>::AccountId,
    {
        ProductCreated(AccountId, ProductId),
        ProductUpdated(AccountId, ProductId),
        ProductDeleted(AccountId, ProductId),
        ProductShared(AccountId, ProductId),
    }
);

decl_error! {
    pub enum Error for Module<T: Trait> {

        // Product related errors:

        /// Product was not found by id.
        ProductNotFound,
        /// Nothing to update in product.
        NoUpdatesForProduct,
        /// Root product should have a storefront id.
        ProductHasNoStorefrontId,
        /// Not allowed to create a product/comment when a scope (storefront or root product) is hidden.
        CannotCreateInHiddenScope,
        /// Product has no any replies
        NoRepliesOnProduct,

        // Sharing related errors:

        /// Original product not found when sharing.
        OriginalProductNotFound,
        /// Cannot share a product that shares another product.
        CannotShareSharingProduct,

        // Comment related errors:

        /// Unknown parent comment id.
        UnknownParentComment,
        /// Product by parent_id is not of Comment extension.
        NotACommentByParentId,
        /// Cannot update storefront id on comment.
        CannotUpdateStorefrontIdOnComment,
        /// Max comment depth reached.
        MaxCommentDepthReached,
        /// Only comment author can update his comment.
        NotACommentAuthor,
        /// Product extension is not a comment.
        NotComment,

        // Permissions related errors:

        /// User has no permission to create root products in this storefront.
        NoPermissionToCreateProducts,
        /// User has no permission to create comments (aka replies) in this storefront.
        NoPermissionToCreateComments,
        /// User has no permission to share products/comments from this storefront to another storefront.
        NoPermissionToShare,
        /// User is not a product author and has no permission to update products in this storefront.
        NoPermissionToUpdateAnyProduct,
        /// A product owner is not allowed to update their own products in this storefront.
        NoPermissionToUpdateOwnProducts,
        /// A comment owner is not allowed to update their own comments in this storefront.
        NoPermissionToUpdateOwnComments,
    }
}

decl_module! {
  pub struct Module<T: Trait> for enum Call where origin: T::Origin {

    const MaxCommentDepth: u32 = T::MaxCommentDepth::get();

    // Initializing errors
    type Error = Error<T>;

    // Initializing events
    fn deposit_event() = default;

    #[weight = 100_000 + T::DbWeight::get().reads_writes(8, 8)]
    pub fn create_product(
      origin,
      storefront_id_opt: Option<StorefrontId>,
      extension: ProductExtension,
      content: Content
    ) -> DispatchResult {
      let creator = ensure_signed(origin)?;

      Utils::<T>::is_valid_content(content.clone())?;

      let new_product_id = Self::next_product_id();
      let new_product: Product<T> = Product::new(new_product_id, creator.clone(), storefront_id_opt, extension, content);

      // Get storefront from either storefront_id_opt or Comment if a comment provided
      let storefront = &mut new_product.get_storefront()?;
      ensure!(!storefront.hidden, Error::<T>::CannotCreateInHiddenScope);

      let root_product = &mut new_product.get_root_product()?;
      ensure!(!root_product.hidden, Error::<T>::CannotCreateInHiddenScope);

      // Check whether account has permission to create Product (by extension)
      let mut permission_to_check = StorefrontPermission::CreateProducts;
      let mut error_on_permission_failed = Error::<T>::NoPermissionToCreateProducts;

      if let ProductExtension::Comment(_) = extension {
        permission_to_check = StorefrontPermission::CreateComments;
        error_on_permission_failed = Error::<T>::NoPermissionToCreateComments;
      }

      Storefronts::ensure_account_has_storefront_permission(
        creator.clone(),
        &storefront,
        permission_to_check,
        error_on_permission_failed.into()
      )?;

      match extension {
        ProductExtension::RegularProduct => storefront.inc_products(),
        ProductExtension::SharedProduct(product_id) => Self::create_sharing_product(&creator, new_product_id, product_id, storefront)?,
        ProductExtension::Comment(comment_ext) => Self::create_comment(&creator, new_product_id, comment_ext, root_product)?,
      }

      if new_product.is_root_product() {
        StorefrontById::insert(storefront.id, storefront.clone());
        ProductIdsByStorefrontId::mutate(storefront.id, |ids| ids.push(new_product_id));
      }

      ProductById::insert(new_product_id, new_product);
      NextProductId::mutate(|n| { *n += 1; });

      Self::deposit_event(RawEvent::ProductCreated(creator, new_product_id));
      Ok(())
    }

    #[weight = 100_000 + T::DbWeight::get().reads_writes(5, 3)]
    pub fn update_product(origin, product_id: ProductId, update: ProductUpdate) -> DispatchResult {
      let editor = ensure_signed(origin)?;

      let has_updates =
        // update.storefront_id.is_some() ||
        update.content.is_some() ||
        update.hidden.is_some();

      ensure!(has_updates, Error::<T>::NoUpdatesForProduct);

      let mut product = Self::require_product(product_id)?;

      let is_owner = product.is_owner(&editor);
      let is_comment = product.is_comment();

      let permission_to_check: StorefrontPermission;
      let permission_error: DispatchError;

      if is_comment {
        if is_owner {
          permission_to_check = StorefrontPermission::UpdateOwnComments;
          permission_error = Error::<T>::NoPermissionToUpdateOwnComments.into();
        } else {
          return Err(Error::<T>::NotACommentAuthor.into());
        }
      } else { // not a comment
        if is_owner {
          permission_to_check = StorefrontPermission::UpdateOwnProducts;
          permission_error = Error::<T>::NoPermissionToUpdateOwnProducts.into();
        } else {
          permission_to_check = StorefrontPermission::UpdateAnyProduct;
          permission_error = Error::<T>::NoPermissionToUpdateAnyProduct.into();
        }
      }

      Storefronts::ensure_account_has_storefront_permission(
        editor.clone(),
        &product.get_storefront()?,
        permission_to_check,
        permission_error
      )?;

      let mut storefront_opt: Option<Storefront<T>> = None;
      let mut is_update_applied = false;
      let mut old_data = ProductUpdate::default();

      if let Some(content) = update.content {
        if content != product.content {
          Utils::<T>::is_valid_content(content.clone())?;
          old_data.content = Some(product.content);
          product.content = content;
          is_update_applied = true;
        }
      }

      if let Some(hidden) = update.hidden {
        if hidden != product.hidden {
          storefront_opt = product.try_get_storefront().map(|mut storefront| {
            if hidden {
                storefront.inc_hidden_products();
            } else {
                storefront.dec_hidden_products();
            }

            storefront
          });

          if let ProductExtension::Comment(comment_ext) = product.extension {
            Self::update_counters_on_comment_hidden_change(&comment_ext, hidden)?;
          }

          old_data.hidden = Some(product.hidden);
          product.hidden = hidden;
          is_update_applied = true;
        }
      }

      /*
      // Move this product to another storefront:
      if let Some(storefront_id) = update.storefront_id {
        ensure!(product.is_root_product(), Error::<T>::CannotUpdateStorefrontIdOnComment);

        if let Some(product_storefront_id) = product.storefront_id {
          if storefront_id != product_storefront_id {
            Storefronts::<T>::ensure_storefront_exists(storefront_id)?;
            // TODO check that the current user has CreateProducts permission in new storefront_id.
            // TODO test whether new_storefront.products_count increases
            // TODO test whether new_storefront.hidden_products_count increases if product is hidden
            // TODO update (hidden_)replies_count of ancestors
            // TODO test whether reactions are updated correctly:
            //  - subtract score from an old storefront
            //  - add score to a new storefront

            // Remove product_id from its old storefront:
            ProductIdsByStorefrontId::mutate(product_storefront_id, |product_ids| vec_remove_on(product_ids, product_id));

            // Add product_id to its new storefront:
            ProductIdsByStorefrontId::mutate(storefront_id, |ids| ids.push(product_id));
            old_data.storefront_id = product.storefront_id;
            product.storefront_id = Some(storefront_id);
            is_update_applied = true;
          }
        }
      }
      */

      // Update this product only if at least one field should be updated:
      if is_update_applied {
        product.updated = Some(WhoAndWhen::<T>::new(editor.clone()));

        if let Some(storefront) = storefront_opt {
            <StorefrontById<T>>::insert(storefront.id, storefront);
        }

        <ProductById<T>>::insert(product.id, product.clone());
        T::AfterProductUpdated::after_product_updated(editor.clone(), &product, old_data);

        Self::deposit_event(RawEvent::ProductUpdated(editor, product_id));
      }
      Ok(())
    }
  }
}
