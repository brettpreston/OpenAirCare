//! Pure protocol definitions for talking to AirPods over the two classic
//! L2CAP channels used by Apple devices:
//!
//! * **AACP** (PSM `0x1001`): the Apple Accessory Communication Protocol,
//!   used for battery, device info, listening mode and the control commands
//!   that enable the hearing-aid feature.
//! * **ATT** (PSM `0x1F`): a raw attribute channel on which the hearing-aid
//!   configuration blob lives as a fixed-handle characteristic.
//!
//! This crate performs no I/O; everything here is byte building/parsing so it
//! can be unit tested on any platform. Transports live in `airpods-link`.

pub mod aacp;
pub mod att;
pub mod hearing;
pub mod model;

pub use aacp::control::{ControlCommand, ControlCommandId, ListeningMode};
pub use aacp::{battery::BatteryInfo, info::DeviceInfo};
pub use hearing::{Adjustments, Audiogram, EarParams, HearingAidData};
