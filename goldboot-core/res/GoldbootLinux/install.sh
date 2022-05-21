#!/bin/bash -x
## Perform a basic Arch Linux install.
set -e

# Synchronize time
timedatectl set-ntp true

# Configure Pacman mirrors
echo "${GB_MIRRORLIST:?}" >/etc/pacman.d/mirrorlist

# Create partitions
parted --script -a optimal -- /dev/vda \
	mklabel gpt \
	mkpart primary 1MiB 256MiB \
	set 1 esp on \
	mkpart primary 256MiB 100%

# Format boot partition
mkfs.vfat /dev/vda1

# Format root
mkfs.ext4 /dev/vda2

# Mount root
mount /dev/vda2 /mnt

# Mount boot partition
mount --mkdir /dev/vda1 /mnt/boot

# Bootstrap filesystem
pacstrap /mnt systemd efibootmgr resize2fs grub dhcpcd xorg-server

# Generate fstab
genfstab -U /mnt >/mnt/etc/fstab

cat <<-EOF >>/mnt/etc/default/grub
	GRUB_CMDLINE_LINUX="root=UUID=$(blkid -s UUID -o value /dev/vda2)"
EOF

# Install bootloader
arch-chroot /mnt grub-install --target=x86_64-efi --efi-directory=/boot --bootloader-id=GRUB
arch-chroot /mnt grub-mkconfig -o /boot/grub/grub.cfg

# Enable dhcpcd
systemctl enable dhcpcd.service --root /mnt

# Set root password
echo "root:${GB_ROOT_PASSWORD:?}" | chpasswd --root /mnt

# Install latest goldboot
# TODO

# Root autologin
# TODO

# Autostart GUI
cat <<-EOF >/root/.xinitrc
	exec goldboot
EOF