use sp_core::{Pair, Public, sr25519, crypto::UncheckedInto};
use dark_runtime::{
	AccountId, AuraConfig, BalancesConfig,
	GenesisConfig, GrandpaConfig, UtilsConfig,
	SudoConfig, StorefrontsConfig, SystemConfig,
	WASM_BINARY, Signature, constants::currency::DARKS,
};
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_finality_grandpa::AuthorityId as GrandpaId;
use sp_runtime::traits::{Verify, IdentifyAccount};
use sc_service::{ChainType, Properties};
use sc_telemetry::TelemetryEndpoints;
use hex_literal::hex;

// Note this is the URL for the telemetry server
const STAGING_TELEMETRY_URL: &str = "wss://telemetry.polkadot.io/submit/";
const DEFAULT_PROTOCOL_ID: &str = "dark";

/// Specialized `ChainSpec`. This is a specialization of the general Substrate ChainSpec type.
pub type ChainSpec = sc_service::GenericChainSpec<GenesisConfig>;

/// Helper function to generate a crypto pair from seed
pub fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
	TPublic::Pair::from_string(&format!("//{}", seed), None)
		.expect("static values are valid; qed")
		.public()
}

type AccountPublic = <Signature as Verify>::Signer;

/// Helper function to generate an account ID from seed
pub fn get_account_id_from_seed<TPublic: Public>(seed: &str) -> AccountId where
	AccountPublic: From<<TPublic::Pair as Pair>::Public>
{
	AccountPublic::from(get_from_seed::<TPublic>(seed)).into_account()
}

/// Helper function to generate an authority key for Aura
pub fn authority_keys_from_seed(s: &str) -> (AuraId, GrandpaId) {
	(
		get_from_seed::<AuraId>(s),
		get_from_seed::<GrandpaId>(s),
	)
}

pub fn development_config() -> ChainSpec {
	ChainSpec::from_genesis(
		"Development",
		"dev0",
		ChainType::Development,
		|| {
			let endowed_accounts = vec![
				get_account_id_from_seed::<sr25519::Public>("Alice"),
				get_account_id_from_seed::<sr25519::Public>("Bob"),
				get_account_id_from_seed::<sr25519::Public>("Charlie"),
				get_account_id_from_seed::<sr25519::Public>("Dave"),
				get_account_id_from_seed::<sr25519::Public>("Eve"),
				
			];

			testnet_genesis(
				vec![
					authority_keys_from_seed("Alice"),
				],
				get_account_id_from_seed::<sr25519::Public>("Alice"),
				endowed_accounts.iter().cloned().map(|k| (k, 66_000_000)).collect(),
				get_account_id_from_seed::<sr25519::Public>("Ferdie"),
				true,
			)
		},
		vec![],
		None,
		Some(DEFAULT_PROTOCOL_ID),
		Some(darkdot_properties()),
		None,
	)
}

pub fn local_testnet_config() -> ChainSpec {
	ChainSpec::from_genesis(
		"Local Testnet",
		"local_testnet",
		ChainType::Local,
		|| {
			let endowed_accounts = vec![
				get_account_id_from_seed::<sr25519::Public>("Alice"),
				get_account_id_from_seed::<sr25519::Public>("Bob"),
				get_account_id_from_seed::<sr25519::Public>("Charlie"),
				get_account_id_from_seed::<sr25519::Public>("Dave"),
				get_account_id_from_seed::<sr25519::Public>("Eve"),
				get_account_id_from_seed::<sr25519::Public>("Ferdie"),
				get_account_id_from_seed::<sr25519::Public>("Alice//stash"),
				get_account_id_from_seed::<sr25519::Public>("Bob//stash"),
				get_account_id_from_seed::<sr25519::Public>("Charlie//stash"),
				get_account_id_from_seed::<sr25519::Public>("Dave//stash"),
				get_account_id_from_seed::<sr25519::Public>("Eve//stash"),
				get_account_id_from_seed::<sr25519::Public>("Ferdie//stash"),
			];

			testnet_genesis(
				vec![
					authority_keys_from_seed("Alice"),
					authority_keys_from_seed("Bob"),
				],
				get_account_id_from_seed::<sr25519::Public>("Alice"),
				endowed_accounts.iter().cloned().map(|k| (k, 777_777)).collect(),
				get_account_id_from_seed::<sr25519::Public>("Alice"),
				true,
			)
		},
		vec![],
		None,
		Some(DEFAULT_PROTOCOL_ID),
		Some(darkdot_properties()),
		None,
	)
}

pub fn darkdot_config() -> Result<ChainSpec, String> {
	ChainSpec::from_json_bytes(&include_bytes!("../res/dystopiaStagingSpec.json")[..])
}

pub fn darkdot_test_config() -> ChainSpec {
	ChainSpec::from_genesis(
		"Darkdot",
		"darkdotT",
		ChainType::Local,
		|| testnet_genesis(
			vec![
				(
					/* AuraId SR25519 */
					hex!["ac940b8ee399d42faeb7169f322e6623f8219d12ad4c42dfe0995fa9f9713a0d"].unchecked_into(),
					/* GrandpaId ED25519 */
					hex!["e97b51af33429b5c4ab8ddd9b3fc542d24154bbeef807d559eff3906afca8413"].unchecked_into()
				),
				(
					/* AuraId SR25519 */
					hex!["0c053087dd7782de467228b5f826c5031be2faf315baa766a89b48bb6e2dfb71"].unchecked_into(),
					/* GrandpaId ED25519 */
					hex!["b48a83ed87ef39bc90c205fb551af3c076e1a952881d7fefec08cbb76e17ab8b"].unchecked_into()
				),
			],
			/* Sudo Account */
			get_account_id_from_seed::<sr25519::Public>("Alice"),
			vec![
				(
					/* Sudo Account */
					get_account_id_from_seed::<sr25519::Public>("Alice"),
					/* Balance */
					16_000_000
				),
				(
					/* Account X1 */
					get_account_id_from_seed::<sr25519::Public>("5GukQt4gJW2XqzFwmm3RHa7x6sYuVcGhuhz72CN7oiBsgffx"),
					/* Balance */
					2_499_000
				),
				(
					/* Account X2 */
					hex!["24d6d8fc5d051fd471e275f14c83e95287d2b863e4cc802de1f78dea06c6ca78"].into(),
					/* Balance */
					2_500_000
				),
				(
					/* Account X3 */
					hex!["24d6d901fb0531124040630e52cfd746ef7d037922c4baf290f513dbc3d47d66"].into(),
					/* Balance */
					2_500_000
				),
				(
					/* Account X4 */
					hex!["24d6d22d63313e82f9461281cb69aacad1828dc74273274751fd24333b182c68"].into(),
					/* Balance */
					2_500_000
				),
			],
			// Treasury
			hex!["24d6d683750c4c10e90dd81430efec95133e1ec1f5be781d3267390d03174706"].into(),
			true,
		),
		vec![],
		Some(TelemetryEndpoints::new(
			vec![(STAGING_TELEMETRY_URL.to_string(), 0)]
		).expect("Staging telemetry url is valid; qed")),
		Some(DEFAULT_PROTOCOL_ID),
		Some(darkdot_properties()),
		None,
	)
}


// dystopia dev
// v0.1
pub fn dystopia_config() -> ChainSpec {
	ChainSpec::from_genesis(
		"DARK ▼ Dystopia PoC testnet",
		"darkdot",
		ChainType::Live,
	   || testnet_genesis(
		   vec![
			   (
				   /* AuraId SR25519 */
				   hex!["92691d55874b695a0a3028ef97eabf23abbdaf55450dd3bffc0806ad906c010d"].unchecked_into(),
				   /* GrandpaId ED25519 */
				   hex!["05d1ba9db0159f216e7237c159b8f0820759d19e3b6e0c7d8f2ec914057a6066"].unchecked_into(),
			   ),
			   (
				   /* AuraId SR25519 */
				   hex!["2097e5734092988b40b6ee361c8db31043bd704778833f7eb03aa77c55f43960"].unchecked_into(),
				   /* GrandpaId ED25519 */
				   hex!["e196a0ecafa800480c4e17e0c6d9464af9b69fa16d06aa94b6ba109cf077cb82"].unchecked_into()
			   ),
		   ],
		   /* Sudo Account */
		   hex!["76c39a83c10ae7e0f7dadb76a230182a1c14423b41a5baad9c648e666da37f2f"].into(),
		   vec![
			   (
				   /* Sudo Account */
				   hex!["76c39a83c10ae7e0f7dadb76a230182a1c14423b41a5baad9c648e666da37f2f"].into(),
				   /* Balance */
				   6_112_044
			   ),
			   (
				   /* Account X1 */
				   hex!["92691d55874b695a0a3028ef97eabf23abbdaf55450dd3bffc0806ad906c010d"].into(),
				   /* Balance */
				   100_000
			   ),
			   (
				   /* Account X2 */
				   hex!["2097e5734092988b40b6ee361c8db31043bd704778833f7eb03aa77c55f43960"].into(),
				   /* Balance */
				   100_000
			   ),
			   (
				   /* Account X3 */
				   hex!["dc0e38d70683646e675c4db0fd911d9a5eb4583efc151ab4bccd5418af222336"].into(),
				   /* Balance */
				   200_000
			   ),
			   (
				/* Account S */
				hex!["b68476eec9542c15389e8e24aa7d5abe9b2dd171c6230ac0cf8854d31a079475"].into(),
				/* Balance */
				200_000
			),
		   ],
		   // Treasury
		   hex!["dc0e38d70683646e675c4db0fd911d9a5eb4583efc151ab4bccd5418af222336"].into(),
		   true,
	   ),
	   vec![],
	   Some(TelemetryEndpoints::new(
		   vec![(STAGING_TELEMETRY_URL.to_string(), 0)]
	   ).expect("Staging telemetry url is valid; qed")),
	   Some(DEFAULT_PROTOCOL_ID),
	   Some(darkdot_properties()),
	   None,
   )
}




fn testnet_genesis(
	initial_authorities: Vec<(AuraId, GrandpaId)>,
	root_key: AccountId,
	endowed_accounts: Vec<(AccountId, u128)>,
	treasury_account_id: AccountId,
	_enable_println: bool
) -> GenesisConfig {
	GenesisConfig {
		system: Some(SystemConfig {
			code: WASM_BINARY.to_vec(),
			changes_trie_config: Default::default(),
		}),
		balances: Some(BalancesConfig {
			balances: endowed_accounts.iter().cloned().map(|(k, b)|(k, b * DARKS)).collect(),
		}),
		aura: Some(AuraConfig {
			authorities: initial_authorities.iter().map(|x| (x.0.clone())).collect(),
		}),
		grandpa: Some(GrandpaConfig {
			authorities: initial_authorities.iter().map(|x| (x.1.clone(), 1)).collect(),
		}),
		sudo: Some(SudoConfig {
			key: root_key.clone(),
		}),
		pallet_utils: Some(UtilsConfig {
			treasury_account: treasury_account_id,
		}),
		pallet_storefronts: Some(StorefrontsConfig {
			endowed_account: root_key,
		}),
	}
}

pub fn darkdot_properties() -> Properties {
	let mut properties = Properties::new();

	properties.insert("ss58Format".into(), 17.into());
	properties.insert("tokenDecimals".into(), 12.into());
	properties.insert("tokenSymbol".into(), "DARK".into());

	properties
}
