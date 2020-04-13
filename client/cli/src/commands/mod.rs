
mod build_spec_cmd;
mod check_block_cmd;
mod export_blocks_cmd;
mod import_blocks_cmd;
mod purge_chain_cmd;
mod revert_cmd;
mod runcmd;

pub use crate::commands::build_spec_cmd::BuildSpecCmd;
pub use crate::commands::check_block_cmd::CheckBlockCmd;
pub use crate::commands::export_blocks_cmd::ExportBlocksCmd;
pub use crate::commands::import_blocks_cmd::ImportBlocksCmd;
pub use crate::commands::purge_chain_cmd::PurgeChainCmd;
pub use crate::commands::revert_cmd::RevertCmd;
pub use crate::commands::runcmd::RunCmd;
use std::fmt::Debug;
use std::path::PathBuf;
use structopt::StructOpt;
use crate::CliConfiguration;
use crate::{SharedParams, ImportParams};

/// All core commands that are provided by default.
///
/// The core commands are split into multiple subcommands and `Run` is the default subcommand. From
/// the CLI user perspective, it is not visible that `Run` is a subcommand. So, all parameters of
/// `Run` are exported as main executable parameters.
#[derive(Debug, Clone, StructOpt)]
pub enum Subcommand {
	/// Build a spec.json file, outputs to stdout.
	BuildSpec(BuildSpecCmd),

	/// Export blocks to a file.
	ExportBlocks(ExportBlocksCmd),

	/// Import blocks from file.
	ImportBlocks(ImportBlocksCmd),

	/// Validate a single block.
	CheckBlock(CheckBlockCmd),

	/// Revert chain to the previous state.
	Revert(RevertCmd),

	/// Remove the whole chain data.
	PurgeChain(PurgeChainCmd),
}

// TODO: move to config.rs?
/// Macro that helps implement CliConfiguration on an enum of subcommand automatically
///
/// # Example
///
/// ```
/// # #[macro_use] extern crate yc_cli;
///
/// # struct EmptyVariant {}
///
///	# impl yc_cli::CliConfiguration for EmptyVariant {
///	#     fn shared_params(&self) -> &yc_cli::SharedParams { unimplemented!() }
///	#     fn chain_id(&self, _: bool) -> yc_cli::Result<String> { Ok("test-chain-id".to_string()) }
///	# }
///
/// # fn main() {
/// enum Subcommand {
///	    Variant1(EmptyVariant),
///	    Variant2(EmptyVariant),
///	}
///
/// substrate_cli_subcommands!(
///     Subcommand => Variant1, Variant2
/// );
///
/// # use yc_cli::CliConfiguration;
/// # assert_eq!(Subcommand::Variant1(EmptyVariant {}).chain_id(false).unwrap(), "test-chain-id");
///
/// # }
/// ```
///
/// Which will expand to:
///
/// ```ignore
/// impl CliConfiguration for Subcommand {
///	    fn base_path(&self) -> Result<Option<PathBuf>> {
///	        match self {
///	            Subcommand::Variant1(cmd) => cmd.base_path(),
///	            Subcommand::Variant2(cmd) => cmd.base_path(),
///	        }
///	    }
///
///	    fn is_dev(&self) -> Result<bool> {
///	        match self {
///	            Subcommand::Variant1(cmd) => cmd.is_dev(),
///	            Subcommand::Variant2(cmd) => cmd.is_dev(),
///	        }
///	    }
///
///     // ...
/// }
/// ```
#[macro_export]
macro_rules! substrate_cli_subcommands {
	($enum:ident => $($variant:ident),*) => {
		impl $crate::CliConfiguration for $enum {
			fn shared_params(&self) -> &$crate::SharedParams {
				match self {
					$($enum::$variant(cmd) => cmd.shared_params()),*
				}
			}

			fn import_params(&self) -> Option<&$crate::ImportParams> {
				match self {
					$($enum::$variant(cmd) => cmd.import_params()),*
				}
			}

			fn pruning_params(&self) -> Option<&$crate::PruningParams> {
				match self {
					$($enum::$variant(cmd) => cmd.pruning_params()),*
				}
			}

			fn keystore_params(&self) -> Option<&$crate::KeystoreParams> {
				match self {
					$($enum::$variant(cmd) => cmd.keystore_params()),*
				}
			}

			fn network_params(&self) -> Option<&$crate::NetworkParams> {
				match self {
					$($enum::$variant(cmd) => cmd.network_params()),*
				}
			}

			fn base_path(&self) -> $crate::Result<::std::option::Option<::std::path::PathBuf>> {
				match self {
					$($enum::$variant(cmd) => cmd.base_path()),*
				}
			}

			fn is_dev(&self) -> $crate::Result<bool> {
				match self {
					$($enum::$variant(cmd) => cmd.is_dev()),*
				}
			}

			fn role(&self, is_dev: bool) -> $crate::Result<::sc_service::Role> {
				match self {
					$($enum::$variant(cmd) => cmd.role(is_dev)),*
				}
			}

			fn transaction_pool(&self)
			-> $crate::Result<::sc_service::config::TransactionPoolOptions> {
				match self {
					$($enum::$variant(cmd) => cmd.transaction_pool()),*
				}
			}

			fn network_config(
				&self,
				chain_spec: &::std::boxed::Box<dyn ::sc_service::ChainSpec>,
				is_dev: bool,
				net_config_dir: &::std::path::PathBuf,
				client_id: &str,
				node_name: &str,
				node_key: ::sc_service::config::NodeKeyConfig,
			) -> $crate::Result<::sc_service::config::NetworkConfiguration> {
				match self {
					$(
						$enum::$variant(cmd) => cmd.network_config(
							chain_spec, is_dev, net_config_dir, client_id, node_name, node_key
						)
					),*
				}
			}

			fn keystore_config(&self, base_path: &::std::path::PathBuf)
			-> $crate::Result<::sc_service::config::KeystoreConfig> {
				match self {
					$($enum::$variant(cmd) => cmd.keystore_config(base_path)),*
				}
			}

			fn database_cache_size(&self) -> $crate::Result<::std::option::Option<usize>> {
				match self {
					$($enum::$variant(cmd) => cmd.database_cache_size()),*
				}
			}

			fn database_config(
				&self,
				base_path: &::std::path::PathBuf,
				cache_size: usize,
			) -> $crate::Result<::sc_service::config::DatabaseConfig> {
				match self {
					$($enum::$variant(cmd) => cmd.database_config(base_path, cache_size)),*
				}
			}

			fn state_cache_size(&self) -> $crate::Result<usize> {
				match self {
					$($enum::$variant(cmd) => cmd.state_cache_size()),*
				}
			}

			fn state_cache_child_ratio(&self) -> $crate::Result<::std::option::Option<usize>> {
				match self {
					$($enum::$variant(cmd) => cmd.state_cache_child_ratio()),*
				}
			}

			fn pruning(&self, is_dev: bool, role: &::sc_service::Role)
			-> $crate::Result<::sc_service::config::PruningMode> {
				match self {
					$($enum::$variant(cmd) => cmd.pruning(is_dev, role)),*
				}
			}

			fn chain_id(&self, is_dev: bool) -> $crate::Result<String> {
				match self {
					$($enum::$variant(cmd) => cmd.chain_id(is_dev)),*
				}
			}

			fn init<C: $crate::YeerootCli>(&self) -> $crate::Result<()> {
				match self {
					$($enum::$variant(cmd) => cmd.init::<C>()),*
				}
			}

			fn node_name(&self) -> $crate::Result<String> {
				match self {
					$($enum::$variant(cmd) => cmd.node_name()),*
				}
			}

			fn wasm_method(&self) -> $crate::Result<::sc_service::config::WasmExecutionMethod> {
				match self {
					$($enum::$variant(cmd) => cmd.wasm_method()),*
				}
			}

			fn execution_strategies(&self, is_dev: bool)
			-> $crate::Result<::sc_service::config::ExecutionStrategies> {
				match self {
					$($enum::$variant(cmd) => cmd.execution_strategies(is_dev)),*
				}
			}

			fn rpc_http(&self) -> $crate::Result<::std::option::Option<::std::net::SocketAddr>> {
				match self {
					$($enum::$variant(cmd) => cmd.rpc_http()),*
				}
			}

			fn rpc_ws(&self) -> $crate::Result<::std::option::Option<::std::net::SocketAddr>> {
				match self {
					$($enum::$variant(cmd) => cmd.rpc_ws()),*
				}
			}

			fn rpc_ws_max_connections(&self) -> $crate::Result<::std::option::Option<usize>> {
				match self {
					$($enum::$variant(cmd) => cmd.rpc_ws_max_connections()),*
				}
			}

			fn rpc_cors(&self, is_dev: bool)
			-> $crate::Result<::std::option::Option<::std::vec::Vec<String>>> {
				match self {
					$($enum::$variant(cmd) => cmd.rpc_cors(is_dev)),*
				}
			}

			fn prometheus_config(&self)
			-> $crate::Result<::std::option::Option<::sc_service::config::PrometheusConfig>> {
				match self {
					$($enum::$variant(cmd) => cmd.prometheus_config()),*
				}
			}

			fn telemetry_endpoints(
				&self,
				chain_spec: &Box<dyn ::sc_service::ChainSpec>,
			) -> $crate::Result<::std::option::Option<::sc_service::config::TelemetryEndpoints>> {
				match self {
					$($enum::$variant(cmd) => cmd.telemetry_endpoints(chain_spec)),*
				}
			}

			fn telemetry_external_transport(&self)
			-> $crate::Result<::std::option::Option<::sc_service::config::ExtTransport>> {
				match self {
					$($enum::$variant(cmd) => cmd.telemetry_external_transport()),*
				}
			}

			fn default_heap_pages(&self) -> $crate::Result<::std::option::Option<u64>> {
				match self {
					$($enum::$variant(cmd) => cmd.default_heap_pages()),*
				}
			}

			fn offchain_worker(&self, role: &::sc_service::Role) -> $crate::Result<bool> {
				match self {
					$($enum::$variant(cmd) => cmd.offchain_worker(role)),*
				}
			}

			fn force_authoring(&self) -> $crate::Result<bool> {
				match self {
					$($enum::$variant(cmd) => cmd.force_authoring()),*
				}
			}

			fn disable_grandpa(&self) -> $crate::Result<bool> {
				match self {
					$($enum::$variant(cmd) => cmd.disable_grandpa()),*
				}
			}

			fn dev_key_seed(&self, is_dev: bool) -> $crate::Result<::std::option::Option<String>> {
				match self {
					$($enum::$variant(cmd) => cmd.dev_key_seed(is_dev)),*
				}
			}

			fn tracing_targets(&self) -> $crate::Result<::std::option::Option<String>> {
				match self {
					$($enum::$variant(cmd) => cmd.tracing_targets()),*
				}
			}

			fn tracing_receiver(&self) -> $crate::Result<::sc_service::TracingReceiver> {
				match self {
					$($enum::$variant(cmd) => cmd.tracing_receiver()),*
				}
			}

			fn node_key(&self, net_config_dir: &::std::path::PathBuf)
			-> $crate::Result<::sc_service::config::NodeKeyConfig> {
				match self {
					$($enum::$variant(cmd) => cmd.node_key(net_config_dir)),*
				}
			}

			fn max_runtime_instances(&self) -> $crate::Result<::std::option::Option<usize>> {
				match self {
					$($enum::$variant(cmd) => cmd.max_runtime_instances()),*
				}
			}

			fn log_filters(&self) -> $crate::Result<::std::option::Option<String>> {
				match self {
					$($enum::$variant(cmd) => cmd.log_filters()),*
				}
			}
		}
	}
}

substrate_cli_subcommands!(
	Subcommand => BuildSpec, ExportBlocks, ImportBlocks, CheckBlock, Revert, PurgeChain
);



#[derive(Debug, StructOpt, Clone)]
pub struct BootnodesRouterCmd {
	#[allow(missing_docs)]
	#[structopt(flatten)]
	pub shared_params: SharedParams,

	#[allow(missing_docs)]
	#[structopt(flatten)]
	pub import_params: ImportParams,

	/// Specify TCP port.
	#[structopt(long = "port", value_name = "PORT")]
	pub port: Option<u16>,

	/// Specify custom base path.
	#[structopt(long = "base-path", short = "d", value_name = "PATH", parse(from_os_str))]
	pub base_path: Option<PathBuf>,

	/// Sets a custom logging filter
	#[structopt(short = "l", long = "log", value_name = "LOG_PATTERN")]
	pub log: Option<String>,

	/// Whether use dev params or not
	#[structopt(long = "dev-params")]
	pub dev_params: bool,

	/// Shard count on dev mode
	#[structopt(long = "dev-shard-count")]
	pub dev_shard_count: Option<u16>,
}

#[derive(Debug, StructOpt, Clone)]
pub struct SwitchCmd {
	#[allow(missing_docs)]
	#[structopt(flatten)]
	pub shared_params: SharedParams,

	#[allow(missing_docs)]
	#[structopt(flatten)]
	pub import_params: ImportParams,

	// todo
}


impl CliConfiguration for BootnodesRouterCmd {
	fn shared_params(&self) -> &SharedParams {
		&self.shared_params
	}

	fn import_params(&self) -> Option<&ImportParams> {
		Some(&self.import_params)
	}

	// todo
}

impl CliConfiguration for SwitchCmd {
	fn shared_params(&self) -> &SharedParams {
		&self.shared_params
	}

	fn import_params(&self) -> Option<&ImportParams> {
		Some(&self.import_params)
	}

	// todo
}
