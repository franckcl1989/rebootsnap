#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"
rm -rf constrained
cp -r normal constrained
# Remove PSI pressure files (simulating kernel without PSI support)
rm -f constrained/proc/pressure/cpu constrained/proc/pressure/io constrained/proc/pressure/memory
# Remove thermal zone (simulating minimal hardware)
rm -rf constrained/sys/class/thermal
# Remove EDAC (no ECC memory)
rm -rf constrained/sys/devices/system/edac
# Remove cpufreq (fixed frequency)
for cpu in 0 1 2 3; do
  rm -rf "constrained/sys/devices/system/cpu/cpu$cpu/cpufreq"
  rm -rf "constrained/sys/devices/system/cpu/cpu$cpu/thermal_throttle"
done
rm -f constrained/sys/devices/system/cpu/cpufreq/boost
