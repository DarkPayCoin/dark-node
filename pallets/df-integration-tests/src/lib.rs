#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod tests {
    use frame_support::{
        assert_ok, assert_noop,
        impl_outer_origin, parameter_types,
        weights::Weight,
        dispatch::DispatchResult,
        storage::StorageMap,
    };
    use sp_core::H256;
    use sp_io::TestExternalities;
    use sp_std::iter::FromIterator;
    use sp_runtime::{
        traits::{BlakeTwo256, IdentityLookup},
        testing::Header,
        Perbill,
    };
    use frame_system::{self as system};

    use pallet_permissions::{
        StorefrontPermission,
        StorefrontPermission as SP,
        StorefrontPermissionSet,
        StorefrontPermissions,
    };
    use pallet_products::{ProductId, Product, ProductUpdate, ProductExtension, Comment, Error as ProductsError};
    use pallet_profiles::{ProfileUpdate, Error as ProfilesError};
    use pallet_profile_follows::Error as ProfileFollowsError;
    use pallet_reactions::{ReactionId, ReactionKind, ProductReactionScores, Error as ReactionsError};
    use pallet_scores::ScoringAction;
    use pallet_storefronts::{StorefrontById, StorefrontUpdate, Error as StorefrontsError};
    use pallet_storefront_follows::Error as StorefrontFollowsError;
    use pallet_storefront_ownership::Error as StorefrontOwnershipError;
    use pallet_utils::{StorefrontId, Error as UtilsError, User, Content};

    impl_outer_origin! {
        pub enum Origin for TestRuntime {}
    }

    #[derive(Clone, Eq, PartialEq)]
    pub struct TestRuntime;

    parameter_types! {
        pub const BlockHashCount: u64 = 250;
        pub const MaximumBlockWeight: Weight = 1024;
        pub const MaximumBlockLength: u32 = 2 * 1024;
        pub const AvailableBlockRatio: Perbill = Perbill::from_percent(75);
    }

    impl system::Trait for TestRuntime {
        type BaseCallFilter = ();
        type Origin = Origin;
        type Call = ();
        type Index = u64;
        type BlockNumber = u64;
        type Hash = H256;
        type Hashing = BlakeTwo256;
        type AccountId = u64;
        type Lookup = IdentityLookup<Self::AccountId>;
        type Header = Header;
        type Event = ();
        type BlockHashCount = BlockHashCount;
        type MaximumBlockWeight = MaximumBlockWeight;
        type DbWeight = ();
        type BlockExecutionWeight = ();
        type ExtrinsicBaseWeight = ();
        type MaximumExtrinsicWeight = MaximumBlockWeight;
        type MaximumBlockLength = MaximumBlockLength;
        type AvailableBlockRatio = AvailableBlockRatio;
        type Version = ();
        type ModuleToIndex = ();
        type AccountData = pallet_balances::AccountData<u64>;
        type OnNewAccount = ();
        type OnKilledAccount = ();
    }

    parameter_types! {
        pub const MinimumPeriod: u64 = 5;
    }

    impl pallet_timestamp::Trait for TestRuntime {
        type Moment = u64;
        type OnTimestampSet = ();
        type MinimumPeriod = MinimumPeriod;
    }

    parameter_types! {
        pub const ExistentialDeposit: u64 = 1;
    }

    impl pallet_balances::Trait for TestRuntime {
        type Balance = u64;
        type DustRemoval = ();
        type Event = ();
        type ExistentialDeposit = ExistentialDeposit;
        type AccountStore = System;
    }

    parameter_types! {
      pub const MinHandleLen: u32 = 5;
      pub const MaxHandleLen: u32 = 50;
    }

    impl pallet_utils::Trait for TestRuntime {
        type Event = ();
        type Currency = Balances;
        type MinHandleLen = MinHandleLen;
        type MaxHandleLen = MaxHandleLen;
    }

    parameter_types! {
      pub DefaultStorefrontPermissions: StorefrontPermissions = StorefrontPermissions {

        // No permissions disabled by default
        none: None,

        everyone: Some(StorefrontPermissionSet::from_iter(vec![
            SP::UpdateOwnSubstorefronts,
            SP::DeleteOwnSubstorefronts,
            SP::HideOwnSubstorefronts,

            SP::UpdateOwnProducts,
            SP::DeleteOwnProducts,
            SP::HideOwnProducts,

            SP::CreateComments,
            SP::UpdateOwnComments,
            SP::DeleteOwnComments,
            SP::HideOwnComments,

            SP::Upvote,
            SP::Downvote,
            SP::Share,
        ].into_iter())),

        // Followers can do everything that everyone else can.
        follower: None,

        storefront_owner: Some(StorefrontPermissionSet::from_iter(vec![
            SP::ManageRoles,
            SP::RepresentStorefrontInternally,
            SP::RepresentStorefrontExternally,
            SP::OverrideSubstorefrontPermissions,
            SP::OverrideProductPermissions,

            SP::CreateSubstorefronts,
            SP::CreateProducts,

            SP::UpdateStorefront,
            SP::UpdateAnySubstorefront,
            SP::UpdateAnyProduct,

            SP::DeleteAnySubstorefront,
            SP::DeleteAnyProduct,

            SP::HideAnySubstorefront,
            SP::HideAnyProduct,
            SP::HideAnyComment,

            SP::SuggestEntityStatus,
            SP::UpdateEntityStatus,

            SP::UpdateStorefrontSettings,
        ].into_iter())),
      };
    }

    impl pallet_permissions::Trait for TestRuntime {
        type DefaultStorefrontPermissions = DefaultStorefrontPermissions;
    }

    parameter_types! {
        pub const MaxCommentDepth: u32 = 10;
    }

    impl pallet_products::Trait for TestRuntime {
        type Event = ();
        type MaxCommentDepth = MaxCommentDepth;
        type ProductScores = Scores;
        type AfterProductUpdated = ProductHistory;
    }

    parameter_types! {}

    impl pallet_product_history::Trait for TestRuntime {}

    parameter_types! {}

    impl pallet_profile_follows::Trait for TestRuntime {
        type Event = ();
        type BeforeAccountFollowed = Scores;
        type BeforeAccountUnfollowed = Scores;
    }

    parameter_types! {}

    impl pallet_profiles::Trait for TestRuntime {
        type Event = ();
        type AfterProfileUpdated = ProfileHistory;
    }

    parameter_types! {}

    impl pallet_profile_history::Trait for TestRuntime {}

    parameter_types! {}

    impl pallet_reactions::Trait for TestRuntime {
        type Event = ();
        type ProductReactionScores = Scores;
    }

    parameter_types! {
        pub const MaxUsersToProcessPerDeleteRole: u16 = 40;
    }

    impl pallet_roles::Trait for TestRuntime {
        type Event = ();
        type MaxUsersToProcessPerDeleteRole = MaxUsersToProcessPerDeleteRole;
        type Storefronts = Storefronts;
        type StorefrontFollows = StorefrontFollows;
    }

    parameter_types! {
        pub const FollowStorefrontActionWeight: i16 = 7;
        pub const FollowAccountActionWeight: i16 = 3;

        pub const ShareProductActionWeight: i16 = 7;
        pub const UpvoteProductActionWeight: i16 = 5;
        pub const DownvoteProductActionWeight: i16 = -3;

        pub const CreateCommentActionWeight: i16 = 5;
        pub const ShareCommentActionWeight: i16 = 5;
        pub const UpvoteCommentActionWeight: i16 = 4;
        pub const DownvoteCommentActionWeight: i16 = -2;
    }

    impl pallet_scores::Trait for TestRuntime {
        type Event = ();

        type FollowStorefrontActionWeight = FollowStorefrontActionWeight;
        type FollowAccountActionWeight = FollowAccountActionWeight;

        type ShareProductActionWeight = ShareProductActionWeight;
        type UpvoteProductActionWeight = UpvoteProductActionWeight;
        type DownvoteProductActionWeight = DownvoteProductActionWeight;

        type CreateCommentActionWeight = CreateCommentActionWeight;
        type ShareCommentActionWeight = ShareCommentActionWeight;
        type UpvoteCommentActionWeight = UpvoteCommentActionWeight;
        type DownvoteCommentActionWeight = DownvoteCommentActionWeight;
    }

    parameter_types! {}

    impl pallet_storefront_follows::Trait for TestRuntime {
        type Event = ();
        type BeforeStorefrontFollowed = Scores;
        type BeforeStorefrontUnfollowed = Scores;
    }

    parameter_types! {}

    impl pallet_storefront_ownership::Trait for TestRuntime {
        type Event = ();
    }

    parameter_types! {}

    impl pallet_storefronts::Trait for TestRuntime {
        type Event = ();
        type Roles = Roles;
        type StorefrontFollows = StorefrontFollows;
        type BeforeStorefrontCreated = StorefrontFollows;
        type AfterStorefrontUpdated = StorefrontHistory;
        type StorefrontCreationFee = ();
    }

    parameter_types! {}

    impl pallet_storefront_history::Trait for TestRuntime {}

    type System = system::Module<TestRuntime>;
    type Balances = pallet_balances::Module<TestRuntime>;

    type Products = pallet_products::Module<TestRuntime>;
    type ProductHistory = pallet_product_history::Module<TestRuntime>;
    type ProfileFollows = pallet_profile_follows::Module<TestRuntime>;
    type Profiles = pallet_profiles::Module<TestRuntime>;
    type ProfileHistory = pallet_profile_history::Module<TestRuntime>;
    type Reactions = pallet_reactions::Module<TestRuntime>;
    type Roles = pallet_roles::Module<TestRuntime>;
    type Scores = pallet_scores::Module<TestRuntime>;
    type StorefrontFollows = pallet_storefront_follows::Module<TestRuntime>;
    type StorefrontHistory = pallet_storefront_history::Module<TestRuntime>;
    type StorefrontOwnership = pallet_storefront_ownership::Module<TestRuntime>;
    type Storefronts = pallet_storefronts::Module<TestRuntime>;

    pub type AccountId = u64;
    type BlockNumber = u64;


    pub struct ExtBuilder;

    // TODO: make created storefront/product/comment configurable or by default
    impl ExtBuilder {
        /// Default ext configuration with BlockNumber 1
        pub fn build() -> TestExternalities {
            let storage = system::GenesisConfig::default()
                .build_storage::<TestRuntime>()
                .unwrap();

            let mut ext = TestExternalities::from(storage);
            ext.execute_with(|| System::set_block_number(1));

            ext
        }

        /// Custom ext configuration with StorefrontId 1 and BlockNumber 1
        pub fn build_with_storefront() -> TestExternalities {
            let storage = system::GenesisConfig::default()
                .build_storage::<TestRuntime>()
                .unwrap();

            let mut ext = TestExternalities::from(storage);
            ext.execute_with(|| {
                System::set_block_number(1);
                assert_ok!(_create_default_storefront());
            });

            ext
        }

        /// Custom ext configuration with StorefrontId 1, ProductId 1 and BlockNumber 1
        pub fn build_with_product() -> TestExternalities {
            let storage = system::GenesisConfig::default()
                .build_storage::<TestRuntime>()
                .unwrap();

            let mut ext = TestExternalities::from(storage);
            ext.execute_with(|| {
                System::set_block_number(1);
                assert_ok!(_create_default_storefront());
                assert_ok!(_create_default_product());
            });

            ext
        }

        /// Custom ext configuration with StorefrontId 1, ProductId 1, ProductId 2 (as comment) and BlockNumber 1
        pub fn build_with_comment() -> TestExternalities {
            let storage = system::GenesisConfig::default()
                .build_storage::<TestRuntime>()
                .unwrap();

            let mut ext = TestExternalities::from(storage);
            ext.execute_with(|| {
                System::set_block_number(1);
                assert_ok!(_create_default_storefront());
                assert_ok!(_create_default_product());
                assert_ok!(_create_default_comment());
            });

            ext
        }

        /// Custom ext configuration with pending ownership transfer without Storefront
        pub fn build_with_pending_ownership_transfer_no_storefront() -> TestExternalities {
            let storage = system::GenesisConfig::default()
                .build_storage::<TestRuntime>()
                .unwrap();

            let mut ext = TestExternalities::from(storage);
            ext.execute_with(|| {
                System::set_block_number(1);

                assert_ok!(_create_default_storefront());
                assert_ok!(_transfer_default_storefront_ownership());

                <StorefrontById<TestRuntime>>::remove(SPACE1);
            });

            ext
        }

        /// Custom ext configuration with specified permissions granted (includes StorefrontId 1)
        pub fn build_with_a_few_roles_granted_to_account2(perms: Vec<SP>) -> TestExternalities {
            let storage = system::GenesisConfig::default()
                .build_storage::<TestRuntime>()
                .unwrap();

            let mut ext = TestExternalities::from(storage);
            ext.execute_with(|| {
                System::set_block_number(1);
                let user = User::Account(ACCOUNT2);

                assert_ok!(_create_default_storefront());

                assert_ok!(_create_role(
                    None,
                    None,
                    None,
                    None,
                    Some(perms)
                ));
                // RoleId 1
                assert_ok!(_create_default_role()); // RoleId 2

                assert_ok!(_grant_role(None, Some(ROLE1), Some(vec![user.clone()])));
                assert_ok!(_grant_role(None, Some(ROLE2), Some(vec![user])));
            });

            ext
        }

        /// Custom ext configuration with storefront follow without Storefront
        pub fn build_with_storefront_follow_no_storefront() -> TestExternalities {
            let storage = system::GenesisConfig::default()
                .build_storage::<TestRuntime>()
                .unwrap();

            let mut ext = TestExternalities::from(storage);
            ext.execute_with(|| {
                System::set_block_number(1);

                assert_ok!(_create_default_storefront());
                assert_ok!(_default_follow_storefront());

                <StorefrontById<TestRuntime>>::remove(SPACE1);
            });

            ext
        }
    }


    /* Integrated tests mocks */

    const ACCOUNT1: AccountId = 1;
    const ACCOUNT2: AccountId = 2;
    const ACCOUNT3: AccountId = 3;

    const SPACE1: StorefrontId = 1001;
    const SPACE2: StorefrontId = 1002;
    const _SPACE3: StorefrontId = 1003;

    const POST1: ProductId = 1;
    const POST2: ProductId = 2;
    const POST3: ProductId = 3;

    const REACTION1: ReactionId = 1;
    const REACTION2: ReactionId = 2;
    const _REACTION3: ReactionId = 3;

    fn storefront_handle() -> Vec<u8> {
        b"storefront_handle".to_vec()
    }

    fn storefront_content_ipfs() -> Content {
        Content::IPFS(b"QmRAQB6YaCyidP37UdDnjFY5vQuiBrcqdyoW1CuDgwxkD4".to_vec())
    }

    fn storefront_update(
        parent_id: Option<Option<StorefrontId>>,
        handle: Option<Option<Vec<u8>>>,
        content: Option<Content>,
        hidden: Option<bool>,
        permissions: Option<Option<StorefrontPermissions>>
    ) -> StorefrontUpdate {
        StorefrontUpdate {
            parent_id,
            handle,
            content,
            hidden,
            permissions
        }
    }

    fn product_content_ipfs() -> Content {
        Content::IPFS(b"QmRAQB6YaCyidP37UdDnjFY5vQuiBrcqdyoW2CuDgwxkD4".to_vec())
    }

    fn product_update(
        storefront_id: Option<StorefrontId>,
        content: Option<Content>,
        hidden: Option<bool>
    ) -> ProductUpdate {
        ProductUpdate {
            storefront_id,
            content,
            hidden,
        }
    }

    fn comment_content_ipfs() -> Content {
        Content::IPFS(b"QmRAQB6YaCyidP37UdDnjFY5vQuiBrcqdyoW1CuDgwxkD4".to_vec())
    }

    fn reply_content_ipfs() -> Content {
        Content::IPFS(b"QmYA2fn8cMbVWo4v95RwcwJVyQsNtnEwHerfWR8UNtEwoE".to_vec())
    }

    fn profile_content_ipfs() -> Content {
        Content::IPFS(b"QmRAQB6YaCyidP37UdDnjFY5vQuiaRtqdyoW2CuDgwxkA5".to_vec())
    }

    fn reaction_upvote() -> ReactionKind {
        ReactionKind::Upvote
    }

    fn reaction_downvote() -> ReactionKind {
        ReactionKind::Downvote
    }

    fn scoring_action_upvote_product() -> ScoringAction {
        ScoringAction::UpvoteProduct
    }

    fn scoring_action_downvote_product() -> ScoringAction {
        ScoringAction::DownvoteProduct
    }

    fn scoring_action_share_product() -> ScoringAction {
        ScoringAction::ShareProduct
    }

    fn scoring_action_create_comment() -> ScoringAction {
        ScoringAction::CreateComment
    }

    fn scoring_action_upvote_comment() -> ScoringAction {
        ScoringAction::UpvoteComment
    }

    fn scoring_action_downvote_comment() -> ScoringAction {
        ScoringAction::DownvoteComment
    }

    fn scoring_action_share_comment() -> ScoringAction {
        ScoringAction::ShareComment
    }

    fn scoring_action_follow_storefront() -> ScoringAction {
        ScoringAction::FollowStorefront
    }

    fn scoring_action_follow_account() -> ScoringAction {
        ScoringAction::FollowAccount
    }

    fn extension_regular_product() -> ProductExtension {
        ProductExtension::RegularProduct
    }

    fn extension_comment(parent_id: Option<ProductId>, root_product_id: ProductId) -> ProductExtension {
        ProductExtension::Comment(Comment { parent_id, root_product_id })
    }

    fn extension_shared_product(product_id: ProductId) -> ProductExtension {
        ProductExtension::SharedProduct(product_id)
    }

    fn _create_default_storefront() -> DispatchResult {
        _create_storefront(None, None, None, None)
    }

    fn _create_storefront(
        origin: Option<Origin>,
        parent_id_opt: Option<Option<StorefrontId>>,
        handle: Option<Option<Vec<u8>>>,
        content: Option<Content>
    ) -> DispatchResult {
        Storefronts::create_storefront(
            origin.unwrap_or_else(|| Origin::signed(ACCOUNT1)),
            parent_id_opt.unwrap_or(None),
            handle.unwrap_or_else(|| Some(self::storefront_handle())),
            content.unwrap_or_else(self::storefront_content_ipfs),
        )
    }

    fn _update_storefront(
        origin: Option<Origin>,
        storefront_id: Option<StorefrontId>,
        update: Option<StorefrontUpdate>
    ) -> DispatchResult {
        Storefronts::update_storefront(
            origin.unwrap_or_else(|| Origin::signed(ACCOUNT1)),
            storefront_id.unwrap_or(SPACE1),
            update.unwrap_or_else(|| self::storefront_update(None, None, None, None, None)),
        )
    }

    fn _default_follow_storefront() -> DispatchResult {
        _follow_storefront(None, None)
    }

    fn _follow_storefront(origin: Option<Origin>, storefront_id: Option<StorefrontId>) -> DispatchResult {
        StorefrontFollows::follow_storefront(
            origin.unwrap_or_else(|| Origin::signed(ACCOUNT2)),
            storefront_id.unwrap_or(SPACE1),
        )
    }

    fn _default_unfollow_storefront() -> DispatchResult {
        _unfollow_storefront(None, None)
    }

    fn _unfollow_storefront(origin: Option<Origin>, storefront_id: Option<StorefrontId>) -> DispatchResult {
        StorefrontFollows::unfollow_storefront(
            origin.unwrap_or_else(|| Origin::signed(ACCOUNT2)),
            storefront_id.unwrap_or(SPACE1),
        )
    }

    fn _create_default_product() -> DispatchResult {
        _create_product(None, None, None, None)
    }

    fn _create_product(
        origin: Option<Origin>,
        storefront_id_opt: Option<Option<StorefrontId>>,
        extension: Option<ProductExtension>,
        content: Option<Content>
    ) -> DispatchResult {
        Products::create_product(
            origin.unwrap_or_else(|| Origin::signed(ACCOUNT1)),
            storefront_id_opt.unwrap_or(Some(SPACE1)),
            extension.unwrap_or_else(self::extension_regular_product),
            content.unwrap_or_else(self::product_content_ipfs),
        )
    }

    fn _update_product(
        origin: Option<Origin>,
        product_id: Option<ProductId>,
        update: Option<ProductUpdate>,
    ) -> DispatchResult {
        Products::update_product(
            origin.unwrap_or_else(|| Origin::signed(ACCOUNT1)),
            product_id.unwrap_or(POST1),
            update.unwrap_or_else(|| self::product_update(None, None, None)),
        )
    }

    fn _create_default_comment() -> DispatchResult {
        _create_comment(None, None, None, None)
    }

    fn _create_comment(
        origin: Option<Origin>,
        product_id: Option<ProductId>,
        parent_id: Option<Option<ProductId>>,
        content: Option<Content>,
    ) -> DispatchResult {
        _create_product(
            origin,
            Some(None),
            Some(self::extension_comment(
                parent_id.unwrap_or(None),
                product_id.unwrap_or(POST1)
            )),
            Some(content.unwrap_or_else(self::comment_content_ipfs)),
        )
    }

    fn _update_comment(
        origin: Option<Origin>,
        product_id: Option<ProductId>,
        update: Option<ProductUpdate>
    ) -> DispatchResult {
        _update_product(
            origin,
            Some(product_id.unwrap_or(POST2)),
            Some(update.unwrap_or_else(||
                self::product_update(None, Some(self::reply_content_ipfs()), None))
            ),
        )
    }

    fn _create_default_product_reaction() -> DispatchResult {
        _create_product_reaction(None, None, None)
    }

    fn _create_default_comment_reaction() -> DispatchResult {
        _create_comment_reaction(None, None, None)
    }

    fn _create_product_reaction(
        origin: Option<Origin>,
        product_id: Option<ProductId>,
        kind: Option<ReactionKind>
    ) -> DispatchResult {
        Reactions::create_product_reaction(
            origin.unwrap_or_else(|| Origin::signed(ACCOUNT1)),
            product_id.unwrap_or(POST1),
            kind.unwrap_or_else(self::reaction_upvote),
        )
    }

    fn _create_comment_reaction(
        origin: Option<Origin>,
        product_id: Option<ProductId>,
        kind: Option<ReactionKind>
    ) -> DispatchResult {
        _create_product_reaction(origin, Some(product_id.unwrap_or(2)), kind)
    }

    fn _update_product_reaction(
        origin: Option<Origin>,
        product_id: Option<ProductId>,
        reaction_id: ReactionId,
        kind: Option<ReactionKind>
    ) -> DispatchResult {
        Reactions::update_product_reaction(
            origin.unwrap_or_else(|| Origin::signed(ACCOUNT1)),
            product_id.unwrap_or(POST1),
            reaction_id,
            kind.unwrap_or_else(self::reaction_upvote),
        )
    }

    fn _update_comment_reaction(
        origin: Option<Origin>,
        product_id: Option<ProductId>,
        reaction_id: ReactionId,
        kind: Option<ReactionKind>
    ) -> DispatchResult {
        _update_product_reaction(origin, Some(product_id.unwrap_or(2)), reaction_id, kind)
    }

    fn _delete_product_reaction(
        origin: Option<Origin>,
        product_id: Option<ProductId>,
        reaction_id: ReactionId
    ) -> DispatchResult {
        Reactions::delete_product_reaction(
            origin.unwrap_or_else(|| Origin::signed(ACCOUNT1)),
            product_id.unwrap_or(POST1),
            reaction_id,
        )
    }

    fn _delete_comment_reaction(
        origin: Option<Origin>,
        product_id: Option<ProductId>,
        reaction_id: ReactionId
    ) -> DispatchResult {
        _delete_product_reaction(origin, Some(product_id.unwrap_or(2)), reaction_id)
    }

    fn _create_default_profile() -> DispatchResult {
        _create_profile(None, None)
    }

    fn _create_profile(
        origin: Option<Origin>,
        content: Option<Content>
    ) -> DispatchResult {
        Profiles::create_profile(
            origin.unwrap_or_else(|| Origin::signed(ACCOUNT1)),
            content.unwrap_or_else(self::profile_content_ipfs),
        )
    }

    fn _update_profile(
        origin: Option<Origin>,
        content: Option<Content>
    ) -> DispatchResult {
        Profiles::update_profile(
            origin.unwrap_or_else(|| Origin::signed(ACCOUNT1)),
            ProfileUpdate {
                content,
            },
        )
    }

    fn _default_follow_account() -> DispatchResult {
        _follow_account(None, None)
    }

    fn _follow_account(origin: Option<Origin>, account: Option<AccountId>) -> DispatchResult {
        ProfileFollows::follow_account(
            origin.unwrap_or_else(|| Origin::signed(ACCOUNT2)),
            account.unwrap_or(ACCOUNT1),
        )
    }

    fn _default_unfollow_account() -> DispatchResult {
        _unfollow_account(None, None)
    }

    fn _unfollow_account(origin: Option<Origin>, account: Option<AccountId>) -> DispatchResult {
        ProfileFollows::unfollow_account(
            origin.unwrap_or_else(|| Origin::signed(ACCOUNT2)),
            account.unwrap_or(ACCOUNT1),
        )
    }

    fn _score_product_on_reaction_with_id(
        account: AccountId,
        product_id: ProductId,
        kind: ReactionKind
    ) -> DispatchResult {
        if let Some(ref mut product) = Products::product_by_id(product_id) {
            Scores::score_product_on_reaction(account, product, kind)
        } else {
            panic!("Test error. Product\\Comment with specified ID not found.");
        }
    }

    fn _score_product_on_reaction(
        account: AccountId,
        product: &mut Product<TestRuntime>,
        kind: ReactionKind
    ) -> DispatchResult {
        Scores::score_product_on_reaction(account, product, kind)
    }

    fn _transfer_default_storefront_ownership() -> DispatchResult {
        _transfer_storefront_ownership(None, None, None)
    }

    fn _transfer_storefront_ownership(
        origin: Option<Origin>,
        storefront_id: Option<StorefrontId>,
        transfer_to: Option<AccountId>
    ) -> DispatchResult {
        StorefrontOwnership::transfer_storefront_ownership(
            origin.unwrap_or_else(|| Origin::signed(ACCOUNT1)),
            storefront_id.unwrap_or(SPACE1),
            transfer_to.unwrap_or(ACCOUNT2),
        )
    }

    fn _accept_default_pending_ownership() -> DispatchResult {
        _accept_pending_ownership(None, None)
    }

    fn _accept_pending_ownership(origin: Option<Origin>, storefront_id: Option<StorefrontId>) -> DispatchResult {
        StorefrontOwnership::accept_pending_ownership(
            origin.unwrap_or_else(|| Origin::signed(ACCOUNT2)),
            storefront_id.unwrap_or(SPACE1),
        )
    }

    fn _reject_default_pending_ownership() -> DispatchResult {
        _reject_pending_ownership(None, None)
    }

    fn _reject_default_pending_ownership_by_current_owner() -> DispatchResult {
        _reject_pending_ownership(Some(Origin::signed(ACCOUNT1)), None)
    }

    fn _reject_pending_ownership(origin: Option<Origin>, storefront_id: Option<StorefrontId>) -> DispatchResult {
        StorefrontOwnership::reject_pending_ownership(
            origin.unwrap_or_else(|| Origin::signed(ACCOUNT2)),
            storefront_id.unwrap_or(SPACE1),
        )
    }
    /* ---------------------------------------------------------------------------------------------- */

    // TODO: fix copy-paste from pallet_roles
    /* Roles pallet mocks */

    type RoleId = u64;

    const ROLE1: RoleId = 1;
    const ROLE2: RoleId = 2;

    fn default_role_content_ipfs() -> Content {
        Content::IPFS(b"QmRAQB6YaCyidP37UdDnjFY5vQuiBrcqdyoW1CuDgwxkD4".to_vec())
    }

    /// Permissions Set that includes next permission: ManageRoles
    fn permission_set_default() -> Vec<StorefrontPermission> {
        vec![SP::ManageRoles]
    }


    pub fn _create_default_role() -> DispatchResult {
        _create_role(None, None, None, None, None)
    }

    pub fn _create_role(
        origin: Option<Origin>,
        storefront_id: Option<StorefrontId>,
        time_to_live: Option<Option<BlockNumber>>,
        content: Option<Content>,
        permissions: Option<Vec<StorefrontPermission>>,
    ) -> DispatchResult {
        Roles::create_role(
            origin.unwrap_or_else(|| Origin::signed(ACCOUNT1)),
            storefront_id.unwrap_or(SPACE1),
            time_to_live.unwrap_or_default(), // Should return 'None'
            content.unwrap_or_else(self::default_role_content_ipfs),
            permissions.unwrap_or_else(self::permission_set_default),
        )
    }

    pub fn _grant_default_role() -> DispatchResult {
        _grant_role(None, None, None)
    }

    pub fn _grant_role(
        origin: Option<Origin>,
        role_id: Option<RoleId>,
        users: Option<Vec<User<AccountId>>>,
    ) -> DispatchResult {
        Roles::grant_role(
            origin.unwrap_or_else(|| Origin::signed(ACCOUNT1)),
            role_id.unwrap_or(ROLE1),
            users.unwrap_or_else(|| vec![User::Account(ACCOUNT2)]),
        )
    }

    pub fn _delete_default_role() -> DispatchResult {
        _delete_role(None, None)
    }

    pub fn _delete_role(
        origin: Option<Origin>,
        role_id: Option<RoleId>,
    ) -> DispatchResult {
        Roles::delete_role(
            origin.unwrap_or_else(|| Origin::signed(ACCOUNT1)),
            role_id.unwrap_or(ROLE1),
        )
    }
    /* ---------------------------------------------------------------------------------------------- */


    // Storefront tests
    #[test]
    fn create_storefront_should_work() {
        ExtBuilder::build().execute_with(|| {
            assert_ok!(_create_default_storefront()); // StorefrontId 1

            // Check storages
            assert_eq!(Storefronts::storefront_ids_by_owner(ACCOUNT1), vec![SPACE1]);
            assert_eq!(Storefronts::storefront_id_by_handle(self::storefront_handle()), Some(SPACE1));
            assert_eq!(Storefronts::next_storefront_id(), SPACE2);

            // Check whether data stored correctly
            let storefront = Storefronts::storefront_by_id(SPACE1).unwrap();

            assert_eq!(storefront.created.account, ACCOUNT1);
            assert!(storefront.updated.is_none());
            assert_eq!(storefront.hidden, false);

            assert_eq!(storefront.owner, ACCOUNT1);
            assert_eq!(storefront.handle, Some(self::storefront_handle()));
            assert_eq!(storefront.content, self::storefront_content_ipfs());

            assert_eq!(storefront.products_count, 0);
            assert_eq!(storefront.followers_count, 1);
            assert!(StorefrontHistory::edit_history(storefront.id).is_empty());
            assert_eq!(storefront.score, 0);
        });
    }

    #[test]
    fn create_storefront_should_store_handle_lowercase() {
        ExtBuilder::build().execute_with(|| {
            let handle: Vec<u8> = b"sPaCe_hAnDlE".to_vec();

            assert_ok!(_create_storefront(None, None, Some(Some(handle.clone())), None)); // StorefrontId 1

            // Handle should be lowercase in storage and original in struct
            let storefront = Storefronts::storefront_by_id(SPACE1).unwrap();
            assert_eq!(storefront.handle, Some(handle.clone()));
            assert_eq!(Storefronts::storefront_id_by_handle(handle.to_ascii_lowercase()), Some(SPACE1));
        });
    }

    #[test]
    fn create_storefront_should_fail_with_handle_too_short() {
        ExtBuilder::build().execute_with(|| {
            let handle: Vec<u8> = vec![65; (MinHandleLen::get() - 1) as usize];

            // Try to catch an error creating a storefront with too short handle
            assert_noop!(_create_storefront(
                None,
                None,
                Some(Some(handle)),
                None
            ), UtilsError::<TestRuntime>::HandleIsTooShort);
        });
    }

    #[test]
    fn create_storefront_should_fail_with_handle_too_long() {
        ExtBuilder::build().execute_with(|| {
            let handle: Vec<u8> = vec![65; (MaxHandleLen::get() + 1) as usize];

            // Try to catch an error creating a storefront with too long handle
            assert_noop!(_create_storefront(
                None,
                None,
                Some(Some(handle)),
                None
            ), UtilsError::<TestRuntime>::HandleIsTooLong);
        });
    }

    #[test]
    fn create_storefront_should_fail_with_handle_not_unique() {
        ExtBuilder::build().execute_with(|| {
            assert_ok!(_create_default_storefront());
            // StorefrontId 1
            // Try to catch an error creating a storefront with not unique handle
            assert_noop!(_create_default_storefront(), StorefrontsError::<TestRuntime>::StorefrontHandleIsNotUnique);
        });
    }

    #[test]
    fn create_storefront_should_fail_with_handle_contains_invalid_char_at() {
        ExtBuilder::build().execute_with(|| {
            let handle: Vec<u8> = b"@storefront_handle".to_vec();

            assert_noop!(_create_storefront(
                None,
                None,
                Some(Some(handle)),
                None
            ), UtilsError::<TestRuntime>::HandleContainsInvalidChars);
        });
    }

    #[test]
    fn create_storefront_should_fail_with_handle_contains_invalid_char_minus() {
        ExtBuilder::build().execute_with(|| {
            let handle: Vec<u8> = b"storefront-handle".to_vec();

            assert_noop!(_create_storefront(
                None,
                None,
                Some(Some(handle)),
                None
            ), UtilsError::<TestRuntime>::HandleContainsInvalidChars);
        });
    }

    #[test]
    fn create_storefront_should_fail_with_handle_contains_invalid_char_storefront() {
        ExtBuilder::build().execute_with(|| {
            let handle: Vec<u8> = b"storefront handle".to_vec();

            assert_noop!(_create_storefront(
                None,
                None,
                Some(Some(handle)),
                None
            ), UtilsError::<TestRuntime>::HandleContainsInvalidChars);
        });
    }

    #[test]
    fn create_storefront_should_fail_with_handle_contains_invalid_chars_unicode() {
        ExtBuilder::build().execute_with(|| {
            let handle: Vec<u8> = String::from("блог_хендл").into_bytes().to_vec();

            assert_noop!(_create_storefront(
                None,
                None,
                Some(Some(handle)),
                None
            ), UtilsError::<TestRuntime>::HandleContainsInvalidChars);
        });
    }

    #[test]
    fn create_storefront_should_fail_with_invalid_ipfs_cid() {
        ExtBuilder::build().execute_with(|| {
            let content_ipfs = Content::IPFS(b"QmV9tSDx9UiPeWExXEeH6aoDvmihvx6j".to_vec());

            // Try to catch an error creating a storefront with invalid content
            assert_noop!(_create_storefront(
                None,
                None,
                None,
                Some(content_ipfs)
            ), UtilsError::<TestRuntime>::InvalidIpfsCid);
        });
    }

    #[test]
    fn update_storefront_should_work() {
        ExtBuilder::build_with_storefront().execute_with(|| {
            let handle: Vec<u8> = b"new_handle".to_vec();
            let content_ipfs = Content::IPFS(b"QmRAQB6YaCyidP37UdDnjFY5vQuiBrcqdyoW2CuDgwxkD4".to_vec());
            // Storefront update with ID 1 should be fine

            assert_ok!(_update_storefront(
                None, // From ACCOUNT1 (has permission as he's an owner)
                None,
                Some(
                    self::storefront_update(
                        None,
                        Some(Some(handle.clone())),
                        Some(content_ipfs.clone()),
                        Some(true),
                        Some(Some(StorefrontPermissions {
                            none: None,
                            everyone: None,
                            follower: None,
                            storefront_owner: None
                        })),
                    )
                )
            ));

            // Check whether storefront updates correctly
            let storefront = Storefronts::storefront_by_id(SPACE1).unwrap();
            assert_eq!(storefront.handle, Some(handle));
            assert_eq!(storefront.content, content_ipfs);
            assert_eq!(storefront.hidden, true);

            // Check whether history recorded correctly
            let edit_history = &StorefrontHistory::edit_history(storefront.id)[0];
            assert_eq!(edit_history.old_data.handle, Some(Some(self::storefront_handle())));
            assert_eq!(edit_history.old_data.content, Some(self::storefront_content_ipfs()));
            assert_eq!(edit_history.old_data.hidden, Some(false));
        });
    }

    #[test]
    fn update_storefront_should_work_with_a_few_roles() {
        ExtBuilder::build_with_a_few_roles_granted_to_account2(vec![SP::UpdateStorefront]).execute_with(|| {
            let storefront_update = self::storefront_update(
                None,
                Some(Some(b"new_handle".to_vec())),
                Some(Content::IPFS(
                    b"QmRAQB6YaCyidP37UdDnjFY5vQuiBrcqdyoW2CuDgwxkD4".to_vec()
                )),
                Some(true),
                None,
            );

            assert_ok!(_update_storefront(
                Some(Origin::signed(ACCOUNT2)),
                Some(SPACE1),
                Some(storefront_update)
            ));
        });
    }

    #[test]
    fn update_storefront_should_fail_with_no_updates_for_storefront() {
        ExtBuilder::build_with_storefront().execute_with(|| {
            // Try to catch an error updating a storefront with no changes
            assert_noop!(_update_storefront(None, None, None), StorefrontsError::<TestRuntime>::NoUpdatesForStorefront);
        });
    }

    #[test]
    fn update_storefront_should_fail_with_storefront_not_found() {
        ExtBuilder::build_with_storefront().execute_with(|| {
            let handle: Vec<u8> = b"new_handle".to_vec();

            // Try to catch an error updating a storefront with wrong storefront ID
            assert_noop!(_update_storefront(
                None,
                Some(SPACE2),
                Some(
                    self::storefront_update(
                        None,
                        Some(Some(handle)),
                        None,
                        None,
                        None,
                    )
                )
            ), StorefrontsError::<TestRuntime>::StorefrontNotFound);
        });
    }

    #[test]
    fn update_storefront_should_fail_with_no_permission() {
        ExtBuilder::build_with_storefront().execute_with(|| {
            let handle: Vec<u8> = b"new_handle".to_vec();

            // Try to catch an error updating a storefront with an account that it not permitted
            assert_noop!(_update_storefront(
                Some(Origin::signed(ACCOUNT2)),
                None,
                Some(
                    self::storefront_update(
                        None,
                        Some(Some(handle)),
                        None,
                        None,
                        None,
                    )
                )
            ), StorefrontsError::<TestRuntime>::NoPermissionToUpdateStorefront);
        });
    }

    #[test]
    fn update_storefront_should_fail_with_handle_too_short() {
        ExtBuilder::build_with_storefront().execute_with(|| {
            let handle: Vec<u8> = vec![65; (MinHandleLen::get() - 1) as usize];

            // Try to catch an error updating a storefront with too short handle
            assert_noop!(_update_storefront(
                None,
                None,
                Some(
                    self::storefront_update(
                        None,
                        Some(Some(handle)),
                        None,
                        None,
                        None,
                    )
                )
            ), UtilsError::<TestRuntime>::HandleIsTooShort);
        });
    }

    #[test]
    fn update_storefront_should_fail_with_handle_too_long() {
        ExtBuilder::build_with_storefront().execute_with(|| {
            let handle: Vec<u8> = vec![65; (MaxHandleLen::get() + 1) as usize];

            // Try to catch an error updating a storefront with too long handle
            assert_noop!(_update_storefront(
                None,
                None,
                Some(
                    self::storefront_update(
                        None,
                        Some(Some(handle)),
                        None,
                        None,
                        None,
                    )
                )
            ), UtilsError::<TestRuntime>::HandleIsTooLong);
        });
    }

    #[test]
    fn update_storefront_should_fail_with_handle_is_not_unique() {
        ExtBuilder::build_with_storefront().execute_with(|| {
            let handle: Vec<u8> = b"unique_handle".to_vec();

            assert_ok!(_create_storefront(
                None,
                None,
                Some(Some(handle.clone())),
                None
            )); // StorefrontId 2 with a custom handle

                // Try to catch an error updating a storefront on ID 1 with a handle of storefront on ID 2
                assert_noop!(_update_storefront(
                None,
                Some(SPACE1),
                Some(
                    self::storefront_update(
                        None,
                        Some(Some(handle)),
                        None,
                        None,
                        None,
                    )
                )
            ), StorefrontsError::<TestRuntime>::StorefrontHandleIsNotUnique);
        });
    }

    #[test]
    fn update_storefront_should_fail_with_handle_contains_invalid_char_at() {
        ExtBuilder::build_with_storefront().execute_with(|| {
            let handle: Vec<u8> = b"@storefront_handle".to_vec();

            assert_noop!(_update_storefront(
                None,
                None,
                Some(
                    self::storefront_update(
                        None,
                        Some(Some(handle)),
                        None,
                        None,
                        None,
                    )
                )
            ), UtilsError::<TestRuntime>::HandleContainsInvalidChars);
        });
    }

    #[test]
    fn update_storefront_should_fail_with_handle_contains_invalid_char_minus() {
        ExtBuilder::build_with_storefront().execute_with(|| {
            let handle: Vec<u8> = b"storefront-handle".to_vec();

            assert_noop!(_update_storefront(
                None,
                None,
                Some(
                    self::storefront_update(
                        None,
                        Some(Some(handle)),
                        None,
                        None,
                        None,
                    )
                )
            ), UtilsError::<TestRuntime>::HandleContainsInvalidChars);
        });
    }

    #[test]
    fn update_storefront_should_fail_with_handle_contains_invalid_storefront() {
        ExtBuilder::build_with_storefront().execute_with(|| {
            let handle: Vec<u8> = b"storefront handle".to_vec();

            assert_noop!(_update_storefront(
                None,
                None,
                Some(
                    self::storefront_update(
                        None,
                        Some(Some(handle)),
                        None,
                        None,
                        None,
                    )
                )
            ), UtilsError::<TestRuntime>::HandleContainsInvalidChars);
        });
    }

    #[test]
    fn update_storefront_should_fail_with_handle_contains_invalid_chars_unicode() {
        ExtBuilder::build_with_storefront().execute_with(|| {
            let handle: Vec<u8> = String::from("блог_хендл").into_bytes().to_vec();

            assert_noop!(_update_storefront(
                None,
                None,
                Some(
                    self::storefront_update(
                        None,
                        Some(Some(handle)),
                        None,
                        None,
                        None,
                    )
                )
            ), UtilsError::<TestRuntime>::HandleContainsInvalidChars);
        });
    }

    #[test]
    fn update_storefront_should_fail_with_invalid_ipfs_cid() {
        ExtBuilder::build_with_storefront().execute_with(|| {
            let content_ipfs = Content::IPFS(b"QmV9tSDx9UiPeWExXEeH6aoDvmihvx6j".to_vec());

            // Try to catch an error updating a storefront with invalid content
            assert_noop!(_update_storefront(
                None,
                None,
                Some(
                    self::storefront_update(
                        None,
                        None,
                        Some(content_ipfs),
                        None,
                        None,
                    )
                )
            ), UtilsError::<TestRuntime>::InvalidIpfsCid);
        });
    }

    #[test]
    fn update_storefront_should_fail_with_a_few_roles_no_permission() {
        ExtBuilder::build_with_a_few_roles_granted_to_account2(vec![SP::UpdateStorefront]).execute_with(|| {
            let storefront_update = self::storefront_update(
                None,
                Some(Some(b"new_handle".to_vec())),
                Some(Content::IPFS(
                    b"QmRAQB6YaCyidP37UdDnjFY5vQuiBrcqdyoW2CuDgwxkD4".to_vec()
                )),
                Some(true),
                None,
            );

            assert_ok!(_delete_default_role());

            assert_noop!(_update_storefront(
                Some(Origin::signed(ACCOUNT2)),
                Some(SPACE1),
                Some(storefront_update)
            ), StorefrontsError::<TestRuntime>::NoPermissionToUpdateStorefront);
        });
    }

    // Product tests
    #[test]
    fn create_product_should_work() {
        ExtBuilder::build_with_storefront().execute_with(|| {
            assert_ok!(_create_default_product()); // ProductId 1 by ACCOUNT1 which is permitted by default

            // Check storages
            assert_eq!(Products::product_ids_by_storefront_id(SPACE1), vec![POST1]);
            assert_eq!(Products::next_product_id(), POST2);

            // Check whether data stored correctly
            let product = Products::product_by_id(POST1).unwrap();

            assert_eq!(product.created.account, ACCOUNT1);
            assert!(product.updated.is_none());
            assert_eq!(product.hidden, false);

            assert_eq!(product.storefront_id, Some(SPACE1));
            assert_eq!(product.extension, self::extension_regular_product());

            assert_eq!(product.content, self::product_content_ipfs());

            assert_eq!(product.replies_count, 0);
            assert_eq!(product.hidden_replies_count, 0);
            assert_eq!(product.shares_count, 0);
            assert_eq!(product.upvotes_count, 0);
            assert_eq!(product.downvotes_count, 0);

            assert_eq!(product.score, 0);

            assert!(ProductHistory::edit_history(POST1).is_empty());
        });
    }

    #[test]
    fn create_product_should_work_with_a_few_roles() {
        ExtBuilder::build_with_a_few_roles_granted_to_account2(vec![SP::CreateProducts]).execute_with(|| {
            assert_ok!(_create_product(
                Some(Origin::signed(ACCOUNT2)),
                None, // StorefrontId 1,
                None, // RegularProduct extension
                None, // Default product content
            ));
        });
    }

    #[test]
    fn create_product_should_fail_with_product_has_no_storefrontid() {
        ExtBuilder::build_with_storefront().execute_with(|| {
            assert_noop!(_create_product(
                None,
                Some(None),
                None,
                None
            ), ProductsError::<TestRuntime>::ProductHasNoStorefrontId);
        });
    }

    #[test]
    fn create_product_should_fail_with_storefront_not_found() {
        ExtBuilder::build().execute_with(|| {
            assert_noop!(_create_default_product(), StorefrontsError::<TestRuntime>::StorefrontNotFound);
        });
    }

    #[test]
    fn create_product_should_fail_with_invalid_ipfs_cid() {
        ExtBuilder::build_with_storefront().execute_with(|| {
            let content_ipfs = Content::IPFS(b"QmV9tSDx9UiPeWExXEeH6aoDvmihvx6j".to_vec());

            // Try to catch an error creating a regular product with invalid content
            assert_noop!(_create_product(
                None,
                None,
                None,
                Some(content_ipfs)
            ), UtilsError::<TestRuntime>::InvalidIpfsCid);
        });
    }

    #[test]
    fn create_product_should_fail_with_no_permission() {
        ExtBuilder::build_with_storefront().execute_with(|| {
            assert_noop!(_create_product(
                Some(Origin::signed(ACCOUNT2)),
                None,
                None,
                None
            ), ProductsError::<TestRuntime>::NoPermissionToCreateProducts);
        });
    }

    #[test]
    fn create_product_should_fail_with_a_few_roles_no_permission() {
        ExtBuilder::build_with_a_few_roles_granted_to_account2(vec![SP::CreateProducts]).execute_with(|| {
            assert_ok!(_delete_default_role());

            assert_noop!(_create_product(
                Some(Origin::signed(ACCOUNT2)),
                None, // StorefrontId 1,
                None, // RegularProduct extension
                None, // Default product content
            ), ProductsError::<TestRuntime>::NoPermissionToCreateProducts);
        });
    }

    #[test]
    fn update_product_should_work() {
        ExtBuilder::build_with_product().execute_with(|| {
            let content_ipfs = Content::IPFS(b"QmRAQB6YaCyidP37UdDnjFY5vQuiBrcqdyoW1CuDgwxkD4".to_vec());

            // Product update with ID 1 should be fine
            assert_ok!(_update_product(
                None, // From ACCOUNT1 (has default permission to UpdateOwnProducts)
                None,
                Some(
                    self::product_update(
                        None,
                        Some(content_ipfs.clone()),
                        Some(true)
                    )
                )
            ));

            // Check whether product updates correctly
            let product = Products::product_by_id(POST1).unwrap();
            assert_eq!(product.storefront_id, Some(SPACE1));
            assert_eq!(product.content, content_ipfs);
            assert_eq!(product.hidden, true);

            // Check whether history recorded correctly
            let product_history = ProductHistory::edit_history(POST1)[0].clone();
            assert!(product_history.old_data.storefront_id.is_none());
            assert_eq!(product_history.old_data.content, Some(self::product_content_ipfs()));
            assert_eq!(product_history.old_data.hidden, Some(false));
        });
    }

    #[test]
    fn update_product_should_work_after_transfer_storefront_ownership() {
        ExtBuilder::build_with_product().execute_with(|| {
            let product_update = self::product_update(
                None,
                Some(Content::IPFS(
                    b"QmRAQB6YaCyidP37UdDnjFY5vQuiBrcqdyoW1CuDgwxkD4".to_vec()
                )),
                Some(true),
            );

            assert_ok!(_transfer_default_storefront_ownership());

            // Product update with ID 1 should be fine
            assert_ok!(_update_product(None, None, Some(product_update)));
        });
    }

    #[test]
    fn update_any_product_should_work_with_default_permission() {
        ExtBuilder::build_with_a_few_roles_granted_to_account2(vec![SP::CreateProducts]).execute_with(|| {
            let product_update = self::product_update(
                None,
                Some(Content::IPFS(
                    b"QmRAQB6YaCyidP37UdDnjFY5vQuiBrcqdyoW1CuDgwxkD4".to_vec()
                )),
                Some(true),
            );
            assert_ok!(_create_product(
                Some(Origin::signed(ACCOUNT2)),
                None, // StorefrontId 1
                None, // RegularProduct extension
                None // Default product content
            )); // ProductId 1

            // Product update with ID 1 should be fine
            assert_ok!(_update_product(
                None, // From ACCOUNT1 (has default permission to UpdateAnyProducts as StorefrontOwner)
                Some(POST1),
                Some(product_update)
            ));
        });
    }

    #[test]
    fn update_any_product_should_work_with_a_few_roles() {
        ExtBuilder::build_with_a_few_roles_granted_to_account2(vec![SP::UpdateAnyProduct]).execute_with(|| {
            let product_update = self::product_update(
                None,
                Some(Content::IPFS(
                    b"QmRAQB6YaCyidP37UdDnjFY5vQuiBrcqdyoW1CuDgwxkD4".to_vec()
                )),
                Some(true),
            );
            assert_ok!(_create_default_product()); // ProductId 1

            // Product update with ID 1 should be fine
            assert_ok!(_update_product(
                Some(Origin::signed(ACCOUNT2)),
                Some(POST1),
                Some(product_update)
            ));
        });
    }

    #[test]
    fn update_product_should_fail_with_no_updates_for_product() {
        ExtBuilder::build_with_product().execute_with(|| {
            // Try to catch an error updating a product with no changes
            assert_noop!(_update_product(None, None, None), ProductsError::<TestRuntime>::NoUpdatesForProduct);
        });
    }

    #[test]
    fn update_product_should_fail_with_product_not_found() {
        ExtBuilder::build_with_product().execute_with(|| {
            assert_ok!(_create_storefront(None, None, Some(Some(b"storefront2_handle".to_vec())), None)); // StorefrontId 2

            // Try to catch an error updating a product with wrong product ID
            assert_noop!(_update_product(
                None,
                Some(POST2),
                Some(
                    self::product_update(
                        // FIXME: when Product's `storefront_id` update is fully implemented
                        None/*Some(SPACE2)*/,
                        None,
                        Some(true)/*None*/
                    )
                )
            ), ProductsError::<TestRuntime>::ProductNotFound);
        });
    }

    #[test]
    fn update_product_should_fail_with_no_permission_to_update_any_product() {
        ExtBuilder::build_with_product().execute_with(|| {
            assert_ok!(_create_storefront(None, None, Some(Some(b"storefront2_handle".to_vec())), None)); // StorefrontId 2

            // Try to catch an error updating a product with different account
            assert_noop!(_update_product(
                Some(Origin::signed(ACCOUNT2)),
                None,
                Some(
                    self::product_update(
                        // FIXME: when Product's `storefront_id` update is fully implemented
                        None/*Some(SPACE2)*/,
                        None,
                        Some(true)/*None*/
                    )
                )
            ), ProductsError::<TestRuntime>::NoPermissionToUpdateAnyProduct);
        });
    }

    #[test]
    fn update_product_should_fail_with_invalid_ipfs_cid() {
        ExtBuilder::build_with_product().execute_with(|| {
            let content_ipfs = Content::IPFS(b"QmV9tSDx9UiPeWExXEeH6aoDvmihvx6j".to_vec());

            // Try to catch an error updating a product with invalid content
            assert_noop!(_update_product(
                None,
                None,
                Some(
                    self::product_update(
                        None,
                        Some(content_ipfs),
                        None
                    )
                )
            ), UtilsError::<TestRuntime>::InvalidIpfsCid);
        });
    }

    #[test]
    fn update_any_product_should_fail_with_a_few_roles_no_permission() {
        ExtBuilder::build_with_a_few_roles_granted_to_account2(vec![SP::UpdateAnyProduct]).execute_with(|| {
            let product_update = self::product_update(
                None,
                Some(Content::IPFS(
                    b"QmRAQB6YaCyidP37UdDnjFY5vQuiBrcqdyoW1CuDgwxkD4".to_vec()
                )),
                Some(true),
            );
            assert_ok!(_create_default_product());
            // ProductId 1
            assert_ok!(_delete_default_role());

            // Product update with ID 1 should be fine
            assert_noop!(_update_product(
                Some(Origin::signed(ACCOUNT2)),
                Some(POST1),
                Some(product_update)
            ), ProductsError::<TestRuntime>::NoPermissionToUpdateAnyProduct);
        });
    }

    // Comment tests
    #[test]
    fn create_comment_should_work() {
        ExtBuilder::build_with_product().execute_with(|| {
            assert_ok!(_create_default_comment()); // ProductId 2 by ACCOUNT1 which is permitted by default

            // Check storages
            let root_product = Products::product_by_id(POST1).unwrap();
            assert_eq!(Products::reply_ids_by_product_id(POST1), vec![POST2]);
            assert_eq!(root_product.replies_count, 1);
            assert_eq!(root_product.hidden_replies_count, 0);

            // Check whether data stored correctly
            let comment = Products::product_by_id(POST2).unwrap();
            let comment_ext = comment.get_comment_ext().unwrap();

            assert!(comment_ext.parent_id.is_none());
            assert_eq!(comment_ext.root_product_id, POST1);
            assert_eq!(comment.created.account, ACCOUNT1);
            assert!(comment.updated.is_none());
            assert_eq!(comment.content, self::comment_content_ipfs());
            assert_eq!(comment.replies_count, 0);
            assert_eq!(comment.hidden_replies_count, 0);
            assert_eq!(comment.shares_count, 0);
            assert_eq!(comment.upvotes_count, 0);
            assert_eq!(comment.downvotes_count, 0);
            assert_eq!(comment.score, 0);

            assert!(ProductHistory::edit_history(POST2).is_empty());
        });
    }

    #[test]
    fn create_comment_should_work_with_parents() {
        ExtBuilder::build_with_comment().execute_with(|| {
            let first_comment_id: ProductId = 2;
            let penultimate_comment_id: ProductId = 8;
            let last_comment_id: ProductId = 9;

            for parent_id in first_comment_id..last_comment_id as ProductId {
                // last created = `last_comment_id`; last parent = `penultimate_comment_id`
                assert_ok!(_create_comment(None, None, Some(Some(parent_id)), None));
            }

            for comment_id in first_comment_id..penultimate_comment_id as ProductId {
                let comment = Products::product_by_id(comment_id).unwrap();
                let replies_should_be = last_comment_id-comment_id;
                assert_eq!(comment.replies_count, replies_should_be as u16);
                assert_eq!(Products::reply_ids_by_product_id(comment_id), vec![comment_id + 1]);

                assert_eq!(comment.hidden_replies_count, 0);
            }

            let last_comment = Products::product_by_id(last_comment_id).unwrap();
            assert_eq!(last_comment.replies_count, 0);
            assert!(Products::reply_ids_by_product_id(last_comment_id).is_empty());

            assert_eq!(last_comment.hidden_replies_count, 0);
        });
    }

    #[test]
    fn create_comment_should_fail_with_product_not_found() {
        ExtBuilder::build().execute_with(|| {
            // Try to catch an error creating a comment with wrong product
            assert_noop!(_create_default_comment(), ProductsError::<TestRuntime>::ProductNotFound);
        });
    }

    #[test]
    fn create_comment_should_fail_with_unknown_parent_comment() {
        ExtBuilder::build_with_product().execute_with(|| {
            // Try to catch an error creating a comment with wrong parent
            assert_noop!(_create_comment(
                None,
                None,
                Some(Some(POST2)),
                None
            ), ProductsError::<TestRuntime>::UnknownParentComment);
        });
    }

    #[test]
    fn create_comment_should_fail_with_invalid_ipfs_cid() {
        ExtBuilder::build_with_product().execute_with(|| {
            let content_ipfs = Content::IPFS(b"QmV9tSDx9UiPeWExXEeH6aoDvmihvx6j".to_vec());

            // Try to catch an error creating a comment with wrong parent
            assert_noop!(_create_comment(
                None,
                None,
                None,
                Some(content_ipfs)
            ), UtilsError::<TestRuntime>::InvalidIpfsCid);
        });
    }

    #[test]
    fn create_comment_should_fail_with_cannot_create_in_hidden_storefront_scope() {
        ExtBuilder::build_with_product().execute_with(|| {
            assert_ok!(_update_storefront(
                None,
                None,
                Some(self::storefront_update(None, None, None, Some(true), None))
            ));

            assert_noop!(_create_default_comment(), ProductsError::<TestRuntime>::CannotCreateInHiddenScope);
        });
    }

    #[test]
    fn create_comment_should_fail_with_cannot_create_in_hidden_product_scope() {
        ExtBuilder::build_with_product().execute_with(|| {
            assert_ok!(_update_product(
                None,
                None,
                Some(self::product_update(None, None, Some(true)))
            ));

            assert_noop!(_create_default_comment(), ProductsError::<TestRuntime>::CannotCreateInHiddenScope);
        });
    }

    #[test]
    fn create_comment_should_fail_with_max_comment_depth_reached() {
        ExtBuilder::build_with_product().execute_with(|| {
            assert_ok!(_create_comment(None, None, Some(None), None)); // ProductId 2

            for parent_id in 2..11 as ProductId {
                assert_ok!(_create_comment(None, None, Some(Some(parent_id)), None)); // ProductId N (last = 10)
            }

            // Some(Some(11)) - here is parent_id 11 of type ProductId
            assert_noop!(_create_comment(
                None,
                None,
                Some(Some(11)),
                None
            ), ProductsError::<TestRuntime>::MaxCommentDepthReached);
        });
    }

    #[test]
    fn update_comment_should_work() {
        ExtBuilder::build_with_comment().execute_with(|| {
            // Product update with ID 1 should be fine
            assert_ok!(_update_comment(None, None, None));

            // Check whether product updates correctly
            let comment = Products::product_by_id(POST2).unwrap();
            assert_eq!(comment.content, self::reply_content_ipfs());

            // Check whether history recorded correctly
            assert_eq!(ProductHistory::edit_history(POST2)[0].old_data.content, Some(self::comment_content_ipfs()));
        });
    }

    #[test]
    fn update_comment_hidden_should_work_with_parents() {
        ExtBuilder::build_with_comment().execute_with(|| {
            let first_comment_id: ProductId = 2;
            let penultimate_comment_id: ProductId = 8;
            let last_comment_id: ProductId = 9;

            for parent_id in first_comment_id..last_comment_id as ProductId {
                // last created = `last_comment_id`; last parent = `penultimate_comment_id`
                assert_ok!(_create_comment(None, None, Some(Some(parent_id)), None));
            }

            assert_ok!(_update_comment(
                None,
                Some(last_comment_id),
                Some(self::product_update(
                    None,
                    None,
                    Some(true) // make comment hidden
                ))
            ));

            for comment_id in first_comment_id..penultimate_comment_id as ProductId {
                let comment = Products::product_by_id(comment_id).unwrap();
                assert_eq!(comment.hidden_replies_count, 1);
            }
            let last_comment = Products::product_by_id(last_comment_id).unwrap();
            assert_eq!(last_comment.hidden_replies_count, 0);
        });
    }

    #[test]
    // `ProductNotFound` here: Product with Comment extension. Means that comment wasn't found.
    fn update_comment_should_fail_with_product_not_found() {
        ExtBuilder::build().execute_with(|| {
            // Try to catch an error updating a comment with wrong ProductId
            assert_noop!(_update_comment(None, None, None), ProductsError::<TestRuntime>::ProductNotFound);
        });
    }

    #[test]
    fn update_comment_should_fail_with_not_a_comment_author() {
        ExtBuilder::build_with_comment().execute_with(|| {
            // Try to catch an error updating a comment with wrong Account
            assert_noop!(_update_comment(
                Some(Origin::signed(ACCOUNT2)),
                None,
                None
            ), ProductsError::<TestRuntime>::NotACommentAuthor);
        });
    }

    #[test]
    fn update_comment_should_fail_with_invalid_ipfs_cid() {
        ExtBuilder::build_with_comment().execute_with(|| {
            let content_ipfs = Content::IPFS(b"QmV9tSDx9UiPeWExXEeH6aoDvmihvx6j".to_vec());

            // Try to catch an error updating a comment with invalid content
            assert_noop!(_update_comment(
                None,
                None,
                Some(
                    self::product_update(
                        None,
                        Some(content_ipfs),
                        None
                    )
                )
            ), UtilsError::<TestRuntime>::InvalidIpfsCid);
        });
    }

    // Reaction tests
    #[test]
    fn create_product_reaction_should_work_upvote() {
        ExtBuilder::build_with_product().execute_with(|| {
            assert_ok!(_create_product_reaction(
                Some(Origin::signed(ACCOUNT2)),
                None,
                None
            )); // ReactionId 1 by ACCOUNT2 which is permitted by default

            // Check storages
            assert_eq!(Reactions::reaction_ids_by_product_id(POST1), vec![REACTION1]);
            assert_eq!(Reactions::next_reaction_id(), REACTION2);

            // Check product reaction counters
            let product = Products::product_by_id(POST1).unwrap();
            assert_eq!(product.upvotes_count, 1);
            assert_eq!(product.downvotes_count, 0);

            // Check whether data stored correctly
            let reaction = Reactions::reaction_by_id(REACTION1).unwrap();
            assert_eq!(reaction.created.account, ACCOUNT2);
            assert_eq!(reaction.kind, self::reaction_upvote());
        });
    }

    #[test]
    fn create_product_reaction_should_work_downvote() {
        ExtBuilder::build_with_product().execute_with(|| {
            assert_ok!(_create_product_reaction(
                Some(Origin::signed(ACCOUNT2)),
                None,
                Some(self::reaction_downvote())
            )); // ReactionId 1 by ACCOUNT2 which is permitted by default

            // Check storages
            assert_eq!(Reactions::reaction_ids_by_product_id(POST1), vec![REACTION1]);
            assert_eq!(Reactions::next_reaction_id(), REACTION2);

            // Check product reaction counters
            let product = Products::product_by_id(POST1).unwrap();
            assert_eq!(product.upvotes_count, 0);
            assert_eq!(product.downvotes_count, 1);

            // Check whether data stored correctly
            let reaction = Reactions::reaction_by_id(REACTION1).unwrap();
            assert_eq!(reaction.created.account, ACCOUNT2);
            assert_eq!(reaction.kind, self::reaction_downvote());
        });
    }

    #[test]
    fn create_product_reaction_should_fail_with_account_already_reacted() {
        ExtBuilder::build_with_product().execute_with(|| {
            assert_ok!(_create_default_product_reaction()); // ReactionId1

            // Try to catch an error creating reaction by the same account
            assert_noop!(_create_default_product_reaction(), ReactionsError::<TestRuntime>::AccountAlreadyReacted);
        });
    }

    #[test]
    fn create_product_reaction_should_fail_with_product_not_found() {
        ExtBuilder::build().execute_with(|| {
            // Try to catch an error creating reaction by the same account
            assert_noop!(_create_default_product_reaction(), ProductsError::<TestRuntime>::ProductNotFound);
        });
    }

    #[test]
    fn create_product_reaction_should_fail_with_cannot_react_when_storefront_hidden() {
        ExtBuilder::build_with_product().execute_with(|| {
            assert_ok!(_update_storefront(
                None,
                None,
                Some(self::storefront_update(None, None, None, Some(true), None))
            ));

            assert_noop!(_create_default_product_reaction(), ReactionsError::<TestRuntime>::CannotReactWhenStorefrontHidden);
        });
    }

    #[test]
    fn create_product_reaction_should_fail_with_cannot_react_when_product_hidden() {
        ExtBuilder::build_with_product().execute_with(|| {
            assert_ok!(_update_product(
                None,
                None,
                Some(self::product_update(None, None, Some(true)))
            ));

            assert_noop!(_create_default_product_reaction(), ReactionsError::<TestRuntime>::CannotReactWhenProductHidden);
        });
    }

// Rating system tests

    #[test]
    fn check_results_of_score_diff_for_action_with_common_values() {
        ExtBuilder::build().execute_with(|| {
            assert_eq!(Scores::score_diff_for_action(1, self::scoring_action_upvote_product()), UpvoteProductActionWeight::get() as i16);
            assert_eq!(Scores::score_diff_for_action(1, self::scoring_action_downvote_product()), DownvoteProductActionWeight::get() as i16);
            assert_eq!(Scores::score_diff_for_action(1, self::scoring_action_share_product()), ShareProductActionWeight::get() as i16);
            assert_eq!(Scores::score_diff_for_action(1, self::scoring_action_create_comment()), CreateCommentActionWeight::get() as i16);
            assert_eq!(Scores::score_diff_for_action(1, self::scoring_action_upvote_comment()), UpvoteCommentActionWeight::get() as i16);
            assert_eq!(Scores::score_diff_for_action(1, self::scoring_action_downvote_comment()), DownvoteCommentActionWeight::get() as i16);
            assert_eq!(Scores::score_diff_for_action(1, self::scoring_action_share_comment()), ShareCommentActionWeight::get() as i16);
            assert_eq!(Scores::score_diff_for_action(1, self::scoring_action_follow_storefront()), FollowStorefrontActionWeight::get() as i16);
            assert_eq!(Scores::score_diff_for_action(1, self::scoring_action_follow_account()), FollowAccountActionWeight::get() as i16);
        });
    }

    #[test]
    fn check_results_of_score_diff_for_action_with_random_values() {
        ExtBuilder::build().execute_with(|| {
            assert_eq!(Scores::score_diff_for_action(32768, self::scoring_action_upvote_product()), 80); // 2^15
            assert_eq!(Scores::score_diff_for_action(32769, self::scoring_action_upvote_product()), 80); // 2^15 + 1
            assert_eq!(Scores::score_diff_for_action(65535, self::scoring_action_upvote_product()), 80); // 2^16 - 1
            assert_eq!(Scores::score_diff_for_action(65536, self::scoring_action_upvote_product()), 85); // 2^16
        });
    }

//--------------------------------------------------------------------------------------------------

    #[test]
    fn change_storefront_score_should_work_for_follow_storefront() {
        ExtBuilder::build_with_storefront().execute_with(|| {
            assert_ok!(_follow_storefront(
                Some(Origin::signed(ACCOUNT2)),
                Some(SPACE1)
            ));

            assert_eq!(Storefronts::storefront_by_id(SPACE1).unwrap().score, FollowStorefrontActionWeight::get() as i32);
            assert_eq!(Profiles::social_account_by_id(ACCOUNT1).unwrap().reputation, 1 + FollowStorefrontActionWeight::get() as u32);
            assert_eq!(Profiles::social_account_by_id(ACCOUNT2).unwrap().reputation, 1);
        });
    }

    #[test]
    fn change_storefront_score_should_work_for_unfollow_storefront() {
        ExtBuilder::build_with_storefront().execute_with(|| {
            assert_ok!(_follow_storefront(
                Some(Origin::signed(ACCOUNT2)),
                Some(SPACE1)
            ));
            assert_ok!(_unfollow_storefront(
                Some(Origin::signed(ACCOUNT2)),
                Some(SPACE1)
            ));

            assert_eq!(Storefronts::storefront_by_id(SPACE1).unwrap().score, 0);
            assert_eq!(Profiles::social_account_by_id(ACCOUNT1).unwrap().reputation, 1);
            assert_eq!(Profiles::social_account_by_id(ACCOUNT2).unwrap().reputation, 1);
        });
    }

    #[test]
    fn change_storefront_score_should_work_for_upvote_product() {
        ExtBuilder::build_with_product().execute_with(|| {
            assert_ok!(_create_product_reaction(Some(Origin::signed(ACCOUNT2)), None, None)); // ReactionId 1

            assert_eq!(Storefronts::storefront_by_id(SPACE1).unwrap().score, UpvoteProductActionWeight::get() as i32);
            assert_eq!(Profiles::social_account_by_id(ACCOUNT1).unwrap().reputation, 1 + UpvoteProductActionWeight::get() as u32);
        });
    }

    #[test]
    fn change_storefront_score_should_work_for_downvote_product() {
        ExtBuilder::build_with_product().execute_with(|| {
            assert_ok!(_create_product_reaction(
                Some(Origin::signed(ACCOUNT2)),
                None,
                Some(self::reaction_downvote())
            )); // ReactionId 1

            assert_eq!(Storefronts::storefront_by_id(SPACE1).unwrap().score, DownvoteProductActionWeight::get() as i32);
            assert_eq!(Profiles::social_account_by_id(ACCOUNT1).unwrap().reputation, 1);
        });
    }

//--------------------------------------------------------------------------------------------------

    #[test]
    fn change_product_score_should_work_for_create_comment() {
        ExtBuilder::build_with_product().execute_with(|| {
            assert_ok!(_create_comment(
                Some(Origin::signed(ACCOUNT2)),
                None,
                None,
                None
            )); // ProductId 2

            assert_eq!(Products::product_by_id(POST1).unwrap().score, CreateCommentActionWeight::get() as i32);
            assert_eq!(Storefronts::storefront_by_id(SPACE1).unwrap().score, CreateCommentActionWeight::get() as i32);
            assert_eq!(Profiles::social_account_by_id(ACCOUNT1).unwrap().reputation, 1 + CreateCommentActionWeight::get() as u32);
            assert_eq!(Scores::product_score_by_account((ACCOUNT2, POST1, self::scoring_action_create_comment())), Some(CreateCommentActionWeight::get()));
        });
    }

    #[test]
    fn change_product_score_should_work_for_upvote_product() {
        ExtBuilder::build_with_product().execute_with(|| {
            assert_ok!(_create_product_reaction(
                Some(Origin::signed(ACCOUNT2)),
                None,
                None
            ));

            assert_eq!(Products::product_by_id(POST1).unwrap().score, UpvoteProductActionWeight::get() as i32);
            assert_eq!(Profiles::social_account_by_id(ACCOUNT1).unwrap().reputation, 1 + UpvoteProductActionWeight::get() as u32);
            assert_eq!(Scores::product_score_by_account((ACCOUNT2, POST1, self::scoring_action_upvote_product())), Some(UpvoteProductActionWeight::get()));
        });
    }

    #[test]
    fn change_product_score_should_work_for_downvote_product() {
        ExtBuilder::build_with_product().execute_with(|| {
            assert_ok!(_create_product_reaction(
                Some(Origin::signed(ACCOUNT2)),
                None,
                Some(self::reaction_downvote())
            ));

            assert_eq!(Products::product_by_id(POST1).unwrap().score, DownvoteProductActionWeight::get() as i32);
            assert_eq!(Profiles::social_account_by_id(ACCOUNT1).unwrap().reputation, 1);
            assert_eq!(Scores::product_score_by_account((ACCOUNT2, POST1, self::scoring_action_downvote_product())), Some(DownvoteProductActionWeight::get()));
        });
    }

    #[test]
    fn change_product_score_should_for_revert_upvote() {
        ExtBuilder::build_with_product().execute_with(|| {
            assert_ok!(_create_product_reaction(
                Some(Origin::signed(ACCOUNT2)),
                None,
                None
            ));
            // ReactionId 1
            assert_ok!(_delete_product_reaction(
                Some(Origin::signed(ACCOUNT2)),
                None,
                REACTION1
            ));

            assert_eq!(Products::product_by_id(POST1).unwrap().score, 0);
            assert_eq!(Profiles::social_account_by_id(ACCOUNT1).unwrap().reputation, 1);
            assert!(Scores::product_score_by_account((ACCOUNT2, POST1, self::scoring_action_upvote_product())).is_none());
        });
    }

    #[test]
    fn change_product_score_should_for_revert_downvote() {
        ExtBuilder::build_with_product().execute_with(|| {
            assert_ok!(_create_product_reaction(
                Some(Origin::signed(ACCOUNT2)),
                None,
                Some(self::reaction_downvote())
            ));
            // ReactionId 1
            assert_ok!(_delete_product_reaction(
                Some(Origin::signed(ACCOUNT2)),
                None,
                REACTION1
            ));

            assert_eq!(Products::product_by_id(POST1).unwrap().score, 0);
            assert_eq!(Profiles::social_account_by_id(ACCOUNT1).unwrap().reputation, 1);
            assert!(Scores::product_score_by_account((ACCOUNT2, POST1, self::scoring_action_downvote_product())).is_none());
        });
    }

    #[test]
    fn change_product_score_should_work_for_change_upvote_with_downvote() {
        ExtBuilder::build_with_product().execute_with(|| {
            assert_ok!(_create_product_reaction(
                Some(Origin::signed(ACCOUNT2)),
                None,
                None
            ));
            // ReactionId 1
            assert_ok!(_update_product_reaction(
                Some(Origin::signed(ACCOUNT2)),
                None,
                REACTION1,
                Some(self::reaction_downvote())
            ));

            assert_eq!(Products::product_by_id(POST1).unwrap().score, DownvoteProductActionWeight::get() as i32);
            assert_eq!(Profiles::social_account_by_id(ACCOUNT1).unwrap().reputation, 1);
            assert!(Scores::product_score_by_account((ACCOUNT2, POST1, self::scoring_action_upvote_product())).is_none());
            assert_eq!(Scores::product_score_by_account((ACCOUNT2, POST1, self::scoring_action_downvote_product())), Some(DownvoteProductActionWeight::get()));
        });
    }

    #[test]
    fn change_product_score_should_work_for_change_downvote_with_upvote() {
        ExtBuilder::build_with_product().execute_with(|| {
            assert_ok!(_create_product_reaction(
                Some(Origin::signed(ACCOUNT2)),
                None,
                Some(self::reaction_downvote())
            ));
            // ReactionId 1
            assert_ok!(_update_product_reaction(
                Some(Origin::signed(ACCOUNT2)),
                None,
                REACTION1,
                None
            ));

            assert_eq!(Products::product_by_id(POST1).unwrap().score, UpvoteProductActionWeight::get() as i32);
            assert_eq!(Profiles::social_account_by_id(ACCOUNT1).unwrap().reputation, 1 + UpvoteProductActionWeight::get() as u32);
            assert!(Scores::product_score_by_account((ACCOUNT2, POST1, self::scoring_action_downvote_product())).is_none());
            assert_eq!(Scores::product_score_by_account((ACCOUNT2, POST1, self::scoring_action_upvote_product())), Some(UpvoteProductActionWeight::get()));
        });
    }

//--------------------------------------------------------------------------------------------------

    #[test]
    fn change_social_account_reputation_should_work_with_max_score_diff() {
        ExtBuilder::build_with_storefront().execute_with(|| {
            assert_ok!(_create_product(Some(Origin::signed(ACCOUNT1)), None, None, None));
            assert_ok!(Scores::change_social_account_reputation(
                ACCOUNT1,
                ACCOUNT2,
                std::i16::MAX,
                self::scoring_action_follow_account())
            );
        });
    }

    #[test]
    fn change_social_account_reputation_should_work_with_min_score_diff() {
        ExtBuilder::build_with_storefront().execute_with(|| {
            assert_ok!(_create_product(Some(Origin::signed(ACCOUNT1)), None, None, None));
            assert_ok!(Scores::change_social_account_reputation(
                ACCOUNT1,
                ACCOUNT2,
                std::i16::MIN,
                self::scoring_action_follow_account())
            );
        });
    }

    #[test]
    fn change_social_account_reputation_should_work() {
        ExtBuilder::build_with_storefront().execute_with(|| {
            assert_ok!(_create_product(Some(Origin::signed(ACCOUNT1)), None, None, None));
            assert_ok!(Scores::change_social_account_reputation(
                ACCOUNT1,
                ACCOUNT2,
                DownvoteProductActionWeight::get(),
                self::scoring_action_downvote_product())
            );
            assert_eq!(Scores::account_reputation_diff_by_account((ACCOUNT2, ACCOUNT1, self::scoring_action_downvote_product())), Some(0));
            assert_eq!(Profiles::social_account_by_id(ACCOUNT1).unwrap().reputation, 1);

            // To ensure function works correctly, multiply default UpvoteProductActionWeight by two
            assert_ok!(Scores::change_social_account_reputation(
                ACCOUNT1,
                ACCOUNT2,
                UpvoteProductActionWeight::get() * 2,
                self::scoring_action_upvote_product())
            );

            assert_eq!(
                Scores::account_reputation_diff_by_account(
                    (
                        ACCOUNT2,
                        ACCOUNT1,
                        self::scoring_action_upvote_product()
                    )
                ), Some(UpvoteProductActionWeight::get() * 2)
            );

            assert_eq!(Profiles::social_account_by_id(ACCOUNT1).unwrap().reputation, 1 + (UpvoteProductActionWeight::get() * 2) as u32);
        });
    }

//--------------------------------------------------------------------------------------------------

    #[test]
    fn change_comment_score_should_work_for_upvote() {
        ExtBuilder::build_with_storefront().execute_with(|| {
            assert_ok!(_create_product(
                Some(Origin::signed(ACCOUNT1)),
                None,
                None,
                None
            ));
            // ProductId 1
            assert_ok!(_create_comment(
                Some(Origin::signed(ACCOUNT2)),
                None,
                None,
                None
            )); // ProductId 2

            assert_ok!(_score_product_on_reaction_with_id(
                ACCOUNT3,
                POST2,
                self::reaction_upvote()
            ));

            assert_eq!(Products::product_by_id(POST2).unwrap().score, UpvoteCommentActionWeight::get() as i32);
            assert_eq!(Profiles::social_account_by_id(ACCOUNT1).unwrap().reputation, 1 + CreateCommentActionWeight::get() as u32);
            assert_eq!(Profiles::social_account_by_id(ACCOUNT2).unwrap().reputation, 1 + UpvoteCommentActionWeight::get() as u32);
            assert_eq!(Profiles::social_account_by_id(ACCOUNT3).unwrap().reputation, 1);
            assert_eq!(Scores::product_score_by_account((ACCOUNT3, POST2, self::scoring_action_upvote_comment())), Some(UpvoteCommentActionWeight::get()));
        });
    }

    #[test]
    fn change_comment_score_should_work_for_downvote() {
        ExtBuilder::build_with_storefront().execute_with(|| {
            assert_ok!(_create_product(
                Some(Origin::signed(ACCOUNT1)),
                None,
                None,
                None
            ));
            // ProductId 1
            assert_ok!(_create_comment(
                Some(Origin::signed(ACCOUNT2)),
                None,
                None,
                None
            )); // ProductId 2

            assert_ok!(_score_product_on_reaction_with_id(ACCOUNT3, POST2, self::reaction_downvote()));

            assert_eq!(Products::product_by_id(POST2).unwrap().score, DownvoteCommentActionWeight::get() as i32);
            assert_eq!(Profiles::social_account_by_id(ACCOUNT1).unwrap().reputation, 1 + CreateCommentActionWeight::get() as u32);
            assert_eq!(Profiles::social_account_by_id(ACCOUNT2).unwrap().reputation, 1);
            assert_eq!(Profiles::social_account_by_id(ACCOUNT3).unwrap().reputation, 1);
            assert_eq!(Scores::product_score_by_account((ACCOUNT3, POST2, self::scoring_action_downvote_comment())), Some(DownvoteCommentActionWeight::get()));
        });
    }

    #[test]
    fn change_comment_score_should_for_revert_upvote() {
        ExtBuilder::build_with_storefront().execute_with(|| {
            assert_ok!(_create_product(
                Some(Origin::signed(ACCOUNT1)),
                None,
                None,
                None
            ));
            // ProductId 1
            assert_ok!(_create_comment(
                Some(Origin::signed(ACCOUNT2)),
                None,
                None,
                None
            )); // ProductId 2

            assert_ok!(_score_product_on_reaction_with_id(ACCOUNT3, POST2, self::reaction_upvote()));
            assert_ok!(_score_product_on_reaction_with_id(ACCOUNT3, POST2, self::reaction_upvote()));

            assert_eq!(Products::product_by_id(POST2).unwrap().score, 0);
            assert_eq!(Profiles::social_account_by_id(ACCOUNT1).unwrap().reputation, 1 + CreateCommentActionWeight::get() as u32);
            assert_eq!(Profiles::social_account_by_id(ACCOUNT2).unwrap().reputation, 1);
            assert_eq!(Profiles::social_account_by_id(ACCOUNT3).unwrap().reputation, 1);
            assert!(Scores::product_score_by_account((ACCOUNT1, POST2, self::scoring_action_upvote_comment())).is_none());
        });
    }

    #[test]
    fn change_comment_score_should_for_revert_downvote() {
        ExtBuilder::build_with_storefront().execute_with(|| {
            assert_ok!(_create_product(
                Some(Origin::signed(ACCOUNT1)),
                None,
                None,
                None
            ));
            // ProductId 1
            assert_ok!(_create_comment(
                Some(Origin::signed(ACCOUNT2)),
                None,
                None,
                None
            )); // ProductId 2

            assert_ok!(_score_product_on_reaction_with_id(ACCOUNT3, POST2, self::reaction_downvote()));
            assert_ok!(_score_product_on_reaction_with_id(ACCOUNT3, POST2, self::reaction_downvote()));

            assert_eq!(Products::product_by_id(POST2).unwrap().score, 0);
            assert_eq!(Profiles::social_account_by_id(ACCOUNT1).unwrap().reputation, 1 + CreateCommentActionWeight::get() as u32);
            assert_eq!(Profiles::social_account_by_id(ACCOUNT2).unwrap().reputation, 1);
            assert_eq!(Profiles::social_account_by_id(ACCOUNT3).unwrap().reputation, 1);
            assert!(Scores::product_score_by_account((ACCOUNT1, POST2, self::scoring_action_downvote_comment())).is_none());
        });
    }

    #[test]
    fn change_comment_score_check_for_cancel_upvote() {
        ExtBuilder::build_with_storefront().execute_with(|| {
            assert_ok!(_create_product(
                Some(Origin::signed(ACCOUNT1)),
                None,
                None,
                None
            ));
            // ProductId 1
            assert_ok!(_create_comment(
                Some(Origin::signed(ACCOUNT2)),
                None,
                None,
                None
            )); // ProductId 2

            assert_ok!(_score_product_on_reaction_with_id(ACCOUNT3, POST2, self::reaction_upvote()));
            assert_ok!(_score_product_on_reaction_with_id(ACCOUNT3, POST2, self::reaction_downvote()));

            assert_eq!(Products::product_by_id(POST2).unwrap().score, DownvoteCommentActionWeight::get() as i32);
            assert_eq!(Profiles::social_account_by_id(ACCOUNT1).unwrap().reputation, 1 + CreateCommentActionWeight::get() as u32);
            assert_eq!(Profiles::social_account_by_id(ACCOUNT2).unwrap().reputation, 1);
            assert_eq!(Profiles::social_account_by_id(ACCOUNT3).unwrap().reputation, 1);
            assert!(Scores::product_score_by_account((ACCOUNT3, POST2, self::scoring_action_upvote_comment())).is_none());
            assert_eq!(Scores::product_score_by_account((ACCOUNT3, POST2, self::scoring_action_downvote_comment())), Some(DownvoteCommentActionWeight::get()));
        });
    }

    #[test]
    fn change_comment_score_check_for_cancel_downvote() {
        ExtBuilder::build_with_storefront().execute_with(|| {
            assert_ok!(_create_product(
                Some(Origin::signed(ACCOUNT1)),
                None,
                None,
                None
            ));
            // ProductId 1
            assert_ok!(_create_comment(
                Some(Origin::signed(ACCOUNT2)),
                None,
                None,
                None
            )); // ProductId 2

            assert_ok!(_score_product_on_reaction_with_id(ACCOUNT3, POST2, self::reaction_downvote()));
            assert_ok!(_score_product_on_reaction_with_id(ACCOUNT3, POST2, self::reaction_upvote()));

            assert_eq!(Products::product_by_id(POST2).unwrap().score, UpvoteCommentActionWeight::get() as i32);
            assert_eq!(Profiles::social_account_by_id(ACCOUNT1).unwrap().reputation, 1 + CreateCommentActionWeight::get() as u32);
            assert_eq!(Profiles::social_account_by_id(ACCOUNT2).unwrap().reputation, 1 + UpvoteCommentActionWeight::get() as u32);
            assert_eq!(Profiles::social_account_by_id(ACCOUNT3).unwrap().reputation, 1);
            assert!(Scores::product_score_by_account((ACCOUNT3, POST2, self::scoring_action_downvote_comment())).is_none());
            assert_eq!(Scores::product_score_by_account((ACCOUNT3, POST2, self::scoring_action_upvote_comment())), Some(UpvoteCommentActionWeight::get()));
        });
    }

// Shares tests

    #[test]
    fn share_product_should_work() {
        ExtBuilder::build_with_product().execute_with(|| {
            assert_ok!(_create_storefront(
                Some(Origin::signed(ACCOUNT2)),
                None,
                Some(Some(b"storefront2_handle".to_vec())),
                None
            )); // StorefrontId 2 by ACCOUNT2

            assert_ok!(_create_product(
                Some(Origin::signed(ACCOUNT2)),
                Some(Some(SPACE2)),
                Some(self::extension_shared_product(POST1)),
                None
            )); // Share ProductId 1 on StorefrontId 2 by ACCOUNT2 which is permitted by default in both storefronts

            // Check storages
            assert_eq!(Products::product_ids_by_storefront_id(SPACE1), vec![POST1]);
            assert_eq!(Products::product_ids_by_storefront_id(SPACE2), vec![POST2]);
            assert_eq!(Products::next_product_id(), POST3);

            assert_eq!(Products::shared_product_ids_by_original_product_id(POST1), vec![POST2]);

            // Check whether data stored correctly
            assert_eq!(Products::product_by_id(POST1).unwrap().shares_count, 1);

            let shared_product = Products::product_by_id(POST2).unwrap();

            assert_eq!(shared_product.storefront_id, Some(SPACE2));
            assert_eq!(shared_product.created.account, ACCOUNT2);
            assert_eq!(shared_product.extension, self::extension_shared_product(POST1));
        });
    }

    #[test]
    fn share_product_should_work_with_a_few_roles() {
        ExtBuilder::build_with_a_few_roles_granted_to_account2(vec![SP::CreateProducts]).execute_with(|| {
            assert_ok!(_create_storefront(
                None, // From ACCOUNT1
                None, // With no parent_id provided
                Some(None), // Provided without any handle
                None // With default storefront content,
            ));
            // StorefrontId 2
            assert_ok!(_create_product(
                None, // From ACCOUNT1
                Some(Some(SPACE2)),
                None, // With RegularProduct extension
                None // With default product content
            )); // ProductId 1 on StorefrontId 2

            assert_ok!(_create_product(
                Some(Origin::signed(ACCOUNT2)),
                Some(Some(SPACE1)),
                Some(self::extension_shared_product(POST1)),
                None
            )); // Share ProductId 1 on StorefrontId 1 by ACCOUNT2 which is permitted by RoleId 1 from ext
        });
    }

    #[test]
    fn share_product_should_work_for_share_own_product_in_same_own_storefront() {
        ExtBuilder::build_with_product().execute_with(|| {
            assert_ok!(_create_product(
                Some(Origin::signed(ACCOUNT1)),
                Some(Some(SPACE1)),
                Some(self::extension_shared_product(POST1)),
                None
            )); // Share ProductId 1

            // Check storages
            assert_eq!(Products::product_ids_by_storefront_id(SPACE1), vec![POST1, POST2]);
            assert_eq!(Products::next_product_id(), POST3);

            assert_eq!(Products::shared_product_ids_by_original_product_id(POST1), vec![POST2]);

            // Check whether data stored correctly
            assert_eq!(Products::product_by_id(POST1).unwrap().shares_count, 1);

            let shared_product = Products::product_by_id(POST2).unwrap();
            assert_eq!(shared_product.storefront_id, Some(SPACE1));
            assert_eq!(shared_product.created.account, ACCOUNT1);
            assert_eq!(shared_product.extension, self::extension_shared_product(POST1));
        });
    }

    #[test]
    fn share_product_should_change_score() {
        ExtBuilder::build_with_product().execute_with(|| {
            assert_ok!(_create_storefront(
                Some(Origin::signed(ACCOUNT2)),
                None,
                Some(Some(b"storefront2_handle".to_vec())),
                None
            )); // StorefrontId 2 by ACCOUNT2

            assert_ok!(_create_product(
                Some(Origin::signed(ACCOUNT2)),
                Some(Some(SPACE2)),
                Some(self::extension_shared_product(POST1)),
                None
            )); // Share ProductId 1 on StorefrontId 2 by ACCOUNT2

            assert_eq!(Products::product_by_id(POST1).unwrap().score, ShareProductActionWeight::get() as i32);
            assert_eq!(Profiles::social_account_by_id(ACCOUNT1).unwrap().reputation, 1 + ShareProductActionWeight::get() as u32);
            assert_eq!(Scores::product_score_by_account((ACCOUNT2, POST1, self::scoring_action_share_product())), Some(ShareProductActionWeight::get()));
        });
    }

    #[test]
    fn share_product_should_not_change_score() {
        ExtBuilder::build_with_product().execute_with(|| {
            assert_ok!(_create_product(
                Some(Origin::signed(ACCOUNT1)),
                Some(Some(SPACE1)),
                Some(self::extension_shared_product(POST1)),
                None
            )); // Share ProductId

            assert_eq!(Products::product_by_id(POST1).unwrap().score, 0);
            assert_eq!(Profiles::social_account_by_id(ACCOUNT1).unwrap().reputation, 1);
            assert!(Scores::product_score_by_account((ACCOUNT1, POST1, self::scoring_action_share_product())).is_none());
        });
    }

    #[test]
    fn share_product_should_fail_with_original_product_not_found() {
        ExtBuilder::build_with_storefront().execute_with(|| {
            assert_ok!(_create_storefront(
                Some(Origin::signed(ACCOUNT2)),
                None,
                Some(Some(b"storefront2_handle".to_vec())),
                None
            )); // StorefrontId 2 by ACCOUNT2

            // Skipped creating ProductId 1
            assert_noop!(_create_product(
                Some(Origin::signed(ACCOUNT2)),
                Some(Some(SPACE2)),
                Some(self::extension_shared_product(POST1)),
                None
            ), ProductsError::<TestRuntime>::OriginalProductNotFound);
        });
    }

    #[test]
    fn share_product_should_fail_with_cannot_share_sharing_product() {
        ExtBuilder::build_with_product().execute_with(|| {
            assert_ok!(_create_storefront(
                Some(Origin::signed(ACCOUNT2)),
                None,
                Some(Some(b"storefront2_handle".to_vec())),
                None
            )); // StorefrontId 2 by ACCOUNT2

            assert_ok!(_create_product(
                Some(Origin::signed(ACCOUNT2)),
                Some(Some(SPACE2)),
                Some(self::extension_shared_product(POST1)),
                None)
            );

            // Try to share product with extension SharedProduct
            assert_noop!(_create_product(
                Some(Origin::signed(ACCOUNT1)),
                Some(Some(SPACE1)),
                Some(self::extension_shared_product(POST2)),
                None
            ), ProductsError::<TestRuntime>::CannotShareSharingProduct);
        });
    }

    #[test]
    fn share_product_should_fail_with_no_permission_to_create_products() {
        ExtBuilder::build_with_product().execute_with(|| {
            assert_ok!(_create_storefront(
                Some(Origin::signed(ACCOUNT1)),
                None, // With no parent_id provided
                Some(None), // No storefront_handle provided (ok)
                None // Default storefront content,
            )); // StorefrontId 2 by ACCOUNT1

            // Try to share product with extension SharedProduct
            assert_noop!(_create_product(
                Some(Origin::signed(ACCOUNT2)),
                Some(Some(SPACE2)),
                Some(self::extension_shared_product(POST1)),
                None
            ), ProductsError::<TestRuntime>::NoPermissionToCreateProducts);
        });
    }

    #[test]
    fn share_product_should_fail_with_a_few_roles_no_permission() {
        ExtBuilder::build_with_a_few_roles_granted_to_account2(vec![SP::CreateProducts]).execute_with(|| {
            assert_ok!(_create_storefront(
                None, // From ACCOUNT1
                None, // With no parent_id provided
                Some(None), // Provided without any handle
                None // With default storefront content
            ));
            // StorefrontId 2
            assert_ok!(_create_product(
                None, // From ACCOUNT1
                Some(Some(SPACE2)),
                None, // With RegularProduct extension
                None // With default product content
            )); // ProductId 1 on StorefrontId 2

            assert_ok!(_delete_default_role());

            assert_noop!(_create_product(
                Some(Origin::signed(ACCOUNT2)),
                Some(Some(SPACE1)),
                Some(self::extension_shared_product(POST1)),
                None
            ), ProductsError::<TestRuntime>::NoPermissionToCreateProducts);
        });
    }

// Profiles tests

    #[test]
    fn create_profile_should_work() {
        ExtBuilder::build().execute_with(|| {
            assert_ok!(_create_default_profile()); // AccountId 1

            let profile = Profiles::social_account_by_id(ACCOUNT1).unwrap().profile.unwrap();
            assert_eq!(profile.created.account, ACCOUNT1);
            assert!(profile.updated.is_none());
            assert_eq!(profile.content, self::profile_content_ipfs());

            assert!(ProfileHistory::edit_history(ACCOUNT1).is_empty());
        });
    }

    #[test]
    fn create_profile_should_fail_with_profile_already_created() {
        ExtBuilder::build().execute_with(|| {
            assert_ok!(_create_default_profile());
            // AccountId 1
            assert_noop!(_create_default_profile(), ProfilesError::<TestRuntime>::ProfileAlreadyCreated);
        });
    }

    #[test]
    fn create_profile_should_fail_with_invalid_ipfs_cid() {
        ExtBuilder::build().execute_with(|| {
            let content_ipfs = Content::IPFS(b"QmV9tSDx9UiPeWExXEeH6aoDvmihvx6j".to_vec());

            assert_noop!(_create_profile(
                None,
                Some(content_ipfs)
            ), UtilsError::<TestRuntime>::InvalidIpfsCid);
        });
    }

    #[test]
    fn update_profile_should_work() {
        ExtBuilder::build().execute_with(|| {
            assert_ok!(_create_default_profile());
            // AccountId 1
            assert_ok!(_update_profile(
                None,
                Some(self::storefront_content_ipfs())
            ));

            // Check whether profile updated correctly
            let profile = Profiles::social_account_by_id(ACCOUNT1).unwrap().profile.unwrap();
            assert!(profile.updated.is_some());
            assert_eq!(profile.content, self::storefront_content_ipfs());

            // Check whether profile history is written correctly
            let profile_history = ProfileHistory::edit_history(ACCOUNT1)[0].clone();
            assert_eq!(profile_history.old_data.content, Some(self::profile_content_ipfs()));
        });
    }

    #[test]
    fn update_profile_should_fail_with_social_account_not_found() {
        ExtBuilder::build().execute_with(|| {
            assert_noop!(_update_profile(
                None,
                Some(self::profile_content_ipfs())
            ), ProfilesError::<TestRuntime>::SocialAccountNotFound);
        });
    }

    #[test]
    fn update_profile_should_fail_with_account_has_no_profile() {
        ExtBuilder::build().execute_with(|| {
            assert_ok!(ProfileFollows::follow_account(Origin::signed(ACCOUNT1), ACCOUNT2));
            assert_noop!(_update_profile(
                None,
                Some(self::profile_content_ipfs())
            ), ProfilesError::<TestRuntime>::AccountHasNoProfile);
        });
    }

    #[test]
    fn update_profile_should_fail_with_no_updates_for_profile() {
        ExtBuilder::build().execute_with(|| {
            assert_ok!(_create_default_profile());
            // AccountId 1
            assert_noop!(_update_profile(
                None,
                None
            ), ProfilesError::<TestRuntime>::NoUpdatesForProfile);
        });
    }

    #[test]
    fn update_profile_should_fail_with_invalid_ipfs_cid() {
        ExtBuilder::build().execute_with(|| {
            let content_ipfs = Content::IPFS(b"QmV9tSDx9UiPeWExXEeH6aoDvmihvx6j".to_vec());

            assert_ok!(_create_default_profile());
            assert_noop!(_update_profile(
                None,
                Some(content_ipfs)
            ), UtilsError::<TestRuntime>::InvalidIpfsCid);
        });
    }

// Storefront following tests

    #[test]
    fn follow_storefront_should_work() {
        ExtBuilder::build_with_storefront().execute_with(|| {
            assert_ok!(_default_follow_storefront()); // Follow StorefrontId 1 by ACCOUNT2

            assert_eq!(Storefronts::storefront_by_id(SPACE1).unwrap().followers_count, 2);
            assert_eq!(StorefrontFollows::storefronts_followed_by_account(ACCOUNT2), vec![SPACE1]);
            assert_eq!(StorefrontFollows::storefront_followers(SPACE1), vec![ACCOUNT1, ACCOUNT2]);
            assert_eq!(StorefrontFollows::storefront_followed_by_account((ACCOUNT2, SPACE1)), true);
        });
    }

    #[test]
    fn follow_storefront_should_fail_with_storefront_not_found() {
        ExtBuilder::build().execute_with(|| {
            assert_noop!(_default_follow_storefront(), StorefrontsError::<TestRuntime>::StorefrontNotFound);
        });
    }

    #[test]
    fn follow_storefront_should_fail_with_already_storefront_follower() {
        ExtBuilder::build_with_storefront().execute_with(|| {
            assert_ok!(_default_follow_storefront()); // Follow StorefrontId 1 by ACCOUNT2

            assert_noop!(_default_follow_storefront(), StorefrontFollowsError::<TestRuntime>::AlreadyStorefrontFollower);
        });
    }

    #[test]
    fn follow_storefront_should_fail_with_cannot_follow_hidden_storefront() {
        ExtBuilder::build_with_storefront().execute_with(|| {
            assert_ok!(_update_storefront(
                None,
                None,
                Some(self::storefront_update(None, None, None, Some(true), None))
            ));

            assert_noop!(_default_follow_storefront(), StorefrontFollowsError::<TestRuntime>::CannotFollowHiddenStorefront);
        });
    }

    #[test]
    fn unfollow_storefront_should_work() {
        ExtBuilder::build_with_storefront().execute_with(|| {
            assert_ok!(_default_follow_storefront());
            // Follow StorefrontId 1 by ACCOUNT2
            assert_ok!(_default_unfollow_storefront());

            assert_eq!(Storefronts::storefront_by_id(SPACE1).unwrap().followers_count, 1);
            assert!(StorefrontFollows::storefronts_followed_by_account(ACCOUNT2).is_empty());
            assert_eq!(StorefrontFollows::storefront_followers(SPACE1), vec![ACCOUNT1]);
        });
    }

    #[test]
    fn unfollow_storefront_should_fail_with_storefront_not_found() {
        ExtBuilder::build_with_storefront_follow_no_storefront().execute_with(|| {
            assert_noop!(_default_unfollow_storefront(), StorefrontsError::<TestRuntime>::StorefrontNotFound);
        });
    }

    #[test]
    fn unfollow_storefront_should_fail_with_not_storefront_follower() {
        ExtBuilder::build_with_storefront().execute_with(|| {
            assert_noop!(_default_unfollow_storefront(), StorefrontFollowsError::<TestRuntime>::NotStorefrontFollower);
        });
    }

// Account following tests

    #[test]
    fn follow_account_should_work() {
        ExtBuilder::build().execute_with(|| {
            assert_ok!(_default_follow_account()); // Follow ACCOUNT1 by ACCOUNT2

            assert_eq!(ProfileFollows::accounts_followed_by_account(ACCOUNT2), vec![ACCOUNT1]);
            assert_eq!(ProfileFollows::account_followers(ACCOUNT1), vec![ACCOUNT2]);
            assert_eq!(ProfileFollows::account_followed_by_account((ACCOUNT2, ACCOUNT1)), true);
        });
    }

    #[test]
    fn follow_account_should_fail_with_account_cannot_follow_itself() {
        ExtBuilder::build().execute_with(|| {
            assert_noop!(_follow_account(
                None,
                Some(ACCOUNT2)
            ), ProfileFollowsError::<TestRuntime>::AccountCannotFollowItself);
        });
    }

    #[test]
    fn follow_account_should_fail_with_already_account_follower() {
        ExtBuilder::build().execute_with(|| {
            assert_ok!(_default_follow_account());

            assert_noop!(_default_follow_account(), ProfileFollowsError::<TestRuntime>::AlreadyAccountFollower);
        });
    }

    #[test]
    fn unfollow_account_should_work() {
        ExtBuilder::build().execute_with(|| {
            assert_ok!(_default_follow_account());
            // Follow ACCOUNT1 by ACCOUNT2
            assert_ok!(_default_unfollow_account());

            assert!(ProfileFollows::accounts_followed_by_account(ACCOUNT2).is_empty());
            assert!(ProfileFollows::account_followers(ACCOUNT1).is_empty());
            assert_eq!(ProfileFollows::account_followed_by_account((ACCOUNT2, ACCOUNT1)), false);
        });
    }

    #[test]
    fn unfollow_account_should_fail_with_account_cannot_unfollow_itself() {
        ExtBuilder::build().execute_with(|| {
            assert_noop!(_unfollow_account(
                None,
                Some(ACCOUNT2)
            ), ProfileFollowsError::<TestRuntime>::AccountCannotUnfollowItself);
        });
    }

    #[test]
    fn unfollow_account_should_fail_with_not_account_follower() {
        ExtBuilder::build().execute_with(|| {
            assert_ok!(_default_follow_account());
            assert_ok!(_default_unfollow_account());

            assert_noop!(_default_unfollow_account(), ProfileFollowsError::<TestRuntime>::NotAccountFollower);
        });
    }

// Transfer ownership tests

    #[test]
    fn transfer_storefront_ownership_should_work() {
        ExtBuilder::build_with_storefront().execute_with(|| {
            assert_ok!(_transfer_default_storefront_ownership()); // Transfer StorefrontId 1 owned by ACCOUNT1 to ACCOUNT2

            assert_eq!(StorefrontOwnership::pending_storefront_owner(SPACE1).unwrap(), ACCOUNT2);
        });
    }

    #[test]
    fn transfer_storefront_ownership_should_fail_with_storefront_not_found() {
        ExtBuilder::build().execute_with(|| {
            assert_noop!(_transfer_default_storefront_ownership(), StorefrontsError::<TestRuntime>::StorefrontNotFound);
        });
    }

    #[test]
    fn transfer_storefront_ownership_should_fail_with_not_a_storefront_owner() {
        ExtBuilder::build_with_storefront().execute_with(|| {
            assert_noop!(_transfer_storefront_ownership(
                Some(Origin::signed(ACCOUNT2)),
                None,
                Some(ACCOUNT1)
            ), StorefrontsError::<TestRuntime>::NotAStorefrontOwner);
        });
    }

    #[test]
    fn transfer_storefront_ownership_should_fail_with_cannot_transfer_to_current_owner() {
        ExtBuilder::build_with_storefront().execute_with(|| {
            assert_noop!(_transfer_storefront_ownership(
                Some(Origin::signed(ACCOUNT1)),
                None,
                Some(ACCOUNT1)
            ), StorefrontOwnershipError::<TestRuntime>::CannotTranferToCurrentOwner);
        });
    }

    #[test]
    fn accept_pending_ownership_should_work() {
        ExtBuilder::build_with_storefront().execute_with(|| {
            assert_ok!(_transfer_default_storefront_ownership());
            // Transfer StorefrontId 1 owned by ACCOUNT1 to ACCOUNT2
            assert_ok!(_accept_default_pending_ownership()); // Accepting a transfer from ACCOUNT2
            // Check whether owner was changed
            let storefront = Storefronts::storefront_by_id(SPACE1).unwrap();
            assert_eq!(storefront.owner, ACCOUNT2);

            // Check whether storage state is correct
            assert!(StorefrontOwnership::pending_storefront_owner(SPACE1).is_none());
        });
    }

    #[test]
    fn accept_pending_ownership_should_fail_with_storefront_not_found() {
        ExtBuilder::build_with_pending_ownership_transfer_no_storefront().execute_with(|| {
            assert_noop!(_accept_default_pending_ownership(), StorefrontsError::<TestRuntime>::StorefrontNotFound);
        });
    }

    #[test]
    fn accept_pending_ownership_should_fail_with_no_pending_transfer_on_storefront() {
        ExtBuilder::build_with_storefront().execute_with(|| {
            assert_noop!(_accept_default_pending_ownership(), StorefrontOwnershipError::<TestRuntime>::NoPendingTransferOnStorefront);
        });
    }

    #[test]
    fn accept_pending_ownership_should_fail_if_origin_is_already_an_owner() {
        ExtBuilder::build_with_storefront().execute_with(|| {
            assert_ok!(_transfer_default_storefront_ownership());

            assert_noop!(_accept_pending_ownership(
                Some(Origin::signed(ACCOUNT1)),
                None
            ), StorefrontOwnershipError::<TestRuntime>::AlreadyAStorefrontOwner);
        });
    }

    #[test]
    fn accept_pending_ownership_should_fail_if_origin_is_not_equal_to_pending_account() {
        ExtBuilder::build_with_storefront().execute_with(|| {
            assert_ok!(_transfer_default_storefront_ownership());

            assert_noop!(_accept_pending_ownership(
                Some(Origin::signed(ACCOUNT3)),
                None
            ), StorefrontOwnershipError::<TestRuntime>::NotAllowedToAcceptOwnershipTransfer);
        });
    }

    #[test]
    fn reject_pending_ownership_should_work() {
        ExtBuilder::build_with_storefront().execute_with(|| {
            assert_ok!(_transfer_default_storefront_ownership());
            // Transfer StorefrontId 1 owned by ACCOUNT1 to ACCOUNT2
            assert_ok!(_reject_default_pending_ownership()); // Rejecting a transfer from ACCOUNT2

            // Check whether owner was not changed
            let storefront = Storefronts::storefront_by_id(SPACE1).unwrap();
            assert_eq!(storefront.owner, ACCOUNT1);

            // Check whether storage state is correct
            assert!(StorefrontOwnership::pending_storefront_owner(SPACE1).is_none());
        });
    }

    #[test]
    fn reject_pending_ownership_should_work_with_reject_by_current_storefront_owner() {
        ExtBuilder::build_with_storefront().execute_with(|| {
            assert_ok!(_transfer_default_storefront_ownership());
            // Transfer StorefrontId 1 owned by ACCOUNT1 to ACCOUNT2
            assert_ok!(_reject_default_pending_ownership_by_current_owner()); // Rejecting a transfer from ACCOUNT2

            // Check whether owner was not changed
            let storefront = Storefronts::storefront_by_id(SPACE1).unwrap();
            assert_eq!(storefront.owner, ACCOUNT1);

            // Check whether storage state is correct
            assert!(StorefrontOwnership::pending_storefront_owner(SPACE1).is_none());
        });
    }

    #[test]
    fn reject_pending_ownership_should_fail_with_storefront_not_found() {
        ExtBuilder::build_with_pending_ownership_transfer_no_storefront().execute_with(|| {
            assert_noop!(_reject_default_pending_ownership(), StorefrontsError::<TestRuntime>::StorefrontNotFound);
        });
    }

    #[test]
    fn reject_pending_ownership_should_fail_with_no_pending_transfer_on_storefront() {
        ExtBuilder::build_with_storefront().execute_with(|| {
            assert_noop!(_reject_default_pending_ownership(), StorefrontOwnershipError::<TestRuntime>::NoPendingTransferOnStorefront); // Rejecting a transfer from ACCOUNT2
        });
    }

    #[test]
    fn reject_pending_ownership_should_fail_with_not_allowed_to_reject() {
        ExtBuilder::build_with_storefront().execute_with(|| {
            assert_ok!(_transfer_default_storefront_ownership()); // Transfer StorefrontId 1 owned by ACCOUNT1 to ACCOUNT2

            assert_noop!(_reject_pending_ownership(
                Some(Origin::signed(ACCOUNT3)),
                None
            ), StorefrontOwnershipError::<TestRuntime>::NotAllowedToRejectOwnershipTransfer); // Rejecting a transfer from ACCOUNT2
        });
    }
}