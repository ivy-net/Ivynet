#!/bin/sh
#
# Simple script to create backend image
# Requires toml cargo package

filename=backend.pkr.hcl

echo "Get verions (tag)"
version=$(toml get ../../backend/Cargo.toml package.version |tr -d [\"\.])

echo "Activate Ansible"
source $HOME/bin/ansible/bin/activate

echo "Initialize packer"
packer init

echo "Run packer"
packer build -var "version=${version}" ${filename}
