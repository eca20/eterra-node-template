#!/bin/bash

# Check if a version argument is provided
if [ -z "$1" ]; then
  echo "Usage: $0 <version>"
  echo "Example: $0 polkadot-stable2409-2"
  exit 1
fi

# Use a default version if no argument is provided
VERSION="${1:-polkadot-stable2409}"

# Exit script on any error
set -e

# Run the commands
LOGFILE="psvm_commands.log"

psvm -v $VERSION -o | tee -a $LOGFILE
psvm -v $VERSION -p runtime/Cargo.toml | tee -a $LOGFILE
psvm -v $VERSION -p node/Cargo.toml | tee -a $LOGFILE
psvm -v $VERSION -p pallets/ajuna-matchmaker/Cargo.toml | tee -a $LOGFILE
psvm -v $VERSION -p pallets/ajuna-board/Cargo.toml | tee -a $LOGFILE


# Indicate completion
echo "All commands executed successfully."
