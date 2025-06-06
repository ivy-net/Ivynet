#!/bin/sh
#
# Simple script to create backend image
# Requires toml cargo package

filename=backend.pkr.hcl

run_packer() {
  echo " Image not present"
  echo "Initialize packer"
  packer init ${filename}

  echo "Run packer"
  packer build -var "version=${version}" -var "release=${release}" ${filename}
}

echo "Find the backend version in Cargo.toml"
release=$(toml get ../../backend/Cargo.toml package.version |tr -d \")
version=$(echo ${release} |tr -d \.)

echo "Activate Ansible (if necessary)"
[ -f "$HOME/bin/ansible/bin/activate" ] && source $HOME/bin/ansible/bin/activate

echo "Check if image present"
gcloud compute images list --filter="name~'ivynet-backend'" | grep  "Listed 0 items." && run_packer || echo " Image present"
