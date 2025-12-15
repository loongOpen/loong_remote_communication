pub mod generated {
    pub mod hello {
        include!("generated/hello.rs");
    }

    pub mod lrc_user_rpc {
        include!("generated/lrc.user.rpc.rs");
    }
}

pub use generated::lrc_user_rpc::portal_launcher_server::{PortalLauncher, PortalLauncherServer};
pub use generated::lrc_user_rpc::{Config, PortalType, SockAddr};
