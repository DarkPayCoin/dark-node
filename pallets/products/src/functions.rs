use frame_support::dispatch::DispatchResult;

use pallet_utils::{StorefrontId, vec_remove_on};

use super::*;

impl<T: Trait> Product<T> {

    pub fn new(
        id: ProductId,
        created_by: T::AccountId,
        storefront_id_opt: Option<StorefrontId>,
        extension: ProductExtension,
        content: Content
    ) -> Self {
        Product {
            id,
            created: WhoAndWhen::<T>::new(created_by.clone()),
            updated: None,
            owner: created_by,
            extension,
            storefront_id: storefront_id_opt,
            content,
            hidden: false,
            replies_count: 0,
            hidden_replies_count: 0,
            shares_count: 0,
            upvotes_count: 0,
            downvotes_count: 0,
            score: 0
        }
    }

    pub fn is_owner(&self, account: &T::AccountId) -> bool {
        self.owner == *account
    }

    pub fn is_root_product(&self) -> bool {
        !self.is_comment()
    }

    pub fn is_comment(&self) -> bool {
        match self.extension {
            ProductExtension::Comment(_) => true,
            _ => false,
        }
    }

    pub fn is_sharing_product(&self) -> bool {
        match self.extension {
            ProductExtension::SharedProduct(_) => true,
            _ => false,
        }
    }

    pub fn get_comment_ext(&self) -> Result<Comment, DispatchError> {
        match self.extension {
            ProductExtension::Comment(comment_ext) => Ok(comment_ext),
            _ => Err(Error::<T>::NotComment.into())
        }
    }

    pub fn get_root_product(&self) -> Result<Product<T>, DispatchError> {
        match self.extension {
            ProductExtension::RegularProduct | ProductExtension::SharedProduct(_) =>
                Ok(self.clone()),
            ProductExtension::Comment(comment) =>
                Module::require_product(comment.root_product_id),
        }
    }

    pub fn get_storefront(&self) -> Result<Storefront<T>, DispatchError> {
        let root_product = self.get_root_product()?;
        let storefront_id = root_product.storefront_id.ok_or(Error::<T>::ProductHasNoStorefrontId)?;
        Storefronts::require_storefront(storefront_id)
    }

    pub fn try_get_storefront(&self) -> Option<Storefront<T>> {
        if self.is_comment() {
            return None
        }

        if let Some(storefront_id) = self.storefront_id {
            return Storefronts::require_storefront(storefront_id).ok()
        }

        None
    }

    // TODO use macros to generate inc/dec fns for Storefront, Product.

    pub fn inc_replies(&mut self) {
        self.replies_count = self.replies_count.saturating_add(1);
    }

    pub fn dec_replies(&mut self) {
        self.replies_count = self.replies_count.saturating_sub(1);
    }

    pub fn inc_hidden_replies(&mut self) {
        self.hidden_replies_count = self.hidden_replies_count.saturating_add(1);
    }

    pub fn dec_hidden_replies(&mut self) {
        self.hidden_replies_count = self.hidden_replies_count.saturating_sub(1);
    }

    pub fn inc_shares(&mut self) {
        self.shares_count = self.shares_count.saturating_add(1);
    }

    pub fn dec_shares(&mut self) {
        self.shares_count = self.shares_count.saturating_sub(1);
    }

    pub fn inc_upvotes(&mut self) {
        self.upvotes_count = self.upvotes_count.saturating_add(1);
    }

    pub fn dec_upvotes(&mut self) {
        self.upvotes_count = self.upvotes_count.saturating_sub(1);
    }

    pub fn inc_downvotes(&mut self) {
        self.downvotes_count = self.downvotes_count.saturating_add(1);
    }

    pub fn dec_downvotes(&mut self) {
        self.downvotes_count = self.downvotes_count.saturating_sub(1);
    }

    #[allow(clippy::comparison_chain)]
    pub fn change_score(&mut self, diff: i16) {
        if diff > 0 {
            self.score = self.score.saturating_add(diff.abs() as i32);
        } else if diff < 0 {
            self.score = self.score.saturating_sub(diff.abs() as i32);
        }
    }
}

impl Default for ProductUpdate {
    fn default() -> Self {
        ProductUpdate {
            storefront_id: None,
            content: None,
            hidden: None
        }
    }
}

impl<T: Trait> Module<T> {

    /// Check that there is a `Product` with such `product_id` in the storage
    /// or return`ProductNotFound` error.
    pub fn ensure_product_exists(product_id: ProductId) -> DispatchResult {
        ensure!(<ProductById<T>>::contains_key(product_id), Error::<T>::ProductNotFound);
        Ok(())
    }

    /// Get `Product` by id from the storage or return `ProductNotFound` error.
    pub fn require_product(product_id: StorefrontId) -> Result<Product<T>, DispatchError> {
        Ok(Self::product_by_id(product_id).ok_or(Error::<T>::ProductNotFound)?)
    }

    fn share_product(
        account: T::AccountId,
        original_product: &mut Product<T>,
        shared_product_id: ProductId
    ) -> DispatchResult {
        original_product.inc_shares();

        T::ProductScores::score_product_on_new_share(account.clone(), original_product)?;

        let original_product_id = original_product.id;
        ProductById::insert(original_product_id, original_product.clone());
        SharedProductIdsByOriginalProductId::mutate(original_product_id, |ids| ids.push(shared_product_id));

        Self::deposit_event(RawEvent::ProductShared(account, original_product_id));

        Ok(())
    }

    pub fn is_root_product_hidden(product_id: ProductId) -> Result<bool, DispatchError> {
        let product = Self::require_product(product_id)?;
        let root_product = product.get_root_product()?;
        Ok(root_product.hidden)
    }

    pub fn is_root_product_visible(product_id: ProductId) -> Result<bool, DispatchError> {
        Self::is_root_product_hidden(product_id).map(|v| !v)
    }

    pub fn mutate_product_by_id<F: FnOnce(&mut Product<T>)> (
        product_id: ProductId,
        f: F
    ) -> Result<Product<T>, DispatchError> {
        <ProductById<T>>::mutate(product_id, |product_opt| {
            if let Some(ref mut product) = product_opt.clone() {
                f(product);
                *product_opt = Some(product.clone());

                return Ok(product.clone());
            }

            Err(Error::<T>::ProductNotFound.into())
        })
    }

    // TODO refactor to a tail recursion
    /// Get all product ancestors (parent_id) including this product
    pub fn get_product_ancestors(product_id: ProductId) -> Vec<Product<T>> {
        let mut ancestors: Vec<Product<T>> = Vec::new();

        if let Some(product) = Self::product_by_id(product_id) {
            ancestors.push(product.clone());
            if let Some(parent_id) = product.get_comment_ext().ok().unwrap().parent_id {
                ancestors.extend(Self::get_product_ancestors(parent_id).iter().cloned());
            }
        }

        ancestors
    }

    /// Applies function to all product ancestors (parent_id) including this product
    pub fn for_each_product_ancestor<F: FnMut(&mut Product<T>) + Copy> (
        product_id: ProductId,
        f: F
    ) -> DispatchResult {
        let product = Self::mutate_product_by_id(product_id, f)?;

        if let ProductExtension::Comment(comment_ext) = product.extension {
            if let Some(parent_id) = comment_ext.parent_id {
                Self::for_each_product_ancestor(parent_id, f)?;
            }
        }

        Ok(())
    }

    fn try_get_product_replies(product_id: ProductId) -> Vec<Product<T>> {
        let mut replies: Vec<Product<T>> = Vec::new();

        if let Some(product) = Self::product_by_id(product_id) {
            replies.push(product);
            for reply_id in Self::reply_ids_by_product_id(product_id).iter() {
                replies.extend(Self::try_get_product_replies(*reply_id).iter().cloned());
            }
        }

        replies
    }

    /// Recursively et all nested product replies (reply_ids_by_product_id)
    pub fn get_product_replies(product_id: ProductId) -> Result<Vec<Product<T>>, DispatchError> {
        let reply_ids = Self::reply_ids_by_product_id(product_id);
        ensure!(!reply_ids.is_empty(), Error::<T>::NoRepliesOnProduct);

        let mut replies: Vec<Product<T>> = Vec::new();
        for reply_id in reply_ids.iter() {
            replies.extend(Self::try_get_product_replies(*reply_id));
        }
        Ok(replies)
    }
    // TODO: maybe add for_each_reply?

    pub(crate) fn create_comment(
        creator: &T::AccountId,
        new_product_id: ProductId,
        comment_ext: Comment,
        root_product: &mut Product<T>
    ) -> DispatchResult {
        let mut commented_product_id = root_product.id;

        if let Some(parent_id) = comment_ext.parent_id {
            let parent_comment = Self::product_by_id(parent_id).ok_or(Error::<T>::UnknownParentComment)?;
            ensure!(parent_comment.is_comment(), Error::<T>::NotACommentByParentId);

            let ancestors = Self::get_product_ancestors(parent_id);
            ensure!(ancestors.len() < T::MaxCommentDepth::get() as usize, Error::<T>::MaxCommentDepthReached);

            commented_product_id = parent_id;
        }

        root_product.inc_replies();
        T::ProductScores::score_root_product_on_new_comment(creator.clone(), root_product)?;

        Self::for_each_product_ancestor(commented_product_id, |product| product.inc_replies())?;
        ProductById::insert(root_product.id, root_product);
        ReplyIdsByProductId::mutate(commented_product_id, |ids| ids.push(new_product_id));

        Ok(())
    }

    pub(crate) fn create_sharing_product(
        creator: &T::AccountId,
        new_product_id: ProductId,
        original_product_id: ProductId,
        storefront: &mut Storefront<T>
    ) -> DispatchResult {
        let original_product = &mut Self::product_by_id(original_product_id)
            .ok_or(Error::<T>::OriginalProductNotFound)?;

        ensure!(!original_product.is_sharing_product(), Error::<T>::CannotShareSharingProduct);

        // Check if it's allowed to share a product from the storefront of original product.
        Storefronts::ensure_account_has_storefront_permission(
            creator.clone(),
            &original_product.get_storefront()?,
            StorefrontPermission::Share,
            Error::<T>::NoPermissionToShare.into()
        )?;

        storefront.inc_products();

        Self::share_product(creator.clone(), original_product, new_product_id)
    }

    pub fn delete_product_from_storefront(product_id: ProductId) -> DispatchResult {
        let mut product = Self::require_product(product_id)?;

        if let ProductExtension::Comment(comment_ext) = product.extension {
            product.extension = ProductExtension::RegularProduct;

            let root_product = &mut Self::require_product(comment_ext.root_product_id)?;
            let parent_id = comment_ext.parent_id.unwrap_or(root_product.id);

            // Choose desired counter change whether comment was hidden or not
            let mut update_replies_change: fn(&mut Product<T>) = Product::dec_replies;
            if product.hidden {
                update_replies_change = Product::dec_hidden_replies;
            }

            update_replies_change(root_product);
            ProductById::<T>::insert(root_product.id, root_product.clone());
            Self::for_each_product_ancestor(parent_id, |p| update_replies_change(p))?;

            // Subtract CreateComment score weight on root product and its storefront
            T::ProductScores::score_root_product_on_new_comment(product.created.account, root_product)?;
            let replies = Self::get_product_replies(product_id)?;
            for reply in replies.iter() {
                T::ProductScores::score_root_product_on_new_comment(reply.created.account.clone(), root_product)?;
            }
        } else {
            let mut storefront = product.get_storefront()?;
            product.storefront_id = None;
            if product.hidden {
                storefront.hidden_products_count = storefront.hidden_products_count.saturating_sub(1);
            } else {
                storefront.products_count = storefront.products_count.saturating_sub(1);
            }

            storefront.score = storefront.score.saturating_sub(product.score);

            ProductIdsByStorefrontId::mutate(storefront.id, |product_ids| vec_remove_on(product_ids, product_id));
        }

        Ok(())
    }

    /// Rewrite ancestor counters when Product hidden status changes
    /// Warning: This will affect storage state!
    pub(crate) fn update_counters_on_comment_hidden_change(
        comment_ext: &Comment,
        becomes_hidden: bool
    ) -> DispatchResult {
        let root_product = &mut Self::require_product(comment_ext.root_product_id)?;
        let commented_product_id = comment_ext.parent_id.unwrap_or(root_product.id);

        let mut update_hidden_replies: fn(&mut Product<T>) = Product::inc_hidden_replies;
        if !becomes_hidden {
            update_hidden_replies = Product::dec_hidden_replies;
        }

        Self::for_each_product_ancestor(commented_product_id, |product| update_hidden_replies(product))?;

        update_hidden_replies(root_product);
        ProductById::insert(root_product.id, root_product);

        Ok(())
    }
}
