#!/bin/bash
# This script converts the current timestamp (YYYYMMDDHHMM) to a base-36 string, to provide
# a short, unique and sortable identifier while retaining original time information.
#
# Usage: ./timestamp_b36.sh
# Example output: 2kzfg7q0
set -euo pipefail

# Function to convert a number to base-36
to_base36() {
    local number=$1
    local base36=""
    local chars="0123456789abcdefghijklmnopqrstuvwxyz"

    while [ "$number" -gt 0 ]; do
        local remainder=$((number % 36))
        base36="${chars:remainder:1}${base36}"
        number=$((number / 36))
    done

    echo "$base36"
}

# Get the current timestamp in YYYYMMDDHHMM
timestamp=$(date +"%Y%m%d%H%M")

# Convert the timestamp to an integer
timestamp_int=$(echo "$timestamp" | bc)

# Convert the integer to a base-36 string
base36_timestamp=$(to_base36 "$timestamp_int")

echo "$base36_timestamp"
exit 0