
// use structopt::StructOpt;
use std::path::PathBuf;
use sc_service::Configuration;


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
    pub fn run(config: Configuration) {

    }
}

