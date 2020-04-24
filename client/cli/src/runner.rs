
use crate::CliConfiguration;
use crate::error::Result;
use crate::YeerootCli;
use crate::Subcommand;
use crate::{BootnodesRouterCmd, SwitchCmd};
use chrono::prelude::*;
use futures::pin_mut;
use futures::select;
use futures::{future, future::FutureExt, Future};
use log::info;
use sc_service::{AbstractService, Configuration, Role, ServiceBuilderCommand};
use sp_runtime::traits::{Block as BlockT, Header as HeaderT};
use sp_utils::metrics::{TOKIO_THREADS_ALIVE, TOKIO_THREADS_TOTAL};
use std::fmt::Debug;
use std::marker::PhantomData;
use std::sync::Arc;
// use yee_bootnodes_router::Cmd as BRExeCmd;
// use yee_switch::Cmd as SExeCmd;

#[cfg(target_family = "unix")]
async fn main<F, E>(func: F) -> std::result::Result<(), Box<dyn std::error::Error>>
where
	F: Future<Output = std::result::Result<(), E>> + future::FusedFuture,
	E: 'static + std::error::Error,
{
	use tokio::signal::unix::{signal, SignalKind};

	let mut stream_int = signal(SignalKind::interrupt())?;
	let mut stream_term = signal(SignalKind::terminate())?;

	let t1 = stream_int.recv().fuse();
	let t2 = stream_term.recv().fuse();
	let t3 = func;

	pin_mut!(t1, t2, t3);

	select! {
		_ = t1 => {},
		_ = t2 => {},
		res = t3 => res?,
	}

	Ok(())
}

#[cfg(not(unix))]
async fn main<F, E>(func: F) -> std::result::Result<(), Box<dyn std::error::Error>>
where
	F: Future<Output = std::result::Result<(), E>> + future::FusedFuture,
	E: 'static + std::error::Error,
{
	use tokio::signal::ctrl_c;

	let t1 = ctrl_c().fuse();
	let t2 = func;

	pin_mut!(t1, t2);

	select! {
		_ = t1 => {},
		res = t2 => res?,
	}

	Ok(())
}

/// Build a tokio runtime with all features
pub fn build_runtime() -> std::result::Result<tokio::runtime::Runtime, std::io::Error> {
	tokio::runtime::Builder::new()
		.thread_name("main-tokio-")
		.threaded_scheduler()
		.on_thread_start(||{
			TOKIO_THREADS_ALIVE.inc();
			TOKIO_THREADS_TOTAL.inc();
		})
		.on_thread_stop(||{
			TOKIO_THREADS_ALIVE.dec();
		})
		.enable_all()
		.build()
}

fn run_until_exit<FUT, ERR>(mut tokio_runtime: tokio::runtime::Runtime, future: FUT) -> Result<()>
where
	FUT: Future<Output = std::result::Result<(), ERR>> + future::Future,
	ERR: 'static + std::error::Error,
{
	let f = future.fuse();
	pin_mut!(f);

	tokio_runtime.block_on(main(f)).map_err(|e| e.to_string())?;

	Ok(())
}

/// A Substrate CLI runtime that can be used to run a node or a command
pub struct Runner<C: YeerootCli> {
	config: Configuration,
	tokio_runtime: tokio::runtime::Runtime,
	phantom: PhantomData<C>,
}

impl<C: YeerootCli> Runner<C> {
	/// Create a new runtime with the command provided in argument
	pub fn new<T: CliConfiguration>(cli: &C, command: &T) -> Result<Runner<C>> {
		let tokio_runtime = build_runtime()?;

		let task_executor = {
			let runtime_handle = tokio_runtime.handle().clone();
			Arc::new(move |fut| {
				runtime_handle.spawn(fut);
			})
		};

		Ok(Runner {
			config: command.create_configuration(cli, task_executor)?,
			tokio_runtime,
			phantom: PhantomData,
		})
	}

	/// A helper function that runs an `AbstractService` with tokio and stops if the process receives
	/// the signal `SIGTERM` or `SIGINT`.
	pub fn run_node<FNL, FNF, SL, SF>(self, new_light: FNL, new_full: FNF) -> Result<()>
	where
		FNL: FnOnce(Configuration) -> sc_service::error::Result<SL>,
		FNF: FnOnce(Configuration) -> sc_service::error::Result<SF>,
		SL: AbstractService + Unpin,
		SF: AbstractService + Unpin,
	{
		info!("{}", C::impl_name());
		info!("✌️  version {}", C::impl_version());
		info!(
			"❤️  by {}, {}-{}",
			C::author(),
			C::copyright_start_year(),
			Local::today().year(),
		);
		info!("📋 Chain specification: {}", self.config.chain_spec.name());
		info!("🏷  Node name: {}", self.config.network.node_name);
		info!("👤 Role: {}", self.config.display_role());

		match self.config.role {
			Role::Light => self.run_service_until_exit(new_light),
			_ => self.run_service_until_exit(new_full),
		}
	}

	/// A helper function that runs a future with tokio and stops if the process receives the signal
	/// `SIGTERM` or `SIGINT`.
	pub fn run_subcommand<B, BC, BB>(self, subcommand: &Subcommand, builder: B) -> Result<()>
	where
		B: FnOnce(Configuration) -> sc_service::error::Result<BC>,
		BC: ServiceBuilderCommand<Block = BB> + Unpin,
		BB: sp_runtime::traits::Block + Debug,
		<<<BB as BlockT>::Header as HeaderT>::Number as std::str::FromStr>::Err: Debug,
		<BB as BlockT>::Hash: std::str::FromStr,
	{
		match subcommand {
			Subcommand::BuildSpec(cmd) => cmd.run(self.config),
			Subcommand::ExportBlocks(cmd) => {
				run_until_exit(self.tokio_runtime, cmd.run(self.config, builder))
			}
			Subcommand::ImportBlocks(cmd) => {
				run_until_exit(self.tokio_runtime, cmd.run(self.config, builder))
			}
			Subcommand::CheckBlock(cmd) => {
				run_until_exit(self.tokio_runtime, cmd.run(self.config, builder))
			}
			Subcommand::Revert(cmd) => cmd.run(self.config, builder),
			Subcommand::PurgeChain(cmd) => cmd.run(self.config),
		}
	}

	pub fn run_bootnodes_router(self, subcommand: &BootnodesRouterCmd) -> Result<()> {
		// let cmd = &BRExeCmd{
		// 	port: subcommand.port,
		// 	base_path: subcommand.shared_params.base_path.clone(),
		// 	log: subcommand.shared_params.log.clone(),
		// 	dev_params: subcommand.dev_params,
		// 	dev_shard_count: subcommand.dev_shard_count
		// };
		// match cmd.run(){
		// 	Err(e) => {
		// 		info!("{}", e);
		// 		Err(crate::Error::Other("unknown error".to_string()))
		// 	},
		// 	Ok(_) => Ok(())
		// }

		Ok(())
	}

	pub fn run_switch(self, subcommand: &SwitchCmd) -> Result<()> {
		// todo
		Ok(())
	}

	fn run_service_until_exit<T, F>(mut self, service_builder: F) -> Result<()>
	where
		F: FnOnce(Configuration) -> std::result::Result<T, sc_service::error::Error>,
		T: AbstractService + Unpin,
	{
		let service = service_builder(self.config)?;

		let informant_future = sc_informant::build(&service, sc_informant::OutputFormat::Coloured);
		let _informant_handle = self.tokio_runtime.spawn(informant_future);

		// we eagerly drop the service so that the internal exit future is fired,
		// but we need to keep holding a reference to the global telemetry guard
		// and drop the runtime first.
		let _telemetry = service.telemetry();

		let f = service.fuse();
		pin_mut!(f);

		self.tokio_runtime
			.block_on(main(f))
			.map_err(|e| e.to_string())?;
		drop(self.tokio_runtime);

		Ok(())
	}

	/// A helper function that runs a command with the configuration of this node
	pub fn sync_run(self, runner: impl FnOnce(Configuration) -> Result<()>) -> Result<()> {
		runner(self.config)
	}

	/// A helper function that runs a future with tokio and stops if the process receives
	/// the signal SIGTERM or SIGINT
	pub fn async_run<FUT>(self, runner: impl FnOnce(Configuration) -> FUT) -> Result<()>
	where
		FUT: Future<Output = Result<()>>,
	{
		run_until_exit(self.tokio_runtime, runner(self.config))
	}

	/// Get an immutable reference to the node Configuration
	pub fn config(&self) -> &Configuration {
		&self.config
	}

	/// Get a mutable reference to the node Configuration
	pub fn config_mut(&mut self) -> &Configuration {
		&mut self.config
	}
}
