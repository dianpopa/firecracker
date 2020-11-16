// Copyright 2020 Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

use kvm_bindings::*;
/// Arm processors don't have a cpuid instruction, but they do have a number of ID registers that
/// provide similar information (e.g. MIDR_EL1, REVIDR_EL1).
/// This module will check that a vcpu contains the cpuid and the cpu features expected for an a1 type
/// of instance.
use kvm_ioctls::VcpuFd;
use std::collections::HashMap;

macro_rules! arm64_sys_reg {
    ($name: tt, $op0: tt, $op1: tt, $crn: tt, $crm: tt, $op2: tt) => {
        const $name: u64 = KVM_REG_ARM64 as u64
            | KVM_REG_SIZE_U64 as u64
            | KVM_REG_ARM64_SYSREG as u64
            | ((($op0 as u64) << KVM_REG_ARM64_SYSREG_OP0_SHIFT)
                & KVM_REG_ARM64_SYSREG_OP0_MASK as u64)
            | ((($op1 as u64) << KVM_REG_ARM64_SYSREG_OP1_SHIFT)
                & KVM_REG_ARM64_SYSREG_OP1_MASK as u64)
            | ((($crn as u64) << KVM_REG_ARM64_SYSREG_CRN_SHIFT)
                & KVM_REG_ARM64_SYSREG_CRN_MASK as u64)
            | ((($crm as u64) << KVM_REG_ARM64_SYSREG_CRM_SHIFT)
                & KVM_REG_ARM64_SYSREG_CRM_MASK as u64)
            | ((($op2 as u64) << KVM_REG_ARM64_SYSREG_OP2_SHIFT)
                & KVM_REG_ARM64_SYSREG_OP2_MASK as u64);
    };
}

// CpuIdRegs
arm64_sys_reg!(SYS_MIDR_EL1, 3, 0, 0, 0, 0);
arm64_sys_reg!(SYS_REVIDR_EL1, 3, 0, 0, 0, 6);
arm64_sys_reg!(SYS_MPIDR_EL1, 3, 0, 0, 0, 5);

// CpuFtrRegs
arm64_sys_reg!(SYS_ID_AA64DFR0_EL1, 3, 0, 0, 5, 0);
arm64_sys_reg!(SYS_ID_AA64DFR1_EL1, 3, 0, 0, 5, 1);

arm64_sys_reg!(SYS_ID_AA64ISAR0_EL1, 3, 0, 0, 6, 0);
arm64_sys_reg!(SYS_ID_AA64ISAR1_EL1, 3, 0, 0, 6, 1);

arm64_sys_reg!(SYS_ID_AA64MMFR0_EL1, 3, 0, 0, 7, 0);
arm64_sys_reg!(SYS_ID_AA64MMFR1_EL1, 3, 0, 0, 7, 1);

arm64_sys_reg!(SYS_ID_AA64PFR0_EL1, 3, 0, 0, 4, 0);
arm64_sys_reg!(SYS_ID_AA64PFR1_EL1, 3, 0, 0, 4, 1);

arm64_sys_reg!(SYS_ID_DFR0_EL1, 3, 0, 0, 1, 2);

arm64_sys_reg!(SYS_ID_ISAR0_EL1, 3, 0, 0, 2, 0);
arm64_sys_reg!(SYS_ID_ISAR1_EL1, 3, 0, 0, 2, 1);
arm64_sys_reg!(SYS_ID_ISAR2_EL1, 3, 0, 0, 2, 2);
arm64_sys_reg!(SYS_ID_ISAR3_EL1, 3, 0, 0, 2, 3);
arm64_sys_reg!(SYS_ID_ISAR4_EL1, 3, 0, 0, 2, 4);
arm64_sys_reg!(SYS_ID_ISAR5_EL1, 3, 0, 0, 2, 5);
arm64_sys_reg!(SYS_ID_MMFR4_EL1, 3, 0, 0, 2, 6);

arm64_sys_reg!(SYS_ID_MMFR0_EL1, 3, 0, 0, 1, 4);
arm64_sys_reg!(SYS_ID_MMFR1_EL1, 3, 0, 0, 1, 5);
arm64_sys_reg!(SYS_ID_MMFR2_EL1, 3, 0, 0, 1, 6);
arm64_sys_reg!(SYS_ID_MMFR3_EL1, 3, 0, 0, 1, 7);

arm64_sys_reg!(SYS_ID_PFR0_EL1, 3, 0, 0, 1, 0);
arm64_sys_reg!(SYS_ID_PFR1_EL1, 3, 0, 0, 1, 1);

arm64_sys_reg!(SYS_MVFR0_EL1, 3, 0, 0, 3, 0);
arm64_sys_reg!(SYS_MVFR1_EL1, 3, 0, 0, 3, 1);
arm64_sys_reg!(SYS_MVFR2_EL1, 3, 0, 0, 3, 2);

// CacheRegs
arm64_sys_reg!(SYS_CLIDR_EL1, 3, 1, 0, 0, 1);
arm64_sys_reg!(SYS_AIDR_EL1, 3, 1, 0, 0, 7);
arm64_sys_reg!(SYS_CSSELR_EL1, 3, 2, 0, 0, 0);
arm64_sys_reg!(SYS_CTR_EL0, 3, 3, 0, 0, 1);
arm64_sys_reg!(SYS_DCZID_EL0, 3, 3, 0, 0, 7);

/// Check that the cpu info of the guest matches those of an a1.
pub fn check_template(vcpu: &VcpuFd) {
    let cpuid_regs: HashMap<u64, u64> = [(SYS_MIDR_EL1, 1091555459), (SYS_REVIDR_EL1, 0)]
        .iter()
        .cloned()
        .collect();
    println!("SYS_MIDR_EL1: {}", vcpu.get_one_reg(SYS_MIDR_EL1).unwrap());
    println!(
        "SYS_REVIDR_EL1 {}",
        vcpu.get_one_reg(SYS_REVIDR_EL1).unwrap()
    );

    let cpu_ftr_regs: HashMap<u64, u64> = [
        (SYS_ID_AA64DFR0_EL1, 100),
        (SYS_ID_AA64DFR1_EL1, 50),
        (SYS_ID_AA64ISAR0_EL1, 10),
        (SYS_ID_AA64ISAR1_EL1, 50),
        (SYS_ID_AA64MMFR0_EL1, 10),
        (SYS_ID_AA64MMFR1_EL1, 50),
        (SYS_ID_AA64PFR0_EL1, 10),
        (SYS_ID_AA64PFR1_EL1, 50),
        (SYS_ID_DFR0_EL1, 10),
        (SYS_ID_ISAR0_EL1, 50),
        (SYS_ID_ISAR1_EL1, 10),
        (SYS_ID_ISAR2_EL1, 50),
        (SYS_ID_ISAR3_EL1, 10),
        (SYS_ID_ISAR4_EL1, 50),
        (SYS_ID_ISAR5_EL1, 10),
        (SYS_ID_MMFR0_EL1, 50),
        (SYS_ID_MMFR1_EL1, 10),
        (SYS_ID_MMFR2_EL1, 50),
        (SYS_ID_MMFR3_EL1, 10),
        (SYS_ID_PFR0_EL1, 50),
        (SYS_ID_PFR1_EL1, 10),
        (SYS_ID_MMFR4_EL1, 50),
        (SYS_MVFR0_EL1, 10),
        (SYS_MVFR1_EL1, 50),
        (SYS_MVFR2_EL1, 10),
    ]
    .iter()
    .cloned()
    .collect();
    println!(
        "SYS_ID_AA64DFR0_EL1 {}",
        vcpu.get_one_reg(SYS_ID_AA64DFR0_EL1).unwrap()
    );
    println!(
        "SYS_ID_AA64DFR1_EL1 {}",
        vcpu.get_one_reg(SYS_ID_AA64DFR1_EL1).unwrap()
    );
    println!(
        "SYS_ID_AA64ISAR0_EL1 {}",
        vcpu.get_one_reg(SYS_ID_AA64ISAR0_EL1).unwrap()
    );
    println!(
        "SYS_ID_AA64MMFR0_EL1 {}",
        vcpu.get_one_reg(SYS_ID_AA64MMFR0_EL1).unwrap()
    );
    println!(
        "SYS_ID_AA64MMFR1_EL1 {}",
        vcpu.get_one_reg(SYS_ID_AA64MMFR1_EL1).unwrap()
    );
    println!(
        "SYS_ID_AA64PFR0_EL1 {}",
        vcpu.get_one_reg(SYS_ID_AA64PFR0_EL1).unwrap()
    );
    println!(
        "SYS_ID_AA64PFR1_EL1 {}",
        vcpu.get_one_reg(SYS_ID_AA64PFR1_EL1).unwrap()
    );
    println!(
        "SYS_ID_AA64DFR0_EL1 {}",
        vcpu.get_one_reg(SYS_ID_AA64DFR0_EL1).unwrap()
    );
    println!(
        "SYS_ID_DFR0_EL1 {}",
        vcpu.get_one_reg(SYS_ID_DFR0_EL1).unwrap()
    );
    println!(
        "SYS_ID_ISAR0_EL1 {}",
        vcpu.get_one_reg(SYS_ID_ISAR0_EL1).unwrap()
    );
    println!(
        "SYS_ID_ISAR1_EL1 {}",
        vcpu.get_one_reg(SYS_ID_ISAR1_EL1).unwrap()
    );
    println!(
        "SYS_ID_ISAR2_EL1 {}",
        vcpu.get_one_reg(SYS_ID_ISAR2_EL1).unwrap()
    );
    println!(
        "SYS_ID_ISAR3_EL1 {}",
        vcpu.get_one_reg(SYS_ID_ISAR3_EL1).unwrap()
    );
    println!(
        "SYS_ID_ISAR4_EL1 {}",
        vcpu.get_one_reg(SYS_ID_ISAR4_EL1).unwrap()
    );
    println!(
        "SYS_ID_ISAR5_EL1 {}",
        vcpu.get_one_reg(SYS_ID_ISAR5_EL1).unwrap()
    );
    println!(
        "SYS_ID_MMFR0_EL1 {}",
        vcpu.get_one_reg(SYS_ID_MMFR0_EL1).unwrap()
    );
    println!(
        "SYS_ID_MMFR1_EL1 {}",
        vcpu.get_one_reg(SYS_ID_MMFR1_EL1).unwrap()
    );
    println!(
        "SYS_ID_MMFR2_EL1 {}",
        vcpu.get_one_reg(SYS_ID_MMFR2_EL1).unwrap()
    );
    println!(
        "SYS_ID_MMFR3_EL1 {}",
        vcpu.get_one_reg(SYS_ID_MMFR3_EL1).unwrap()
    );
    println!(
        "SYS_ID_PFR0_EL1 {}",
        vcpu.get_one_reg(SYS_ID_PFR0_EL1).unwrap()
    );
    println!(
        "SYS_ID_PFR1_EL1 {}",
        vcpu.get_one_reg(SYS_ID_PFR1_EL1).unwrap()
    );
    println!(
        "SYS_ID_MMFR4_EL1 {}",
        vcpu.get_one_reg(SYS_ID_MMFR4_EL1).unwrap()
    );
    println!(
        "SYS_MVFR0_EL1 {}",
        vcpu.get_one_reg(SYS_MVFR0_EL1).unwrap()
    );
    println!(
        "SYS_ID_AA64DFR0_EL1 {}",
        vcpu.get_one_reg(SYS_ID_AA64DFR0_EL1,).unwrap()
    );
    println!(
        "SYS_ID_AA64DFR0_EL1 {}",
        vcpu.get_one_reg(SYS_ID_AA64DFR0_EL1,).unwrap()
    );
    println!(
        "SYS_ID_AA64DFR0_EL1 {}",
        vcpu.get_one_reg(SYS_ID_AA64DFR0_EL1,).unwrap()
    );
    println!(
        "SYS_ID_AA64DFR0_EL1 {}",
        vcpu.get_one_reg(SYS_ID_AA64DFR0_EL1,).unwrap()
    );
    println!(
        "SYS_ID_AA64DFR0_EL1 {}",
        vcpu.get_one_reg(SYS_ID_AA64DFR0_EL1,).unwrap()
    );
    println!(
        "SYS_ID_AA64DFR0_EL1 {}",
        vcpu.get_one_reg(SYS_ID_AA64DFR0_EL1,).unwrap()
    );
    let cache_regs: HashMap<u64, u64> = [
        (SYS_CLIDR_EL1, 100),
        (SYS_AIDR_EL1, 10),
        (SYS_CSSELR_EL1, 100),
        (SYS_CTR_EL0, 50),
        (SYS_DCZID_EL0, 10),
    ]
    .iter()
    .cloned()
    .collect();
}
