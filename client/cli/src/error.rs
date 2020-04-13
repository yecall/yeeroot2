
//! Initialization errors.

/// Result type alias for the CLI.
pub type Result<T> = std::result::Result<T, Error>;

/// Error type for the CLI.
#[derive(Debug, derive_more::Display, derive_more::From)]
pub enum Error {
	/// Io error
	Io(std::io::Error),
	/// Cli error
	Cli(clap::Error),
	/// Service error
	Service(sc_service::Error),
	/// Client error
	Client(sp_blockchain::Error),
	/// Input error
	#[from(ignore)]
	Input(String),
	/// Invalid listen multiaddress
	#[display(fmt="Invalid listen multiaddress")]
	InvalidListenMultiaddress,
	/// Other uncategorized error.
	#[from(ignore)]
	Other(String),
}

/// Must be implemented explicitly because `derive_more` won't generate this
/// case due to conflicting derive for `Other(String)`.
impl std::convert::From<String> for Error {
	fn from(s: String) -> Error {
		Error::Input(s)
	}
}

impl std::convert::From<&str> for Error {
	fn from(s: &str) -> Error {
		Error::Input(s.to_string())
	}
}

impl std::error::Error for Error {
	fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
		match self {
			Error::Io(ref err) => Some(err),
			Error::Cli(ref err) => Some(err),
			Error::Service(ref err) => Some(err),
			Error::Client(ref err) => Some(err),
			Error::Input(_) => None,
			Error::InvalidListenMultiaddress => None,
			Error::Other(_) => None,
		}
	}
}
