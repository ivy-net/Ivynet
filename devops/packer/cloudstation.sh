#!/bin/sh
#
# Simple script to create backend image
# Requires toml cargo package

filename=cloudstation.pkr.hcl

echo "Get verions (tag)"
version=$(toml get ../../cli/Cargo.toml package.version |tr -d [\"\.])
version="003"
echo "Activate Ansible (if necessary)"
[ -f "$HOME/bin/ansible/bin/activate" ] && source $HOME/bin/ansible/bin/activate

echo "Initialize packer"
packer init ${filename}

echo "Adjust gcp credentials (if necessary)"
[ -f /tmp/packer_gcp.json ] && sed -i.bak 's/gcp.yml/gcp_packer.yml/' cloudstation.pkr.hcl

echo "Run packer"
packer build -var "version=${version}" ${filename}
