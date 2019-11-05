use std::any::Any;
use std::fmt;
use std::io;
use std::os::unix::io::AsRawFd;
use std::os::unix::io::RawFd;

use kvm_ioctls::IoEventAddress;

use memory_model::GuestMemory;
use sys_util::EventFd;

/// Trait that helps in upcasting an object to Any
pub trait AsAny {
    fn as_any(&self) -> &dyn Any;

    fn as_mut_any(&mut self) -> &mut dyn Any;
}
impl<T: Any> AsAny for T {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_mut_any(&mut self) -> &mut dyn Any {
        self
    }
}

/// Trait for devices that respond to reads or writes in an arbitrary address space.
///
/// The device does not care where it exists in address space as each method is only given an offset
/// into its allocated portion of address space.
#[allow(unused_variables)]
pub trait BusDevice: AsAny + Send {
    /// Reads at `offset` from this device
    fn read(&mut self, offset: u64, data: &mut [u8]) {}
    /// Writes at `offset` into this device
    fn write(&mut self, offset: u64, data: &[u8]) {}
    /// Triggers the `irq_mask` interrupt on this device
    fn interrupt(&self, irq_mask: u32) {}
}

/// Trait for devices that handle raw non-blocking I/O requests.
pub trait RawIOHandler {
    /// Send raw input to this emulated device.
    fn raw_input(&mut self, _data: &[u8]) -> io::Result<()> {
        Ok(())
    }
    /// Receive raw output from this emulated device.
    fn raw_output(&mut self, _data: &mut [u8]) -> io::Result<()> {
        Ok(())
    }
}

pub trait FirecrackerDevice: Send + BusDevice {
    /// Gets the device type.
    fn dev_type(&self) -> DeviceType;

    /// Devices needing memory for functioning will need to implement this.
    /// Up until now the device manager would receive a clone of the guest memory and it would
    /// clone it for each MMIODevice.
    fn set_mem(&mut self, _mem: GuestMemory) {}

    /// Generate the mmio address to which to tie an ioeventfd.
    /// See KVM_IOEVENTFD.
    fn mmio_ioevents(&self, base_addr: u64) -> Vec<(RawFd, kvm_ioctls::IoEventAddress, u64)> {
        vec![(
            EventFd::new().unwrap().as_raw_fd(),
            IoEventAddress::Mmio(base_addr),
            0,
        )]
    }

    /// Generate the EventFd that will be used to toggle some irqchip pin.
    fn irq_fds(&self) -> Vec<RawFd>;

    /// Serialize device.
    fn serialize(&self) -> Vec<u8> {
        vec![]
    }

    /// Deserialize device.
    fn deserialize(&self, blob: &[u8]) -> Self
    where
        Self: Sized;
}

/// Types of devices that can get attached to this platform.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Copy)]
pub enum DeviceType {
    /// Device Type: Virtio.
    Virtio(u32),
    /// Device Type: Serial.
    Serial,
    /// Device Type: i8042.
    I8042,
    /// Device Type: RTC.
    RTC,
}

impl fmt::Display for DeviceType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// Trait for devices to be added to the Flattened Device Tree.
pub trait DeviceInfoForFDT {
    /// Returns the address where this device will be loaded.
    fn addr(&self) -> u64;
    /// Returns the associated interrupt for this device.
    fn irq(&self) -> u32;
    /// Returns the amount of memory that needs to be reserved for this device.
    fn length(&self) -> u64;
}
