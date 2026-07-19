pub mod actor;
pub mod ast;
pub mod codec;
pub mod messages;
pub mod mock;
pub mod network_layer;
pub mod protocol;
pub mod types;

pub use actor::NetworkActor;
// #151: re-export libp2p's identity types so downstream crates can construct
// a persistent keypair without depending on libp2p directly.
pub use libp2p::identity;
pub use messages::*;
pub use mock::MockNetwork;
pub use network_layer::NetworkLayer;
pub use protocol::{recv_message, send_message, MEERKAT_PROTOCOL};
pub use types::*;
