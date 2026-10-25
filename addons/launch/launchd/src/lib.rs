//! The core of `vetro-launchd`: `SpaceX` launch data as the launch card, the lock
//! screen and the login screen read it, and the weather card's forecast. Nothing here prints, touches a file
//! or the network: the binary does that over these.

pub mod cache;
pub mod clock;
pub mod config;
pub mod launch;
pub mod library;
pub mod plan;
pub mod state;
pub mod weather;
