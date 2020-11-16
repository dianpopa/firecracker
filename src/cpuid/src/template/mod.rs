// Copyright 2019 Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

// Contains Intel specific templates.
pub mod arm;
#[cfg(target_arch = "x86_64")]
pub mod intel;
