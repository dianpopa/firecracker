// Copyright 2019 Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

use std::cmp::{Ord, Ordering, PartialEq, PartialOrd};
use std::collections::btree_map::BTreeMap;
use std::fmt;
use std::result;
use std::sync::{Arc, Mutex};

use fc_util::device_config::FirecrackerDevice;
use vmm_config::drive::*;
use vmm_config::net::*;
use vmm_config::vsock::*;

/// A data structure that encapsulates the device configurations
/// held in the Vmm.
pub struct DeviceConfigs {
    /// The configurations for block devices.
    pub block: BlockDeviceConfigs,
    /// The configurations for network interface devices.
    pub network_interface: NetworkInterfaceConfigs,
    /// The configurations for vsock devices.
    pub vsock: Option<VsockDeviceConfig>,
}

impl DeviceConfigs {
    /// Construct a `DeviceConfigs` structure from its constituent parts.
    pub fn new(
        block: BlockDeviceConfigs,
        network_interface: NetworkInterfaceConfigs,
        vsock: Option<VsockDeviceConfig>,
    ) -> DeviceConfigs {
        DeviceConfigs {
            block,
            network_interface,
            vsock,
        }
    }
}

#[derive(Debug)]
/// Errors for Bus.
pub enum Error {
    /// The insertion failed because the new device overlapped with an old device.
    Overlap,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use self::Error::*;

        match *self {
            Overlap => write!(f, "New device overlaps with an old device."),
        }
    }
}

#[derive(Debug, Copy, Clone)]
struct BusRange(u64, u64);

impl Eq for BusRange {}

impl PartialEq for BusRange {
    fn eq(&self, other: &BusRange) -> bool {
        self.0 == other.0
    }
}

impl Ord for BusRange {
    fn cmp(&self, other: &BusRange) -> Ordering {
        self.0.cmp(&other.0)
    }
}

impl PartialOrd for BusRange {
    fn partial_cmp(&self, other: &BusRange) -> Option<Ordering> {
        self.0.partial_cmp(&other.0)
    }
}

type Result<T> = result::Result<T, Error>;

/// A device container for routing reads and writes over some address space.
///
/// This doesn't have any restrictions on what kind of device or address space this applies to. The
/// only restriction is that no two devices can overlap in this address space.
#[derive(Clone, Default)]
pub struct Bus {
    devices: BTreeMap<BusRange, Arc<Mutex<dyn FirecrackerDevice>>>,
}

impl Bus {
    /// Constructs an a bus with an empty address space.
    pub fn new() -> Bus {
        Bus {
            devices: BTreeMap::new(),
        }
    }

    fn first_before(&self, addr: u64) -> Option<(BusRange, &Mutex<dyn FirecrackerDevice>)> {
        // for when we switch to rustc 1.17: self.devices.range(..addr).iter().rev().next()
        for (range, dev) in self.devices.iter().rev() {
            if range.0 <= addr {
                return Some((*range, dev));
            }
        }
        None
    }

    /// omg this does not have doc.
    pub fn get_device(&self, addr: u64) -> Option<(u64, &Mutex<dyn FirecrackerDevice>)> {
        if let Some((BusRange(start, len), dev)) = self.first_before(addr) {
            let offset = addr - start;
            if offset < len {
                return Some((offset, dev));
            }
        }
        None
    }

    /// Puts the given device at the given address space.
    pub fn insert(
        &mut self,
        device: Arc<Mutex<dyn FirecrackerDevice>>,
        base: u64,
        len: u64,
    ) -> Result<()> {
        if len == 0 {
            return Err(Error::Overlap);
        }

        // Reject all cases where the new device's base is within an old device's range.
        if self.get_device(base).is_some() {
            return Err(Error::Overlap);
        }

        // The above check will miss an overlap in which the new device's base address is before the
        // range of another device. To catch that case, we search for a device with a range before
        // the new device's range's end. If there is no existing device in that range that starts
        // after the new device, then there will be no overlap.
        if let Some((BusRange(start, _), _)) = self.first_before(base + len - 1) {
            // Such a device only conflicts with the new device if it also starts after the new
            // device because of our initial `get_device` check above.
            if start >= base {
                return Err(Error::Overlap);
            }
        }

        if self.devices.insert(BusRange(base, len), device).is_some() {
            return Err(Error::Overlap);
        }

        Ok(())
    }

    /// Reads data from the device that owns the range containing `addr` and puts it into `data`.
    ///
    /// Returns true on success, otherwise `data` is untouched.
    pub fn read(&self, addr: u64, data: &mut [u8]) -> bool {
        if let Some((offset, dev)) = self.get_device(addr) {
            // OK to unwrap as lock() failing is a serious error condition and should panic.
            dev.lock()
                .expect("Failed to acquire device lock")
                .read(offset, data);
            true
        } else {
            false
        }
    }

    /// Writes `data` to the device that owns the range containing `addr`.
    ///
    /// Returns true on success, otherwise `data` is untouched.
    pub fn write(&self, addr: u64, data: &[u8]) -> bool {
        if let Some((offset, dev)) = self.get_device(addr) {
            // OK to unwrap as lock() failing is a serious error condition and should panic.
            dev.lock()
                .expect("Failed to acquire device lock")
                .write(offset, data);
            true
        } else {
            false
        }
    }
}
