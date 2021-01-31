//! Service and ServiceFactory implementation. Specialized wrapper over substrate service.

use std::sync::Arc;
use std::time::Duration;
use sc_client_api::{ExecutorProvider, RemoteBackend};
use sc_consensus::LongestChain;
use dark_runtime::{self, opaque::Block, RuntimeApi};
use sc_service::{error::{Error as ServiceError}, AbstractService, Configuration, ServiceBuilder};
use sp_inherents::InherentDataProviders;
use sc_executor::native_executor_instance;
pub use sc_executor::NativeExecutor;
use sp_consensus_aura::sr25519::{AuthorityPair as AuraPair};
use sc_finality_grandpa::{
	FinalityProofProvider as GrandpaFinalityProofProvider, StorageAndProofProvider, SharedVoterState,
};


// ****** PRINT GENESIS INFO
use codec::{Encode, Decode};
use sp_runtime::generic::BlockId;
use sp_core::storage::StorageKey;
use sp_finality_grandpa::{AuthorityList, VersionedAuthorityList, GRANDPA_AUTHORITIES_KEY};
use sc_client_api::StorageProof;
use sc_client_api::backend::StorageProvider;
use sc_client_api::proof_provider::ProofProvider;

// Shared struct between chain client and pRuntime. Used to init the bridge.
#[derive(Encode, Debug)]
pub struct GenesisGrandpaInfo {
	block_header: dark_runtime::Header,
	pub validator_set: AuthorityList,
	validator_set_proof: Vec<Vec<u8>>,
}

impl GenesisGrandpaInfo {
	fn new(
		block_header: dark_runtime::Header,
		validator_set: AuthorityList,
		validator_set_proof: StorageProof
	) -> Self {
		let raw_proof: Vec<Vec<u8>> = validator_set_proof.iter_nodes().collect();

		Self {
			block_header,
			validator_set,
			validator_set_proof: raw_proof,
		}
	}
}

use std::path::PathBuf;
use std::fs;

fn output_genesis_grandpa(genesis_info: GenesisGrandpaInfo, save_dir: &PathBuf) {
	let data = genesis_info.encode();
	let b64 = base64::encode(&data);
	println!("Genesis Grandpa Info ({}): {}", save_dir.to_str().unwrap(), b64);
	fs::write(save_dir, &b64).expect("Unable to write genesis-info.txt");
}





// ****** END GENESIS

// Our native executor instance.
native_executor_instance!(
	pub Executor,
	dark_runtime::api::dispatch,
	dark_runtime::native_version,
);

/// Starts a `ServiceBuilder` for a full service.
///
/// Use this macro if you don't actually need the full service, but just the builder in order to
/// be able to perform chain operations.
macro_rules! new_full_start {
	($config:expr) => {{
		use std::sync::Arc;
		use sp_consensus_aura::sr25519::AuthorityPair as AuraPair;

		let mut import_setup = None;
		let inherent_data_providers = sp_inherents::InherentDataProviders::new();

		let builder = sc_service::ServiceBuilder::new_full::<
			dark_runtime::opaque::Block,
			dark_runtime::RuntimeApi,
			crate::service::Executor
		>($config)?
			.with_select_chain(|_config, backend| {
				Ok(sc_consensus::LongestChain::new(backend.clone()))
			})?
			.with_transaction_pool(|builder| {
				let pool_api = sc_transaction_pool::FullChainApi::new(
					builder.client().clone(),
				);
				Ok(sc_transaction_pool::BasicPool::new(
					builder.config().transaction_pool.clone(),
					std::sync::Arc::new(pool_api),
					builder.prometheus_registry(),
				))
			})?
			.with_import_queue(|
				_config,
				client,
				mut select_chain,
				_transaction_pool,
				spawn_task_handle,
				registry,
			| {
				let select_chain = select_chain.take()
					.ok_or_else(|| sc_service::Error::SelectChainRequired)?;

				let (grandpa_block_import, grandpa_link) = sc_finality_grandpa::block_import(
					client.clone(),
					&(client.clone() as Arc<_>),
					select_chain,
				)?;

				let aura_block_import = sc_consensus_aura::AuraBlockImport::<_, _, _, AuraPair>::new(
					grandpa_block_import.clone(), client.clone(),
				);

				let import_queue = sc_consensus_aura::import_queue::<_, _, _, AuraPair, _>(
					sc_consensus_aura::slot_duration(&*client)?,
					aura_block_import,
					Some(Box::new(grandpa_block_import.clone())),
					None,
					client,
					inherent_data_providers.clone(),
					spawn_task_handle,
					registry,
				)?;

				import_setup = Some((grandpa_block_import, grandpa_link));

				Ok(import_queue)
			})?;

		(builder, import_setup, inherent_data_providers)
	}}
}

/// Builds a new service for a full client.
pub fn new_full(config: Configuration) -> Result<impl AbstractService, ServiceError> {
	let role = config.role.clone();
	let force_authoring = config.force_authoring;
	let name = config.network.node_name.clone();
	let disable_grandpa = config.disable_grandpa;
	


	let genesis_info_path = config.network.net_config_path.clone().expect("Missing base_path").join("../genesis-info.txt"); // GENISIS



	let (builder, mut import_setup, inherent_data_providers) = new_full_start!(config);

	let (block_import, grandpa_link) =
		import_setup.take()
			.expect("Link Half and Block Import are present for Full Services or setup failed before. qed");

	let service = builder
		.with_finality_proof_provider(|client, backend| {
			// GenesisAuthoritySetProvider is implemented for StorageAndProofProvider
			let provider = client as Arc<dyn StorageAndProofProvider<_, _>>;
			Ok(Arc::new(GrandpaFinalityProofProvider::new(backend, provider)) as _)
		})?
		.build_full()?;

// ***** FOR GENISIS

let grandpa_info = {
	let client = &service.client();
	let blocknum = BlockId::Number(0);
	let header = client.header(&blocknum)
		.expect("Missing genesis block; qed")
		.expect("Missing genesis block; qed");
	 println!("###### genesis header: {:?}", header);
	let storage_proof = client.read_proof(&blocknum, &mut std::iter::once(GRANDPA_AUTHORITIES_KEY)).expect("No GRANNDPA authorties key; qed");
	 println!("###### genesis grandpa_authorities proof: {:?}", storage_proof);

	let storage_key = StorageKey(GRANDPA_AUTHORITIES_KEY.to_vec());
	let validator_set: AuthorityList = client.storage(&blocknum, &storage_key)?
		.and_then(|encoded| VersionedAuthorityList::decode(&mut encoded.0.as_slice()).ok())
		.map(|versioned| versioned.into())
		.expect("Bad genesis config; qed");
	 println!("###### authority list: {:?}", validator_set);

	GenesisGrandpaInfo::new(header, validator_set, storage_proof)
};
println!("###### GRANDPA_GENESIS: {:?}", grandpa_info);
output_genesis_grandpa(grandpa_info, &genesis_info_path);


// ***** END GENISIS



// AUTHORITY


	if role.is_authority() {
		let proposer = sc_basic_authorship::ProposerFactory::new(
			service.client(),
			service.transaction_pool(),
			service.prometheus_registry().as_ref(),
		);

		let client = service.client();
		let select_chain = service.select_chain()
			.ok_or(ServiceError::SelectChainRequired)?;

		let can_author_with =
			sp_consensus::CanAuthorWithNativeVersion::new(client.executor().clone());

		let aura = sc_consensus_aura::start_aura::<_, _, _, _, _, AuraPair, _, _, _>(
			sc_consensus_aura::slot_duration(&*client)?,
			client,
			select_chain,
			block_import,
			proposer,
			service.network(),
			inherent_data_providers.clone(),
			force_authoring,
			service.keystore(),
			can_author_with,
		)?;

		// the AURA authoring task is considered essential, i.e. if it
		// fails we take down the service with it.
		service.spawn_essential_task_handle().spawn_blocking("aura", aura);
	}

	// if the node isn't actively participating in consensus then it doesn't
	// need a keystore, regardless of which protocol we use below.
	let keystore = if role.is_authority() {
		Some(service.keystore() as sp_core::traits::BareCryptoStorePtr)
	} else {
		None
	};

	let grandpa_config = sc_finality_grandpa::Config {
		// FIXME #1578 make this available through chainspec
		gossip_duration: Duration::from_millis(333),
		justification_period: 1, // 512, default before GENISIS
		name: Some(name),
		observer_enabled: false,
		keystore,
		is_authority: role.is_network_authority(),
	};


// OCW
// let dev_seed = config.dev_key_seed.clone();
// if let Some(seed) = dev_seed {
// 	keystore
// 		.write()
// 		.insert_ephemeral_from_seed_by_type::<dark_runtime::pallet_ocw::crypto::Pair>(
// 			&seed,
// 			dark_runtime::pallet_ocw::KEY_TYPE,
// 		)
// 		.expect("Dev Seed should always succeed.");
// }



	let enable_grandpa = !disable_grandpa;
	if enable_grandpa {
		// start the full GRANDPA voter
		// NOTE: non-authorities could run the GRANDPA observer protocol, but at
		// this point the full voter should provide better guarantees of block
		// and vote data availability than the observer. The observer has not
		// been tested extensively yet and having most nodes in a network run it
		// could lead to finality stalls.
		let grandpa_config = sc_finality_grandpa::GrandpaParams {
			config: grandpa_config,
			link: grandpa_link,
			network: service.network(),
			inherent_data_providers: inherent_data_providers.clone(),
			telemetry_on_connect: Some(service.telemetry_on_connect_stream()),
			voting_rule: sc_finality_grandpa::VotingRulesBuilder::default().build(),
			prometheus_registry: service.prometheus_registry(),
			shared_voter_state: SharedVoterState::empty(),
		};

		// the GRANDPA voter task is considered infallible, i.e.
		// if it fails we take down the service with it.
		service.spawn_essential_task_handle().spawn_blocking(
			"grandpa-voter",
			sc_finality_grandpa::run_grandpa_voter(grandpa_config)?
		);
	} else {
		sc_finality_grandpa::setup_disabled_grandpa(
			service.client(),
			&inherent_data_providers,
			service.network(),
		)?;
	}

	Ok(service)
}

/// Builds a new service for a light client.
pub fn new_light(config: Configuration) -> Result<impl AbstractService, ServiceError> {
	let inherent_data_providers = InherentDataProviders::new();

	ServiceBuilder::new_light::<Block, RuntimeApi, Executor>(config)?
		.with_select_chain(|_config, backend| {
			Ok(LongestChain::new(backend.clone()))
		})?
		.with_transaction_pool(|builder| {
			let fetcher = builder.fetcher()
				.ok_or_else(|| "Trying to start light transaction pool without active fetcher")?;

			let pool_api = sc_transaction_pool::LightChainApi::new(
				builder.client().clone(),
				fetcher.clone(),
			);
			let pool = sc_transaction_pool::BasicPool::with_revalidation_type(
				builder.config().transaction_pool.clone(),
				Arc::new(pool_api),
				builder.prometheus_registry(),
				sc_transaction_pool::RevalidationType::Light,
			);
			Ok(pool)
		})?
		.with_import_queue_and_fprb(|
			_config,
			client,
			backend,
			fetcher,
			_select_chain,
			_tx_pool,
			spawn_task_handle,
			prometheus_registry,
		| {
			let fetch_checker = fetcher
				.map(|fetcher| fetcher.checker().clone())
				.ok_or_else(|| "Trying to start light import queue without active fetch checker")?;
			let grandpa_block_import = sc_finality_grandpa::light_block_import(
				client.clone(),
				backend,
				&(client.clone() as Arc<_>),
				Arc::new(fetch_checker),
			)?;
			let finality_proof_import = grandpa_block_import.clone();
			let finality_proof_request_builder =
				finality_proof_import.create_finality_proof_request_builder();

			let import_queue = sc_consensus_aura::import_queue::<_, _, _, AuraPair, _>(
				sc_consensus_aura::slot_duration(&*client)?,
				grandpa_block_import,
				None,
				Some(Box::new(finality_proof_import)),
				client,
				inherent_data_providers.clone(),
				spawn_task_handle,
				prometheus_registry,
			)?;

			Ok((import_queue, finality_proof_request_builder))
		})?
		.with_finality_proof_provider(|client, backend| {
			// GenesisAuthoritySetProvider is implemented for StorageAndProofProvider
			let provider = client as Arc<dyn StorageAndProofProvider<_, _>>;
			Ok(Arc::new(GrandpaFinalityProofProvider::new(backend, provider)) as _)
		})?
		.build_light()
}
