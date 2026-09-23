//! Fixed attribute handles on AirPods Pro 2/3.

/// Customized transparency settings (100/104-byte float blob).
pub const TRANSPARENCY: u16 = 0x0018;
pub const TRANSPARENCY_CCCD: u16 = 0x0019;

/// Loud sound reduction (1 byte, 0x01/0x00). Its CCCD does not work.
pub const LOUD_SOUND_REDUCTION: u16 = 0x001B;

/// Hearing aid configuration (104-byte float blob).
pub const HEARING_AID: u16 = 0x002A;
pub const HEARING_AID_CCCD: u16 = 0x002B;
