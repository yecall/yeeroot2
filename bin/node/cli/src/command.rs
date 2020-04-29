use crate::{chain_spec, factory_impl::FactoryState, service, Cli, FactoryCmd, Subcommand};
use node_executor::Executor;
use node_runtime::{Block, RuntimeApi};
use node_transaction_factory::RuntimeAdapter;
//use yc_cli::{YeerootCli, CliConfiguration, ImportParams, Result, SharedParams};
use sc_cli::{SubstrateCli, CliConfiguration, ImportParams, Result, SharedParams};
use sc_service::Configuration;
//use yc_cli::{BootnodesRouterCmd, SwitchCmd};

impl SubstrateCli for Cli {
    fn impl_name() -> &'static str {
        "Yeeroot2 Node"
    }

    fn impl_version() -> &'static str {
        "0.1.0"
    }

    fn executable_name() -> &'static str {
        "YEEROOT"
    }

    fn description() -> &'static str {
        env!("CARGO_PKG_DESCRIPTION")
    }

    fn author() -> &'static str {
        env!("CARGO_PKG_AUTHORS")
    }

    fn support_url() -> &'static str {
        "https://github.com/yeeco/yeeroot2/issues/new"
    }

    fn copyright_start_year() -> i32 {
        2019
    }

    fn load_spec(&self, id: &str) -> std::result::Result<Box<dyn sc_service::ChainSpec>, String> {
        Ok(match id {
            "dev" => Box::new(chain_spec::development_config()),
            "" | "local" => Box::new(chain_spec::local_testnet_config()),
            "staging" => Box::new(chain_spec::staging_testnet_config()),
            path => Box::new(chain_spec::ChainSpec::from_json_file(
                std::path::PathBuf::from(path),
            )?),
        })
    }
}

/// Parse command line arguments into service configuration.
pub fn run() -> Result<()> {
    sc_cli::reset_signal_pipe_handler()?;

    let cli = Cli::from_args();

    match &cli.subcommand {
        None => {
            // set global params
            let mut params = service::CustomParams.write().unwrap();
            params.bootnodes_routers = cli.run.bootnodes_routers.clone();
            params.coinbase = cli.run.coinbase.clone();
            params.shard_num = cli.run.shard_num;
            params.foreign_port = cli.run.foreign_port.clone();
            params.dev_params = cli.run.dev_params.clone();
            params.dev_params_num = cli.run.dev_params_num.clone();
            params.mine = cli.run.mine.clone();

            let runner = cli.create_runner(&cli.run)?;
            runner.run_node(service::new_light, service::new_full)
        }
        // Some(Subcommand::Inspect(cmd)) => {
        // 	let runner = cli.create_runner(cmd)?;
        //
        // 	runner.sync_run(|config| cmd.run::<Block, RuntimeApi, Executor>(config))
        // }
        // Some(Subcommand::Benchmark(cmd)) => {
        // 	if cfg!(feature = "runtime-benchmarks") {
        // 		let runner = cli.create_runner(cmd)?;
        //
        // 		runner.sync_run(|config| cmd.run::<Block, Executor>(config))
        // 	} else {
        // 		println!("Benchmarking wasn't enabled when building the node. \
        // 		You can enable it with `--features runtime-benchmarks`.");
        // 		Ok(())
        // 	}
        // }
        Some(Subcommand::Base(subcommand)) => {
            let runner = cli.create_runner(subcommand)?;

            runner.run_subcommand(subcommand, |config| Ok(new_full_start!(config).0))
        }
        Some(Subcommand::BootnodesRouter(subcommand)) => {
            let runner = cli.create_runner(subcommand)?;

            runner.run_bootnodes_router(subcommand)
        }
        Some(Subcommand::Switch(subcommand)) => {
            let runner = cli.create_runner(subcommand)?;

            runner.run_switch(subcommand)
        }
        _ => {
            Ok(())
        }
    }
}

