pub mod record;
pub mod settings;
pub mod system;

pub use record::*;
pub use settings::*;
pub use system::*;

pub type CmdResult<T> = Result<T, String>;
