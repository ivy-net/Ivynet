#!/bin/sh
#
# Simple script to create backend image
# Requires toml cargo package

filename=cloudstation.pkr.hcl

echo "Find the backend version in Cargo.toml"
version=$(toml get ../../cli/Cargo.toml package.version |tr -d [\"\.])

echo "Activate Ansible (if necessary)"
[ -f "$HOME/bin/ansible/bin/activate" ] && source $HOME/bin/ansible/bin/activate

echo "Initialize packer"
packer init ${filename}

echo "Run packer"
packer build -var "version=${version}" ${filename}
