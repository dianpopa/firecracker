// Copyright 2020 Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

use kvm_bindings::*;
use kvm_ioctls::DeviceFd;

use crate::aarch64::gic::{
    regs::{SimpleReg, VgicRegEngine, VgicSysRegsState},
    Result,
};

static MAIN_VGIC_ICC_REGS: &[SimpleReg] = &[
    SimpleReg::new(0x00, 4),
    SimpleReg::new(0x04, 4),
    SimpleReg::new(0x08, 4),
    SimpleReg::new(0x1c, 4),
];

const KVM_DEV_ARM_VGIC_CPUID_SHIFT: u32 = 32;
const KVM_DEV_ARM_VGIC_OFFSET_SHIFT: u32 = 0;

struct VgicSysRegEngine {}

impl VgicRegEngine for VgicSysRegEngine {
    type Reg = SimpleReg;
    type RegChunk = u64;

    fn group() -> u32 {
        KVM_DEV_ARM_VGIC_GRP_CPU_REGS
    }

    fn kvm_device_attr(offset: u64, val: &mut Self::RegChunk, cpuid: u64) -> kvm_device_attr {
        println!("off {}", offset);
        println!("mpidr {}", cpuid);
        kvm_device_attr {
            group: Self::group(),
            attr: ((cpuid << KVM_DEV_ARM_VGIC_CPUID_SHIFT)
                & (0xff << KVM_DEV_ARM_VGIC_CPUID_SHIFT))
                | ((offset << KVM_DEV_ARM_VGIC_OFFSET_SHIFT)
                    & (0xffffffff << KVM_DEV_ARM_VGIC_OFFSET_SHIFT)),
            addr: val as *mut Self::RegChunk as u64,
            flags: 0,
        }
    }
}

pub(crate) fn get_icc_regs(fd: &DeviceFd, mpidr: u64) -> Result<VgicSysRegsState> {
    println!("bla icc");
    let main_icc_regs =
        VgicSysRegEngine::get_regs_data(fd, Box::new(MAIN_VGIC_ICC_REGS.iter()), mpidr)?;
    println!("bla");

    Ok(VgicSysRegsState {
        main_icc_regs,
        ap_icc_regs: Vec::new(),
    })
}

pub(crate) fn set_icc_regs(fd: &DeviceFd, mpidr: u64, state: &VgicSysRegsState) -> Result<()> {
    VgicSysRegEngine::set_regs_data(
        fd,
        Box::new(MAIN_VGIC_ICC_REGS.iter()),
        &state.main_icc_regs,
        mpidr,
    )?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aarch64::gic::create_gic;
    use kvm_ioctls::Kvm;
    use std::os::unix::io::AsRawFd;

    #[test]
    fn test_access_icc_regs() {
        let kvm = Kvm::new().unwrap();
        let vm = kvm.create_vm().unwrap();
        let _ = vm.create_vcpu(0).unwrap();
        let gic_fd = create_gic(&vm, 1).expect("Cannot create gic");

        let gicr_typer = 123;
        let res = get_icc_regs(&gic_fd.device_fd(), gicr_typer);
        assert!(res.is_ok());
        let mut state = res.unwrap();
        assert_eq!(state.main_icc_regs.len(), 7);
        assert_eq!(state.ap_icc_regs.len(), 8);

        assert!(set_icc_regs(&gic_fd.device_fd(), gicr_typer, &state).is_ok());

        for reg in state.ap_icc_regs.iter_mut() {
            *reg = None;
        }
        let res = set_icc_regs(&gic_fd.device_fd(), gicr_typer, &state);
        assert!(res.is_err());
        assert_eq!(format!("{:?}", res.unwrap_err()), "InvalidVgicSysRegState");

        unsafe { libc::close(gic_fd.device_fd().as_raw_fd()) };

        let res = set_icc_regs(&gic_fd.device_fd(), gicr_typer, &state);
        assert!(res.is_err());
        assert_eq!(
            format!("{:?}", res.unwrap_err()),
            "DeviceAttribute(Error(9), true, 6)"
        );

        let res = get_icc_regs(&gic_fd.device_fd(), gicr_typer);
        assert!(res.is_err());
        assert_eq!(
            format!("{:?}", res.unwrap_err()),
            "DeviceAttribute(Error(9), false, 6)"
        );
    }
}
