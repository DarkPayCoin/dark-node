use crate::{StorefrontId};

pub trait IsAccountBlocked {
    type AccountId;

    fn is_account_blocked(account: Self::AccountId, scope: StorefrontId) -> bool;
}

pub trait IsStorefrontBlocked {
    type StorefrontId;

    fn is_storefront_blocked(storefront_id: Self::StorefrontId, scope: StorefrontId) -> bool;
}

pub trait IsProductBlocked {
    type ProductId;

    fn is_product_blocked(product_id: Self::ProductId, scope: StorefrontId) -> bool;
}

pub trait IsContentBlocked {
    type Content;

    fn is_content_blocked(content: Self::Content, scope: StorefrontId) -> bool;
}
