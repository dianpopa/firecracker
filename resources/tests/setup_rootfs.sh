#!/usr/bin/env bash

# Copyright 2021 Amazon.com, Inc. or its affiliates. All Rights Reserved.
# SPDX-License-Identifier: Apache-2.0

# Script that customizes a rootfs for CI images.
#

create_basic_rootfs() {
    BUILD_DIR="$1"
    DISKMNT="$1/mnt/rootfs"
    IMAGE="$3"
    SSH_DIR="$BUILD_DIR/ssh"
    RESOURCE_DIR="$2"
    FLAVOUR="$4"

    mount "$IMAGE" "$DISKMNT"

    # Set a hostname.
    echo "ubuntu-fc-uvm" > "$DISKMNT/etc/hostname"

    # Setup fcnet service. This is a custom Firecracker
    # setup for assigning IPs to the network interfaces in the guests spawned
    # by the CI.
    cp "$RESOURCE_DIR/fcnet-setup.sh"  "$DISKMNT/usr/local/bin/"
    chmod +x "$DISKMNT"/usr/local/bin/fcnet-setup.sh
    touch "$DISKMNT"/etc/systemd/system/fcnet.service
cat >> "$DISKMNT"/etc/systemd/system/fcnet.service << EOF
[Service]
Type=oneshot
ExecStart=/usr/local/bin/fcnet-setup.sh
[Install]
WantedBy=sshd.service
EOF
    chroot "$DISKMNT" /bin/bash -c "ln -s /etc/systemd/system/fcnet.service /etc/systemd/system/sysinit.target.wants/fcnet.service"

    # Generate key for ssh access from host
    mkdir -p "$SSH_DIR"
    ssh-keygen -f "$SSH_DIR/id_rsa" -N ""
    mkdir -m 0600 -p "$DISKMNT/root/.ssh/"
    cp "$SSH_DIR/id_rsa.pub" "$DISKMNT/root/.ssh/authorized_keys"

    # Setup ssh access.
    sed -E -i $DISKMNT/etc/ssh/sshd_config \
        -e "/^[# ]*PermitRootLogin .+$/d" \
        -e "/^[# ]*PermitEmptyPasswords .+$/d" \
        -e "/^[# ]*PubkeyAuthentication .+$/d"

        echo "
    PermitRootLogin yes
    PermitEmptyPasswords yes
    PubkeyAuthentication yes
    " | tee -a $DISKMNT/etc/ssh/sshd_config >/dev/null

    chroot "$DISKMNT" /bin/bash -c "export LC_ALL=C; export LANG=C.UTF-8; passwd -d root"

    # Copy init file sending the boot done signal.
    mv "$DISKMNT/sbin/init" "$DISKMNT/sbin/openrc-init"
    gcc -o "$DISKMNT/sbin/init" "$RESOURCE_DIR/init.c"

    # Copy fillmem tool used by balloon tests.
    gcc -o "$DISKMNT/sbin/fillmem" "$RESOURCE_DIR/fillmem.c"
    gcc -o "$DISKMNT/sbin/readmem" "$RESOURCE_DIR/readmem.c"

    source="http://archive.ubuntu.com/ubuntu"
    packets="iperf3 curl fio screen"
    arch=$(uname -m)
    if [ "$arch" = "x86_64" ]; then
        packets="$packets cpuid"
    elif [ "$arch" = "aarch64" ]; then
        source="http://ports.ubuntu.com/ubuntu-ports"
    fi
    echo "deb $source $FLAVOUR-updates main" >> "$DISKMNT/etc/apt/sources.list"
    echo "deb $source $FLAVOUR universe" >> "$DISKMNT/etc/apt/sources.list"



    chroot "$DISKMNT" /bin/bash -c "apt-get update; apt-get -y install --no-install-recommends $packets"
    umount "$DISKMNT"
}

create_partuuid_rootfs() {
    IMAGE="$1"
    PARTUUID_IMAGE="$2"

    initial_size=$(ls -l --block-size=M $IMAGE | cut -d ' ' -f 5)
    size=${initial_size//M/}

    fallocate -l "$((size + 100))M" "$PARTUUID_IMAGE"
    say "We will call fdisk on partuuid image. You need to select n, p, 1, 2048, w..."
    fdisk "$PARTUUID_IMAGE"

    loop_dev=$(losetup --partscan --show --find "$PARTUUID_IMAGE")
    echo $loop_dev
    losetup -d $loop_dev
    losetup --partscan "$loop_dev" "$PARTUUID_IMAGE"

    mkfs.ext4 "${loop_dev}p1"
    dd if="$IMAGE" of="${loop_dev}p1"
    losetup -d "$loop_dev"
}
