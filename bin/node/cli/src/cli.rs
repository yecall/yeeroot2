
use sc_cli::{ImportParams, RunCmd, SharedParams};
use structopt::StructOpt;

/// An overarching CLI command definition.
#[derive(Clone, Debug, StructOpt)]
pub struct Cli {
	/// Possible subcommand with parameters.
	#[structopt(subcommand)]
	pub subcommand: Option<Subcommand>,
	#[allow(missing_docs)]
	#[structopt(flatten)]
	pub run: RunCmd,
}

/// Possible subcommands of the main binary.
#[derive(Clone, Debug, StructOpt)]
pub enum Subcommand {
	/// A set of base subcommands handled by `sc_cli`.
	#[structopt(flatten)]
	Base(sc_cli::Subcommand),

	/// The custom inspect subcommmand for decoding blocks and extrinsics.
	#[structopt(
		name = "inspect",
		about = "Decode given block or extrinsic using current native runtime."
	)]
	Inspect(node_inspect::cli::InspectCmd),

	/// The custom benchmark subcommmand benchmarking runtime pallets.
	#[structopt(name = "benchmark", about = "Benchmark runtime pallets.")]
	Benchmark(frame_benchmarking_cli::BenchmarkCmd),

	// /// The custom bootnodes-router sub-command
	// #[structopt(name = "bootnodes-router", about = "Run yee in `bootnodes-router` mode.")]
	// BootnodesRouter(yc_cli::BootnodesRouterCmd),

	// /// The custom switch sub-command
	// #[structopt(name = "switch", about = "Run yee in `switch` mode.")]
	// Switch(yc_cli::SwitchCmd),
}

/// The `factory` command used to generate transactions.
/// Please note: this command currently only works on an empty database!
#[derive(Debug, StructOpt, Clone)]
pub struct FactoryCmd {
	/// Number of blocks to generate.
	#[structopt(long = "blocks", default_value = "1")]
	pub blocks: u32,

	/// Number of transactions to push per block.
	#[structopt(long = "transactions", default_value = "8")]
	pub transactions: u32,

	#[allow(missing_docs)]
	#[structopt(flatten)]
	pub shared_params: SharedParams,

	#[allow(missing_docs)]
	#[structopt(flatten)]
	pub import_params: ImportParams,
}