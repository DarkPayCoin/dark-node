#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use frame_support::{
    decl_error, decl_event, decl_module, decl_storage,
    dispatch::DispatchResult, ensure, traits::Get,
};
use sp_runtime::RuntimeDebug;
use sp_std::prelude::*;
use frame_system::{self as system};

use pallet_products::{ProductScores, Product, ProductById, ProductExtension, ProductId};
use pallet_profile_follows::{BeforeAccountFollowed, BeforeAccountUnfollowed};
use pallet_profiles::{Module as Profiles, SocialAccountById};
use pallet_reactions::{ProductReactionScores, ReactionKind};
use pallet_storefront_follows::{BeforeStorefrontFollowed, BeforeStorefrontUnfollowed};
use pallet_storefronts::{Storefront, StorefrontById};
use pallet_utils::log_2;

#[derive(Encode, Decode, Clone, Copy, Eq, PartialEq, RuntimeDebug)]
pub enum ScoringAction {
    UpvoteProduct,
    DownvoteProduct,
    ShareProduct,
    CreateComment,
    UpvoteComment,
    DownvoteComment,
    ShareComment,
    FollowStorefront,
    FollowAccount,
}

impl Default for ScoringAction {
    fn default() -> Self {
        ScoringAction::FollowAccount
    }
}

/// The pallet's configuration trait.
pub trait Trait: system::Trait
    + pallet_utils::Trait
    + pallet_profiles::Trait
    + pallet_profile_follows::Trait
    + pallet_products::Trait
    + pallet_storefronts::Trait
    + pallet_storefront_follows::Trait
    + pallet_reactions::Trait
{
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;

    // Weights of the social actions
    type FollowStorefrontActionWeight: Get<i16>;
    type FollowAccountActionWeight: Get<i16>;

    type ShareProductActionWeight: Get<i16>;
    type UpvoteProductActionWeight: Get<i16>;
    type DownvoteProductActionWeight: Get<i16>;

    type CreateCommentActionWeight: Get<i16>;
    type ShareCommentActionWeight: Get<i16>;
    type UpvoteCommentActionWeight: Get<i16>;
    type DownvoteCommentActionWeight: Get<i16>;
}

decl_error! {
    pub enum Error for Module<T: Trait> {
        /// Scored account reputation difference by account and action not found.
        ReputationDiffNotFound,
        /// Product extension is a comment.
        NotRootProduct,
        /// Product extension is not a comment.
        NotComment,
    }
}

// This pallet's storage items.
decl_storage! {
    trait Store for Module<T: Trait> as ScoresModule {

        // TODO shorten name? (refactor)
        pub AccountReputationDiffByAccount get(fn account_reputation_diff_by_account):
            map hasher(blake2_128_concat) (/* actor */ T::AccountId, /* subject */ T::AccountId, ScoringAction) => Option<i16>;

        pub ProductScoreByAccount get(fn product_score_by_account):
            map hasher(blake2_128_concat) (/* actor */ T::AccountId, /* subject */ ProductId, ScoringAction) => Option<i16>;
    }
}

decl_event!(
    pub enum Event<T> where
        <T as system::Trait>::AccountId,
    {
        AccountReputationChanged(AccountId, ScoringAction, u32),
    }
);

// The pallet's dispatchable functions.
decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {

        /// Weights of the related social account actions
        const FollowStorefrontActionWeight: i16 = T::FollowStorefrontActionWeight::get();
        const FollowAccountActionWeight: i16 = T::FollowAccountActionWeight::get();
        const UpvoteProductActionWeight: i16 = T::UpvoteProductActionWeight::get();
        const DownvoteProductActionWeight: i16 = T::DownvoteProductActionWeight::get();
        const ShareProductActionWeight: i16 = T::ShareProductActionWeight::get();
        const CreateCommentActionWeight: i16 = T::CreateCommentActionWeight::get();
        const UpvoteCommentActionWeight: i16 = T::UpvoteCommentActionWeight::get();
        const DownvoteCommentActionWeight: i16 = T::DownvoteCommentActionWeight::get();
        const ShareCommentActionWeight: i16 = T::ShareCommentActionWeight::get();

        // Initializing errors
        type Error = Error<T>;

        // Initializing events
        fn deposit_event() = default;
    }
}

impl<T: Trait> Module<T> {

    pub fn scoring_action_by_product_extension(
        extension: ProductExtension,
        reaction_kind: ReactionKind,
    ) -> ScoringAction {
        match extension {
            ProductExtension::RegularProduct | ProductExtension::SharedProduct(_) => match reaction_kind {
                ReactionKind::Upvote => ScoringAction::UpvoteProduct,
                ReactionKind::Downvote => ScoringAction::DownvoteProduct,
            },
            ProductExtension::Comment(_) => match reaction_kind {
                ReactionKind::Upvote => ScoringAction::UpvoteComment,
                ReactionKind::Downvote => ScoringAction::DownvoteComment,
            },
        }
    }

    fn change_product_score_with_reaction(
        actor: T::AccountId,
        product: &mut Product<T>,
        reaction_kind: ReactionKind,
    ) -> DispatchResult {

        // Product owner should not be able to change the score of their product.
        if product.is_owner(&actor) {
            return Ok(())
        }

        let action = Self::scoring_action_by_product_extension(product.extension, reaction_kind);
        Self::change_product_score(actor, product, action)
    }

    pub fn change_product_score(
        account: T::AccountId,
        product: &mut Product<T>,
        action: ScoringAction,
    ) -> DispatchResult {
        if product.is_comment() {
            Self::change_comment_score(account, product, action)
        } else {
            Self::change_root_product_score(account, product, action)
        }
    }

    fn change_root_product_score(
        account: T::AccountId,
        product: &mut Product<T>,
        action: ScoringAction,
    ) -> DispatchResult {
        ensure!(product.is_root_product(), Error::<T>::NotRootProduct);

        let social_account = Profiles::get_or_new_social_account(account.clone());

        // TODO inspect: this insert could be redundant if the account already exists.
        <SocialAccountById<T>>::insert(account.clone(), social_account.clone());

        let product_id = product.id;

        // TODO inspect: maybe this check is redundant such as we use change_root_product_score() internally and product was already loaded.
        // Products::<T>::ensure_product_exists(product_id)?;

        // Product owner should not have any impact on their product score.
        if product.is_owner(&account) {
            return Ok(())
        }

        let mut storefront = product.get_storefront()?;

        if let Some(score_diff) = Self::product_score_by_account((account.clone(), product_id, action)) {
            let reputation_diff = Self::account_reputation_diff_by_account((account.clone(), product.owner.clone(), action))
                .ok_or(Error::<T>::ReputationDiffNotFound)?;

            // Revert this score diff:
            product.change_score(-score_diff);
            storefront.change_score(-score_diff);
            Self::change_social_account_reputation(product.owner.clone(), account.clone(), -reputation_diff, action)?;
            <ProductScoreByAccount<T>>::remove((account, product_id, action));
        } else {
            match action {
                ScoringAction::UpvoteProduct => {
                    if Self::product_score_by_account((account.clone(), product_id, ScoringAction::DownvoteProduct)).is_some() {
                        // TODO inspect this recursion. Doesn't look good:
                        Self::change_root_product_score(account.clone(), product, ScoringAction::DownvoteProduct)?;
                    }
                }
                ScoringAction::DownvoteProduct => {
                    if Self::product_score_by_account((account.clone(), product_id, ScoringAction::UpvoteProduct)).is_some() {
                        // TODO inspect this recursion. Doesn't look good:
                        Self::change_root_product_score(account.clone(), product, ScoringAction::UpvoteProduct)?;
                    }
                }
                _ => (),
            }
            let score_diff = Self::score_diff_for_action(social_account.reputation, action);
            product.change_score(score_diff);
            storefront.change_score(score_diff);
            Self::change_social_account_reputation(product.owner.clone(), account.clone(), score_diff, action)?;
            <ProductScoreByAccount<T>>::insert((account, product_id, action), score_diff);
        }

        <ProductById<T>>::insert(product_id, product.clone());
        <StorefrontById<T>>::insert(storefront.id, storefront);

        Ok(())
    }

    fn change_comment_score(
        account: T::AccountId,
        comment: &mut Product<T>,
        action: ScoringAction,
    ) -> DispatchResult {
        ensure!(comment.is_comment(), Error::<T>::NotComment);

        let social_account = Profiles::get_or_new_social_account(account.clone());

        // TODO inspect: this insert could be redundant if the account already exists.
        <SocialAccountById<T>>::insert(account.clone(), social_account.clone());

        let comment_id = comment.id;

        // TODO inspect: maybe this check is redundant such as we use change_comment_score() internally and comment was already loaded.
        // Products::<T>::ensure_product_exists(comment_id)?;

        // Comment owner should not have any impact on their comment score.
        if comment.is_owner(&account) {
            return Ok(())
        }

        if let Some(score_diff) = Self::product_score_by_account((account.clone(), comment_id, action)) {
            let reputation_diff = Self::account_reputation_diff_by_account((account.clone(), comment.owner.clone(), action))
                .ok_or(Error::<T>::ReputationDiffNotFound)?;

            // Revert this score diff:
            comment.change_score(-score_diff);
            Self::change_social_account_reputation(comment.owner.clone(), account.clone(), -reputation_diff, action)?;
            <ProductScoreByAccount<T>>::remove((account, comment_id, action));
        } else {
            match action {
                ScoringAction::UpvoteComment => {
                    if Self::product_score_by_account((account.clone(), comment_id, ScoringAction::DownvoteComment)).is_some() {
                        Self::change_comment_score(account.clone(), comment, ScoringAction::DownvoteComment)?;
                    }
                }
                ScoringAction::DownvoteComment => {
                    if Self::product_score_by_account((account.clone(), comment_id, ScoringAction::UpvoteComment)).is_some() {
                        Self::change_comment_score(account.clone(), comment, ScoringAction::UpvoteComment)?;
                    }
                }
                ScoringAction::CreateComment => {
                    let root_product = &mut comment.get_root_product()?;
                    Self::change_root_product_score(account.clone(), root_product, action)?;
                }
                _ => (),
            }
            let score_diff = Self::score_diff_for_action(social_account.reputation, action);
            comment.change_score(score_diff);
            Self::change_social_account_reputation(comment.owner.clone(), account.clone(), score_diff, action)?;
            <ProductScoreByAccount<T>>::insert((account, comment_id, action), score_diff);
        }
        <ProductById<T>>::insert(comment_id, comment.clone());

        Ok(())
    }

    // TODO change order of args to: actor (scorer), subject (account), ...
    pub fn change_social_account_reputation(
        account: T::AccountId,
        scorer: T::AccountId,
        mut score_diff: i16,
        action: ScoringAction,
    ) -> DispatchResult {

        // TODO return Ok(()) if score_diff == 0?

        // TODO seems like we can pass a &mut social account as an arg to this func
        let mut social_account = Profiles::get_or_new_social_account(account.clone());

        if social_account.reputation as i64 + score_diff as i64 <= 1 {
            social_account.reputation = 1;
            score_diff = 0;
        }

        social_account.change_reputation(score_diff);

        if Self::account_reputation_diff_by_account((scorer.clone(), account.clone(), action)).is_some() {
            <AccountReputationDiffByAccount<T>>::remove((scorer, account.clone(), action));
        } else {
            <AccountReputationDiffByAccount<T>>::insert((scorer, account.clone(), action), score_diff);
        }

        <SocialAccountById<T>>::insert(account.clone(), social_account.clone());

        Self::deposit_event(RawEvent::AccountReputationChanged(account, action, social_account.reputation));

        Ok(())
    }

    pub fn score_diff_for_action(reputation: u32, action: ScoringAction) -> i16 {
        Self::smooth_reputation(reputation) as i16 * Self::weight_of_scoring_action(action)
    }

    fn smooth_reputation(reputation: u32) -> u8 {
        log_2(reputation).map_or(1, |r| {
            let d = (reputation as u64 - (2 as u64).pow(r)) * 100
                / (2 as u64).pow(r);

            // We can safely cast this result to i16 because a score diff for u32::MAX is 32.
            (((r + 1) * 100 + d as u32) / 100) as u8
        })
    }

    fn weight_of_scoring_action(action: ScoringAction) -> i16 {
        use ScoringAction::*;
        match action {
            UpvoteProduct => T::UpvoteProductActionWeight::get(),
            DownvoteProduct => T::DownvoteProductActionWeight::get(),
            ShareProduct => T::ShareProductActionWeight::get(),
            CreateComment => T::CreateCommentActionWeight::get(),
            UpvoteComment => T::UpvoteCommentActionWeight::get(),
            DownvoteComment => T::DownvoteCommentActionWeight::get(),
            ShareComment => T::ShareCommentActionWeight::get(),
            FollowStorefront => T::FollowStorefrontActionWeight::get(),
            FollowAccount => T::FollowAccountActionWeight::get(),
        }
    }
}

impl<T: Trait> BeforeStorefrontFollowed<T> for Module<T> {
    fn before_storefront_followed(follower: T::AccountId, follower_reputation: u32, storefront: &mut Storefront<T>) -> DispatchResult {
        // Change a storefront score only if the follower is NOT a storefront owner.
        if !storefront.is_owner(&follower) {
            let storefront_owner = storefront.owner.clone();
            let action = ScoringAction::FollowStorefront;
            let score_diff = Self::score_diff_for_action(follower_reputation, action);
            storefront.change_score(score_diff);
            return Self::change_social_account_reputation(
                storefront_owner, follower, score_diff, action)
        }
        Ok(())
    }
}

impl<T: Trait> BeforeStorefrontUnfollowed<T> for Module<T> {
    fn before_storefront_unfollowed(follower: T::AccountId, storefront: &mut Storefront<T>) -> DispatchResult {
        // Change a storefront score only if the follower is NOT a storefront owner.
        if !storefront.is_owner(&follower) {
            let storefront_owner = storefront.owner.clone();
            let action = ScoringAction::FollowStorefront;
            if let Some(score_diff) = Self::account_reputation_diff_by_account(
                (follower.clone(), storefront_owner.clone(), action)
            ) {
                // Subtract a score diff that was added when this user followed this storefront in the past:
                storefront.change_score(-score_diff);
                return Self::change_social_account_reputation(
                    storefront_owner, follower, -score_diff, action)
            }
        }
        Ok(())
    }
}

impl<T: Trait> BeforeAccountFollowed<T> for Module<T> {
    fn before_account_followed(follower: T::AccountId, follower_reputation: u32, following: T::AccountId) -> DispatchResult {
        let action = ScoringAction::FollowAccount;
        let score_diff = Self::score_diff_for_action(follower_reputation, action);
        Self::change_social_account_reputation(following, follower, score_diff, action)
    }
}

impl<T: Trait> BeforeAccountUnfollowed<T> for Module<T> {
    fn before_account_unfollowed(follower: T::AccountId, following: T::AccountId) -> DispatchResult {
        let action = ScoringAction::FollowAccount;

        let rep_diff = Self::account_reputation_diff_by_account(
            (follower.clone(), following.clone(), action)
        ).ok_or(Error::<T>::ReputationDiffNotFound)?;

        Self::change_social_account_reputation(following, follower, rep_diff, action)
    }
}

impl<T: Trait> ProductScores<T> for Module<T> {
    fn score_product_on_new_share(account: T::AccountId, original_product: &mut Product<T>) -> DispatchResult {
        let action =
            if original_product.is_comment() { ScoringAction::ShareComment }
            else { ScoringAction::ShareProduct };

        let account_never_shared_this_product =
            Self::product_score_by_account(
                (account.clone(), original_product.id, action)
            ).is_none();

        // It makes sense to change a score of this product only once:
        // i.e. when this account sharing it for the first time.
        if account_never_shared_this_product {
            Self::change_product_score(account, original_product, action)
        } else {
            Ok(())
        }
    }

    fn score_root_product_on_new_comment(account: T::AccountId, root_product: &mut Product<T>) -> DispatchResult {
        Self::change_product_score(account, root_product, ScoringAction::CreateComment)
    }
}

impl<T: Trait> ProductReactionScores<T> for Module<T> {
    fn score_product_on_reaction(
        actor: T::AccountId,
        product: &mut Product<T>,
        reaction_kind: ReactionKind,
    ) -> DispatchResult {
        Self::change_product_score_with_reaction(actor, product, reaction_kind)
    }
}
