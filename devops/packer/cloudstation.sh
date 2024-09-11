#!/bin/sh
#
# Simple script to create backend image
# Requires toml cargo package

version=$(toml get ../../cli/Cargo.toml package.version |tr -d [\"\.])


source $HOME/bin/ansible/bin/activate

packer build -var "version=${version}" cloudstation.pkr.hcl
