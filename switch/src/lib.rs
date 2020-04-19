
// use structopt::StructOpt;
use std::path::PathBuf;
use sc_service::Configuration;

mod error;

pub const DEFAULT_RPC_PORT: u16 = 10033;
pub const DEFAULT_WS_PORT: u16 = 10044;

#[derive(Debug, Clone)]
pub struct Cmd {

    /// Specify HTTP RPC server TCP port
    pub rpc_port: Option<u16>,

    /// Specify WebSockets RPC server TCP port
    pub ws_port: Option<u16>,

    /// Listen to all RPC interfaces (default is local)
    pub rpc_external: bool,

    /// Listen to all Websocket interfaces (default is local)
    pub ws_external: bool,

    /// Whether use dev params or not
    pub dev_params: bool,

    /// Specify custom base path.
    pub base_path: Option<PathBuf>,

    /// Sets a custom logging filter
    pub log: Option<String>,

    ///Specify miner poll interval
    pub job_refresh_interval: u64,

    /// start miner
    pub mine: bool,

    /// enable work manager
    pub enable_work_manager: bool,

    /// Shard count on dev mode
    pub dev_shard_count: Option<u16>,

}

impl Cmd {
    pub fn run(config: Configuration) -> error::Result<()> {
        let config = get_config(&cmd, &version)?;

        let rpc_config: yee_primitives::Config = config.into();


        let rpc_interface: &str = if cmd.rpc_external { "0.0.0.0" } else { "127.0.0.1" };

        let ws_interface: &str = if cmd.ws_external { "0.0.0.0" } else { "127.0.0.1" };

        let rpc_address_http = parse_address(&format!("{}:{}", rpc_interface, DEFAULT_RPC_PORT), cmd.rpc_port)?;

        let rpc_address_ws = parse_address(&format!("{}:{}", ws_interface, DEFAULT_WS_PORT), cmd.ws_port)?;

        let (signal, exit) = exit_future::signal();

        let work_manger = if cmd.enable_work_manager || cmd.mine {
            Some(yee_mining2::start_work_manager(&rpc_config)?)
        } else {
            None
        };

        if cmd.mine {
            let work_manager = work_manger.clone().expect("qed");
            yee_mining2::start_mining(work_manager, &rpc_config).map_err(|e| "mining error")?;
        }

        let handler = || {
            let author = Author::new(rpc_config.clone());
            let state = State::new(rpc_config.clone());
            let system = System::new(rpc_config.clone());
            let chain = Chain::new(rpc_config.clone());

            let pow =  work_manger.clone().map(Pow::new);
            yee_switch_rpc_servers::rpc_handler::<_, _, _, _, _, yee_runtime::Hash, yee_runtime::BlockNumber>(
                author,
                state,
                system,
                chain,
                pow,
            )
        };


        let _server = yee_switch_rpc_servers::start_http(&rpc_address_http, handler())?;

        info!(target: TARGET, "Switch rpc http listen on: {}", rpc_address_http);

        let _server = yee_switch_rpc_servers::start_ws(&rpc_address_ws, handler())?;

        info!(target: TARGET, "Switch rpc ws listen on: {}", rpc_address_ws);


        exit.wait().unwrap();

        signal.fire();

        Ok(())
    }
}

fn parse_address(
    address: &str,
    port: Option<u16>,
) -> error::Result<SocketAddr> {
    let mut address: SocketAddr = address.parse().map_err(
        |_| format!("Invalid address: {}", address)
    )?;
    if let Some(port) = port {
        address.set_port(port);
    }

    Ok(address)
}
