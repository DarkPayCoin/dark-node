use crate::*;

use frame_support::dispatch::DispatchError;
use pallet_products::Module as Products;
use pallet_storefronts::Storefront;
use pallet_storefront_follows::Module as StorefrontFollows;
use df_traits::moderation::*;

impl<T: Trait> Module<T> {
    pub fn require_report(report_id: ReportId) -> Result<Report<T>, DispatchError> {
        Ok(Self::report_by_id(report_id).ok_or(Error::<T>::ReportNotFound)?)
    }

    /// Get entity storefront_id if it exists.
    /// Content and Account has no scope, consider check with `if let Some`
    fn get_entity_scope(entity: &EntityId<T::AccountId>) -> Result<Option<StorefrontId>, DispatchError> {
        match entity {
            EntityId::Content(content) => {
                Utils::<T>::ensure_content_is_some(content).map(|_| None)
            },
            EntityId::Account(_) => Ok(None),
            EntityId::Storefront(storefront_id) => {
                let storefront = Storefronts::<T>::require_storefront(*storefront_id)?;
                let root_storefront_id = storefront.try_get_parent()?;

                Ok(Some(root_storefront_id))
            },
            EntityId::Product(product_id) => {
                let product = Products::<T>::require_product(*product_id)?;
                let storefront_id = product.get_storefront()?.id;

                Ok(Some(storefront_id))
            },
        }
    }

    #[allow(dead_code)]
    // fixme: do we need this?
    fn ensure_entity_exists(entity: &EntityId<T::AccountId>) -> DispatchResult {
        match entity {
            EntityId::Content(content) => Utils::<T>::ensure_content_is_some(content),
            EntityId::Account(_) => Ok(()),
            EntityId::Storefront(storefront_id) => Storefronts::<T>::ensure_storefront_exists(*storefront_id),
            EntityId::Product(product_id) => Products::<T>::ensure_product_exists(*product_id),
        }.map_err(|_| Error::<T>::EntityNotFound.into())
    }

    pub(crate) fn block_entity_in_scope(entity: &EntityId<T::AccountId>, scope: StorefrontId) -> DispatchResult {
        // TODO: update counters, when entity is moved
        // TODO: think, what and where we should change something if entity is moved
        match entity {
            EntityId::Content(_) => (),
            EntityId::Account(account_id)
                => StorefrontFollows::<T>::unfollow_storefront_by_account(account_id.clone(), scope)?,
            EntityId::Storefront(storefront_id) => Storefronts::<T>::try_move_storefront_to_root(*storefront_id)?,
            EntityId::Product(product_id) => Products::<T>::delete_product_from_storefront(*product_id)?,
        }
        StatusByEntityInStorefront::<T>::insert(entity, scope, EntityStatus::Blocked);
        Ok(())
    }

    pub(crate) fn ensure_account_status_manager(who: T::AccountId, storefront: &Storefront<T>) -> DispatchResult {
        Storefronts::<T>::ensure_account_has_storefront_permission(
            who,
            &storefront,
            pallet_permissions::StorefrontPermission::UpdateEntityStatus,
            Error::<T>::NoPermissionToUpdateEntityStatus.into(),
        )
    }

    pub(crate) fn ensure_entity_in_scope(entity: &EntityId<T::AccountId>, scope: StorefrontId) -> DispatchResult {
        if let Some(entity_scope) = Self::get_entity_scope(entity)? {
            ensure!(entity_scope == scope, Error::<T>::EntityIsNotInScope);
        }
        Ok(())
    }

    pub fn default_autoblock_threshold_as_settings() -> StorefrontModerationSettings {
        StorefrontModerationSettings {
            autoblock_threshold: Some(T::DefaultAutoblockThreshold::get())
        }
    }
}

impl<T: Trait> Report<T> {
    pub fn new(
        id: ReportId,
        created_by: T::AccountId,
        reported_entity: EntityId<T::AccountId>,
        scope: StorefrontId,
        reason: Content
    ) -> Self {
        Self {
            id,
            created: WhoAndWhen::<T>::new(created_by),
            reported_entity,
            reported_within: scope,
            reason
        }
    }
}

impl<T: Trait> SuggestedStatus<T> {
    pub fn new(who: T::AccountId, status: Option<EntityStatus>, report_id: Option<ReportId>) -> Self {
        Self {
            suggested: WhoAndWhen::<T>::new(who),
            status,
            report_id
        }
    }
}

// TODO: maybe simplify using one common trait?
impl<T: Trait> IsAccountBlocked for Module<T> {
    type AccountId = T::AccountId;

    fn is_account_blocked(account: Self::AccountId, scope: StorefrontId) -> bool {
        let entity = EntityId::Account(account);

        Self::status_by_entity_in_storefront(entity, scope) == Some(EntityStatus::Blocked)
    }
}

impl<T: Trait> IsStorefrontBlocked for Module<T> {
    type StorefrontId = StorefrontId;

    fn is_storefront_blocked(storefront_id: Self::StorefrontId, scope: StorefrontId) -> bool {
        let entity = EntityId::Storefront(storefront_id);

        Self::status_by_entity_in_storefront(entity, scope) == Some(EntityStatus::Blocked)
    }
}

impl<T: Trait> IsProductBlocked for Module<T> {
    type ProductId = ProductId;

    fn is_product_blocked(product_id: Self::ProductId, scope: StorefrontId) -> bool {
        let entity = EntityId::Product(product_id);

        Self::status_by_entity_in_storefront(entity, scope) == Some(EntityStatus::Blocked)
    }
}

impl<T: Trait> IsContentBlocked for Module<T> {
    type Content = Content;

    fn is_content_blocked(content: Self::Content, scope: StorefrontId) -> bool {
        let entity = EntityId::Content(content);

        Self::status_by_entity_in_storefront(entity, scope) == Some(EntityStatus::Blocked)
    }
}
